//! Phase 4 tests: sample, multisample, granular and spectral oscillators.

use std::f32::consts::TAU;
use std::sync::Arc;

use musiccraft_dsp::SampleBuffer;

use crate::params::{OSCS, osc, osc_param, oscx, oscx_param};
use crate::{Antiserum, Source, Zone};

const SR: f32 = 48_000.0;

/// `seconds` of a sine at `hz`, then `silence` seconds of nothing.
fn tone(hz: f32, seconds: f32, silence: f32) -> Arc<SampleBuffer> {
    let n = (seconds * SR) as usize;
    let mut data: Vec<f32> = (0..n)
        .map(|i| (TAU * hz * i as f32 / SR).sin() * 0.8)
        .collect();
    data.extend(std::iter::repeat_n(0.0, (silence * SR) as usize));
    Arc::new(SampleBuffer::mono(data, f64::from(SR)))
}

/// A saw at `hz` for the spectral analysis.
fn saw(hz: f32, seconds: f32) -> Arc<SampleBuffer> {
    let data: Vec<f32> = (0..(seconds * SR) as usize)
        .map(|i| 2.0 * (hz * i as f32 / SR).fract() - 1.0)
        .collect();
    Arc::new(SampleBuffer::mono(data, f64::from(SR)))
}

/// A synth whose oscillator 1 is of `kind` (0–4) and plays `source`.
fn synth(kind: f32, source: Source) -> Antiserum {
    let mut synth = Antiserum::new(SR);
    synth.set_param(oscx_param(0, oscx::TYPE), kind);
    synth.set_param(oscx_param(0, oscx::ROOT), 69.0);
    let _ = synth.set_source(0, Arc::new(source));
    synth
}

fn play(synth: &mut Antiserum, key: u8, frames: usize) -> Vec<f32> {
    synth.note_on(key, 1.0);
    let mut left = vec![0.0; frames];
    let mut right = vec![0.0; frames];
    for (l, r) in left.chunks_mut(100).zip(right.chunks_mut(100)) {
        synth.process(l, r);
    }
    assert!(left.iter().all(|x| x.is_finite()));
    left
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

/// Frequency from upward zero crossings.
fn frequency(x: &[f32]) -> f32 {
    let crossings: Vec<usize> = x
        .windows(2)
        .enumerate()
        .filter(|(_, w)| w[0] <= 0.0 && w[1] > 0.0)
        .map(|(i, _)| i)
        .collect();
    match (crossings.first(), crossings.last()) {
        (Some(a), Some(b)) if b > a => (crossings.len() - 1) as f32 * SR / (b - a) as f32,
        _ => 0.0,
    }
}

fn sample_source(buffer: Arc<SampleBuffer>) -> Source {
    Source::new(Some(buffer), Vec::new())
}

#[test]
fn repitch_follows_the_key_and_fixed_does_not() {
    let source = || sample_source(tone(440.0, 1.0, 0.0));
    let at_root = play(&mut synth(1.0, source()), 69, 9_600);
    assert!((frequency(&at_root[2_400..]) - 440.0).abs() < 3.0);
    let octave = play(&mut synth(1.0, source()), 81, 9_600);
    assert!((frequency(&octave[2_400..]) - 880.0).abs() < 6.0);
    let mut fixed = synth(1.0, source());
    fixed.set_param(oscx_param(0, oscx::SAMPLE_MODE), 1.0);
    let fixed = play(&mut fixed, 81, 9_600);
    assert!(
        (frequency(&fixed[2_400..]) - 440.0).abs() < 3.0,
        "fixed keeps its speed"
    );
}

#[test]
fn stretch_keeps_the_duration_at_a_new_pitch() {
    // 0.25 s of tone, then silence.
    let source = || sample_source(tone(440.0, 0.25, 0.5));
    let window = 8_000..10_500; // 0.17–0.22 s
    let repitch = play(&mut synth(1.0, source()), 81, 12_000);
    assert!(
        rms(&repitch[window.clone()]) < 0.01,
        "an octave up ends at 0.125 s"
    );
    let mut stretch = synth(1.0, source());
    stretch.set_param(oscx_param(0, oscx::SAMPLE_MODE), 2.0);
    let stretch = play(&mut stretch, 81, 12_000);
    assert!(rms(&stretch[window]) > 0.05, "stretched, it lasts 0.25 s");
    let f = frequency(&stretch[2_400..7_200]);
    assert!((f - 880.0).abs() < 40.0, "{f}");
}

#[test]
fn loops_keep_a_short_sample_going() {
    let source = || sample_source(tone(440.0, 0.05, 0.0));
    let late = 19_200..24_000; // 0.4–0.5 s
    let once = play(&mut synth(1.0, source()), 69, 24_000);
    assert!(rms(&once[late.clone()]) < 1e-4);
    for mode in [1.0, 2.0] {
        let mut looped = synth(1.0, source());
        looped.set_param(oscx_param(0, oscx::LOOP), mode);
        let looped = play(&mut looped, 69, 24_000);
        assert!(rms(&looped[late.clone()]) > 0.05, "loop mode {mode}");
    }
}

#[test]
fn multisample_picks_the_zone_by_key_and_velocity() {
    let zone = |buffer, low, high, root, vel_low, vel_high| Zone {
        buffer,
        low,
        high,
        root,
        vel_low,
        vel_high,
    };
    let zones = vec![
        zone(tone(220.0, 1.0, 0.0), 0, 59, 57, 1, 127),
        zone(tone(880.0, 1.0, 0.0), 60, 127, 81, 1, 64),
        zone(tone(660.0, 1.0, 0.0), 60, 127, 76, 65, 127),
    ];
    let source = || Source::new(None, zones.clone());
    let low = play(&mut synth(2.0, source()), 57, 9_600);
    assert!((frequency(&low[2_400..]) - 220.0).abs() < 3.0);
    // Full velocity: the third zone, at its root.
    let high = play(&mut synth(2.0, source()), 76, 9_600);
    assert!((frequency(&high[2_400..]) - 660.0).abs() < 5.0);
    let mut soft = synth(2.0, source());
    soft.note_on(81, 0.3);
    let mut l = vec![0.0; 9_600];
    let mut r = vec![0.0; 9_600];
    soft.process(&mut l, &mut r);
    assert!((frequency(&l[2_400..]) - 880.0).abs() < 6.0);
}

#[test]
fn granular_sounds_and_scan_zero_holds_still() {
    // Tone for the first half, silence after.
    let source = || sample_source(tone(440.0, 0.5, 0.5));
    // 0.6–0.8 s: scanning from 0.1 s at real time reaches 0.7–0.9 s, inside
    // the silence (it wraps back to the tone after 1 s).
    let late = 28_800..38_400;
    let mut held = synth(3.0, source());
    held.set_param(osc_param(0, osc::POSITION), 0.1);
    held.set_param(oscx_param(0, oscx::GRAIN_SPREAD), 0.0);
    let held = play(&mut held, 69, 48_000);
    assert!(rms(&held[late.clone()]) > 0.05, "frozen in the tone");
    let mut moving = synth(3.0, source());
    moving.set_param(osc_param(0, osc::POSITION), 0.1);
    moving.set_param(oscx_param(0, oscx::GRAIN_SPREAD), 0.0);
    moving.set_param(oscx_param(0, oscx::GRAIN_SCAN), 1.0);
    let moving = play(&mut moving, 69, 48_000);
    assert!(rms(&moving[late]) < 0.01, "scanned into the silence");
}

#[test]
fn spectral_plays_the_partials_at_the_note() {
    let source = || sample_source(saw(220.0, 1.0));
    let plain = play(&mut synth(4.0, source()), 69, 24_000);
    let f = frequency(&plain[4_800..]);
    assert!((f - 440.0).abs() < 8.0, "{f}");
    let brightness = |x: &[f32]| {
        let diff: Vec<f32> = x.windows(2).map(|w| w[1] - w[0]).collect();
        rms(&diff) / rms(x).max(1e-9)
    };
    let tilted = |tilt: f32| {
        let mut s = synth(4.0, source());
        s.set_param(oscx_param(0, oscx::SPECTRAL_TILT), tilt);
        play(&mut s, 69, 24_000)
    };
    assert!(brightness(&tilted(1.0)[4_800..]) > 1.5 * brightness(&tilted(-1.0)[4_800..]));
    let mut shifted = synth(4.0, source());
    shifted.set_param(oscx_param(0, oscx::SPECTRAL_TILT), -1.0);
    shifted.set_param(oscx_param(0, oscx::SPECTRAL_SHIFT), 300.0);
    let shifted = play(&mut shifted, 69, 24_000);
    let g = frequency(&shifted[4_800..]);
    assert!(g > 600.0, "the fundamental moved up by 300 Hz: {g}");
}

#[test]
fn every_type_without_a_source_is_silent() {
    for kind in 1..=4 {
        for osc in 0..OSCS {
            let mut s = Antiserum::new(SR);
            // Only the oscillator under test is on (oscillator 1 is on by
            // default as a wavetable).
            for other in 0..OSCS {
                s.set_param(osc_param(other, osc::ON), f32::from(u8::from(other == osc)));
            }
            s.set_param(oscx_param(osc, oscx::TYPE), kind as f32);
            let out = play(&mut s, 60, 4_800);
            assert!(rms(&out) < 1e-6, "type {kind} osc {osc}");
        }
    }
}
