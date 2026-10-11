//! Static curves for editors to draw: filter and EQ magnitude responses and
//! the transfer curves of the dynamics effects. Main thread only (they are
//! cheap, but nothing here is needed while processing).

use std::f32::consts::PI;

use crate::{Biquad, SvfMode, db_to_gain, gain_to_db};

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

/// Output sample of the distortion for an input sample: `[drive, mix, output]`.
#[must_use]
pub fn distortion_shape(params: &[f32], x: f32) -> f32 {
    let param = |i: usize, default: f32| params.get(i).copied().unwrap_or(default);
    let drive = db_to_gain(param(0, 12.0));
    let mix = param(1, 1.0).clamp(0.0, 1.0);
    let output = db_to_gain(param(2, -6.0));
    (x + ((x * drive).tanh() - x) * mix) * output
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
