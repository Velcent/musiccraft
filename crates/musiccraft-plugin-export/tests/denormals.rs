//! Denormal input through a real host (`docs/spec/validation.md`): every
//! exported effect, loaded by our CLAP host, processes subnormal input no
//! slower than ordinary noise. Both inputs are generated before timing, so
//! only the plugin is measured (clap-validator's own check also times its
//! generation of subnormal numbers, which is slow in itself).

mod common;

use musiccraft_clap::ClapHost;
use musiccraft_engine::{External, ExternalTransport, TimeSig};
use musiccraft_plugin_export::ids::CLAP_IDS;
use musiccraft_render::{PluginFactory, PluginRequest};

const RATE: u32 = 48_000;
const BLOCK: usize = 512;
const BLOCKS: usize = 40;
/// Slower than this on subnormal input fails. Real denormal stalls cost ten
/// to a hundred times; timing noise on a shared machine stays well below.
const LIMIT: f64 = 1.6;

fn transport() -> ExternalTransport {
    ExternalTransport {
        playing: false,
        tempo_bpm: 120.0,
        position_beats: 0.0,
        sample_rate: f64::from(RATE),
        time_signature: TimeSig::COMMON,
        bar_start_beats: 0.0,
        bar_number: 0,
    }
}

/// [`BLOCKS`] blocks of input, each sample made by `sample` from a counter.
fn blocks(sample: impl Fn(u32) -> f32) -> Vec<Vec<f32>> {
    let mut seed = 0x2545_f491_u32;
    (0..BLOCKS)
        .map(|_| {
            (0..BLOCK)
                .map(|_| {
                    seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                    sample(seed)
                })
                .collect()
        })
        .collect()
}

/// The fastest of five runs of `input` through `processor`, in seconds.
fn fastest(processor: &mut dyn External, input: &[Vec<f32>]) -> f64 {
    let mut best = f64::MAX;
    let (mut l, mut r) = (vec![0.0; BLOCK], vec![0.0; BLOCK]);
    for _ in 0..5 {
        let start = std::time::Instant::now();
        for block in input {
            l.copy_from_slice(block);
            r.copy_from_slice(block);
            processor.process(&mut l, &mut r, &transport());
        }
        best = best.min(start.elapsed().as_secs_f64());
    }
    best
}

#[test]
fn subnormal_input_is_no_slower_than_noise() {
    let dir = common::install("denormals", "MusicCraft Effects");
    let host = ClapHost::with_folders(vec![dir.clone()]);
    let noise = blocks(|seed| (seed >> 8) as f32 / (1 << 24) as f32 - 0.5);
    // Uniform over the subnormal range, both signs.
    let subnormal = blocks(|seed| {
        let tiny = f32::from_bits(seed & 0x007f_ffff);
        if seed & 0x8000_0000 == 0 { tiny } else { -tiny }
    });
    assert!(subnormal.iter().flatten().any(|s| s.is_subnormal()));
    let mut slow = Vec::new();
    for id in CLAP_IDS {
        let request = PluginRequest {
            id,
            sample_rate: RATE,
            max_frames: BLOCK as u32,
            state: None,
        };
        let mut live = host.instantiate(&request).unwrap();
        // Warm up, then time the same work on each input; a ratio over the
        // limit is measured again (a busy machine can stall one run).
        fastest(live.processor.as_mut(), &noise);
        let mut ratio = f64::MAX;
        for _ in 0..3 {
            let normal = fastest(live.processor.as_mut(), &noise);
            let denormal = fastest(live.processor.as_mut(), &subnormal);
            ratio = ratio.min(denormal / normal.max(1e-9));
            if ratio <= LIMIT {
                break;
            }
        }
        if ratio > LIMIT {
            slow.push(format!("{id}: {ratio:.2}x"));
        }
        live.processor.stop();
    }
    drop(host);
    let _ = std::fs::remove_dir_all(dir);
    assert!(
        slow.is_empty(),
        "slower on subnormal input:\n{}",
        slow.join("\n")
    );
}
