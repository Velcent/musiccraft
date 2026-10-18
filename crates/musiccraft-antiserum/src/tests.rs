//! Whole-synth tests: offline renders of note events.

use crate::params::{
    FILTER_ROUTING, GLIDE, GLIDE_MODE, MACRO1, NOISE_BASE, POLYPHONY, SUB_BASE, VOICE_MODE, VOLUME,
    WARP_COUNT, filter, filter_param, lfo, lfo_param, noise, osc, osc_param, slot, slot_param,
    source, sub,
};
use crate::{Antiserum, PARAM_COUNT, params};

const SR: f32 = 48_000.0;

fn render(synth: &mut Antiserum, frames: usize) -> (Vec<f32>, Vec<f32>) {
    let mut left = vec![0.0; frames];
    let mut right = vec![0.0; frames];
    // Odd block sizes exercise partial control blocks.
    for (l, r) in left.chunks_mut(100).zip(right.chunks_mut(100)) {
        synth.process(l, r);
    }
    (left, right)
}

fn peak(x: &[f32]) -> f32 {
    x.iter().fold(0.0, |m, v| m.max(v.abs()))
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

fn sine_synth() -> Antiserum {
    let mut synth = Antiserum::new(SR);
    // Position 0 of the basic table is a sine; no random phase.
    synth.set_param(osc_param(0, osc::RANDOM_PHASE), 0.0);
    synth
}

#[test]
fn a_note_plays_at_its_pitch_and_stops_after_release() {
    let mut synth = sine_synth();
    synth.note_on(69, 1.0);
    let (left, right) = render(&mut synth, 9_600);
    let f = frequency(&left[2_400..]);
    assert!((f - 440.0).abs() < 2.0, "{f}");
    assert_eq!(left, right, "centred");
    assert!(peak(&left) > 0.2 && peak(&left) < 1.0, "{}", peak(&left));
    synth.note_off(69);
    let (tail, _) = render(&mut synth, 48_000);
    assert_eq!(synth.active_voices(), 0);
    assert_eq!(peak(&tail[24_000..]), 0.0);
}

#[test]
fn octave_semi_fine_and_bend_move_the_pitch() {
    let mut synth = sine_synth();
    synth.set_param(osc_param(0, osc::OCTAVE), 1.0);
    synth.set_param(osc_param(0, osc::SEMI), -12.0);
    synth.set_param(osc_param(0, osc::FINE), 100.0);
    synth.set_param(crate::params::BEND_RANGE, 12.0);
    synth.set_pitch_bend(-1.0 / 12.0);
    synth.note_on(69, 1.0);
    let (left, _) = render(&mut synth, 9_600);
    let f = frequency(&left[2_400..]);
    assert!(
        (f - 440.0).abs() < 2.0,
        "+1 oct −12 st +100 ct −1 st = 0: {f}"
    );
}

#[test]
fn polyphony_steals_the_oldest_voice_without_a_click() {
    let mut synth = sine_synth();
    synth.set_param(POLYPHONY, 2.0);
    synth.note_on(60, 1.0);
    synth.note_on(64, 1.0);
    render(&mut synth, 480);
    synth.note_on(67, 1.0);
    assert_eq!(synth.active_voices(), 3, "the stolen voice fades");
    let (left, _) = render(&mut synth, 480);
    assert_eq!(synth.active_voices(), 2);
    let jump = left
        .windows(2)
        .map(|w| (w[1] - w[0]).abs())
        .fold(0.0, f32::max);
    assert!(jump < 0.2, "{jump}");
    let mut pitches = synth.pitches();
    pitches.sort_by(f32::total_cmp);
    assert_eq!(pitches, [64.0, 67.0]);
}

#[test]
fn mono_returns_to_the_held_key_and_legato_keeps_one_voice() {
    for mode in [1.0, 2.0] {
        let mut synth = sine_synth();
        synth.set_param(VOICE_MODE, mode);
        synth.note_on(60, 1.0);
        render(&mut synth, 480);
        synth.note_on(67, 1.0);
        render(&mut synth, 480);
        assert_eq!(synth.pitches(), [67.0]);
        assert_eq!(synth.active_voices(), 1);
        synth.note_off(67);
        render(&mut synth, 480);
        assert_eq!(synth.pitches(), [60.0], "back to the held key");
        synth.note_off(60);
        render(&mut synth, 48_000);
        assert_eq!(synth.active_voices(), 0);
    }
}

#[test]
fn glide_moves_between_notes_and_legato_mode_glides_only_when_overlapping() {
    let mut synth = sine_synth();
    synth.set_param(GLIDE, 0.1);
    synth.note_on(60, 1.0);
    synth.note_off(60);
    synth.note_on(72, 1.0);
    render(&mut synth, 2_400);
    let mid = synth.pitches().iter().copied().fold(0.0, f32::max);
    assert!(mid > 60.5 && mid < 71.5, "half-way: {mid}");
    render(&mut synth, 4_800);
    assert!(synth.pitches().contains(&72.0));

    let mut synth = sine_synth();
    synth.set_param(GLIDE, 0.1);
    synth.set_param(GLIDE_MODE, 1.0);
    synth.note_on(60, 1.0);
    synth.note_off(60);
    synth.note_on(72, 1.0);
    render(&mut synth, 32);
    assert!(synth.pitches().contains(&72.0), "no overlap, no glide");
}

#[test]
fn mod_matrix_routes_a_macro_to_the_volume() {
    let mut synth = sine_synth();
    synth.set_param(slot_param(0, slot::SOURCE), source::MACRO1 as f32);
    synth.set_param(slot_param(0, slot::DEST), (VOLUME + 1) as f32);
    synth.set_param(slot_param(0, slot::AMOUNT), -1.0);
    synth.note_on(60, 1.0);
    let (loud, _) = render(&mut synth, 4_800);
    synth.set_param(MACRO1, 1.0);
    let (quiet, _) = render(&mut synth, 4_800);
    assert!(rms(&loud[2_400..]) > 0.1);
    assert!(rms(&quiet[2_400..]) < 1e-4, "{}", rms(&quiet[2_400..]));
}

#[test]
fn an_lfo_on_the_cutoff_moves_the_brightness() {
    let mut synth = Antiserum::new(SR);
    synth.set_param(osc_param(0, osc::POSITION), 0.66); // saw
    synth.set_param(filter_param(0, filter::CUTOFF), 200.0);
    synth.set_param(lfo_param(0, lfo::SYNC), 0.0);
    synth.set_param(lfo_param(0, lfo::RATE), 1.0);
    synth.set_param(lfo_param(0, lfo::SHAPE), 2.0); // saw up
    synth.set_param(slot_param(0, slot::SOURCE), source::LFO1 as f32);
    let cutoff = filter_param(0, filter::CUTOFF);
    synth.set_param(slot_param(0, slot::DEST), (cutoff + 1) as f32);
    synth.set_param(slot_param(0, slot::AMOUNT), 1.0);
    synth.note_on(48, 1.0);
    let (left, _) = render(&mut synth, 48_000);
    // High-frequency share: the first difference's RMS over the signal's.
    let brightness = |x: &[f32]| {
        let diff: Vec<f32> = x.windows(2).map(|w| w[1] - w[0]).collect();
        rms(&diff) / rms(x)
    };
    let early = brightness(&left[1_000..5_000]);
    let late = brightness(&left[40_000..44_000]);
    assert!(late > early * 3.0, "{early} → {late}");
}

#[test]
fn every_source_warp_and_filter_type_stays_finite_and_bounded() {
    for warp in 0..WARP_COUNT {
        for kind in 0..crate::params::FILTER_TYPES.len() {
            let mut synth = Antiserum::new(SR);
            for o in 0..3 {
                synth.set_param(osc_param(o, osc::ON), 1.0);
                synth.set_param(osc_param(o, osc::UNISON), 16.0);
                synth.set_param(osc_param(o, osc::WARP1), warp as f32);
                synth.set_param(osc_param(o, osc::WARP1_AMOUNT), 0.7);
                synth.set_param(osc_param(o, osc::MOD_SOURCE), o as f32);
                synth.set_param(osc_param(o, osc::FILTER2), 0.5);
            }
            synth.set_param(SUB_BASE + sub::ON, 1.0);
            synth.set_param(NOISE_BASE + noise::ON, 1.0);
            synth.set_param(NOISE_BASE + noise::TYPE, (kind % 5) as f32);
            for f in 0..2 {
                synth.set_param(filter_param(f, filter::ON), 1.0);
                synth.set_param(filter_param(f, filter::TYPE), kind as f32);
                synth.set_param(filter_param(f, filter::RESONANCE), 1.0);
                synth.set_param(filter_param(f, filter::DRIVE), 1.0);
            }
            synth.set_param(FILTER_ROUTING, (kind % 2) as f32);
            for key in [24, 60, 96, 127] {
                synth.note_on(key, 1.0);
            }
            let (left, right) = render(&mut synth, 4_800);
            let p = peak(&left).max(peak(&right));
            assert!(p.is_finite() && p < 16.0, "warp {warp}, filter {kind}: {p}");
            assert!(p > 0.0, "warp {warp}, filter {kind}: silent");
        }
    }
}

#[test]
fn renders_are_deterministic_and_parameters_are_sanitized() {
    let run = || {
        let mut synth = Antiserum::new(SR);
        synth.set_param(osc_param(0, osc::UNISON), 7.0);
        synth.set_param(NOISE_BASE + noise::ON, 1.0);
        synth.note_on(60, 0.8);
        synth.note_on(64, 0.5);
        render(&mut synth, 4_800).0
    };
    assert_eq!(run(), run());
    let mut synth = Antiserum::new(SR);
    synth.set_param(VOLUME, f32::NAN);
    assert_eq!(synth.param(VOLUME), params()[VOLUME].default);
    synth.set_param(POLYPHONY, 99.0);
    assert_eq!(synth.param(POLYPHONY), 32.0);
    synth.set_param(PARAM_COUNT + 5, 1.0);
    assert_eq!(synth.param(PARAM_COUNT + 5), 0.0);
    // Empty and mismatched buffers.
    synth.note_on(60, 1.0);
    synth.process(&mut [], &mut []);
    let (mut l, mut r) = ([1.0; 10], [1.0; 4]);
    synth.process(&mut l, &mut r);
    assert_eq!(l[9], 1.0, "only the common length is written");
}

#[test]
fn reset_and_all_notes_off_silence_it() {
    let mut synth = sine_synth();
    for key in 60..70 {
        synth.note_on(key, 1.0);
    }
    render(&mut synth, 480);
    synth.all_notes_off();
    render(&mut synth, 48_000);
    assert_eq!(synth.active_voices(), 0);
    synth.note_on(60, 1.0);
    synth.reset();
    let (left, _) = render(&mut synth, 480);
    assert_eq!(peak(&left), 0.0);
}
