//! Denormal checks (`docs/spec/validation.md`): every exported effect and
//! instrument, run as the app runs it (no flush-to-zero), decays to silence
//! without ever producing a subnormal sample, so tails stay cheap. The
//! plugins add flush-to-zero on top; the app's engine does not.

use musiccraft_antiserum::Antiserum;
use musiccraft_dsp::{Effect, ProcessContext};
use musiccraft_instruments::default_params;
use musiccraft_instruments::live::Poly;
use musiccraft_plugin_export::ids::{INSTRUMENT_KINDS, INSTRUMENT_NAMES, KINDS};
use musiccraft_plugin_export::instrument::data::Shared;

const RATE: f32 = 48_000.0;
const BLOCK: usize = 256;
/// Blocks of sound, then blocks of silence for the tail (about 0.5 s and 8 s).
const SOUND: usize = 94;
const TAIL: usize = 1_500;
const CHORD: [u8; 4] = [48, 60, 64, 67];

/// Where a block first holds a subnormal sample, if it does.
fn subnormal_at(l: &[f32], r: &[f32]) -> Option<usize> {
    l.iter().chain(r).position(|s| s.is_subnormal())
}

/// A deterministic noise block.
fn noise(seed: &mut u32, l: &mut [f32], r: &mut [f32]) {
    for (a, b) in l.iter_mut().zip(r.iter_mut()) {
        *seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *a = ((*seed >> 8) as f32 / (1 << 24) as f32 - 0.5) * 0.8;
        *b = -*a;
    }
}

#[test]
fn effect_tails_never_hold_subnormals() {
    let ctx = ProcessContext {
        sample_rate: RATE,
        tempo_bpm: 120.0,
        beats: 0.0,
    };
    let mut wrong = Vec::new();
    for kind in KINDS {
        let mut effect = Effect::new(kind, RATE);
        let values = kind.defaults();
        let mut seed = 7;
        for block in 0..SOUND + TAIL {
            let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
            if block < SOUND {
                noise(&mut seed, &mut l, &mut r);
            }
            effect.process(&mut l, &mut r, &values, ctx);
            if let Some(at) = subnormal_at(&l, &r) {
                wrong.push(format!("{kind}: block {block}, sample {at}"));
                break;
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn instrument_tails_never_hold_subnormals() {
    let mut wrong = Vec::new();
    for (kind, name) in INSTRUMENT_KINDS.into_iter().zip(INSTRUMENT_NAMES) {
        let (shared, _) = Shared::new(kind);
        shared.set_sample_rate(RATE);
        let instrument = shared.build();
        let values = default_params();
        let mut poly = Poly::new(RATE);
        for key in CHORD {
            poly.note_on(key, 0.9, &instrument, &values);
        }
        for block in 0..SOUND + TAIL {
            if block == SOUND {
                for key in CHORD {
                    poly.note_off(key);
                }
            }
            let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
            poly.render(&instrument, &values, &mut l, &mut r);
            if let Some(at) = subnormal_at(&l, &r) {
                wrong.push(format!("{name}: block {block}, sample {at}"));
                break;
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}

#[test]
fn antiserum_tails_never_hold_subnormals() {
    let mut wrong = Vec::new();
    // Every seventh factory preset: all categories and oscillator types.
    for preset in musiccraft_antiserum_presets::factory().iter().step_by(7) {
        let mut synth: Antiserum = musiccraft_antiserum_presets::synth(preset, RATE);
        for key in CHORD {
            synth.note_on(key, 0.9);
        }
        for block in 0..SOUND + TAIL {
            if block == SOUND {
                for key in CHORD {
                    synth.note_off(key);
                }
            }
            let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
            synth.process(&mut l, &mut r);
            if let Some(at) = subnormal_at(&l, &r) {
                wrong.push(format!("{}: block {block}, sample {at}", preset.name));
                break;
            }
        }
    }
    assert!(wrong.is_empty(), "{}", wrong.join("\n"));
}
