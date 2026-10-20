//! Antiserum's performance target: 8 notes × 3 oscillators × 16 unison
//! voices, both filters on and ten effects in the rack, faster than real
//! time at 48 kHz, for every oscillator type (wavetable, sample,
//! multisample, granular, spectral).
//!
//! `cargo run --release -p musiccraft-antiserum --example bench` prints the
//! real-time factor of each type and fails if any is below 1.

use std::sync::Arc;
use std::time::Instant;

use musiccraft_antiserum::params::{
    FX_SLOT_BASE, filter, filter_param, lfo, lfo_param, osc, osc_param, oscx, oscx_param, slot,
    slot_param, source,
};
use musiccraft_antiserum::{Antiserum, Source, Zone};
use musiccraft_dsp::SampleBuffer;

const SAMPLE_RATE: f32 = 48_000.0;
/// The cases: oscillator types 0–4, then spectral with stretch (which plays
/// its partials one by one).
const TYPES: [&str; 6] = [
    "wavetable",
    "sample",
    "multisample",
    "granular",
    "spectral",
    "stretched",
];

/// Two seconds of a stereo saw with some noise, to play.
fn sample() -> Arc<SampleBuffer> {
    let n = SAMPLE_RATE as usize * 2;
    let mut seed = 1_u32;
    let mut noise = move || {
        seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        (seed >> 8) as f32 / (1 << 24) as f32 - 0.5
    };
    let (left, right): (Vec<f32>, Vec<f32>) = (0..n)
        .map(|i| {
            let saw = 2.0 * (220.0 * i as f32 / SAMPLE_RATE).fract() - 1.0;
            (0.6 * saw + 0.1 * noise(), 0.6 * saw - 0.1 * noise())
        })
        .unzip();
    Arc::new(SampleBuffer::stereo(left, right, f64::from(SAMPLE_RATE)))
}

/// The heavy patch with every oscillator of type `kind` (0–4).
fn synth(kind: usize, buffer: &Arc<SampleBuffer>) -> Antiserum {
    let mut synth = Antiserum::new(SAMPLE_RATE);
    let zones = (0..4)
        .map(|z| Zone {
            buffer: Arc::clone(buffer),
            low: z * 32,
            high: z * 32 + 31,
            root: z * 32 + 16,
            vel_low: 1,
            vel_high: 127,
        })
        .collect();
    let source = Arc::new(Source::new(Some(Arc::clone(buffer)), zones));
    for o in 0..3 {
        synth.set_param(osc_param(o, osc::ON), 1.0);
        synth.set_param(osc_param(o, osc::UNISON), 16.0);
        synth.set_param(osc_param(o, osc::WARP1), 2.0);
        synth.set_param(osc_param(o, osc::WARP1_AMOUNT), 0.3);
        synth.set_param(oscx_param(o, oscx::TYPE), kind.min(4) as f32);
        synth.set_param(oscx_param(o, oscx::LOOP), 1.0);
        if kind == 5 {
            synth.set_param(oscx_param(o, oscx::SPECTRAL_STRETCH), 0.3);
        }
        let _ = synth.set_source(o, Arc::clone(&source));
    }
    for f in 0..2 {
        synth.set_param(filter_param(f, filter::ON), 1.0);
        synth.set_param(filter_param(f, filter::TYPE), 1.0);
    }
    // An LFO sweeping the cutoff, so filters recompute every block.
    synth.set_param(lfo_param(0, lfo::SYNC), 0.0);
    synth.set_param(slot_param(0, slot::SOURCE), source::LFO1 as f32);
    let cutoff = filter_param(0, filter::CUTOFF);
    synth.set_param(slot_param(0, slot::DEST), (cutoff + 1) as f32);
    synth.set_param(slot_param(0, slot::AMOUNT), -0.5);
    // A full effects rack: ten modules in the ten slots.
    for (slot, module) in [0, 1, 2, 3, 4, 5, 6, 8, 9, 10].into_iter().enumerate() {
        synth.set_param(FX_SLOT_BASE + slot, (module + 1) as f32);
    }
    synth
}

/// Seconds of audio per second of work for oscillator type `kind`.
fn factor(kind: usize, buffer: &Arc<SampleBuffer>) -> f32 {
    let mut synth = synth(kind, buffer);
    for key in [48, 52, 55, 59, 60, 64, 67, 71] {
        synth.note_on(key, 0.8);
    }
    let block = 256;
    let seconds = 5.0;
    let blocks = (seconds * SAMPLE_RATE) as usize / block;
    let mut left = vec![0.0; block];
    let mut right = vec![0.0; block];
    let started = Instant::now();
    let mut sum = 0.0_f32;
    for _ in 0..blocks {
        synth.process(&mut left, &mut right);
        sum += left[0];
    }
    let elapsed = started.elapsed().as_secs_f32();
    let factor = seconds / elapsed;
    println!(
        "{:12} 8 notes × 3 osc × 16 unison: {seconds} s in {elapsed:.2} s, \
         {factor:.1}× real time ({} voices; checksum {sum:.3})",
        TYPES[kind],
        synth.active_voices()
    );
    factor
}

fn main() {
    let buffer = sample();
    let slowest = (0..TYPES.len())
        .map(|kind| factor(kind, &buffer))
        .fold(f32::INFINITY, f32::min);
    if slowest < 1.0 {
        eprintln!("slower than real time");
        std::process::exit(1);
    }
}
