//! Mastering effects: the three-band limiter and the band splitter. All
//! allocation happens in `new`.

use crate::creative::Split;
use crate::{Limiter, ProcessContext, db_to_gain};

fn param(p: &[f32], i: usize, default: f32) -> f32 {
    p.get(i).copied().unwrap_or(default)
}

/// One band of the multiband limiter for a block.
#[derive(Debug, Clone, Copy)]
struct Band {
    drive: f32,
    ceiling: f32,
    level: f32,
}

/// Three-band limiter: the multiband compressor's Linkwitz–Riley split, a
/// peak limiter per band (drive into a ceiling, then a level), the bands
/// summed, and a master limiter so the output never exceeds its ceiling.
///
/// Parameters: `[low/mid Hz, mid/high Hz, (drive dB, ceiling dB, level dB) ×
/// 3, release ms, amount, ceiling dB]`. `amount` scales every band's drive,
/// so one knob goes from untouched (0) to fully driven (1).
#[derive(Debug, Clone)]
pub struct MultibandLimiter {
    split: [Split; 2],
    gains: [f32; 3],
    master: Limiter,
}

impl Default for MultibandLimiter {
    fn default() -> Self {
        Self {
            split: [Split::default(), Split::default()],
            gains: [1.0; 3],
            master: Limiter::default(),
        }
    }
}

impl MultibandLimiter {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let sr = ctx.sample_rate.max(1.0);
        for split in &mut self.split {
            split.set(param(p, 0, 200.0), param(p, 1, 2_500.0), sr);
        }
        let amount = param(p, 12, 1.0).clamp(0.0, 1.0);
        let bands: [Band; 3] = std::array::from_fn(|b| Band {
            drive: db_to_gain(param(p, 2 + b * 3, 0.0) * amount),
            ceiling: db_to_gain(param(p, 3 + b * 3, 0.0).min(0.0)),
            level: db_to_gain(param(p, 4 + b * 3, 0.0)),
        });
        let release_ms = param(p, 11, 100.0).max(1.0);
        let release = (-1.0 / (release_ms * 0.001 * sr)).exp();
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let [split_l, split_r] = &mut self.split;
            let (lb, rb) = (split_l.process(*l), split_r.process(*r));
            let mut out = [0.0_f32; 2];
            for (((band, gain), x), y) in bands.iter().zip(&mut self.gains).zip(lb).zip(rb) {
                let (x, y) = (x * band.drive, y * band.drive);
                let peak = x.abs().max(y.abs());
                let target = if peak > band.ceiling {
                    band.ceiling / peak
                } else {
                    1.0
                };
                *gain = if target < *gain {
                    target
                } else {
                    target + (*gain - target) * release
                };
                out[0] += x * *gain * band.level;
                out[1] += y * *gain * band.level;
            }
            *l = out[0];
            *r = out[1];
        }
        let master = [0.0, param(p, 13, -0.3), release_ms];
        self.master.process(left, right, &master, ctx);
    }

    /// Clears the filters and limiters.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

/// Band splitter: the multiband split (Linkwitz–Riley, bands sum flat) whose
/// bands the engine sends to other inserts. On its own (offline, or with no
/// targets) the bands are summed back, which leaves the sound unchanged
/// apart from the crossovers' phase.
///
/// Parameters: `[low/mid Hz, mid/high Hz, low target, mid target, high
/// target]`.
#[derive(Debug, Clone, Default)]
pub struct BandSplitter {
    split: [Split; 2],
}

impl BandSplitter {
    /// Sets the crossovers for a block.
    pub fn prepare(&mut self, p: &[f32], sample_rate: f32) {
        for split in &mut self.split {
            split.set(param(p, 0, 200.0), param(p, 1, 2_500.0), sample_rate);
        }
    }

    /// Splits one stereo frame into `[low, mid, high]`, each `[left, right]`.
    pub fn frame(&mut self, l: f32, r: f32) -> [[f32; 2]; 3] {
        let [split_l, split_r] = &mut self.split;
        let (lb, rb) = (split_l.process(l), split_r.process(r));
        [[lb[0], rb[0]], [lb[1], rb[1]], [lb[2], rb[2]]]
    }

    /// Processes a block in place with every band kept.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        self.prepare(p, ctx.sample_rate);
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let bands = self.frame(*l, *r);
            *l = bands.iter().map(|b| b[0]).sum();
            *r = bands.iter().map(|b| b[1]).sum();
        }
    }

    /// Clears the filters.
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_params::EffectKind;

    fn ctx() -> ProcessContext {
        ProcessContext {
            sample_rate: 48_000.0,
            tempo_bpm: 120.0,
            beats: 0.0,
        }
    }

    fn sine(hz: f32, n: usize, level: f32) -> Vec<f32> {
        (0..n)
            .map(|i| (i as f32 * hz * std::f32::consts::TAU / 48_000.0).sin() * level)
            .collect()
    }

    fn peak(x: &[f32]) -> f32 {
        x.iter().fold(0.0_f32, |m, s| m.max(s.abs()))
    }

    #[test]
    fn quiet_input_passes_at_unity() {
        let mut fx = MultibandLimiter::default();
        let p = EffectKind::MultibandLimiter.defaults();
        let mut l = sine(1_000.0, 24_000, 0.25);
        let mut r = l.clone();
        fx.process(&mut l, &mut r, &p, ctx());
        let got = peak(&l[12_000..]);
        assert!((got - 0.25).abs() < 0.01, "{got}");
    }

    #[test]
    fn output_never_exceeds_the_ceiling() {
        let mut fx = MultibandLimiter::default();
        let mut p = EffectKind::MultibandLimiter.defaults();
        for b in 0..3 {
            p[2 + b * 3] = 18.0;
        }
        let mut l: Vec<f32> = sine(80.0, 24_000, 0.9)
            .iter()
            .zip(sine(5_000.0, 24_000, 0.5))
            .map(|(a, b)| a + b)
            .collect();
        let mut r = l.clone();
        fx.process(&mut l, &mut r, &p, ctx());
        assert!(peak(&l) <= db_to_gain(-0.3) + 1e-6);
    }

    #[test]
    fn a_band_ceiling_squashes_only_its_band() {
        let run = |hz: f32| {
            let mut fx = MultibandLimiter::default();
            let mut p = EffectKind::MultibandLimiter.defaults();
            p[3] = -18.0; // low band ceiling
            let mut l = sine(hz, 24_000, 0.5);
            let mut r = l.clone();
            fx.process(&mut l, &mut r, &p, ctx());
            peak(&l[12_000..])
        };
        assert!(run(60.0) < 0.15, "low band held down");
        assert!(run(1_000.0) > 0.45, "mids untouched");
    }

    #[test]
    fn band_splitter_bands_sum_back_and_separate() {
        let mut fx = BandSplitter::default();
        fx.prepare(&[200.0, 2_500.0], 48_000.0);
        let low = sine(60.0, 24_000, 0.5);
        let mut energy = [0.0_f32; 3];
        for (i, x) in low.iter().enumerate() {
            let bands = fx.frame(*x, *x);
            if i > 12_000 {
                for (e, b) in energy.iter_mut().zip(bands) {
                    *e += b[0] * b[0];
                }
            }
        }
        assert!(energy[0] > 100.0 * (energy[1] + energy[2]), "{energy:?}");
        let mut fx = BandSplitter::default();
        let input = sine(1_000.0, 24_000, 0.5);
        let (mut l, mut r) = (input.clone(), input.clone());
        fx.process(&mut l, &mut r, &[200.0, 2_500.0, 0.0, 0.0, 0.0], ctx());
        assert!((peak(&l[12_000..]) - 0.5).abs() < 0.01, "flat sum");
    }

    #[test]
    fn amount_zero_removes_the_drive() {
        let mut fx = MultibandLimiter::default();
        let mut p = EffectKind::MultibandLimiter.defaults();
        p[5] = 12.0;
        p[12] = 0.0;
        let mut l = sine(1_000.0, 24_000, 0.25);
        let mut r = l.clone();
        fx.process(&mut l, &mut r, &p, ctx());
        assert!((peak(&l[12_000..]) - 0.25).abs() < 0.01);
    }
}
