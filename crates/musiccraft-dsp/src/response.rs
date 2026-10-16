//! Static curves for editors to draw: filter and EQ magnitude responses and
//! the transfer curves of the dynamics effects. Main thread only (they are
//! cheap, but nothing here is needed while processing).

use std::f32::consts::PI;

use musiccraft_params::EffectKind;

use crate::character::{
    Transient, formants, graphic_eq_db, overdrive_curve, parametric_eq_db, soft_clip_curve,
    waveshaper_curve,
};
use crate::{Biquad, ProcessContext, SvfMode, db_to_gain, gain_to_db};

/// Magnitude in dB of the state-variable filter at `freq`. The TPT filter is
/// the bilinear transform of the analog prototype, so the prewarped analog
/// response is exact.
#[must_use]
pub fn svf_db(mode: SvfMode, cutoff: f32, resonance: f32, freq: f32, sample_rate: f32) -> f32 {
    let nyquist = sample_rate * 0.49;
    let warp = |f: f32| (PI * f.clamp(1.0, nyquist) / sample_rate.max(1.0)).tan();
    let w = warp(freq) / warp(cutoff.clamp(10.0, nyquist.max(10.0)));
    let q = 0.5 + resonance.clamp(0.0, 1.0).powi(2) * 24.5;
    let k = 1.0 / q;
    // |s^2 + k s + 1| with s = jw.
    let re = 1.0 - w * w;
    let im = k * w;
    let denominator = (re * re + im * im).sqrt().max(1e-9);
    let numerator = match mode {
        SvfMode::LowPass => 1.0,
        SvfMode::BandPass => w,
        SvfMode::HighPass => w * w,
    };
    gain_to_db(numerator / denominator)
}

/// Magnitude in dB of the three-band EQ effect for its parameter values.
#[must_use]
pub fn eq_db(params: &[f32], freq: f32, sample_rate: f32) -> f32 {
    let param = |i: usize, default: f32| params.get(i).copied().unwrap_or(default);
    let mut low = Biquad::default();
    let mut mid = Biquad::default();
    let mut high = Biquad::default();
    low.set_low_shelf(param(1, 120.0), param(0, 0.0), sample_rate);
    mid.set_peak(param(3, 1_000.0), param(2, 0.0), param(4, 0.7), sample_rate);
    high.set_high_shelf(param(6, 8_000.0), param(5, 0.0), sample_rate);
    [low, mid, high]
        .iter()
        .map(|b| b.magnitude_db(freq, sample_rate))
        .sum()
}

/// Output level (dB) of the compressor's static curve for an input level.
/// Parameters as the effect: `[threshold, ratio, attack, release, makeup]`.
#[must_use]
pub fn compressor_db(params: &[f32], input_db: f32) -> f32 {
    let param = |i: usize, default: f32| params.get(i).copied().unwrap_or(default);
    let threshold = param(0, -18.0);
    let ratio = param(1, 4.0).max(1.0);
    let makeup = param(4, 0.0);
    let over = (input_db - threshold).max(0.0);
    input_db - over + over / ratio + makeup
}

/// Output level (dB) of the limiter's static curve: `[input, ceiling, release]`.
#[must_use]
pub fn limiter_db(params: &[f32], input_db: f32) -> f32 {
    let param = |i: usize, default: f32| params.get(i).copied().unwrap_or(default);
    (input_db + param(0, 0.0)).min(param(1, -0.3))
}

/// Output sample of the distortion for an input sample:
/// `[drive, mix, output, mode, threshold]`.
#[must_use]
pub fn distortion_shape(params: &[f32], x: f32) -> f32 {
    let param = |i: usize, default: f32| params.get(i).copied().unwrap_or(default);
    let drive = db_to_gain(param(0, 12.0));
    let mix = param(1, 1.0).clamp(0.0, 1.0);
    let output = db_to_gain(param(2, -6.0));
    let hard = param(3, 0.0) >= 0.5;
    let threshold = db_to_gain(param(4, -6.0).min(0.0));
    let wet = crate::dynamics::distortion_curve(hard, threshold, x * drive);
    (x + (wet - x) * mix) * output
}

fn at(params: &[f32], i: usize, default: f32) -> f32 {
    params.get(i).copied().unwrap_or(default)
}

/// Magnitude in dB at `freq` of the newer frequency-shaping effects
/// (graphic and parametric EQ, bass boost, DC removal, formant and auto
/// filter at rest); `None` for other kinds.
#[must_use]
pub fn effect_db(kind: EffectKind, params: &[f32], freq: f32, sample_rate: f32) -> Option<f32> {
    let mixed = |wet: f32, mix: f32| gain_to_db((1.0 - mix) + wet * mix);
    Some(match kind {
        EffectKind::GraphicEq => graphic_eq_db(params, freq, sample_rate),
        EffectKind::ParametricEq => parametric_eq_db(params, freq, sample_rate),
        EffectKind::BassBoost => {
            let mut shelf = Biquad::default();
            shelf.set_low_shelf(at(params, 0, 80.0), at(params, 1, 6.0), sample_rate);
            shelf.magnitude_db(freq, sample_rate)
        }
        EffectKind::DcBlock => {
            // |1 - z^-1| / |1 - r z^-1|.
            let r = (-2.0 * PI * at(params, 0, 10.0).max(1.0) / sample_rate.max(1.0)).exp();
            let w = 2.0 * PI * freq / sample_rate.max(1.0);
            let num = (2.0 - 2.0 * w.cos()).sqrt();
            let den = (1.0 + r * r - 2.0 * r * w.cos()).sqrt().max(1e-9);
            gain_to_db(num / den)
        }
        EffectKind::Formant => {
            let resonance = 0.4 + at(params, 2, 0.6).clamp(0.0, 1.0) * 0.5;
            let wet: f32 = formants(at(params, 0, 0.0), at(params, 1, 0.0))
                .iter()
                .zip([1.0, 0.7, 0.4])
                .map(|(f, g)| {
                    db_to_gain(svf_db(SvfMode::BandPass, *f, resonance, freq, sample_rate)) * g
                })
                .sum();
            mixed(wet * 0.7, at(params, 3, 1.0).clamp(0.0, 1.0))
        }
        EffectKind::AutoFilter => {
            let wet = db_to_gain(svf_db(
                SvfMode::from_param(at(params, 0, 0.0)),
                at(params, 1, 1_000.0),
                at(params, 2, 0.5),
                freq,
                sample_rate,
            ));
            mixed(wet, at(params, 6, 1.0).clamp(0.0, 1.0))
        }
        _ => return None,
    })
}

/// Output sample for an input sample of the waveshaping effects
/// (distortion, overdrive before its tone filter, soft clipper,
/// waveshaper); `None` for other kinds.
#[must_use]
pub fn shaper(kind: EffectKind, params: &[f32], x: f32) -> Option<f32> {
    let blend = |wet: f32, mix: f32, out_db: f32| (x + (wet - x) * mix) * db_to_gain(out_db);
    Some(match kind {
        EffectKind::Distortion => distortion_shape(params, x),
        EffectKind::Overdrive => blend(
            overdrive_curve(at(params, 0, 18.0), at(params, 2, 0.1), x),
            at(params, 3, 1.0),
            at(params, 4, -6.0),
        ),
        EffectKind::SoftClipper => {
            soft_clip_curve(at(params, 0, -3.0), at(params, 1, 0.5), x)
                * db_to_gain(at(params, 2, 0.0))
        }
        EffectKind::Waveshaper => blend(
            waveshaper_curve(params, x * db_to_gain(at(params, 1, 12.0))),
            at(params, 2, 1.0),
            at(params, 3, -6.0),
        ),
        _ => return None,
    })
}

/// Output level (dB) of the noise gate's static curve:
/// `[threshold, attack, hold, release, range]`.
#[must_use]
pub fn noise_gate_db(params: &[f32], input_db: f32) -> f32 {
    if input_db >= at(params, 0, -40.0) {
        input_db
    } else {
        input_db + at(params, 4, -80.0)
    }
}

/// Output level (dB) of the dynamics chain's static curve: the gate (when
/// on), then the compressor, then the limiter.
#[must_use]
pub fn dynamics_db(params: &[f32], input_db: f32) -> f32 {
    use crate::toolbox::Dynamics;
    let gated = if Dynamics::gate_on(params) {
        noise_gate_db(Dynamics::gate_params(params), input_db)
    } else {
        input_db
    };
    let compressed = compressor_db(Dynamics::compressor_params(params), gated);
    limiter_db(Dynamics::limiter_params(params), compressed)
}

/// A drum-like hit's level envelope before and after the transient shaper,
/// `points` values each over 300 ms.
#[must_use]
pub fn transient_envelopes(params: &[f32], points: usize) -> (Vec<f32>, Vec<f32>) {
    let sample_rate = 8_000.0;
    let n = (sample_rate * 0.3) as usize;
    let hit: Vec<f32> = (0..n)
        .map(|i| {
            let t = i as f32 / sample_rate - 0.02;
            if t < 0.0 {
                0.0
            } else {
                0.25 + 0.5 * (-t / 0.03).exp() - 0.25 * (-t / 0.002).exp()
            }
        })
        .collect();
    let (mut left, mut right) = (hit.clone(), hit.clone());
    let ctx = ProcessContext {
        sample_rate,
        tempo_bpm: 120.0,
        beats: 0.0,
    };
    Transient::default().process(&mut left, &mut right, params, ctx);
    let pick = |v: &[f32]| -> Vec<f32> {
        (0..points)
            .map(|i| v.get(i * n / points.max(1)).copied().unwrap_or(0.0))
            .collect()
    };
    (pick(&hit), pick(&left))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filter_curves_have_the_expected_shape() {
        let sr = 48_000.0;
        let lp = |f| svf_db(SvfMode::LowPass, 1_000.0, 0.0, f, sr);
        assert!(lp(50.0).abs() < 0.1);
        assert!((lp(1_000.0) + 6.0).abs() < 0.2, "{}", lp(1_000.0));
        assert!(lp(10_000.0) < -35.0);
        let hp = |f| svf_db(SvfMode::HighPass, 1_000.0, 0.0, f, sr);
        assert!(hp(50.0) < -45.0 && hp(15_000.0).abs() < 0.5);
        // Resonance peaks at the cutoff.
        assert!(svf_db(SvfMode::LowPass, 1_000.0, 0.8, 1_000.0, sr) > 10.0);
    }

    #[test]
    fn eq_curve_matches_band_gains() {
        let sr = 48_000.0;
        let params = [6.0, 120.0, -4.0, 1_000.0, 1.0, 0.0, 8_000.0];
        assert!((eq_db(&params, 20.0, sr) - 6.0).abs() < 0.5);
        assert!((eq_db(&params, 1_000.0, sr) + 4.0).abs() < 0.5);
        assert!(eq_db(&params, 4_000.0, sr).abs() < 1.0);
    }

    #[test]
    fn dynamics_curves() {
        let comp = [-20.0, 4.0, 10.0, 100.0, 3.0];
        assert!((compressor_db(&comp, -30.0) + 27.0).abs() < 1e-4);
        assert!((compressor_db(&comp, 0.0) - (-20.0 + 5.0 + 3.0)).abs() < 1e-4);
        assert!((limiter_db(&[6.0, -1.0, 50.0], -3.0) + 1.0).abs() < 1e-6);
        let shape = |x| distortion_shape(&[24.0, 1.0, 0.0], x);
        assert!(shape(1.0) <= 1.0 && shape(0.01) > 0.1);
    }
}
