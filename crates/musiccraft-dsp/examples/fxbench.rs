//! Times each built-in effect on ten seconds of stereo audio.
use musiccraft_dsp::{Effect, ProcessContext};
use musiccraft_params::{EFFECT_KINDS, MAX_EFFECT_PARAMS};

fn main() {
    let sr = 48_000.0;
    let ctx = ProcessContext {
        sample_rate: sr,
        tempo_bpm: 120.0,
        beats: 0.0,
    };
    for kind in EFFECT_KINDS {
        let mut effect = if kind == musiccraft_params::EffectKind::Convolution {
            Effect::Convolution(musiccraft_dsp::convolution::Convolver::builtin(
                "cathedral",
                sr,
            ))
        } else {
            Effect::new(kind, sr)
        };
        let mut params = [0.0; MAX_EFFECT_PARAMS];
        params[..kind.params().len()].copy_from_slice(&kind.defaults()[..kind.params().len()]);
        let mut l = vec![0.0f32; 256];
        let mut r = vec![0.0f32; 256];
        let start = std::time::Instant::now();
        let blocks = (sr as usize * 10) / 256;
        let mut seed = 1u32;
        for _ in 0..blocks {
            for (a, b) in l.iter_mut().zip(r.iter_mut()) {
                seed = seed.wrapping_mul(1664525).wrapping_add(1013904223);
                *a = (seed as f32 / u32::MAX as f32 - 0.5) * 0.5;
                *b = *a;
            }
            effect.process(&mut l, &mut r, &params, ctx);
        }
        let secs = start.elapsed().as_secs_f64();
        println!(
            "{:<14} {:6.2}% of one core",
            kind.name(),
            secs / 10.0 * 100.0
        );
    }
}
