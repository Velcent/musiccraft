//! A fixed guitar-pedal chain in one insert: compressor, drive, three-band
//! tone stack, chorus and a speaker-cabinet filter, each switchable. All
//! allocation happens in `new`.

use crate::{Biquad, Chorus, Compressor, ProcessContext, Svf, SvfMode, db_to_gain};

fn param(p: &[f32], i: usize, default: f32) -> f32 {
    p.get(i).copied().unwrap_or(default)
}

/// The drive stage's curves, by `drive_type`: soft (tanh), hard (clip) and
/// fuzz (asymmetric, gated near silence for the "sputter" of a fuzz pedal).
#[must_use]
pub fn drive_curve(kind: f32, x: f32) -> f32 {
    match kind.round() as i32 {
        1 => x.clamp(-1.0, 1.0),
        2 => {
            let shaped = if x >= 0.0 {
                x.tanh()
            } else {
                (x * 2.0).tanh() * 0.7
            };
            // Below a small level the fuzz cuts out.
            if x.abs() < 0.02 {
                shaped * x.abs() * 50.0
            } else {
                shaped
            }
        }
        _ => x.tanh(),
    }
}

/// Cabinet filters: (high-pass Hz, low-pass Hz, resonance peak Hz, peak dB)
/// for the small and the large cabinet.
const CABINETS: [(f32, f32, f32, f32); 2] =
    [(110.0, 5_000.0, 1_800.0, 4.0), (70.0, 3_800.0, 120.0, 3.0)];

/// Guitar chain.
///
/// Parameters: `[compress 0–1, drive dB, drive type, bass dB, mid dB, treble
/// dB, chorus 0–1, cabinet (0 off, 1 small, 2 large), output dB, mix]`. A
/// compress or chorus of 0 and a drive of 0 dB switch their stage off; the
/// chorus comes after the wet/dry mix.
#[derive(Debug, Clone)]
pub struct GuitarChain {
    compressor: Compressor,
    tone: [[Biquad; 3]; 2],
    chorus: Chorus,
    cabinet: [(Svf, Svf, Biquad); 2],
}

impl GuitarChain {
    /// Allocates the chorus delay. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            compressor: Compressor::new(),
            tone: Default::default(),
            chorus: Chorus::new(sample_rate),
            cabinet: Default::default(),
        }
    }

    /// Clears every stage.
    pub fn reset(&mut self) {
        self.compressor.reset();
        self.tone = Default::default();
        self.chorus.reset();
        self.cabinet = Default::default();
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let sr = ctx.sample_rate.max(1.0);
        let mix = param(p, 9, 1.0).clamp(0.0, 1.0);
        let compress = param(p, 0, 0.3).clamp(0.0, 1.0);
        let drive_db = param(p, 1, 18.0).clamp(0.0, 48.0);
        let drive_type = param(p, 2, 0.0);
        let chorus = param(p, 6, 0.0).clamp(0.0, 1.0);
        let cabinet = (param(p, 7, 1.0).round().max(0.0) as usize).min(CABINETS.len());
        let output = db_to_gain(param(p, 8, -6.0));

        for bands in &mut self.tone {
            let [bass, mid, treble] = bands;
            bass.set_low_shelf(120.0, param(p, 3, 0.0), sr);
            mid.set_peak(700.0, param(p, 4, 0.0), 0.8, sr);
            treble.set_high_shelf(3_200.0, param(p, 5, 0.0), sr);
        }
        if let Some((hp_hz, lp_hz, peak_hz, peak_db)) =
            cabinet.checked_sub(1).and_then(|i| CABINETS.get(i))
        {
            for (hp, lp, peak) in &mut self.cabinet {
                hp.set_q(*hp_hz, 0.707, sr);
                lp.set_q(*lp_hz, 0.9, sr);
                peak.set_peak(*peak_hz, *peak_db, 1.2, sr);
            }
        }

        let settings = [
            -6.0 - 34.0 * compress,
            1.0 + 7.0 * compress,
            5.0,
            120.0,
            12.0 * compress,
        ];
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let (dry_l, dry_r) = (*l, *r);
            let (mut wl, mut wr) = ([dry_l], [dry_r]);
            if compress > 0.0 {
                self.compressor.process(&mut wl, &mut wr, &settings, ctx);
            }
            let wet = self.rest(wl[0], wr[0], drive_db, drive_type, cabinet, output);
            *l = dry_l + (wet.0 - dry_l) * mix;
            *r = dry_r + (wet.1 - dry_r) * mix;
        }
        if chorus > 0.0 {
            let settings = [0.9, 3.0, 9.0, chorus * 0.6, 2.0, 20_000.0];
            self.chorus.process(left, right, &settings, ctx);
        }
    }

    /// Drive, tone stack and cabinet for one stereo sample.
    fn rest(
        &mut self,
        l: f32,
        r: f32,
        drive_db: f32,
        kind: f32,
        cabinet: usize,
        output: f32,
    ) -> (f32, f32) {
        let gain = db_to_gain(drive_db);
        let mut out = [l, r];
        for (ch, x) in out.iter_mut().enumerate() {
            if drive_db > 0.0 {
                // Normalized so a full-scale input comes out near full scale.
                *x = drive_curve(kind, *x * gain) * (1.0 / gain.sqrt()).max(0.25);
            }
            if let Some(bands) = self.tone.get_mut(ch) {
                for band in bands.iter_mut() {
                    *x = band.process(*x);
                }
            }
            if cabinet > 0 {
                if let Some((hp, lp, peak)) = self.cabinet.get_mut(ch) {
                    *x = hp.process(*x, SvfMode::HighPass);
                    *x = lp.process(*x, SvfMode::LowPass);
                    *x = peak.process(*x);
                }
            }
            *x *= output;
        }
        (out[0], out[1])
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

    fn run(p: &[f32], input: &[f32]) -> Vec<f32> {
        let mut fx = GuitarChain::new(48_000.0);
        let mut l = input.to_vec();
        let mut r = input.to_vec();
        fx.process(&mut l, &mut r, p, ctx());
        assert!(l.iter().chain(&r).all(|s| s.is_finite() && s.abs() < 4.0));
        l
    }

    #[test]
    fn every_stage_off_is_transparent() {
        // compress 0, drive 0 dB, flat tone, no chorus, no cabinet, 0 dB out.
        let p = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let input = sine(440.0, 4_800, 0.5);
        let out = run(&p, &input);
        for (a, b) in out.iter().zip(&input).skip(100) {
            assert!((a - b).abs() < 1e-3, "{a} vs {b}");
        }
    }

    #[test]
    fn defaults_distort_and_stay_bounded() {
        let p = EffectKind::GuitarChain.defaults();
        let input = sine(220.0, 24_000, 0.8);
        let out = run(&p, &input);
        let peak = out.iter().fold(0.0_f32, |m, s| m.max(s.abs()));
        assert!(peak > 0.05 && peak < 1.5, "{peak}");
        assert!(out.iter().zip(&input).any(|(a, b)| (a - b).abs() > 0.05));
    }

    #[test]
    fn cabinet_rolls_off_highs() {
        let mut p = [0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let level = |p: &[f32]| {
            let out = run(p, &sine(9_000.0, 24_000, 0.5));
            out[12_000..].iter().fold(0.0_f32, |m, s| m.max(s.abs()))
        };
        let open = level(&p);
        p[7] = 2.0;
        assert!(level(&p) < open * 0.3);
    }

    #[test]
    fn drive_curves_are_bounded() {
        for kind in 0..3 {
            for i in -50..=50 {
                let y = drive_curve(kind as f32, i as f32 / 5.0);
                assert!(y.is_finite() && y.abs() <= 1.0, "{kind}: {y}");
            }
        }
    }
}
