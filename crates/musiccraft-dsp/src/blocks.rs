//! Macro blocks: a small library of effects, each played with just two
//! controls (X and Y, both 0–1), used by the X/Y multi-effect and the
//! four-stage effect chain. All allocation happens in `new`.

use std::f32::consts::TAU;

use crate::character::{FreqShift, RingMod};
use crate::modulation::{Bitcrusher, Flanger, Phaser};
use crate::{Chorus, Distortion, ProcessContext, Reverb, Svf, SvfMode, db_to_gain};

/// Names of the macro blocks, by block number; 0 is off.
pub const BLOCK_NAMES: [&str; 14] = [
    "off",
    "low-pass",
    "high-pass",
    "drive",
    "crush",
    "ring mod",
    "phaser",
    "flanger",
    "chorus",
    "reverb",
    "width",
    "tremolo",
    "freq shift",
    "fold",
];

/// What X and Y do for each block, by block number.
pub const BLOCK_CONTROLS: [(&str, &str); 14] = [
    ("", ""),
    ("cutoff", "resonance"),
    ("cutoff", "resonance"),
    ("drive", "mix"),
    ("bits", "rate"),
    ("pitch", "mix"),
    ("speed", "feedback"),
    ("speed", "feedback"),
    ("speed", "depth"),
    ("size", "mix"),
    ("width", "balance"),
    ("speed", "depth"),
    ("shift", "mix"),
    ("drive", "mix"),
];

/// `0..1` mapped exponentially onto `lo..hi`.
fn exp_range(t: f32, lo: f32, hi: f32) -> f32 {
    lo * (hi / lo).powf(t.clamp(0.0, 1.0))
}

/// One instance of every block, so switching blocks never allocates.
#[derive(Debug, Clone)]
pub struct MacroBlock {
    filters: [Svf; 2],
    crusher: Bitcrusher,
    ring: RingMod,
    phaser: Phaser,
    flanger: Flanger,
    chorus: Chorus,
    reverb: Reverb,
    shifter: FreqShift,
    tremolo: f32,
    last: usize,
}

impl MacroBlock {
    /// Allocates every block's buffers. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            filters: [Svf::new(), Svf::new()],
            crusher: Bitcrusher::default(),
            ring: RingMod::default(),
            phaser: Phaser::default(),
            flanger: Flanger::new(sample_rate),
            chorus: Chorus::new(sample_rate),
            reverb: Reverb::new(sample_rate),
            shifter: FreqShift::default(),
            tremolo: 0.0,
            last: 0,
        }
    }

    /// Clears every block's memory.
    pub fn reset(&mut self) {
        self.filters = [Svf::new(), Svf::new()];
        self.crusher.reset();
        self.ring = RingMod::default();
        self.phaser.reset();
        self.flanger.reset();
        self.chorus.reset();
        self.reverb.reset();
        self.shifter = FreqShift::default();
        self.tremolo = 0.0;
    }

    /// Runs block `block` (see [`BLOCK_NAMES`]) on a stereo block with the
    /// controls `x` and `y`.
    pub fn process(
        &mut self,
        block: usize,
        x: f32,
        y: f32,
        left: &mut [f32],
        right: &mut [f32],
        ctx: ProcessContext,
    ) {
        let (x, y) = (x.clamp(0.0, 1.0), y.clamp(0.0, 1.0));
        if block != self.last {
            // A block picked again starts clean instead of replaying an old tail.
            self.reset();
            self.last = block;
        }
        match block {
            1 | 2 => {
                let mode = if block == 1 {
                    SvfMode::LowPass
                } else {
                    SvfMode::HighPass
                };
                let cutoff = exp_range(x, 40.0, 18_000.0);
                for (filter, channel) in self.filters.iter_mut().zip([left, right]) {
                    filter.set(cutoff, y * 0.95, ctx.sample_rate);
                    for s in channel.iter_mut() {
                        *s = filter.process(*s, mode);
                    }
                }
            }
            3 => Distortion::process(left, right, &[x * 36.0, y, -x * 9.0]),
            4 => {
                let bits = 16.0 - (x * 14.0).round();
                let hold = 1.0 + (y * 31.0).round();
                self.crusher
                    .process(left, right, &[bits, hold, 1.0, 0.0, 0.0]);
            }
            5 => self
                .ring
                .process(left, right, &[exp_range(x, 20.0, 3_000.0), y], ctx),
            6 => self.phaser.process(
                left,
                right,
                &[exp_range(x, 0.05, 8.0), 0.8, y * 0.9, 6.0, 0.5, 0.0],
                ctx,
            ),
            7 => self.flanger.process(
                left,
                right,
                &[exp_range(x, 0.05, 8.0), 2.0, 1.5, y * 1.9 - 0.95, 0.5],
                ctx,
            ),
            8 => self.chorus.process(
                left,
                right,
                &[exp_range(x, 0.05, 6.0), y * 8.0, 12.0, 0.5, 2.0, 20_000.0],
                ctx,
            ),
            9 => self.reverb.process(left, right, &[x, 0.4, 1.0, y]),
            10 => crate::modulation::stereo(left, right, &[x * 2.0, y * 2.0 - 1.0, 0.0, 0.0]),
            11 => {
                let step = exp_range(x, 0.5, 20.0) / ctx.sample_rate.max(1.0);
                for (l, r) in left.iter_mut().zip(right.iter_mut()) {
                    let gain = 1.0 - y * (0.5 + 0.5 * (self.tremolo * TAU).sin());
                    *l *= gain;
                    *r *= gain;
                    self.tremolo = (self.tremolo + step).fract();
                }
            }
            12 => self
                .shifter
                .process(left, right, &[x * 2_000.0 - 1_000.0, y], ctx),
            13 => crate::character::waveshaper(left, right, &[2.0, x * 30.0, y, -x * 6.0]),
            _ => {}
        }
    }
}

/// X/Y multi-effect: one block from [`BLOCK_NAMES`] played with two controls.
///
/// Parameters: `[block, x, y, mix, output dB]`.
#[derive(Debug, Clone)]
pub struct MultiFx {
    block: MacroBlock,
}

impl MultiFx {
    /// Allocates the blocks. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            block: MacroBlock::new(sample_rate),
        }
    }

    /// Clears the blocks.
    pub fn reset(&mut self) {
        self.block.reset();
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let block = param(0, 1.0).round().max(0.0) as usize;
        let mix = param(3, 1.0).clamp(0.0, 1.0);
        let output = db_to_gain(param(4, 0.0));
        blend(left, right, mix, output, |l, r| {
            self.block
                .process(block, param(1, 0.5), param(2, 0.5), l, r, ctx);
        });
    }
}

/// Number of stages of the effect chain.
pub const CHAIN_STAGES: usize = 4;

/// Four macro blocks in series.
///
/// Parameters: `[(block, x, y) × 4, mix, output dB]`.
#[derive(Debug, Clone)]
pub struct FxChain {
    stages: [MacroBlock; CHAIN_STAGES],
}

impl FxChain {
    /// Allocates every stage's blocks. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        Self {
            stages: std::array::from_fn(|_| MacroBlock::new(sample_rate)),
        }
    }

    /// Clears every stage.
    pub fn reset(&mut self) {
        for stage in &mut self.stages {
            stage.reset();
        }
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let mix = param(CHAIN_STAGES * 3, 1.0).clamp(0.0, 1.0);
        let output = db_to_gain(param(CHAIN_STAGES * 3 + 1, 0.0));
        let stages = &mut self.stages;
        blend(left, right, mix, output, |l, r| {
            for (s, stage) in stages.iter_mut().enumerate() {
                let block = param(s * 3, 0.0).round().max(0.0) as usize;
                stage.process(
                    block,
                    param(s * 3 + 1, 0.5),
                    param(s * 3 + 2, 0.5),
                    l,
                    r,
                    ctx,
                );
            }
        });
    }
}

/// Most frames blended per pass; larger blocks are processed in pieces.
const BLEND_FRAMES: usize = 256;

/// Runs `wet` on the audio and blends the result with the dry signal,
/// without allocating: the dry copy lives in a fixed stack buffer, so the
/// audio is processed `BLEND_FRAMES` at a time.
fn blend(
    left: &mut [f32],
    right: &mut [f32],
    mix: f32,
    output: f32,
    mut wet: impl FnMut(&mut [f32], &mut [f32]),
) {
    for (l, r) in left
        .chunks_mut(BLEND_FRAMES)
        .zip(right.chunks_mut(BLEND_FRAMES))
    {
        let mut dry = [[0.0_f32; BLEND_FRAMES]; 2];
        for (d, s) in dry.iter_mut().zip([&*l, &*r]) {
            for (a, b) in d.iter_mut().zip(s.iter()) {
                *a = *b;
            }
        }
        wet(l, r);
        for (channel, d) in [l, r].into_iter().zip(&dry) {
            for (s, dry) in channel.iter_mut().zip(d.iter()) {
                *s = (dry + (*s - dry) * mix) * output;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ProcessContext {
        ProcessContext {
            sample_rate: 48_000.0,
            tempo_bpm: 120.0,
            beats: 0.0,
        }
    }

    fn noise(n: usize) -> Vec<f32> {
        let mut seed = 12_345_u32;
        (0..n)
            .map(|_| {
                seed = seed.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
                (seed >> 9) as f32 / (1 << 23) as f32 - 0.5
            })
            .collect()
    }

    #[test]
    fn every_block_is_bounded_and_off_is_transparent() {
        for (block, name) in BLOCK_NAMES.iter().enumerate() {
            let mut fx = MacroBlock::new(48_000.0);
            let input = noise(9_600);
            let (mut l, mut r) = (input.clone(), input.clone());
            for x in [0.0, 0.5, 1.0] {
                fx.process(block, x, 1.0 - x, &mut l, &mut r, ctx());
            }
            assert!(
                l.iter().chain(&r).all(|s| s.is_finite() && s.abs() < 4.0),
                "{name}"
            );
            if block == 0 {
                assert_eq!(l, input);
            }
        }
    }

    #[test]
    fn multi_fx_mix_zero_is_dry_and_output_scales() {
        let mut fx = MultiFx::new(48_000.0);
        let input = noise(1_000);
        let (mut l, mut r) = (input.clone(), input.clone());
        fx.process(&mut l, &mut r, &[3.0, 1.0, 1.0, 0.0, -6.0206], ctx());
        for (a, b) in l.iter().zip(&input) {
            assert!((a - b * 0.5).abs() < 1e-4);
        }
    }

    #[test]
    fn chain_runs_stages_in_order() {
        // A low-pass then nothing: the same as the low-pass alone.
        let mut chain = FxChain::new(48_000.0);
        let mut single = MacroBlock::new(48_000.0);
        let input = noise(2_000);
        let (mut l, mut r) = (input.clone(), input.clone());
        let mut p = [0.0; 14];
        p[0] = 1.0;
        p[1] = 0.3;
        p[2] = 0.2;
        p[12] = 1.0;
        chain.process(&mut l, &mut r, &p, ctx());
        let (mut l2, mut r2) = (input.clone(), input);
        single.process(1, 0.3, 0.2, &mut l2, &mut r2, ctx());
        for (a, b) in l.iter().zip(&l2) {
            assert!((a - b).abs() < 1e-5);
        }
    }
}
