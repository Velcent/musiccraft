//! The effect rack: up to four parallel chains of built-in effects inside one
//! insert slot, mixed with the dry signal (see `docs/spec/mixer.md`).

use musiccraft_params::MAX_EFFECT_PARAMS;
pub use musiccraft_params::{RACK_BRANCH_EFFECTS, RACK_BRANCHES};

use crate::{Effect, ProcessContext};

/// Samples processed at a time; longer blocks are split.
const CHUNK: usize = 256;

/// One effect in a branch, with its settings.
#[derive(Debug, Clone)]
pub struct RackItem {
    /// The effect and its state.
    pub effect: Effect,
    /// When false the effect is bypassed.
    pub enabled: bool,
    /// Parameter values in the order of the effect's parameter table.
    pub values: [f32; MAX_EFFECT_PARAMS],
}

/// Parallel effect chains mixed with the dry signal.
///
/// Parameters: `[dry, branch1 level, branch1 pan, … branch4 level, branch4
/// pan, output dB]`.
#[derive(Debug, Clone)]
pub struct Rack {
    branches: [Vec<RackItem>; RACK_BRANCHES],
    /// The input of the current chunk.
    input: [Vec<f32>; 2],
    /// One branch's signal as it moves down the chain.
    work: [Vec<f32>; 2],
    /// The branches' sum.
    sum: [Vec<f32>; 2],
}

impl Default for Rack {
    fn default() -> Self {
        Self::new(Default::default())
    }
}

impl Rack {
    /// A rack of `branches`, allocating its buffers. Main thread only.
    #[must_use]
    pub fn new(branches: [Vec<RackItem>; RACK_BRANCHES]) -> Self {
        let buffer = || [vec![0.0; CHUNK], vec![0.0; CHUNK]];
        Self {
            branches,
            input: buffer(),
            work: buffer(),
            sum: buffer(),
        }
    }

    /// Number of effects on `branch`.
    #[must_use]
    pub fn len(&self, branch: usize) -> usize {
        self.branches.get(branch).map_or(0, Vec::len)
    }

    /// Updates the settings of effect `index` on `branch`; does nothing if
    /// there is no such effect.
    pub fn set(
        &mut self,
        branch: usize,
        index: usize,
        enabled: bool,
        values: [f32; MAX_EFFECT_PARAMS],
    ) {
        if let Some(item) = self.branches.get_mut(branch).and_then(|b| b.get_mut(index)) {
            item.enabled = enabled;
            item.values = values;
        }
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let param = |i: usize, default: f32| {
            p.get(i)
                .copied()
                .filter(|v| v.is_finite())
                .unwrap_or(default)
        };
        let dry = param(0, 1.0).clamp(0.0, 1.0);
        let gains: [(f32, f32); RACK_BRANCHES] = std::array::from_fn(|b| {
            let level = param(1 + b * 2, 1.0).clamp(0.0, 1.25);
            let (l, r) = crate::pan_gains(param(2 + b * 2, 0.0).clamp(-1.0, 1.0));
            // Balance: the centre passes at unity.
            let centre = crate::pan_gains(0.0);
            (level * l / centre.0, level * r / centre.1)
        });
        let output = 10.0_f32.powf(param(9, 0.0).clamp(-24.0, 12.0) / 20.0);
        let seconds_per_sample = 1.0 / f64::from(ctx.sample_rate.max(1.0));
        let mut done = 0;
        for (l, r) in left.chunks_mut(CHUNK).zip(right.chunks_mut(CHUNK)) {
            let beats = ctx.beats + done as f64 * seconds_per_sample * ctx.tempo_bpm / 60.0;
            let chunk = ProcessContext { beats, ..ctx };
            self.chunk(l, r, dry, &gains, output, chunk);
            done += l.len();
        }
    }

    fn chunk(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        dry: f32,
        gains: &[(f32, f32); RACK_BRANCHES],
        output: f32,
        ctx: ProcessContext,
    ) {
        let n = left.len().min(right.len()).min(CHUNK);
        let [in_l, in_r] = &mut self.input;
        let [work_l, work_r] = &mut self.work;
        let [sum_l, sum_r] = &mut self.sum;
        let (Some(in_l), Some(in_r)) = (in_l.get_mut(..n), in_r.get_mut(..n)) else {
            return;
        };
        let (Some(work_l), Some(work_r)) = (work_l.get_mut(..n), work_r.get_mut(..n)) else {
            return;
        };
        let (Some(sum_l), Some(sum_r)) = (sum_l.get_mut(..n), sum_r.get_mut(..n)) else {
            return;
        };
        in_l.copy_from_slice(left.get(..n).unwrap_or_default());
        in_r.copy_from_slice(right.get(..n).unwrap_or_default());
        sum_l.fill(0.0);
        sum_r.fill(0.0);
        for (branch, (gl, gr)) in self.branches.iter_mut().zip(gains) {
            if branch.is_empty() {
                continue;
            }
            work_l.copy_from_slice(in_l);
            work_r.copy_from_slice(in_r);
            for item in branch.iter_mut().filter(|i| i.enabled) {
                item.effect.process(work_l, work_r, &item.values, ctx);
            }
            for (s, w) in sum_l.iter_mut().zip(work_l.iter()) {
                *s += w * gl;
            }
            for (s, w) in sum_r.iter_mut().zip(work_r.iter()) {
                *s += w * gr;
            }
        }
        for (out, (x, s)) in left.iter_mut().zip(in_l.iter().zip(sum_l.iter())) {
            *out = (x * dry + s) * output;
        }
        for (out, (x, s)) in right.iter_mut().zip(in_r.iter().zip(sum_r.iter())) {
            *out = (x * dry + s) * output;
        }
    }

    /// Clears every branch effect's memory.
    pub fn reset(&mut self) {
        for item in self.branches.iter_mut().flatten() {
            item.effect.reset();
        }
    }
}

#[cfg(test)]
mod tests {
    use musiccraft_params::EffectKind;

    use super::*;

    const CTX: ProcessContext = ProcessContext {
        sample_rate: 48_000.0,
        tempo_bpm: 120.0,
        beats: 0.0,
    };

    fn item(kind: EffectKind, values: &[(usize, f32)]) -> RackItem {
        let mut v = kind.defaults();
        for (i, x) in values {
            v[*i] = *x;
        }
        RackItem {
            effect: Effect::new(kind, CTX.sample_rate),
            enabled: true,
            values: v,
        }
    }

    fn params(dry: f32) -> [f32; MAX_EFFECT_PARAMS] {
        let mut p = EffectKind::Rack.defaults();
        p[0] = dry;
        p
    }

    fn noise(n: usize) -> Vec<f32> {
        let mut x = 1_u32;
        (0..n)
            .map(|_| {
                x = x.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (x >> 8) as f32 / (1 << 24) as f32 - 0.5
            })
            .collect()
    }

    #[test]
    fn an_empty_rack_passes_audio_unchanged() {
        let mut rack = Rack::default();
        let input = noise(1_000);
        let (mut l, mut r) = (input.clone(), input.clone());
        rack.process(&mut l, &mut r, &params(1.0), CTX);
        assert_eq!(l, input);
        assert_eq!(r, input);
        rack.process(&mut l, &mut r, &params(0.0), CTX);
        assert!(
            l.iter().all(|s| *s == 0.0),
            "no dry and no branches is silent"
        );
    }

    #[test]
    fn branches_run_in_parallel_and_sum_with_the_dry_signal() {
        // A flat equalizer is linear, so two branches of it double the level.
        let flat = || item(EffectKind::Eq, &[]);
        let mut a = Rack::new([vec![flat()], vec![flat()], vec![], vec![]]);
        let input = noise(4_096);
        let (mut l, mut r) = (input.clone(), input.clone());
        a.process(&mut l, &mut r, &params(0.0), CTX);
        let mut b = Rack::new([vec![flat()], vec![], vec![], vec![]]);
        let (mut l1, mut r1) = (input.clone(), input.clone());
        b.process(&mut l1, &mut r1, &params(0.0), CTX);
        for (two, one) in l.iter().zip(&l1) {
            assert!((two - 2.0 * one).abs() < 1e-5, "two equal branches double");
        }
    }

    #[test]
    fn branch_level_pan_bypass_and_output_apply() {
        let lowpass = || item(EffectKind::Filter, &[(1, 200.0)]);
        let mut rack = Rack::new([vec![lowpass()], vec![], vec![], vec![]]);
        let input = noise(4_096);
        let mut p = params(0.0);
        p[2] = 1.0; // branch 1 hard right
        let (mut l, mut r) = (input.clone(), input.clone());
        rack.process(&mut l, &mut r, &p, CTX);
        assert!(
            l.iter().all(|s| s.abs() < 1e-6),
            "panned away from the left"
        );
        let energy = |x: &[f32]| x.iter().map(|s| s * s).sum::<f32>();
        assert!(energy(&r) < energy(&input) * 0.2, "the lowpass ran");

        rack.set(0, 0, false, EffectKind::Filter.defaults());
        let mut p = params(0.0);
        p[9] = -6.0;
        let (mut l, mut r) = (input.clone(), input.clone());
        rack.process(&mut l, &mut r, &p, CTX);
        let half = 10.0_f32.powf(-6.0 / 20.0);
        for (out, x) in l.iter().zip(&input) {
            assert!(
                (out - x * half).abs() < 1e-5,
                "bypassed effect, output trim"
            );
        }
        assert_eq!(rack.len(0), 1);
        assert_eq!(rack.len(7), 0);
    }

    #[test]
    fn long_blocks_are_processed_in_chunks() {
        let delay = || item(EffectKind::Delay, &[]);
        let input = noise(1_000);
        let mut whole = Rack::new([vec![delay()], vec![], vec![], vec![]]);
        let (mut l, mut r) = (input.clone(), input.clone());
        whole.process(&mut l, &mut r, &params(0.5), CTX);
        let mut parts = Rack::new([vec![delay()], vec![], vec![], vec![]]);
        let (mut l2, mut r2) = (input.clone(), input.clone());
        for (a, b) in l2.chunks_mut(100).zip(r2.chunks_mut(100)) {
            parts.process(a, b, &params(0.5), CTX);
        }
        for (x, y) in l.iter().zip(&l2) {
            assert!((x - y).abs() < 1e-5);
        }
        assert!(l.iter().all(|s| s.is_finite()));
    }
}
