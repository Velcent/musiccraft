//! CPU use of every exported effect and built-in instrument
//! (`docs/spec/validation.md`), as each plugin runs it, at 48 kHz in
//! 512-frame blocks. Exits with an error when one goes over its budget.
//! Antiserum has its own benchmark (`musiccraft-antiserum --example bench`).
//!
//! `cargo run --release -p musiccraft-plugin-export --example bench`

use std::time::Instant;

use musiccraft_dsp::{Effect, ProcessContext, flush_subnormals};
use musiccraft_instruments::default_params;
use musiccraft_instruments::live::Poly;
use musiccraft_plugin_export::ids::{INSTRUMENT_KINDS, INSTRUMENT_NAMES, KINDS};
use musiccraft_plugin_export::instrument::data::Shared;

const RATE: f32 = 48_000.0;
const BLOCK: usize = 512;
const SECONDS: f32 = 5.0;
/// Most of one core an effect may use (percent).
const EFFECT_BUDGET: f64 = 10.0;
/// Most of one core an instrument playing [`CHORD`] may use (percent). The
/// heaviest (keys) used about 13% on the machine that set it.
const INSTRUMENT_BUDGET: f64 = 30.0;
/// Eight voices: a common pad or chord load.
const CHORD: [u8; 8] = [36, 48, 55, 60, 64, 67, 71, 76];

fn blocks() -> usize {
    (RATE * SECONDS) as usize / BLOCK
}

/// Percent of one core used by `run`, which renders [`SECONDS`] of audio.
/// The fastest of three runs, so a busy machine does not fail the check.
fn percent(mut run: impl FnMut()) -> f64 {
    let mut best = f64::MAX;
    for _ in 0..3 {
        let start = Instant::now();
        run();
        best = best.min(start.elapsed().as_secs_f64());
    }
    best / f64::from(SECONDS) * 100.0
}

fn main() {
    let mut over = Vec::new();
    let ctx = ProcessContext {
        sample_rate: RATE,
        tempo_bpm: 120.0,
        beats: 0.0,
    };
    let mut input = vec![0.0f32; BLOCK];
    let mut seed = 1u32;
    for s in &mut input {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        *s = ((seed >> 8) as f32 / (1 << 24) as f32 - 0.5) * 0.5;
    }
    for kind in KINDS {
        let mut effect = Effect::new(kind, RATE);
        let values = kind.defaults();
        let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
        let used = percent(|| {
            for _ in 0..blocks() {
                l.copy_from_slice(&input);
                r.copy_from_slice(&input);
                flush_subnormals(&mut l);
                flush_subnormals(&mut r);
                effect.process(&mut l, &mut r, &values, ctx);
            }
        });
        println!("effect     {:<18} {used:5.2}% of one core", kind.name());
        if used > EFFECT_BUDGET {
            over.push(format!("{kind}: {used:.2}% (budget {EFFECT_BUDGET}%)"));
        }
    }
    for (kind, name) in INSTRUMENT_KINDS.into_iter().zip(INSTRUMENT_NAMES) {
        let (shared, _) = Shared::new(kind);
        shared.set_sample_rate(RATE);
        let instrument = shared.build();
        let values = default_params();
        let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
        let used = percent(|| {
            let mut poly = Poly::new(RATE);
            for key in CHORD {
                poly.note_on(key, 0.8, &instrument, &values);
            }
            for _ in 0..blocks() {
                l.fill(0.0);
                r.fill(0.0);
                poly.render(&instrument, &values, &mut l, &mut r);
            }
        });
        println!("instrument {name:<18} {used:5.2}% of one core (8 voices)");
        if used > INSTRUMENT_BUDGET {
            over.push(format!("{name}: {used:.2}% (budget {INSTRUMENT_BUDGET}%)"));
        }
    }
    if !over.is_empty() {
        eprintln!("over budget:\n{}", over.join("\n"));
        std::process::exit(1);
    }
}
