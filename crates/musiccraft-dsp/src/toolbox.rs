//! Utility effects built from simpler blocks: the all-in-one dynamics chain
//! and the stereo shaper. All allocation happens in `new`.

use crate::character::NoiseGate;
use crate::{Compressor, Limiter, ProcessContext, db_to_gain};

fn param(p: &[f32], i: usize, default: f32) -> f32 {
    p.get(i).copied().unwrap_or(default)
}

/// The parameters `start..end` of `p`, or none when out of range.
fn section(p: &[f32], start: usize, end: usize) -> &[f32] {
    p.get(start..end).unwrap_or(&[])
}

/// Below this gate threshold (the parameter's minimum) the gate is off.
const GATE_OFF_DB: f32 = -99.95;

/// Noise gate, compressor and limiter in series, each identical to the
/// stand-alone effect.
///
/// Parameters: `[gate threshold, gate attack, gate hold, gate release, gate
/// range, comp threshold, comp ratio, comp attack, comp release, comp makeup,
/// limit gain, limit ceiling, limit release]`; each stage reads its own
/// section in the stand-alone effect's order.
#[derive(Debug, Clone)]
pub struct Dynamics {
    gate: NoiseGate,
    compressor: Compressor,
    limiter: Limiter,
}

impl Default for Dynamics {
    fn default() -> Self {
        Self {
            gate: NoiseGate::default(),
            compressor: Compressor::new(),
            limiter: Limiter::default(),
        }
    }
}

impl Dynamics {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        if param(p, 0, -100.0) > GATE_OFF_DB {
            self.gate.process(left, right, section(p, 0, 5), ctx);
        }
        self.compressor.process(left, right, section(p, 5, 10), ctx);
        self.limiter.process(left, right, section(p, 10, 13), ctx);
    }

    /// Whether the gate stage is on for these parameters.
    #[must_use]
    pub fn gate_on(p: &[f32]) -> bool {
        param(p, 0, -100.0) > GATE_OFF_DB
    }

    /// The gate's parameters within the effect's.
    #[must_use]
    pub fn gate_params(p: &[f32]) -> &[f32] {
        section(p, 0, 5)
    }

    /// The compressor's parameters within the effect's.
    #[must_use]
    pub fn compressor_params(p: &[f32]) -> &[f32] {
        section(p, 5, 10)
    }

    /// The limiter's parameters within the effect's.
    #[must_use]
    pub fn limiter_params(p: &[f32]) -> &[f32] {
        section(p, 10, 13)
    }
}

/// Longest inter-channel delay, in milliseconds.
const MAX_SHAPER_DELAY_MS: f32 = 20.0;

/// Stereo shaper: each output side is a weighted mix of both inputs, then one
/// side is delayed by up to 20 ms.
///
/// Parameters: `[left→left, right→left, left→right, right→right, delay ms
/// (positive delays the right side), output dB]`.
#[derive(Debug, Clone)]
pub struct StereoShaper {
    lines: [Vec<f32>; 2],
    write: usize,
}

impl StereoShaper {
    /// Creates a shaper with room for the longest delay at `sample_rate`.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (MAX_SHAPER_DELAY_MS * 0.001 * sample_rate.max(1.0)).ceil() as usize + 2;
        Self {
            lines: [vec![0.0; len], vec![0.0; len]],
            write: 0,
        }
    }

    /// Clears the delay lines.
    pub fn reset(&mut self) {
        for line in &mut self.lines {
            line.fill(0.0);
        }
        self.write = 0;
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let gain = |i: usize, default: f32| param(p, i, default).clamp(-1.0, 1.0);
        let (ll, rl, lr, rr) = (gain(0, 1.0), gain(1, 0.0), gain(2, 0.0), gain(3, 1.0));
        let delay_ms = param(p, 4, 0.0).clamp(-MAX_SHAPER_DELAY_MS, MAX_SHAPER_DELAY_MS);
        let output = db_to_gain(param(p, 5, 0.0));
        let len = self.lines[0].len().max(1);
        let delay = ((delay_ms.abs() * 0.001 * ctx.sample_rate.max(1.0)).round() as usize)
            .min(len.saturating_sub(1));
        // Index of the delayed side: 0 is left, 1 is right.
        let delayed = usize::from(delay_ms > 0.0);
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let mut out = [*l * ll + *r * rl, *l * lr + *r * rr];
            if delay > 0 {
                let line = &mut self.lines[delayed];
                if let Some(slot) = line.get_mut(self.write) {
                    *slot = out[delayed];
                }
                let read = (self.write + len - delay) % len;
                out[delayed] = line.get(read).copied().unwrap_or(0.0);
            }
            self.write = (self.write + 1) % len;
            *l = out[0] * output;
            *r = out[1] * output;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::gain_to_db;

    fn ctx() -> ProcessContext {
        ProcessContext {
            sample_rate: 48_000.0,
            tempo_bpm: 120.0,
            beats: 0.0,
        }
    }

    fn peak(samples: &[f32]) -> f32 {
        samples.iter().fold(0.0_f32, |m, s| m.max(s.abs()))
    }

    fn dynamics_params(values: &[(usize, f32)]) -> Vec<f32> {
        let mut p = vec![
            -100.0, 1.0, 20.0, 100.0, -80.0, -12.0, 1.0, 10.0, 120.0, 0.0, 0.0, -0.3, 80.0,
        ];
        for (i, v) in values {
            p[*i] = *v;
        }
        p
    }

    #[test]
    fn dynamics_by_default_only_limits() {
        let mut fx = Dynamics::default();
        let mut l = vec![0.5; 4_800];
        let mut r = l.clone();
        fx.process(&mut l, &mut r, &dynamics_params(&[]), ctx());
        assert!(
            l.iter().chain(&r).all(|s| (s - 0.5).abs() < 1e-6),
            "below the ceiling"
        );
        let mut l = vec![2.0; 4_800];
        let mut r = l.clone();
        fx.process(&mut l, &mut r, &dynamics_params(&[]), ctx());
        assert!(
            peak(&l).max(peak(&r)) <= db_to_gain(-0.3) + 1e-6,
            "the ceiling holds"
        );
    }

    #[test]
    fn dynamics_gate_closes_on_quiet_input() {
        let mut fx = Dynamics::default();
        let mut l = vec![0.001; 48_000];
        let mut r = l.clone();
        fx.process(&mut l, &mut r, &dynamics_params(&[(0, -40.0)]), ctx());
        assert!(gain_to_db(l[47_999] / 0.001) < -70.0, "{}", l[47_999]);
    }

    #[test]
    fn dynamics_compressor_reduces_loud_input() {
        let mut fx = Dynamics::default();
        let mut l = vec![0.5; 48_000];
        let mut r = l.clone();
        // -6 dB in, threshold -18 dB, 4:1 → 9 dB of reduction, then the limiter
        // has nothing to do.
        fx.process(
            &mut l,
            &mut r,
            &dynamics_params(&[(5, -18.0), (6, 4.0)]),
            ctx(),
        );
        let out_db = gain_to_db(l[47_999]);
        assert!((out_db - -15.0).abs() < 0.2, "{out_db}");
    }

    #[test]
    fn shaper_default_is_transparent() {
        let mut fx = StereoShaper::new(48_000.0);
        let mut l: Vec<f32> = (0..512).map(|i| (i as f32 * 0.1).sin()).collect();
        let mut r: Vec<f32> = (0..512).map(|i| (i as f32 * 0.07).cos()).collect();
        let (dry_l, dry_r) = (l.clone(), r.clone());
        fx.process(&mut l, &mut r, &[1.0, 0.0, 0.0, 1.0, 0.0, 0.0], ctx());
        assert_eq!(l, dry_l);
        assert_eq!(r, dry_r);
    }

    #[test]
    fn shaper_swaps_sides() {
        let mut fx = StereoShaper::new(48_000.0);
        let mut l = vec![0.25; 64];
        let mut r = vec![-0.5; 64];
        fx.process(&mut l, &mut r, &[0.0, 1.0, 1.0, 0.0, 0.0, 0.0], ctx());
        assert!((l[10] + 0.5).abs() < 1e-6 && (r[10] - 0.25).abs() < 1e-6);
    }

    #[test]
    fn shaper_delays_the_chosen_side() {
        for (delay_ms, delayed_right) in [(10.0, true), (-10.0, false)] {
            let mut fx = StereoShaper::new(48_000.0);
            let mut l = vec![0.0; 1_000];
            let mut r = vec![0.0; 1_000];
            l[0] = 1.0;
            r[0] = 1.0;
            fx.process(&mut l, &mut r, &[1.0, 0.0, 0.0, 1.0, delay_ms, 0.0], ctx());
            let (late, early) = if delayed_right { (&r, &l) } else { (&l, &r) };
            assert_eq!(early[0], 1.0);
            assert_eq!(late[0], 0.0);
            assert_eq!(late[480], 1.0, "10 ms at 48 kHz is 480 samples");
        }
    }

    #[test]
    fn shaper_delay_survives_block_boundaries_and_reset() {
        let mut fx = StereoShaper::new(48_000.0);
        let p = [1.0, 0.0, 0.0, 1.0, 5.0, 0.0];
        let mut l = vec![0.0; 100];
        let mut r = vec![0.0; 100];
        r[99] = 1.0;
        fx.process(&mut l, &mut r, &p, ctx());
        let mut l2 = vec![0.0; 400];
        let mut r2 = vec![0.0; 400];
        fx.process(&mut l2, &mut r2, &p, ctx());
        assert_eq!(r2[239], 1.0, "sample 99 comes out 240 samples later");
        fx.reset();
        let mut r3 = vec![0.0; 400];
        fx.process(&mut vec![0.0; 400], &mut r3, &p, ctx());
        assert!(r3.iter().all(|s| *s == 0.0));
    }
}
