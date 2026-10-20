//! Release-blocker tests for the sample-based oscillators
//! (`docs/spec/antiserum-oscillators.md`): stereo samples, unison on every
//! type, and warps beyond wavetables.

use std::f32::consts::TAU;
use std::sync::Arc;

use musiccraft_dsp::SampleBuffer;

use crate::params::{osc, osc_param, oscx, oscx_param};
use crate::{Antiserum, Source};

const SR: f32 = 48_000.0;
const FRAMES: usize = 24_000;

/// A tone on the left only, silence on the right.
fn left_only() -> Arc<SampleBuffer> {
    let n = SR as usize * 2;
    let left: Vec<f32> = (0..n)
        .map(|i| 2.0 * (220.0 * i as f32 / SR).fract() - 1.0)
        .collect();
    Arc::new(SampleBuffer::stereo(left, vec![0.0; n], f64::from(SR)))
}

/// A saw on both sides.
fn saw() -> Arc<SampleBuffer> {
    let data: Vec<f32> = (0..SR as usize * 2)
        .map(|i| {
            0.3 * (TAU * 220.0 * i as f32 / SR).sin()
                + 0.5 * ((220.0 * i as f32 / SR).fract() - 0.5)
        })
        .collect();
    Arc::new(SampleBuffer::mono(data, f64::from(SR)))
}

/// A synth whose oscillator 1 is of `kind` and plays `buffer`; oscillator 2
/// runs silently as the mod source.
fn synth(kind: f32, buffer: Arc<SampleBuffer>, settings: &[(usize, f32)]) -> Antiserum {
    let mut synth = Antiserum::new(SR);
    synth.set_param(oscx_param(0, oscx::TYPE), kind);
    synth.set_param(oscx_param(0, oscx::ROOT), 57.0);
    synth.set_param(oscx_param(0, oscx::LOOP), 1.0);
    synth.set_param(osc_param(1, osc::ON), 1.0);
    synth.set_param(osc_param(1, osc::LEVEL), 0.0);
    synth.set_param(osc_param(1, osc::SEMI), 7.0);
    synth.set_param(osc_param(2, osc::ON), 0.0);
    for (index, value) in settings {
        synth.set_param(*index, *value);
    }
    let _ = synth.set_source(0, Arc::new(Source::new(Some(buffer), Vec::new())));
    synth
}

fn play(synth: &mut Antiserum) -> (Vec<f32>, Vec<f32>) {
    synth.note_on(57, 1.0);
    let mut left = vec![0.0; FRAMES];
    let mut right = vec![0.0; FRAMES];
    for (l, r) in left.chunks_mut(128).zip(right.chunks_mut(128)) {
        synth.process(l, r);
    }
    assert!(left.iter().chain(&right).all(|x| x.is_finite()));
    (left, right)
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

fn difference(a: &[f32], b: &[f32]) -> f32 {
    let d: Vec<f32> = a.iter().zip(b).map(|(x, y)| x - y).collect();
    rms(&d)
}

const SAMPLE: f32 = 1.0;
const GRANULAR: f32 = 3.0;
const SPECTRAL: f32 = 4.0;

#[test]
fn stereo_samples_keep_their_sides() {
    for kind in [SAMPLE, GRANULAR, SPECTRAL] {
        let (l, r) = play(&mut synth(kind, left_only(), &[]));
        let (l, r) = (rms(&l[4_800..]), rms(&r[4_800..]));
        assert!(l > 0.01, "type {kind} sounds ({l})");
        assert!(r < l * 0.05, "type {kind}: right {r} vs left {l}");
    }
}

#[test]
fn unison_widens_every_type() {
    let stretch = (oscx_param(0, oscx::SAMPLE_MODE), 2.0);
    for (kind, extra) in [(GRANULAR, None), (SPECTRAL, None), (SAMPLE, Some(stretch))] {
        let base: Vec<(usize, f32)> = extra.into_iter().collect();
        let (l, r) = play(&mut synth(kind, saw(), &base));
        assert!(difference(&l, &r) < 1e-4, "type {kind} alone is centred");
        let mut wide = base.clone();
        wide.extend([
            (osc_param(0, osc::UNISON), 4.0),
            (osc_param(0, osc::DETUNE), 0.4),
            (osc_param(0, osc::WIDTH), 1.0),
        ]);
        let (l, r) = play(&mut synth(kind, saw(), &wide));
        assert!(
            difference(&l, &r) > 0.01 * rms(&l).max(1e-6),
            "type {kind}: unison spreads it"
        );
        assert!(rms(&l) > 0.005, "type {kind} sounds with unison");
    }
}

/// Oscillator 1's warp 1 set to `warp` at `amount`.
fn warp(warp: f32, amount: f32) -> [(usize, f32); 2] {
    [
        (osc_param(0, osc::WARP1), warp),
        (osc_param(0, osc::WARP1_AMOUNT), amount),
    ]
}

#[test]
fn warps_reach_sample_and_spectral_oscillators() {
    let plain = |kind| play(&mut synth(kind, saw(), &[])).0;
    let warped = |kind, w| play(&mut synth(kind, saw(), &warp(w, 0.8))).0;
    // Sample types: FM, AM, RM, PWM, flip and quantize change the sound;
    // sync, bend, asym, mirror and remap have no cycle to reshape.
    for kind in [SAMPLE, GRANULAR] {
        let dry = plain(kind);
        for w in [4.0, 8.0, 9.0, 11.0, 12.0, 13.0] {
            let wet = warped(kind, w);
            assert!(
                difference(&dry, &wet) > 0.01 * rms(&dry),
                "type {kind}, warp {w} changes it"
            );
        }
        for w in [1.0, 2.0, 3.0, 5.0, 6.0, 7.0, 10.0] {
            assert!(
                difference(&dry, &warped(kind, w)) < 1e-6,
                "type {kind}, warp {w} leaves it"
            );
        }
    }
    // Spectral: every warp.
    let dry = plain(SPECTRAL);
    for w in 1..=13 {
        let wet = warped(SPECTRAL, w as f32);
        assert!(
            difference(&dry, &wet) > 0.01 * rms(&dry),
            "spectral, warp {w} changes it"
        );
    }
}
