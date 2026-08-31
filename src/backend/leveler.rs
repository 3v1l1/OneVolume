#[derive(Debug, Clone, Copy)]
pub struct LevelerConfig {
    /// Desired long-term loudness in dBFS.
    pub target_db: f32,

    /// Maximum attenuation for sustained loud content.
    pub max_cut_db: f32,

    /// Maximum boost for quiet but non-silent content.
    pub max_boost_db: f32,

    /// Below this level, treat the signal as silence/noise floor.
    pub gate_threshold_db: f32,

    /// How quickly sustained loud content is pulled down.
    pub attack_seconds: f32,

    /// How slowly the leveler recovers toward more gain.
    pub release_seconds: f32,

    /// How quickly gain returns toward 0 dB during silence.
    pub silence_decay_seconds: f32,
}

impl Default for LevelerConfig {
    fn default() -> Self {
        Self {
            target_db: -21.5,

            // Main leveler handles sustained scene differences.
            max_cut_db: 15.0,

            // Quiet dialogue is now allowed to come up substantially.
            max_boost_db: 4.0,

            // Do not amplify true silence/noise floor.
            gate_threshold_db: -55.0,

            // Smooth scene transitions.
            attack_seconds: 0.75,
            release_seconds: 2.0,

            // Return toward unity during actual silence.
            silence_decay_seconds: 1.5,
        }
    }
}

pub struct Leveler {
    config: LevelerConfig,

    current_gain_db: f32,
}

impl Leveler {
    pub fn new(config: LevelerConfig) -> Self {
        Self {
            config,
            current_gain_db: 0.0,
        }
    }

    /// Process a short-term loudness measurement and return gain in dB.
    ///
    /// Quiet non-silent material is boosted.
    /// Sustained loud material is attenuated.
    /// Silence is never boosted.
    ///
    /// Positive gain is reduced when the current peak indicates that the
    /// quiet RMS is accompanied by strong music/effects/transients.
    pub fn process(&mut self, level_db: f32, peak_db: f32, dt_secs: f32) -> f32 {
        let cfg = &self.config;
        let level_db = level_db.clamp(-100.0, 0.0);
        let peak_db = peak_db.clamp(-100.0, 0.0);

        let (desired_gain_db, time_constant) = if level_db < cfg.gate_threshold_db {
            // Actual silence: never amplify it.
            (0.0, cfg.silence_decay_seconds)
        } else {
            let error_db = cfg.target_db - level_db;

            let mut desired = error_db.clamp(-cfg.max_cut_db, cfg.max_boost_db);

            // Apply slightly stronger correction only when sustained content
            // is above the target. Positive dialogue/content boost remains
            // governed by the existing crest-factor logic.
            if desired < 0.0 {
                desired *= 1.35;
                desired = desired.max(-cfg.max_cut_db);
            }

            // Only limit positive boost. Sustained attenuation is left
            // unchanged and continues to be handled by the leveler.
            if desired > 0.0 && peak_db > -100.0 {
                // Compare the instantaneous peak with the smoothed RMS
                // estimate. A large crest factor is more consistent with
                // speech/transient material; a small crest factor suggests
                // dense music or sustained effects, where boosting the whole
                // mix is less desirable.
                let crest_db = (peak_db - level_db).max(0.0);

                // Keep boost very conservative for dense material.
                // Low crest factors are more likely to represent sustained
                // music/effects, while larger crest factors leave more room
                // for useful quiet-content/dialogue boost.
                let boost_factor = if crest_db <= 4.0 {
                    0.0
                } else if crest_db < 8.0 {
                    0.25 * (crest_db - 4.0) / 4.0
                } else if crest_db < 12.0 {
                    0.25 + 0.35 * (crest_db - 8.0) / 4.0
                } else if crest_db < 16.0 {
                    0.60 + 0.40 * (crest_db - 12.0) / 4.0
                } else {
                    1.0
                };

                let boost_limit = cfg.max_boost_db * boost_factor;

                desired = desired.min(boost_limit);
            }

            // Moving toward more attenuation = attack.
            // Moving toward more boost = release.
            let time_constant = if desired < self.current_gain_db {
                cfg.attack_seconds
            } else {
                cfg.release_seconds
            };

            (desired, time_constant)
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

/// Convert dB to a linear multiplier.
pub fn db_to_linear(db: f32) -> f32 {
    10f32.powf(db / 20.0)
}

/// The loudest sample in a buffer, in dBFS.
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

        for _ in 0..100 {
            leveler.process(-22.5, -22.5, 0.05);
        }

        assert!(leveler.current_gain_db().abs() < 0.1);
    }

    #[test]
    fn boosts_quiet_dialogue() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        for _ in 0..100 {
            leveler.process(-35.0, -15.0, 0.1);
        }

        assert!(
            leveler.current_gain_db() > 2.0,
            "quiet dialogue should receive meaningful boost, got {} dB",
            leveler.current_gain_db()
        );

        assert!(leveler.current_gain_db() <= 4.0);
    }

    #[test]
    fn attack_moves_gradually_with_small_dt() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        let first = leveler.process(0.0, 0.0, 0.05);
        let second = leveler.process(0.0, 0.0, 0.05);

        assert!(first > -1.01, "first step should be gradual, got {first}");
        assert!(
            second < first,
            "gain should continue moving toward attenuation"
        );
        assert!(
            second > -3.0,
            "attack should not jump several dB in 50ms, got {second}"
        );
    }

    #[test]
    fn reduces_boost_for_dense_quiet_content() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        for _ in 0..100 {
            leveler.process(-35.0, -32.0, 0.1);
        }

        assert!(
            leveler.current_gain_db() < 3.0,
            "dense quiet content should receive limited boost, got {} dB",
            leveler.current_gain_db()
        );
    }

    #[test]
    fn suppresses_boost_for_low_crest_content() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        for _ in 0..100 {
            leveler.process(-35.0, -25.0, 0.1);
        }

        assert!(
            leveler.current_gain_db() < 2.5,
            "low-crest quiet content should receive conservative boost, got {} dB",
            leveler.current_gain_db()
        );
    }

    #[test]
    fn strongly_limits_boost_for_dense_material() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        for _ in 0..100 {
            leveler.process(-35.0, -30.0, 0.1);
        }

        assert!(
            leveler.current_gain_db() < 1.1,
            "dense material should receive very little boost, got {} dB",
            leveler.current_gain_db()
        );
    }

    #[test]
    fn allows_more_boost_as_crest_increases() {
        let mut dense = Leveler::new(LevelerConfig::default());
        let mut speech_like = Leveler::new(LevelerConfig::default());

        for _ in 0..100 {
            dense.process(-35.0, -28.0, 0.1);
            speech_like.process(-35.0, -19.0, 0.1);
        }

        assert!(
            speech_like.current_gain_db() > dense.current_gain_db(),
            "higher crest factor should allow more boost"
        );
        assert!(speech_like.current_gain_db() <= 4.0);
    }

    #[test]
    fn preserves_more_boost_for_high_crest_content() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        for _ in 0..100 {
            leveler.process(-35.0, -15.0, 0.1);
        }

        assert!(
            leveler.current_gain_db() > 3.0,
            "high-crest quiet content should retain strong boost, got {} dB",
            leveler.current_gain_db()
        );
        assert!(leveler.current_gain_db() <= 4.0);
    }

    #[test]
    fn suppresses_sustained_loud_content() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        for _ in 0..100 {
            leveler.process(-5.0, -5.0, 0.1);
        }

        assert!(
            leveler.current_gain_db() < -10.0,
            "loud content should be strongly attenuated, got {} dB",
            leveler.current_gain_db()
        );
    }

    #[test]
    fn does_not_boost_silence() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        for _ in 0..100 {
            leveler.process(-90.0, -90.0, 0.1);
        }

        assert!(
            leveler.current_gain_db().abs() < 0.01,
            "silence should not be boosted, got {}",
            leveler.current_gain_db()
        );
    }

    #[test]
    fn recovers_to_neutral_during_silence() {
        let mut leveler = Leveler::new(LevelerConfig::default());

        for _ in 0..100 {
            leveler.process(-5.0, -5.0, 0.1);
        }

        let before = leveler.current_gain_db();

        for _ in 0..200 {
            leveler.process(-90.0, -90.0, 0.1);
        }

        assert!(before < -1.0);
        assert!(
            leveler.current_gain_db().abs() < 0.1,
            "expected recovery to unity, got {}",
            leveler.current_gain_db()
        );
    }

    #[test]
    fn db_to_linear_unity() {
        assert!((db_to_linear(0.0) - 1.0).abs() < 1e-6);
    }

    #[test]
    fn peak_dbfs_finds_single_loud_sample() {
        let mut samples = vec![0.01f32; 1000];
        samples[500] = 0.9;

        let peak = peak_dbfs(&samples);

        assert!((peak - (20.0 * 0.9f32.log10())).abs() < 0.01);
    }

    #[test]
    fn peak_dbfs_silence_floor() {
        assert_eq!(peak_dbfs(&[]), -100.0);
        assert_eq!(peak_dbfs(&[0.0, 0.0, 0.0]), -100.0);
    }
}
