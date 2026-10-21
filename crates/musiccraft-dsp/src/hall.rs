//! A dense hall reverb: predelay, early reflections and an eight-line
//! feedback delay network (Householder mixing, per-line damping and slowly
//! modulated lengths). All allocation happens in `new`.

use std::f32::consts::TAU;

use crate::{ProcessContext, flush_denormal};

fn param(p: &[f32], i: usize, default: f32) -> f32 {
    p.get(i).copied().unwrap_or(default)
}

/// Number of feedback delay lines.
const LINES: usize = 8;

/// Base lengths of the network's lines in milliseconds (mutually prime-ish
/// so their echoes do not line up), scaled by the size.
const LINE_MS: [f32; LINES] = [29.7, 37.1, 41.1, 43.7, 53.3, 59.9, 67.7, 73.1];

/// Early reflection taps: (milliseconds after the predelay, gain), scaled by
/// the size. Odd taps go to the right side, even to the left.
const EARLY_TAPS: [(f32, f32); 6] = [
    (7.1, 0.84),
    (11.3, 0.71),
    (17.9, 0.62),
    (23.3, 0.52),
    (31.7, 0.41),
    (39.1, 0.33),
];

/// Longest predelay, in milliseconds.
const MAX_PREDELAY_MS: f32 = 200.0;
/// Largest size scale (lengths are multiplied by `0.5 + size`).
const MAX_SCALE: f32 = 1.5;
/// Longest modulation swing of a line, in milliseconds.
const MOD_MS: f32 = 1.2;

/// A delay line read at a fractional position.
#[derive(Debug, Clone)]
struct Line {
    buffer: Vec<f32>,
    write: usize,
}

impl Line {
    fn new(samples: usize) -> Self {
        Self {
            buffer: vec![0.0; samples.max(4)],
            write: 0,
        }
    }

    fn push(&mut self, x: f32) {
        if let Some(slot) = self.buffer.get_mut(self.write) {
            *slot = if x.is_finite() {
                flush_denormal(x)
            } else {
                0.0
            };
        }
        self.write = (self.write + 1) % self.buffer.len();
    }

    /// The sample `delay` samples before the most recent push.
    fn read(&self, delay: f32) -> f32 {
        let len = self.buffer.len();
        let delay = delay.clamp(1.0, (len - 2) as f32);
        let whole = delay as usize;
        let frac = delay - whole as f32;
        let at = |back: usize| {
            self.buffer
                .get((self.write + len - back) % len)
                .copied()
                .unwrap_or(0.0)
        };
        let a = at(whole);
        a + (at(whole + 1) - a) * frac
    }

    fn clear(&mut self) {
        self.buffer.fill(0.0);
    }
}

/// Hall reverb.
///
/// Parameters: `[predelay ms, decay s, size, damping Hz, low cut Hz, early,
/// mod depth, mod rate Hz, width, mix]`.
#[derive(Debug, Clone)]
pub struct HallReverb {
    predelay: [Line; 2],
    lines: [Line; LINES],
    damp: [f32; LINES],
    low_cut: [f32; 2],
    phase: f32,
}

impl HallReverb {
    /// Allocates the delay lines for `sample_rate`. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let sr = sample_rate.max(1.0);
        let ms = |ms: f32| (ms * 0.001 * sr) as usize + 4;
        let longest_early = EARLY_TAPS[EARLY_TAPS.len() - 1].0 * MAX_SCALE;
        let pre = ms(MAX_PREDELAY_MS + longest_early);
        Self {
            predelay: [Line::new(pre), Line::new(pre)],
            lines: LINE_MS.map(|l| Line::new(ms(l * MAX_SCALE + MOD_MS))),
            damp: [0.0; LINES],
            low_cut: [0.0; 2],
            phase: 0.0,
        }
    }

    /// Clears the tail.
    pub fn reset(&mut self) {
        for line in self.predelay.iter_mut().chain(self.lines.iter_mut()) {
            line.clear();
        }
        self.damp = [0.0; LINES];
        self.low_cut = [0.0; 2];
        self.phase = 0.0;
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let sr = ctx.sample_rate.max(1.0);
        let per_ms = 0.001 * sr;
        let predelay = param(p, 0, 20.0).clamp(0.0, MAX_PREDELAY_MS) * per_ms;
        let decay = param(p, 1, 2.5).clamp(0.1, 30.0);
        let scale = (0.5 + param(p, 2, 0.6).clamp(0.0, 1.0)).min(MAX_SCALE);
        let damping_hz = param(p, 3, 6_000.0).clamp(200.0, 20_000.0);
        let damp_coeff = 1.0 - (-TAU * damping_hz / sr).exp();
        let low_cut_hz = param(p, 4, 100.0).clamp(10.0, 2_000.0);
        let low_coeff = 1.0 - (-TAU * low_cut_hz / sr).exp();
        let early = param(p, 5, 0.4).clamp(0.0, 1.0);
        let mod_depth = param(p, 6, 0.3).clamp(0.0, 1.0) * MOD_MS * per_ms;
        let mod_step = param(p, 7, 0.4).clamp(0.0, 5.0) / sr;
        let width = param(p, 8, 1.0).clamp(0.0, 1.0);
        let mix = param(p, 9, 0.3).clamp(0.0, 1.0);

        let lengths: [f32; LINES] = LINE_MS.map(|l| l * scale * per_ms);
        // Each pass through a line loses enough to fall 60 dB in `decay`.
        let gains: [f32; LINES] = lengths.map(|len| 10.0_f32.powf(-3.0 * len / (decay * sr)));
        let taps: [(f32, f32); 6] = EARLY_TAPS.map(|(ms, g)| (predelay + ms * scale * per_ms, g));

        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let (dry_l, dry_r) = (*l, *r);
            let [pre_l, pre_r] = &mut self.predelay;
            pre_l.push(dry_l);
            pre_r.push(dry_r);
            let fed = [pre_l.read(predelay.max(1.0)), pre_r.read(predelay.max(1.0))];
            let mut reflections = [0.0_f32; 2];
            for (k, (at, g)) in taps.iter().enumerate() {
                let side = k % 2;
                let line = if side == 0 { &*pre_l } else { &*pre_r };
                if let Some(out) = reflections.get_mut(side) {
                    *out += line.read(*at) * g;
                }
            }

            // Read the network, damp each line, mix with a Householder matrix.
            let mut outs = [0.0_f32; LINES];
            for (k, ((out, line), damp)) in outs
                .iter_mut()
                .zip(&self.lines)
                .zip(self.damp.iter_mut())
                .enumerate()
            {
                let lfo = ((self.phase + k as f32 / LINES as f32) * TAU).sin();
                let delayed = line.read(lengths[k] + mod_depth * (1.0 + lfo));
                *damp = flush_denormal(*damp + (delayed - *damp) * damp_coeff);
                *out = *damp * gains[k];
            }
            let sum: f32 = outs.iter().sum::<f32>() * (2.0 / LINES as f32);
            for (k, (line, out)) in self.lines.iter_mut().zip(outs).enumerate() {
                let input = fed[k % 2] * if k % 4 < 2 { 0.5 } else { -0.5 };
                line.push(out - sum + input);
            }

            let mut wet = [0.0_f32; 2];
            for (k, out) in outs.iter().enumerate() {
                let sign = if (k / 2) % 2 == 0 { 1.0 } else { -1.0 };
                if let Some(w) = wet.get_mut(k % 2) {
                    *w += out * sign * 0.5;
                }
            }
            for (side, w) in wet.iter_mut().enumerate() {
                *w += reflections[side] * early * 0.6;
                let lc = &mut self.low_cut[side];
                *lc = flush_denormal(*lc + (*w - *lc) * low_coeff);
                *w -= *lc;
            }
            let mid = (wet[0] + wet[1]) * 0.5;
            let side = (wet[0] - wet[1]) * 0.5 * width;
            *l = dry_l + (mid + side - dry_l) * mix;
            *r = dry_r + (mid - side - dry_r) * mix;
            self.phase = (self.phase + mod_step).fract();
        }
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

    fn energy(x: &[f32]) -> f32 {
        x.iter().map(|s| s * s).sum()
    }

    fn tail(decay: f32) -> Vec<f32> {
        let mut p = EffectKind::HallReverb.defaults();
        p[1] = decay;
        p[9] = 1.0; // all wet
        let mut fx = HallReverb::new(48_000.0);
        let (mut l, mut r) = (vec![0.0; 96_000], vec![0.0; 96_000]);
        l[0] = 1.0;
        r[0] = 1.0;
        fx.process(&mut l, &mut r, &p, ctx());
        assert!(l.iter().chain(&r).all(|s| s.is_finite() && s.abs() < 2.0));
        l
    }

    #[test]
    fn predelay_holds_back_the_wet_signal() {
        let l = tail(2.0);
        // Default predelay 20 ms = 960 samples, first reflection 7 ms later.
        assert!(l[..900].iter().all(|s| s.abs() < 1e-6));
        assert!(energy(&l[900..4_000]) > 1e-4);
    }

    #[test]
    fn decay_time_sets_the_tail_length() {
        let short = tail(0.5);
        let long = tail(4.0);
        let late = 48_000..96_000;
        assert!(
            energy(&short[late.clone()]) < 1e-6,
            "short tail is gone after 1 s"
        );
        assert!(energy(&long[late]) > 1e-3, "long tail still rings");
        // The long tail falls: its second second is quieter than its first.
        assert!(energy(&long[48_000..96_000]) < energy(&long[0..48_000]));
    }

    #[test]
    fn dry_passes_with_mix_zero() {
        let mut p = EffectKind::HallReverb.defaults();
        p[9] = 0.0;
        let mut fx = HallReverb::new(48_000.0);
        let (mut l, mut r) = (vec![0.3; 1_000], vec![-0.2; 1_000]);
        fx.process(&mut l, &mut r, &p, ctx());
        assert!(
            l.iter().all(|s| (s - 0.3).abs() < 1e-6) && r.iter().all(|s| (s + 0.2).abs() < 1e-6)
        );
    }
}
