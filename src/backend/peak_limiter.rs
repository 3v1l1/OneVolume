#[derive(Debug, Clone, Copy)]
pub struct PeakLimiterConfig {
    /// Peak level at which limiting begins.
    pub threshold_db: f32,

    /// Maximum additional attenuation applied by the limiter.
    pub max_cut_db: f32,

    /// How quickly the limiter applies attenuation.
    pub attack_seconds: f32,

    /// How quickly the limiter releases after a peak.
    pub release_seconds: f32,
}

impl Default for PeakLimiterConfig {
    fn default() -> Self {
        Self {
            threshold_db: -8.0,
            max_cut_db: 12.0,
            attack_seconds: 0.01,
            release_seconds: 0.50,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PeakLimiter {
    config: PeakLimiterConfig,
    current_gain_db: f32,
}

impl PeakLimiter {
    pub fn new(config: PeakLimiterConfig) -> Self {
        Self {
            config,
            current_gain_db: 0.0,
        }
    }

    /// Process a peak measurement in dBFS.
    ///
    /// The limiter can only attenuate. It never boosts the signal.
    pub fn process(&mut self, peak_db: f32, dt_secs: f32) -> f32 {
        let peak_db = peak_db.clamp(-100.0, 0.0);

        let desired_gain_db = if peak_db > self.config.threshold_db {
            -(peak_db - self.config.threshold_db).min(self.config.max_cut_db)
        } else {
            0.0
        };

        let time_constant = if desired_gain_db < self.current_gain_db {
            self.config.attack_seconds
        } else {
            self.config.release_seconds
        };

        let alpha = if time_constant > 0.0 {
            (dt_secs / time_constant).clamp(0.0, 1.0)
        } else {
            1.0
        };

        self.current_gain_db += (desired_gain_db - self.current_gain_db) * alpha;

        self.current_gain_db
    }

    #[cfg(test)]
    pub fn current_gain_db(&self) -> f32 {
        self.current_gain_db
    }

    pub fn reset(&mut self) {
        self.current_gain_db = 0.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stays_at_unity_below_threshold() {
        let mut limiter = PeakLimiter::new(PeakLimiterConfig::default());

        for _ in 0..20 {
            limiter.process(-15.0, 0.05);
        }

        assert!(limiter.current_gain_db().abs() < 0.01);
    }

    #[test]
    fn attenuates_loud_peak() {
        let mut limiter = PeakLimiter::new(PeakLimiterConfig::default());

        let gain = limiter.process(0.0, 0.02);

        assert!(gain < -5.0);
        assert!(gain >= -12.0);
    }

    #[test]
    fn never_boosts() {
        let mut limiter = PeakLimiter::new(PeakLimiterConfig::default());

        for peak in [-40.0, -20.0, -10.0, -5.0, 0.0] {
            let gain = limiter.process(peak, 0.02);
            assert!(gain <= 0.0);
        }
    }

    #[test]
    fn recovers_after_peak() {
        let mut limiter = PeakLimiter::new(PeakLimiterConfig::default());

        limiter.process(0.0, 0.02);
        let gain_during_peak = limiter.current_gain_db();

        let gain_after_peak = limiter.process(-20.0, 0.15);

        assert!(
            gain_after_peak > gain_during_peak,
            "expected limiter to recover: {gain_after_peak} vs {gain_during_peak}"
        );
    }
}
