use std::f64::consts::PI;

/// Second-order Butterworth high-pass filter used only by the
/// Stage 1 loudness detector.
///
/// The actual audio samples are never modified by this filter.
#[derive(Debug, Clone, Copy)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    z1: f64,
    z2: f64,
}

impl Biquad {
    fn high_pass(sample_rate_hz: f64, cutoff_hz: f64) -> Self {
        let omega = 2.0 * PI * cutoff_hz / sample_rate_hz;
        let cos_omega = omega.cos();
        let sin_omega = omega.sin();

        let q = 1.0 / 2.0_f64.sqrt();
        let alpha = sin_omega / (2.0 * q);

        let b0 = (1.0 + cos_omega) / 2.0;
        let b1 = -(1.0 + cos_omega);
        let b2 = b0;

        let a0 = 1.0 + alpha;
        let a1 = -2.0 * cos_omega;
        let a2 = 1.0 - alpha;

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn process(&mut self, input: f64) -> f64 {
        // Direct Form II Transposed.
        let output = self.b0 * input + self.z1;

        self.z1 = self.b1 * input - self.a1 * output + self.z2;
        self.z2 = self.b2 * input - self.a2 * output;

        output
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

/// Detector-only high-pass filter.
///
/// One biquad is maintained per channel so channel state does not bleed
/// between left and right audio.
#[derive(Debug, Clone)]
pub struct SidechainFilter {
    sample_rate_hz: u32,
    cutoff_hz: f64,
    filters: Vec<Biquad>,
}

impl SidechainFilter {
    pub fn new(sample_rate_hz: u32, channels: usize, cutoff_hz: f64) -> Self {
        let filters = (0..channels)
            .map(|_| Biquad::high_pass(sample_rate_hz as f64, cutoff_hz))
            .collect();

        Self {
            sample_rate_hz,
            cutoff_hz,
            filters,
        }
    }

    pub fn reconfigure(&mut self, sample_rate_hz: u32, channels: usize) {
        if self.sample_rate_hz == sample_rate_hz && self.filters.len() == channels {
            return;
        }

        *self = Self::new(sample_rate_hz, channels, self.cutoff_hz);
    }

    pub fn reset(&mut self) {
        for filter in &mut self.filters {
            filter.reset();
        }
    }

    /// Filter an interleaved F32 buffer and return its mean-square power.
    ///
    /// This assumes `samples` contains `channels` interleaved channels.
    /// The input slice is never modified.
    pub fn mean_square(&mut self, samples: &[f32], channels: usize) -> f64 {
        if channels == 0 || self.filters.len() != channels || samples.is_empty() {
            return 0.0;
        }

        let mut sum_sq = 0.0_f64;

        for (index, sample) in samples.iter().enumerate() {
            let channel = index % channels;
            let filtered = self.filters[channel].process(*sample as f64);
            sum_sq += filtered * filtered;
        }

        sum_sq / samples.len() as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn preserves_high_frequency_signal() {
        let mut filter = SidechainFilter::new(48_000, 1, 120.0);

        let mut sum_sq = 0.0;
        let sample_count = 48_000;

        for n in 0..sample_count {
            let t = n as f64 / 48_000.0;
            let sample = (2.0 * PI * 1_000.0 * t).sin() as f32;
            let output = filter.filters[0].process(sample as f64);

            if n > 1_000 {
                sum_sq += output * output;
            }
        }

        let rms = (sum_sq / (sample_count - 1_000) as f64).sqrt();

        assert!(
            rms > 0.65,
            "1 kHz signal should largely pass the 120 Hz high-pass, got RMS {rms}"
        );
    }

    #[test]
    fn strongly_reduces_low_frequency_signal() {
        let mut filter = SidechainFilter::new(48_000, 1, 120.0);

        let mut sum_sq = 0.0;
        let sample_count = 48_000;

        for n in 0..sample_count {
            let t = n as f64 / 48_000.0;
            let sample = (2.0 * PI * 40.0 * t).sin() as f32;
            let output = filter.filters[0].process(sample as f64);

            if n > 1_000 {
                sum_sq += output * output;
            }
        }

        let rms = (sum_sq / (sample_count - 1_000) as f64).sqrt();

        assert!(
            rms < 0.15,
            "40 Hz signal should be strongly attenuated by the 120 Hz high-pass, got RMS {rms}"
        );
    }

    #[test]
    fn reset_clears_filter_state() {
        let mut filter = SidechainFilter::new(48_000, 2, 120.0);

        let _ = filter.mean_square(&[1.0, 1.0, 1.0, 1.0], 2);
        filter.reset();

        assert_eq!(filter.filters[0].z1, 0.0);
        assert_eq!(filter.filters[0].z2, 0.0);
        assert_eq!(filter.filters[1].z1, 0.0);
        assert_eq!(filter.filters[1].z2, 0.0);
    }

    #[test]
    fn reconfigure_updates_sample_rate_and_channel_count() {
        let mut filter = SidechainFilter::new(48_000, 2, 120.0);

        filter.reconfigure(44_100, 1);

        assert_eq!(filter.sample_rate_hz, 44_100);
        assert_eq!(filter.filters.len(), 1);
    }

    #[test]
    fn mean_square_handles_interleaved_channels() {
        let mut filter = SidechainFilter::new(48_000, 2, 120.0);

        let samples = [0.5_f32, -0.5, 0.5, -0.5];
        let power = filter.mean_square(&samples, 2);

        assert!(power > 0.0);
    }
}
