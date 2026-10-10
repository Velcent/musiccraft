use crate::{db_to_gain, effect::ProcessContext, gain_to_db};

/// Stereo-linked feed-forward peak compressor.
///
/// Parameters: `[threshold dB, ratio, attack ms, release ms, makeup dB]`.
#[derive(Debug, Clone, Default)]
pub struct Compressor {
    envelope_db: f32,
    gain_reduction_db: f32,
}

impl Compressor {
    /// Creates a compressor with no gain reduction.
    #[must_use]
    pub fn new() -> Self {
        Self {
            envelope_db: -144.0,
            gain_reduction_db: 0.0,
        }
    }

    /// Clears the detector.
    pub fn reset(&mut self) {
        *self = Self::new();
    }

    /// Current gain reduction in dB (≥ 0), for metering.
    #[must_use]
    pub fn gain_reduction_db(&self) -> f32 {
        self.gain_reduction_db
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let threshold = param(0, -18.0);
        let ratio = param(1, 4.0).max(1.0);
        let coeff = |ms: f32| (-1.0 / (ms.max(0.01) * 0.001 * ctx.sample_rate.max(1.0))).exp();
        let attack = coeff(param(2, 10.0));
        let release = coeff(param(3, 120.0));
        let makeup = db_to_gain(param(4, 0.0));
        let slope = 1.0 - 1.0 / ratio;

        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let level_db = gain_to_db(l.abs().max(r.abs()));
            let coeff = if level_db > self.envelope_db {
                attack
            } else {
                release
            };
            self.envelope_db = level_db + (self.envelope_db - level_db) * coeff;
            let over = self.envelope_db - threshold;
            self.gain_reduction_db = if over > 0.0 { over * slope } else { 0.0 };
            let gain = db_to_gain(-self.gain_reduction_db) * makeup;
            *l *= gain;
            *r *= gain;
        }
    }
}

/// Peak limiter with instant attack and exponential release. Output never
/// exceeds the ceiling.
///
/// Parameters: `[input gain dB, ceiling dB, release ms]`.
#[derive(Debug, Clone)]
pub struct Limiter {
    gain: f32,
}

impl Default for Limiter {
    fn default() -> Self {
        Self { gain: 1.0 }
    }
}

impl Limiter {
    /// Current gain reduction in dB (≥ 0), for metering.
    #[must_use]
    pub fn gain_reduction_db(&self) -> f32 {
        -gain_to_db(self.gain)
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let input = db_to_gain(param(0, 0.0));
        let ceiling = db_to_gain(param(1, -0.3).min(0.0));
        let release_ms = param(2, 80.0).max(1.0);
        let release = (-1.0 / (release_ms * 0.001 * ctx.sample_rate.max(1.0))).exp();
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let (xl, xr) = (*l * input, *r * input);
            let peak = xl.abs().max(xr.abs());
            let target = if peak > ceiling { ceiling / peak } else { 1.0 };
            self.gain = if target < self.gain {
                target
            } else {
                target + (self.gain - target) * release
            };
            *l = (xl * self.gain).clamp(-ceiling, ceiling);
            *r = (xr * self.gain).clamp(-ceiling, ceiling);
        }
    }
}

/// Soft-clipping (tanh) distortion.
///
/// Parameters: `[drive dB, mix, output dB]`.
#[derive(Debug, Clone, Default)]
pub struct Distortion;

impl Distortion {
    /// Processes a block in place.
    pub fn process(left: &mut [f32], right: &mut [f32], p: &[f32]) {
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let drive = db_to_gain(param(0, 12.0));
        let mix = param(1, 1.0).clamp(0.0, 1.0);
        let output = db_to_gain(param(2, -6.0));
        for sample in left.iter_mut().chain(right.iter_mut()) {
            let wet = (*sample * drive).tanh();
            *sample = (*sample + (wet - *sample) * mix) * output;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loud_signal_is_reduced_by_ratio() {
        let mut comp = Compressor::new();
        let ctx = ProcessContext {
            sample_rate: 1_000.0,
            tempo_bpm: 120.0,
        };
        // Constant 0 dBFS input, threshold -20, ratio 4 → steady state -15 dB.
        let mut l = vec![1.0; 5_000];
        let mut r = vec![1.0; 5_000];
        comp.process(&mut l, &mut r, &[-20.0, 4.0, 1.0, 50.0, 0.0], ctx);
        let out_db = gain_to_db(l[4_999]);
        assert!((out_db + 15.0).abs() < 0.1, "out {out_db} dB");
        assert!((comp.gain_reduction_db() - 15.0).abs() < 0.1);
    }

    #[test]
    fn quiet_signal_passes_unchanged() {
        let mut comp = Compressor::new();
        let ctx = ProcessContext {
            sample_rate: 1_000.0,
            tempo_bpm: 120.0,
        };
        let mut l = vec![0.01; 100];
        let mut r = vec![0.01; 100];
        comp.process(&mut l, &mut r, &[-20.0, 4.0, 1.0, 50.0, 0.0], ctx);
        assert!((l[99] - 0.01).abs() < 1e-6);
    }

    #[test]
    fn limiter_holds_the_ceiling_and_recovers() {
        let mut limiter = Limiter::default();
        let ctx = ProcessContext {
            sample_rate: 1_000.0,
            tempo_bpm: 120.0,
        };
        let mut l = vec![2.0; 100];
        let mut r = vec![-2.0; 100];
        limiter.process(&mut l, &mut r, &[0.0, -6.0206, 20.0], ctx);
        assert!(l.iter().chain(&r).all(|s| s.abs() <= 0.5 + 1e-4));
        assert!((l[99] - 0.5).abs() < 1e-3);
        assert!((limiter.gain_reduction_db() - 12.04).abs() < 0.05);
        let mut quiet_l = vec![0.1; 500];
        let mut quiet_r = vec![0.1; 500];
        limiter.process(&mut quiet_l, &mut quiet_r, &[0.0, -6.0206, 20.0], ctx);
        assert!((quiet_l[499] - 0.1).abs() < 1e-3, "released back to unity");
    }

    #[test]
    fn distortion_is_bounded_by_output_gain() {
        let mut l = vec![10.0, -10.0, 0.0];
        let mut r = vec![0.5; 3];
        Distortion::process(&mut l, &mut r, &[24.0, 1.0, 0.0]);
        assert!(l.iter().chain(&r).all(|s| s.abs() <= 1.0));
        assert!(l[2].abs() < f32::EPSILON);
    }
}
