//! Antiserum's performance target: 8 notes × 3 oscillators × 16 unison
//! voices, both filters on and ten effects in the rack, faster than real
//! time at 48 kHz.
//!
//! `cargo run --release -p musiccraft-antiserum --example bench` prints the
//! real-time factor and fails if it is below 1.

use std::time::Instant;

use musiccraft_antiserum::Antiserum;
use musiccraft_antiserum::params::{
    FX_SLOT_BASE, filter, filter_param, lfo, lfo_param, osc, osc_param, slot, slot_param, source,
};

fn main() {
    let sample_rate = 48_000.0;
    let mut synth = Antiserum::new(sample_rate);
    for o in 0..3 {
        synth.set_param(osc_param(o, osc::ON), 1.0);
        synth.set_param(osc_param(o, osc::UNISON), 16.0);
        synth.set_param(osc_param(o, osc::WARP1), 2.0);
        synth.set_param(osc_param(o, osc::WARP1_AMOUNT), 0.3);
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
    for key in [48, 52, 55, 59, 60, 64, 67, 71] {
        synth.note_on(key, 0.8);
    }
    let block = 256;
    let seconds = 10.0;
    let blocks = (seconds * sample_rate) as usize / block;
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
        "8 notes × 3 osc × 16 unison: {seconds} s rendered in {elapsed:.2} s, \
         {factor:.1}× real time ({} voices; checksum {sum:.3})",
        synth.active_voices()
    );
    if factor < 1.0 {
        eprintln!("slower than real time");
        std::process::exit(1);
    }
}
