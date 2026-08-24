//! Automatic level control ("night mode").
//!
//! This is the brain of OneVolume. It does not touch PipeWire directly —
//! you feed it a loudness measurement (in dBFS) at a steady tick rate,
//! and it hands back the volume multiplier to apply to the stream.
//!
//! The idea:
//!   - Pick a target loudness (roughly where movie dialogue sits).
//!   - When the signal goes louder than that, pull the gain down FAST.
//!   - When the signal is quieter than that, push the gain up SLOWLY.
//!   - Never boost near-silence. Instead, gently decay back toward neutral
//!     so a paused/silent stretch doesn't leave gain stuck wherever it was.

#[derive(Debug, Clone, Copy)]
pub struct LevelerConfig {
    /// Target loudness in dBFS.
    pub target_db: f32,

    /// Maximum amount the gain may reduce loud content, in dB.
    pub max_cut_db: f32,

    /// Maximum amount the gain may boost quiet content, in dB.
    /// Kept modest so whispers don't turn into amplified hiss.
    /// This is also the practical hard ceiling used by capture.rs.
    pub max_boost_db: f32,

    /// Anything quieter than this is treated as silence/noise floor.
    pub gate_threshold_db: f32,

    /// Time constant for reacting to loud content.
    pub attack_seconds: f32,

    /// Time constant for recovering after loud content.
    pub release_seconds: f32,

    /// Time constant for returning toward neutral (0 dB) during silence.
    pub silence_decay_seconds: f32,
}

impl Default for LevelerConfig {
    fn default() -> Self {
        Self {
            target_db: -20.0,
            max_cut_db: 18.0,
            max_boost_db: 6.0,
            gate_threshold_db: -55.0,
            attack_seconds: 0.15,
            release_seconds: 2.5,
            silence_decay_seconds: 3.0,
        }
    }
}

pub struct Leveler {
    config: LevelerConfig,

    /// Current smoothed gain, in dB.
    /// 0 dB = no change.
    current_gain_db: f32,
}

impl Leveler {
    pub fn new(config: LevelerConfig) -> Self {
        Self {
            config,
            current_gain_db: 0.0,
        }
    }

    /// Feed one loudness measurement and get back the gain to apply.
    ///
    /// `level_db` — measured loudness of the current audio window, in dBFS.
    ///
    /// `dt_secs` — time in seconds since the last call.
    ///
    /// Returns the smoothed gain in dB. Convert with `db_to_linear`
    /// to get the multiplier to apply to the stream's volume.
    pub fn process(&mut self, level_db: f32, dt_secs: f32) -> f32 {
        let cfg = &self.config;

        let is_silent = level_db < cfg.gate_threshold_db;

        let (desired_gain_db, time_constant) = if is_silent {
            // Silence/gap: decay toward neutral rather than freezing
            // at whatever gain was last active.
            (0.0, cfg.silence_decay_seconds)
        } else {
            let error_db = cfg.target_db - level_db;
            let desired = error_db.clamp(-cfg.max_cut_db, cfg.max_boost_db);

            // Fast attack when we need less gain than we currently have
            // (something got loud).
            // Slow release when we need more gain
            // (audio is calming down or getting quiet).
            let time_constant = if desired < self.current_gain_db {
                cfg.attack_seconds
            } else {
                cfg.release_seconds
            };

            (desired, time_constant)
        };

        // Exponential approach toward the desired gain.
        let alpha = if time_constant > 0.0 {
            (dt_secs / time_constant).clamp(0.0, 1.0)
        } else {
            1.0
        };

        self.current_gain_db += (desired_gain_db - self.current_gain_db) * alpha;

        self.current_gain_db
    }

    /// Current smoothed gain in dB.
    /// Used by diagnostics/tests.
    #[allow(dead_code)]
    pub fn current_gain_db(&self) -> f32 {
        self.current_gain_db
    }

    /// Reset to unity gain, e.g. when playback stops or a new stream starts.
    pub fn reset(&mut self) {
        self.current_gain_db = 0.0;
    }
}

/// Convert a dB value to a linear multiplier (1.0 = unity).
pub fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// Compute the RMS loudness of an interleaved f32 sample buffer, in dBFS.
/// Returns a very low number (effectively -inf, clamped) for silence.
pub fn rms_dbfs(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return -100.0;
    }

    let sum_sq: f64 = samples.iter().map(|s| (*s as f64) * (*s as f64)).sum();

    let mean_sq = sum_sq / samples.len() as f64;
    let rms = mean_sq.sqrt();

    if rms <= 0.0 {
        -100.0
    } else {
        (20.0 * rms.log10() as f32).max(-100.0)
    }
}

/// The single loudest sample in this buffer, in dBFS.
pub fn peak_dbfs(samples: &[f32]) -> f32 {
    if samples.is_empty() {
        return -100.0;
    }

    let peak = samples.iter().fold(0.0f32, |max, s| max.max(s.abs()));

    if peak <= 0.0 {
        -100.0
    } else {
        (20.0 * peak.log10()).max(-100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn holds_unity_gain_at_target_level() {
        let mut leveler = Leveler::new(LevelerConfig::default());
        let target = LevelerConfig::default().target_db;

        for _ in 0..50 {
            leveler.process(target, 0.05);
        }

        assert!(leveler.current_gain_db().abs() < 0.5);
    }

    #[test]
    fn pulls_down_fast_on_loud_blast() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        // One tick of a very loud blast (0 dBFS).
        let gain = leveler.process(0.0, 0.15);

        // Attack time constant is 0.15s, so after exactly one attack
        // period we should have moved most of the way toward the max cut.
        assert!(gain < -10.0, "expected a strong cut, got {gain}");
    }

    #[test]
    fn does_not_boost_silence() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        for _ in 0..100 {
            leveler.process(-90.0, 0.05);
        }

        assert!(
            leveler.current_gain_db().abs() < 0.01,
            "silence should not be boosted, got {}",
            leveler.current_gain_db()
        );
    }

    #[test]
    fn decays_to_neutral_during_silence() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        // Get gain up to a non-zero value first.
        for _ in 0..200 {
            leveler.process(-35.0, 0.1);
        }

        let gain_before_silence = leveler.current_gain_db();

        assert!(
            gain_before_silence > 1.0,
            "expected meaningful boost before silence test, got {gain_before_silence}"
        );

        // Silence should immediately start pulling gain down.
        let gain_after_one_tick = leveler.process(-90.0, 0.1);

        assert!(
            gain_after_one_tick < gain_before_silence,
            "expected gain to start decaying during silence, got \
             {gain_after_one_tick} (was {gain_before_silence})"
        );

        // Sustained silence should eventually return to neutral.
        for _ in 0..200 {
            leveler.process(-90.0, 0.1);
        }

        assert!(
            leveler.current_gain_db().abs() < 0.1,
            "expected gain to decay to ~0dB after sustained silence, got {}",
            leveler.current_gain_db()
        );
    }

    #[test]
    fn boosts_quiet_dialogue_gradually() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        // Quiet dialogue at -35 dBFS, target is -20 dBFS.
        let gain_after_one_tick = leveler.process(-35.0, 0.1);

        assert!(gain_after_one_tick > 0.0 && gain_after_one_tick < 2.0);

        for _ in 0..200 {
            leveler.process(-35.0, 0.1);
        }

        // Error is +15dB, clamped to the +6dB maximum boost.
        assert!((leveler.current_gain_db() - 6.0).abs() < 0.1);
    }

    #[test]
    fn db_to_linear_unity() {
        assert!((db_to_linear(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn peak_dbfs_finds_a_single_loud_sample() {
        // Mostly quiet, one sharp transient — the kind of event
        // RMS can hide.
        let mut samples = vec![0.01f32; 1000];
        samples[500] = 0.9;

        let peak = peak_dbfs(&samples);
        let rms = rms_dbfs(&samples);

        assert!(
            peak > -1.0,
            "expected peak near 0dBFS for a 0.9 sample, got {peak}"
        );

        assert!(
            peak > rms + 20.0,
            "peak ({peak}) should be dramatically higher than RMS ({rms}) \
             for a single transient in near-silence"
        );
    }

    #[test]
    fn peak_dbfs_silence_floor() {
        assert_eq!(peak_dbfs(&[]), -100.0);
        assert_eq!(peak_dbfs(&[0.0, 0.0, 0.0]), -100.0);
    }
}
