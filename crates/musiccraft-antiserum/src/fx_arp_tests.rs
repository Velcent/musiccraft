//! Phase 5 tests: the effects rack, the arpeggiator and the sequencer.

use musiccraft_dsp::{Effect, ProcessContext};
use musiccraft_params::{EffectKind, MAX_EFFECT_PARAMS};

use crate::fx::{Dimension, MODULES};
use crate::notes::{Event, Sequence, Step};
use crate::params::{MACRO1, osc, osc_param, slot, slot_param, source};
use crate::{Antiserum, index_of};

const SR: f32 = 48_000.0;
/// One 1/16 step at 120 BPM.
const STEP: u64 = 6_000;

fn param(name: &str) -> usize {
    index_of(name).unwrap_or_else(|| panic!("no parameter {name}"))
}

fn module(name: &str) -> f32 {
    (MODULES.iter().position(|m| m.name == name).unwrap() + 1) as f32
}

/// A saw-ish note, rendered in odd blocks.
fn render_note(setup: impl Fn(&mut Antiserum)) -> (Vec<f32>, Vec<f32>) {
    let mut synth = Antiserum::new(SR);
    synth.set_param(osc_param(0, osc::POSITION), 0.66);
    setup(&mut synth);
    synth.note_on(57, 1.0);
    let mut left = vec![0.0; 9_600];
    let mut right = vec![0.0; 9_600];
    for (l, r) in left.chunks_mut(100).zip(right.chunks_mut(100)) {
        synth.process(l, r);
    }
    (left, right)
}

fn differ(a: &[f32], b: &[f32]) -> f32 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0, f32::max)
}

#[test]
fn modules_play_only_from_a_slot_and_once() {
    let dry = render_note(|_| {});
    let unslotted = render_note(|s| s.set_param(param("fx_distortion_drive"), 30.0));
    assert_eq!(dry, unslotted, "a module outside the slots does nothing");
    let distorted = render_note(|s| {
        s.set_param(param("fx_slot3"), module("distortion"));
        s.set_param(param("fx_distortion_drive"), 30.0);
    });
    assert!(differ(&dry.0, &distorted.0) > 0.05);
    let twice = render_note(|s| {
        s.set_param(param("fx_slot3"), module("distortion"));
        s.set_param(param("fx_slot7"), module("distortion"));
        s.set_param(param("fx_distortion_drive"), 30.0);
    });
    assert_eq!(distorted, twice, "named twice, played once");
}

#[test]
fn the_distortion_module_matches_the_mixer_effect() {
    let drive = 12.0;
    let dry = render_note(|_| {});
    let synth = render_note(|s| {
        s.set_param(param("fx_slot1"), module("distortion"));
        s.set_param(param("fx_distortion_drive"), drive);
    });
    let mut params: [f32; MAX_EFFECT_PARAMS] = EffectKind::Distortion.defaults();
    let drive_index = EffectKind::Distortion
        .params()
        .iter()
        .position(|p| p.name == "drive")
        .unwrap();
    params[drive_index] = drive;
    let mut effect = Effect::new(EffectKind::Distortion, SR);
    let (mut l, mut r) = dry;
    let ctx = ProcessContext {
        sample_rate: SR,
        tempo_bpm: 120.0,
        beats: 0.0,
    };
    effect.process(&mut l, &mut r, &params, ctx);
    assert!(differ(&l, &synth.0) < 1e-6);
    assert!(differ(&r, &synth.1) < 1e-6);
}

#[test]
fn slot_order_matters() {
    let order = |first: &str, second: &str| {
        render_note(|s| {
            s.set_param(param("fx_slot1"), module(first));
            s.set_param(param("fx_slot2"), module(second));
            s.set_param(param("fx_distortion_drive"), 30.0);
            s.set_param(param("fx_filter_cutoff"), 400.0);
        })
    };
    let a = order("distortion", "filter");
    let b = order("filter", "distortion");
    assert!(differ(&a.0, &b.0) > 0.05);
}

#[test]
fn every_module_stays_finite() {
    for (m, module) in MODULES.iter().enumerate() {
        let (l, r) = render_note(|s| s.set_param(param("fx_slot1"), (m + 1) as f32));
        assert!(
            l.iter().chain(&r).all(|x| x.is_finite() && x.abs() < 8.0),
            "{}",
            module.name
        );
    }
}

#[test]
fn dimension_widens_and_stays_mono_compatible() {
    let mut dimension = Dimension::new(SR);
    let input: Vec<f32> = (0..24_000)
        .map(|i| (std::f32::consts::TAU * 110.0 * i as f32 / SR).sin() * 0.5)
        .collect();
    let (mut l, mut r) = (input.clone(), input.clone());
    for (lb, rb) in l.chunks_mut(64).zip(r.chunks_mut(64)) {
        dimension.process(lb, rb, 0.5, 0.7);
    }
    let rms = |x: &mut dyn Iterator<Item = f32>| {
        let v: Vec<f32> = x.collect();
        (v.iter().map(|s| s * s).sum::<f32>() / v.len() as f32).sqrt()
    };
    let tail = 4_800..;
    let side = rms(&mut l[tail.clone()]
        .iter()
        .zip(&r[tail.clone()])
        .map(|(a, b)| a - b));
    let mono = rms(&mut l[tail.clone()]
        .iter()
        .zip(&r[tail.clone()])
        .map(|(a, b)| a + b));
    let original = rms(&mut input[tail].iter().map(|x| 2.0 * x));
    assert!(side > 0.3, "wider: {side}");
    assert!(
        (mono / original - 1.0).abs() < 0.15,
        "mono sum {mono} vs {original}"
    );
}

#[test]
fn a_macro_modulates_an_effect_parameter() {
    let setup = |macro_value: f32| {
        move |s: &mut Antiserum| {
            s.set_param(param("fx_slot1"), module("distortion"));
            s.set_param(param("fx_distortion_drive"), 30.0);
            s.set_param(param("fx_distortion_mix"), 0.0);
            s.set_param(slot_param(0, slot::SOURCE), source::MACRO1 as f32);
            s.set_param(
                slot_param(0, slot::DEST),
                (param("fx_distortion_mix") + 1) as f32,
            );
            s.set_param(slot_param(0, slot::AMOUNT), 1.0);
            s.set_param(MACRO1, macro_value);
        }
    };
    let unmodulated = render_note(|s| {
        s.set_param(param("fx_slot1"), module("distortion"));
        s.set_param(param("fx_distortion_drive"), 30.0);
        s.set_param(param("fx_distortion_mix"), 0.0);
    });
    let off = render_note(setup(0.0));
    let on = render_note(setup(1.0));
    assert!(differ(&unmodulated.0, &off.0) < 1e-6, "macro at 0");
    assert!(
        differ(&unmodulated.0, &on.0) > 0.05,
        "the macro raised the mix"
    );
}

/// A synth with the arpeggiator in `mode` at 1/16, holding `keys`, rendered
/// for `frames`; returns the generated events.
fn arp(setup: impl Fn(&mut Antiserum), keys: &[u8], frames: usize) -> Vec<(u64, Event)> {
    let mut synth = Antiserum::new(SR);
    setup(&mut synth);
    for &key in keys {
        synth.note_on(key, 0.8);
    }
    let mut left = vec![0.0; frames];
    let mut right = vec![0.0; frames];
    for (l, r) in left.chunks_mut(512).zip(right.chunks_mut(512)) {
        synth.process(l, r);
    }
    synth.log.clone()
}

fn ons(events: &[(u64, Event)]) -> Vec<(u64, u8)> {
    events
        .iter()
        .filter_map(|(t, e)| match e {
            Event::On(k, _) => Some((*t, *k)),
            Event::Off(_) => None,
        })
        .collect()
}

fn arp_mode(mode: f32, range: f32) -> impl Fn(&mut Antiserum) {
    move |s: &mut Antiserum| {
        s.set_param(param("arp_mode"), mode);
        s.set_param(param("arp_range"), range);
    }
}

#[test]
fn the_arpeggiator_steps_up_on_exact_samples_with_its_gate() {
    let events = arp(arp_mode(1.0, 1.0), &[64, 60, 67], 25_000);
    assert_eq!(
        ons(&events),
        [
            (0, 60),
            (STEP, 64),
            (2 * STEP, 67),
            (3 * STEP, 60),
            (4 * STEP, 64)
        ]
    );
    // Gate 0.5: each note ends half a step later.
    assert!(events.contains(&(STEP / 2, Event::Off(60))), "{events:?}");
    assert!(events.contains(&(STEP + STEP / 2, Event::Off(64))));
    assert!(matches!(events[0].1, Event::On(60, v) if (v - 0.8).abs() < 1e-6));
}

#[test]
fn arpeggiator_orders_and_octaves() {
    let keys =
        |events: Vec<(u64, Event)>| -> Vec<u8> { ons(&events).iter().map(|e| e.1).collect() };
    let frames = (7 * STEP) as usize - 10;
    assert_eq!(
        keys(arp(arp_mode(2.0, 1.0), &[60, 64, 67], frames)),
        [67, 64, 60, 67, 64, 60, 67]
    );
    assert_eq!(
        keys(arp(arp_mode(3.0, 1.0), &[60, 64, 67], frames)),
        [60, 64, 67, 64, 60, 64, 67]
    );
    assert_eq!(
        keys(arp(arp_mode(1.0, 2.0), &[60, 64], frames)),
        [60, 64, 72, 76, 60, 64, 72]
    );
    let random = keys(arp(arp_mode(4.0, 1.0), &[60, 64, 67], frames));
    assert_eq!(
        random,
        keys(arp(arp_mode(4.0, 1.0), &[60, 64, 67], frames)),
        "repeats"
    );
    assert!(random.iter().all(|k| [60, 64, 67].contains(k)));
}

#[test]
fn releasing_the_last_key_stops_the_pattern() {
    let mut synth = Antiserum::new(SR);
    synth.set_param(param("arp_mode"), 1.0);
    synth.set_param(param("arp_gate"), 1.0);
    synth.note_on(60, 1.0);
    let mut l = vec![0.0; 1_000];
    let mut r = vec![0.0; 1_000];
    synth.process(&mut l, &mut r);
    synth.note_off(60);
    assert_eq!(synth.log.last(), Some(&(1_000, Event::Off(60))));
    for _ in 0..20 {
        synth.process(&mut l, &mut r);
    }
    assert_eq!(ons(&synth.log).len(), 1, "no more steps");
    assert_eq!(synth.active_voices(), 0, "and the note has died away");
}

#[test]
fn the_sequencer_transposes_skips_and_scales_velocity() {
    let mut sequence = Sequence::default();
    sequence.steps[0] = Step {
        transpose: 12,
        ..Step::default()
    };
    sequence.steps[1].on = false;
    sequence.steps[2] = Step {
        transpose: -5,
        velocity: 0.5,
        gate: 1.0,
        ..Step::default()
    };
    let setup = move |s: &mut Antiserum| {
        s.set_param(param("seq_on"), 1.0);
        s.set_param(param("seq_length"), 3.0);
        // The arpeggiator is ignored while the sequencer runs.
        s.set_param(param("arp_mode"), 2.0);
        s.set_sequence(&sequence);
    };
    let events = arp(setup, &[60], (4 * STEP) as usize + 10);
    assert_eq!(ons(&events), [(0, 72), (2 * STEP, 55), (3 * STEP, 72)]);
    let velocity = events.iter().find_map(|(t, e)| match e {
        Event::On(55, v) if *t == 2 * STEP => Some(*v),
        _ => None,
    });
    assert!((velocity.unwrap() - 0.4).abs() < 1e-6);
    // Gate 1: the note ends just as the next step starts.
    assert!(events.contains(&(3 * STEP, Event::Off(55))));
    // A chord plays the pattern as a chord.
    let chord = arp(setup, &[60, 64], STEP as usize);
    assert_eq!(ons(&chord), [(0, 72), (0, 76)]);
}

#[test]
fn sequence_values_are_clamped() {
    let mut synth = Antiserum::new(SR);
    let mut sequence = Sequence::default();
    sequence.steps[5] = Step {
        on: true,
        transpose: 100,
        velocity: f32::NAN,
        gate: 7.0,
    };
    synth.set_sequence(&sequence);
    let step = synth.sequence().steps[5];
    assert_eq!((step.transpose, step.velocity, step.gate), (24, 1.0, 1.0));
}
