use musiccraft_params::{EffectKind, MAX_EFFECT_PARAMS};

use crate::modulation::{Bitcrusher, Flanger, Gate, Phaser};
use crate::{Biquad, Chorus, Compressor, Delay, Distortion, Limiter, Reverb, Svf, SvfMode};

/// Information every effect may need while processing.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ProcessContext {
    /// Sample rate in hertz.
    pub sample_rate: f32,
    /// Current project tempo.
    pub tempo_bpm: f64,
    /// Beat position at the start of the block: the song position while
    /// playing, a free-running clock while stopped.
    pub beats: f64,
}

/// Stereo state-variable filter effect.
///
/// Parameters: `[mode, cutoff Hz, resonance, mix]`.
#[derive(Debug, Clone, Default)]
pub struct FilterEffect {
    left: Svf,
    right: Svf,
}

impl FilterEffect {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let mode = SvfMode::from_param(param(0, 0.0));
        let cutoff = param(1, 2_000.0);
        let resonance = param(2, 0.3);
        let mix = param(3, 1.0).clamp(0.0, 1.0);
        self.left.set(cutoff, resonance, ctx.sample_rate);
        self.right.set(cutoff, resonance, ctx.sample_rate);
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let wet_l = self.left.process(*l, mode);
            let wet_r = self.right.process(*r, mode);
            *l += (wet_l - *l) * mix;
            *r += (wet_r - *r) * mix;
        }
    }
}

/// Three-band equalizer: low shelf, peak, high shelf.
///
/// Parameters: `[low gain, low freq, mid gain, mid freq, mid q, high gain, high freq]`.
#[derive(Debug, Clone, Default)]
pub struct ThreeBandEq {
    bands_l: [Biquad; 3],
    bands_r: [Biquad; 3],
}

impl ThreeBandEq {
    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let sr = ctx.sample_rate;
        for bands in [&mut self.bands_l, &mut self.bands_r] {
            let [low, mid, high] = bands;
            low.set_low_shelf(param(1, 120.0), param(0, 0.0), sr);
            mid.set_peak(param(3, 1_000.0), param(2, 0.0), param(4, 0.7), sr);
            high.set_high_shelf(param(6, 8_000.0), param(5, 0.0), sr);
        }
        for (channel, bands) in [(left, &mut self.bands_l), (right, &mut self.bands_r)] {
            for sample in channel.iter_mut() {
                let mut x = *sample;
                for band in bands.iter_mut() {
                    x = band.process(x);
                }
                *sample = x;
            }
        }
    }
}

/// One insert effect with its own state.
#[derive(Debug, Clone)]
pub enum Effect {
    /// See [`FilterEffect`].
    Filter(FilterEffect),
    /// See [`ThreeBandEq`].
    Eq(ThreeBandEq),
    /// See [`Delay`].
    Delay(Delay),
    /// See [`Reverb`].
    Reverb(Reverb),
    /// See [`Compressor`].
    Compressor(Compressor),
    /// See [`Distortion`].
    Distortion,
    /// See [`Chorus`].
    Chorus(Chorus),
    /// See [`Limiter`].
    Limiter(Limiter),
    /// See [`Phaser`].
    Phaser(Phaser),
    /// See [`Flanger`].
    Flanger(Flanger),
    /// See [`Bitcrusher`].
    Bitcrusher(Bitcrusher),
    /// See [`crate::modulation::stereo`].
    Stereo,
    /// See [`Gate`].
    Gate(Gate),
    /// See [`crate::modulation::autopan`].
    Autopan,
    /// Leaves audio untouched; a plugin slot, processed by the engine's
    /// external processor instead.
    Passthrough,
}

impl Effect {
    /// Creates an effect of `kind`, allocating any buffers it needs. Main thread
    /// only.
    #[must_use]
    pub fn new(kind: EffectKind, sample_rate: f32) -> Self {
        match kind {
            EffectKind::Filter => Self::Filter(FilterEffect::default()),
            EffectKind::Eq => Self::Eq(ThreeBandEq::default()),
            EffectKind::Delay => Self::Delay(Delay::new(sample_rate)),
            EffectKind::Reverb => Self::Reverb(Reverb::new(sample_rate)),
            EffectKind::Compressor => Self::Compressor(Compressor::new()),
            EffectKind::Distortion => Self::Distortion,
            EffectKind::Chorus => Self::Chorus(Chorus::new(sample_rate)),
            EffectKind::Limiter => Self::Limiter(Limiter::default()),
            EffectKind::Phaser => Self::Phaser(Phaser::default()),
            EffectKind::Flanger => Self::Flanger(Flanger::new(sample_rate)),
            EffectKind::Bitcrusher => Self::Bitcrusher(Bitcrusher::default()),
            EffectKind::Stereo => Self::Stereo,
            EffectKind::Gate => Self::Gate(Gate::default()),
            EffectKind::Autopan => Self::Autopan,
            EffectKind::Plugin => Self::Passthrough,
        }
    }

    /// The kind this effect was created as.
    #[must_use]
    pub fn kind(&self) -> EffectKind {
        match self {
            Self::Filter(_) => EffectKind::Filter,
            Self::Eq(_) => EffectKind::Eq,
            Self::Delay(_) => EffectKind::Delay,
            Self::Reverb(_) => EffectKind::Reverb,
            Self::Compressor(_) => EffectKind::Compressor,
            Self::Distortion => EffectKind::Distortion,
            Self::Chorus(_) => EffectKind::Chorus,
            Self::Limiter(_) => EffectKind::Limiter,
            Self::Phaser(_) => EffectKind::Phaser,
            Self::Flanger(_) => EffectKind::Flanger,
            Self::Bitcrusher(_) => EffectKind::Bitcrusher,
            Self::Stereo => EffectKind::Stereo,
            Self::Gate(_) => EffectKind::Gate,
            Self::Autopan => EffectKind::Autopan,
            Self::Passthrough => EffectKind::Plugin,
        }
    }

    /// Processes a stereo block in place.
    pub fn process(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        params: &[f32; MAX_EFFECT_PARAMS],
        ctx: ProcessContext,
    ) {
        self.process_keyed(left, right, params, ctx, None);
    }

    /// Like [`Effect::process`], with an optional sidechain signal for
    /// effects that use one (the compressor).
    pub fn process_keyed(
        &mut self,
        left: &mut [f32],
        right: &mut [f32],
        params: &[f32; MAX_EFFECT_PARAMS],
        ctx: ProcessContext,
        key: Option<(&[f32], &[f32])>,
    ) {
        match self {
            Self::Filter(fx) => fx.process(left, right, params, ctx),
            Self::Eq(fx) => fx.process(left, right, params, ctx),
            Self::Delay(fx) => fx.process(left, right, params, ctx),
            Self::Reverb(fx) => fx.process(left, right, params),
            Self::Compressor(fx) => fx.process_keyed(left, right, params, ctx, key),
            Self::Distortion => Distortion::process(left, right, params),
            Self::Chorus(fx) => fx.process(left, right, params, ctx),
            Self::Limiter(fx) => fx.process(left, right, params, ctx),
            Self::Phaser(fx) => fx.process(left, right, params, ctx),
            Self::Flanger(fx) => fx.process(left, right, params, ctx),
            Self::Bitcrusher(fx) => fx.process(left, right, params),
            Self::Stereo => crate::modulation::stereo(left, right, params),
            Self::Gate(fx) => fx.process(left, right, params, ctx),
            Self::Autopan => crate::modulation::autopan(left, right, params, ctx),
            Self::Passthrough => {}
        }
    }

    /// Clears any internal memory (tails, detectors).
    pub fn reset(&mut self) {
        match self {
            Self::Filter(fx) => *fx = FilterEffect::default(),
            Self::Eq(fx) => *fx = ThreeBandEq::default(),
            Self::Delay(fx) => fx.reset(),
            Self::Reverb(fx) => fx.reset(),
            Self::Compressor(fx) => fx.reset(),
            Self::Distortion | Self::Passthrough | Self::Stereo | Self::Autopan => {}
            Self::Phaser(fx) => fx.reset(),
            Self::Flanger(fx) => fx.reset(),
            Self::Bitcrusher(fx) => fx.reset(),
            Self::Gate(fx) => fx.reset(),
            Self::Chorus(fx) => fx.reset(),
            Self::Limiter(fx) => *fx = Limiter::default(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_params::EFFECT_KINDS;

    #[test]
    fn every_effect_is_finite_with_default_params() {
        let ctx = ProcessContext {
            beats: 0.0,
            sample_rate: 48_000.0,
            tempo_bpm: 130.0,
        };
        for kind in EFFECT_KINDS {
            let mut fx = Effect::new(kind, ctx.sample_rate);
            assert_eq!(fx.kind(), kind);
            let mut l: Vec<f32> = (0..4_800).map(|i| ((i as f32) * 0.05).sin()).collect();
            let mut r = l.clone();
            fx.process(&mut l, &mut r, &kind.defaults(), ctx);
            assert!(
                l.iter().chain(&r).all(|s| s.is_finite() && s.abs() < 8.0),
                "{kind} misbehaved"
            );
            fx.reset();
        }
    }

    #[test]
    fn flat_eq_is_transparent() {
        let ctx = ProcessContext {
            beats: 0.0,
            sample_rate: 44_100.0,
            tempo_bpm: 120.0,
        };
        let mut fx = Effect::new(EffectKind::Eq, ctx.sample_rate);
        let mut l = vec![0.25; 256];
        let mut r = vec![0.25; 256];
        fx.process(&mut l, &mut r, &EffectKind::Eq.defaults(), ctx);
        assert!((l[255] - 0.25).abs() < 1e-3);
    }
}
