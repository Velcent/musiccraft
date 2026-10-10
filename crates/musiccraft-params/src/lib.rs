//! Parameter tables shared by the project model, the engine and the UI.
//!
//! The project stores parameters by name; the engine reads them from fixed-size
//! arrays by index. The tables here are the single contract between the two.

use std::fmt;

use serde::{Deserialize, Serialize};

/// Ticks per quarter note (one beat). A step (1/16 note) is [`TICKS_PER_STEP`].
pub const PPQ: u32 = 96;
/// Ticks per sixteenth-note step.
pub const TICKS_PER_STEP: u32 = PPQ / 4;
/// Beats per bar. MusicCraft uses 4/4 time throughout for now.
pub const BEATS_PER_BAR: u32 = 4;
/// Ticks per bar.
pub const TICKS_PER_BAR: u32 = PPQ * BEATS_PER_BAR;

/// Capacity of an effect parameter array.
pub const MAX_EFFECT_PARAMS: usize = 8;
/// Capacity of an instrument parameter array.
pub const MAX_INSTRUMENT_PARAMS: usize = 16;
/// Maximum number of effect slots on one mixer track.
pub const MAX_EFFECT_SLOTS: usize = 8;
/// Maximum number of mixer tracks, master included.
pub const MAX_MIXER_TRACKS: usize = 128;

/// Description of one numeric parameter.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ParamSpec {
    /// Stable identifier used by commands and the project file.
    pub name: &'static str,
    /// Lowest accepted value.
    pub min: f32,
    /// Highest accepted value.
    pub max: f32,
    /// Value for a newly created instrument or effect.
    pub default: f32,
    /// Display unit (may be empty).
    pub unit: &'static str,
    /// Whether only whole numbers are meaningful.
    pub integer: bool,
}

const fn spec(
    name: &'static str,
    min: f32,
    max: f32,
    default: f32,
    unit: &'static str,
) -> ParamSpec {
    ParamSpec {
        name,
        min,
        max,
        default,
        unit,
        integer: false,
    }
}

const fn int_spec(name: &'static str, min: f32, max: f32, default: f32) -> ParamSpec {
    ParamSpec {
        name,
        min,
        max,
        default,
        unit: "",
        integer: true,
    }
}

impl ParamSpec {
    /// Clamps and, for integer parameters, rounds a value into range.
    #[must_use]
    pub fn sanitize(&self, value: f32) -> f32 {
        let value = value.clamp(self.min, self.max);
        if self.integer { value.round() } else { value }
    }

    /// Whether `value` is finite and inside the range.
    #[must_use]
    pub fn accepts(&self, value: f32) -> bool {
        value.is_finite() && value >= self.min && value <= self.max
    }
}

/// Looks up a parameter's index by name.
#[must_use]
pub fn index_of(specs: &[ParamSpec], name: &str) -> Option<usize> {
    specs.iter().position(|spec| spec.name == name)
}

/// Default values of a table, padded with zeros to the array size `N`.
#[must_use]
pub fn defaults<const N: usize>(specs: &[ParamSpec]) -> [f32; N] {
    let mut values = [0.0; N];
    for (value, spec) in values.iter_mut().zip(specs) {
        *value = spec.default;
    }
    values
}

/// Oscillator wave shapes, in the order used by the `wave1`/`wave2` parameters.
pub const WAVEFORMS: [&str; 5] = ["sine", "saw", "square", "triangle", "noise"];

/// Indices into the instrument parameter array.
pub mod inst {
    /// Envelope attack time, seconds.
    pub const ATTACK: usize = 0;
    /// Envelope decay time, seconds.
    pub const DECAY: usize = 1;
    /// Envelope sustain level, 0..1.
    pub const SUSTAIN: usize = 2;
    /// Envelope release time, seconds.
    pub const RELEASE: usize = 3;
    /// Synth oscillator 1 waveform index.
    pub const WAVE1: usize = 4;
    /// Synth oscillator 2 waveform index.
    pub const WAVE2: usize = 5;
    /// Oscillator 2 detune, cents.
    pub const DETUNE: usize = 6;
    /// Oscillator mix: 0 = only osc 1, 1 = only osc 2.
    pub const OSC_MIX: usize = 7;
    /// Low-pass cutoff, hertz.
    pub const CUTOFF: usize = 8;
    /// Filter resonance, 0..1.
    pub const RESONANCE: usize = 9;
    /// Envelope to cutoff modulation, octaves.
    pub const ENV_AMOUNT: usize = 10;
    /// Sampler: 1 = play the whole sample regardless of note length.
    pub const ONE_SHOT: usize = 11;
    /// Sampler: key at which the sample plays at its recorded pitch.
    pub const ROOT_KEY: usize = 12;
    /// Coarse pitch offset applied to every note, semitones.
    pub const PITCH: usize = 13;
}

/// Parameters shared by every instrument (sampler and synth), in array order.
pub const INSTRUMENT_PARAMS: &[ParamSpec] = &[
    spec("attack", 0.0, 5.0, 0.002, "s"),
    spec("decay", 0.0, 5.0, 0.3, "s"),
    spec("sustain", 0.0, 1.0, 0.8, ""),
    spec("release", 0.0, 5.0, 0.12, "s"),
    int_spec("wave1", 0.0, 4.0, 1.0),
    int_spec("wave2", 0.0, 4.0, 2.0),
    spec("detune", -1200.0, 1200.0, 7.0, "ct"),
    spec("osc_mix", 0.0, 1.0, 0.35, ""),
    spec("cutoff", 20.0, 20_000.0, 6_000.0, "Hz"),
    spec("resonance", 0.0, 1.0, 0.2, ""),
    spec("env_amount", -8.0, 8.0, 0.0, "oct"),
    int_spec("one_shot", 0.0, 1.0, 1.0),
    int_spec("root_key", 0.0, 127.0, 60.0),
    spec("pitch", -48.0, 48.0, 0.0, "st"),
];

/// Built-in insert effects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EffectKind {
    /// State-variable filter (low-pass, band-pass, high-pass).
    Filter,
    /// Three-band equalizer.
    Eq,
    /// Tempo-synced stereo delay.
    Delay,
    /// Algorithmic reverb.
    Reverb,
    /// Feed-forward compressor.
    Compressor,
    /// Soft-clipping distortion.
    Distortion,
}

/// Every effect kind, in menu order.
pub const EFFECT_KINDS: [EffectKind; 6] = [
    EffectKind::Filter,
    EffectKind::Eq,
    EffectKind::Delay,
    EffectKind::Reverb,
    EffectKind::Compressor,
    EffectKind::Distortion,
];

const FILTER_PARAMS: &[ParamSpec] = &[
    int_spec("mode", 0.0, 2.0, 0.0),
    spec("cutoff", 20.0, 20_000.0, 2_000.0, "Hz"),
    spec("resonance", 0.0, 1.0, 0.3, ""),
    spec("mix", 0.0, 1.0, 1.0, ""),
];

const EQ_PARAMS: &[ParamSpec] = &[
    spec("low_gain", -18.0, 18.0, 0.0, "dB"),
    spec("low_freq", 20.0, 1_000.0, 120.0, "Hz"),
    spec("mid_gain", -18.0, 18.0, 0.0, "dB"),
    spec("mid_freq", 100.0, 10_000.0, 1_000.0, "Hz"),
    spec("mid_q", 0.1, 10.0, 0.7, ""),
    spec("high_gain", -18.0, 18.0, 0.0, "dB"),
    spec("high_freq", 1_000.0, 20_000.0, 8_000.0, "Hz"),
];

const DELAY_PARAMS: &[ParamSpec] = &[
    spec("time", 0.0625, 4.0, 0.75, "beats"),
    spec("feedback", 0.0, 0.95, 0.4, ""),
    spec("mix", 0.0, 1.0, 0.3, ""),
    int_spec("ping_pong", 0.0, 1.0, 1.0),
    spec("damping", 0.0, 1.0, 0.3, ""),
];

const REVERB_PARAMS: &[ParamSpec] = &[
    spec("size", 0.0, 1.0, 0.6, ""),
    spec("damping", 0.0, 1.0, 0.5, ""),
    spec("width", 0.0, 1.0, 1.0, ""),
    spec("mix", 0.0, 1.0, 0.25, ""),
];

const COMPRESSOR_PARAMS: &[ParamSpec] = &[
    spec("threshold", -60.0, 0.0, -18.0, "dB"),
    spec("ratio", 1.0, 20.0, 4.0, ":1"),
    spec("attack", 0.1, 200.0, 10.0, "ms"),
    spec("release", 5.0, 2_000.0, 120.0, "ms"),
    spec("makeup", 0.0, 24.0, 0.0, "dB"),
];

const DISTORTION_PARAMS: &[ParamSpec] = &[
    spec("drive", 0.0, 48.0, 12.0, "dB"),
    spec("mix", 0.0, 1.0, 1.0, ""),
    spec("output", -24.0, 6.0, -6.0, "dB"),
];

impl EffectKind {
    /// Stable identifier used in commands and project files.
    #[must_use]
    pub fn name(self) -> &'static str {
        match self {
            Self::Filter => "filter",
            Self::Eq => "eq",
            Self::Delay => "delay",
            Self::Reverb => "reverb",
            Self::Compressor => "compressor",
            Self::Distortion => "distortion",
        }
    }

    /// Parses an identifier produced by [`EffectKind::name`].
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        EFFECT_KINDS.into_iter().find(|kind| kind.name() == name)
    }

    /// The parameter table for this effect.
    #[must_use]
    pub fn params(self) -> &'static [ParamSpec] {
        match self {
            Self::Filter => FILTER_PARAMS,
            Self::Eq => EQ_PARAMS,
            Self::Delay => DELAY_PARAMS,
            Self::Reverb => REVERB_PARAMS,
            Self::Compressor => COMPRESSOR_PARAMS,
            Self::Distortion => DISTORTION_PARAMS,
        }
    }

    /// Default parameter values for this effect.
    #[must_use]
    pub fn defaults(self) -> [f32; MAX_EFFECT_PARAMS] {
        defaults(self.params())
    }
}

impl fmt::Display for EffectKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.name())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_fit_their_arrays_and_defaults_are_in_range() {
        assert!(INSTRUMENT_PARAMS.len() <= MAX_INSTRUMENT_PARAMS);
        let mut tables = vec![INSTRUMENT_PARAMS];
        tables.extend(EFFECT_KINDS.iter().map(|kind| kind.params()));
        for table in tables {
            assert!(table.len() <= MAX_EFFECT_PARAMS.max(MAX_INSTRUMENT_PARAMS));
            for spec in table {
                assert!(
                    spec.accepts(spec.default),
                    "{} default out of range",
                    spec.name
                );
                assert!(spec.min < spec.max, "{} has an empty range", spec.name);
            }
        }
        for kind in EFFECT_KINDS {
            assert!(kind.params().len() <= MAX_EFFECT_PARAMS);
        }
    }

    #[test]
    fn instrument_indices_match_table() {
        assert_eq!(INSTRUMENT_PARAMS[inst::ATTACK].name, "attack");
        assert_eq!(INSTRUMENT_PARAMS[inst::CUTOFF].name, "cutoff");
        assert_eq!(INSTRUMENT_PARAMS[inst::ROOT_KEY].name, "root_key");
        assert_eq!(INSTRUMENT_PARAMS[inst::PITCH].name, "pitch");
    }

    #[test]
    fn effect_names_round_trip() {
        for kind in EFFECT_KINDS {
            assert_eq!(EffectKind::parse(kind.name()), Some(kind));
        }
        assert_eq!(EffectKind::parse("phaser"), None);
    }

    #[test]
    fn sanitize_clamps_and_rounds() {
        let wave = INSTRUMENT_PARAMS[inst::WAVE1];
        assert!((wave.sanitize(2.6) - 3.0).abs() < f32::EPSILON);
        assert!((wave.sanitize(99.0) - 4.0).abs() < f32::EPSILON);
        assert_eq!(index_of(INSTRUMENT_PARAMS, "release"), Some(inst::RELEASE));
    }
}
