//! Parameter tables shared by the project model, the engine and the UI.
//!
//! The project stores parameters by name; the engine reads them from fixed-size
//! arrays by index. The tables here are the single contract between the two.

use std::fmt;

pub mod presets;

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
pub const MAX_INSTRUMENT_PARAMS: usize = 48;
/// Maximum number of effect slots on one mixer track.
pub const MAX_EFFECT_SLOTS: usize = 10;
/// Maximum number of mixer tracks, master included.
pub const MAX_MIXER_TRACKS: usize = 128;

/// Sounds synthesized at start-up, addressed as `builtin:<name>` sample names.
pub const BUILTIN_SAMPLES: [&str; 8] = [
    "kick", "snare", "clap", "hat", "open_hat", "tom", "rim", "crash",
];
/// Prefix of built-in sample names.
pub const BUILTIN_PREFIX: &str = "builtin:";

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

    /// Maps a value to a 0..1 control position. Frequencies use a log scale and
    /// times a cubic one, so useful values get most of a knob's travel.
    #[must_use]
    pub fn to_unit(&self, value: f32) -> f32 {
        let value = value.clamp(self.min, self.max);
        match self.unit {
            "Hz" if self.min > 0.0 => (value / self.min).ln() / (self.max / self.min).ln(),
            "s" | "ms" => ((value - self.min) / (self.max - self.min)).cbrt(),
            _ => (value - self.min) / (self.max - self.min),
        }
    }

    /// Inverse of [`ParamSpec::to_unit`], sanitized into range.
    #[must_use]
    pub fn from_unit(&self, t: f32) -> f32 {
        let t = if t.is_finite() {
            t.clamp(0.0, 1.0)
        } else {
            0.0
        };
        let value = match self.unit {
            "Hz" if self.min > 0.0 => self.min * (self.max / self.min).powf(t),
            "s" | "ms" => self.min + t.powi(3) * (self.max - self.min),
            _ => self.min + t * (self.max - self.min),
        };
        self.sanitize(value)
    }

    /// Formats a value with its unit for display.
    #[must_use]
    pub fn format(&self, value: f32) -> String {
        match self.unit {
            "Hz" if value >= 1_000.0 => format!("{:.2} kHz", value / 1_000.0),
            "Hz" => format!("{value:.0} Hz"),
            "s" if value < 1.0 => format!("{:.0} ms", value * 1_000.0),
            "s" => format!("{value:.2} s"),
            "" if self.integer => format!("{value:.0}"),
            "" => format!("{:.0}%", value * 100.0),
            unit => format!("{value:.1} {unit}"),
        }
    }
}

/// Fader position (0..1) of a linear gain: silence at the bottom, -24 dB at a
/// quarter, 0 dB at three quarters and `max` at the top.
#[must_use]
pub fn volume_to_unit(gain: f32, max: f32) -> f32 {
    let max_db = 20.0 * max.max(1e-6).log10();
    let low = 10.0_f32.powf(-24.0 / 20.0);
    if gain <= 0.0 || !gain.is_finite() {
        0.0
    } else if gain < low {
        0.25 * gain / low
    } else {
        let db = 20.0 * gain.log10();
        if db <= 0.0 {
            0.75 + db / 48.0
        } else {
            (0.75 + 0.25 * db / max_db).min(1.0)
        }
    }
}

/// Inverse of [`volume_to_unit`].
#[must_use]
pub fn volume_from_unit(position: f32, max: f32) -> f32 {
    let p = if position.is_finite() {
        position.clamp(0.0, 1.0)
    } else {
        0.0
    };
    let max_db = 20.0 * max.max(1e-6).log10();
    let low = 10.0_f32.powf(-24.0 / 20.0);
    if p < 0.25 {
        p / 0.25 * low
    } else if p <= 0.75 {
        10.0_f32.powf((p - 0.75) * 48.0 / 20.0)
    } else {
        10.0_f32.powf((p - 0.75) / 0.25 * max_db / 20.0)
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
    /// Synth portamento: time to slide from a held note to a new one, seconds.
    pub const GLIDE: usize = 14;
    pub const UNISON: usize = 15;
    pub const SPREAD: usize = 16;
    pub const SUB: usize = 17;
    pub const LFO_RATE: usize = 18;
    pub const LFO_DEPTH: usize = 19;
    pub const LFO_TARGET: usize = 20;
    pub const FILTER_TYPE: usize = 21;
    pub const DRIVE: usize = 22;
    pub const ARP_MODE: usize = 23;
    pub const ARP_RATE: usize = 24;
    pub const ARP_RANGE: usize = 25;
    pub const ARP_GATE: usize = 26;
    pub const DRUM_TYPE: usize = 27;
    pub const DRUM_TUNE: usize = 28;
    pub const DRUM_DECAY: usize = 29;
    pub const DRUM_SWEEP: usize = 30;
    pub const DRUM_TONE: usize = 31;
    pub const DRUM_CLICK: usize = 32;
    pub const PLUCK_DECAY: usize = 33;
    pub const PLUCK_DAMPING: usize = 34;
    pub const PLUCK_TONE: usize = 35;
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
    spec("glide", 0.0, 2.0, 0.0, "s"),
    int_spec("unison", 1.0, 7.0, 1.0),
    spec("spread", 0.0, 100.0, 15.0, "ct"),
    spec("sub", 0.0, 1.0, 0.0, ""),
    spec("lfo_rate", 0.05, 20.0, 5.0, "Hz"),
    spec("lfo_depth", 0.0, 1.0, 0.0, ""),
    int_spec("lfo_target", 0.0, 2.0, 0.0),
    int_spec("filter_type", 0.0, 2.0, 0.0),
    spec("drive", 0.0, 1.0, 0.0, ""),
    int_spec("arp_mode", 0.0, 4.0, 0.0),
    int_spec("arp_rate", 0.0, 4.0, 1.0),
    int_spec("arp_range", 1.0, 4.0, 1.0),
    spec("arp_gate", 0.05, 1.0, 0.8, ""),
    int_spec("drum_type", 0.0, 4.0, 0.0),
    spec("drum_tune", -24.0, 24.0, 0.0, "st"),
    spec("drum_decay", 0.02, 2.0, 0.4, "s"),
    spec("drum_sweep", 0.0, 48.0, 24.0, "st"),
    spec("drum_tone", 0.0, 1.0, 0.5, ""),
    spec("drum_click", 0.0, 1.0, 0.3, ""),
    spec("pluck_decay", 0.1, 10.0, 2.0, "s"),
    spec("pluck_damping", 0.0, 1.0, 0.3, ""),
    spec("pluck_tone", 0.0, 1.0, 0.7, ""),
];

/// Names of the drum synth's sounds, by `drum_type` value.
pub const DRUM_TYPES: [&str; 5] = ["kick", "snare", "hat", "tom", "clap"];

/// Names of the arpeggiator modes, by `arp_mode` value.
pub const ARP_MODES: [&str; 5] = ["off", "up", "down", "up-down", "random"];
/// Arpeggiator rates by `arp_rate` value: (label, ticks).
pub const ARP_RATES: [(&str, u32); 5] = [
    ("1/32", TICKS_PER_STEP / 2),
    ("1/16", TICKS_PER_STEP),
    ("1/8", TICKS_PER_STEP * 2),
    ("1/4", TICKS_PER_STEP * 4),
    ("1/2", TICKS_PER_STEP * 8),
];

/// Names of the LFO targets, by `lfo_target` value.
pub const LFO_TARGETS: [&str; 3] = ["pitch", "cutoff", "volume"];
/// Names of the synth filter types, by `filter_type` value.
pub const FILTER_TYPES: [&str; 3] = ["low-pass", "band-pass", "high-pass"];

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
    /// Stereo chorus (modulated delay).
    Chorus,
    /// Peak limiter.
    Limiter,
    /// Swept all-pass phaser.
    Phaser,
    /// Short modulated delay with feedback.
    Flanger,
    /// Bit depth and sample-rate reduction.
    Bitcrusher,
    /// Width, balance and phase.
    Stereo,
    /// Tempo-synced 16-step volume gate.
    Gate,
    /// Tempo-synced auto-pan.
    Autopan,
    /// A hosted plugin; its parameters belong to the plugin, so its table is
    /// empty. Not in [`EFFECT_KINDS`] and not accepted by [`EffectKind::parse`].
    Plugin,
}

/// Every effect kind, in menu order.
pub const EFFECT_KINDS: [EffectKind; 14] = [
    EffectKind::Filter,
    EffectKind::Eq,
    EffectKind::Delay,
    EffectKind::Reverb,
    EffectKind::Chorus,
    EffectKind::Phaser,
    EffectKind::Flanger,
    EffectKind::Compressor,
    EffectKind::Limiter,
    EffectKind::Gate,
    EffectKind::Autopan,
    EffectKind::Stereo,
    EffectKind::Distortion,
    EffectKind::Bitcrusher,
];

const PHASER_PARAMS: &[ParamSpec] = &[
    spec("rate", 0.05, 10.0, 0.5, "Hz"),
    spec("depth", 0.0, 1.0, 0.7, ""),
    spec("feedback", 0.0, 0.9, 0.5, ""),
    int_spec("stages", 2.0, 12.0, 6.0),
    spec("mix", 0.0, 1.0, 0.5, ""),
];

const FLANGER_PARAMS: &[ParamSpec] = &[
    spec("rate", 0.05, 5.0, 0.25, "Hz"),
    spec("depth", 0.1, 5.0, 2.0, "ms"),
    spec("delay", 0.5, 10.0, 2.0, "ms"),
    spec("feedback", -0.95, 0.95, 0.5, ""),
    spec("mix", 0.0, 1.0, 0.5, ""),
];

const BITCRUSHER_PARAMS: &[ParamSpec] = &[
    int_spec("bits", 1.0, 16.0, 8.0),
    int_spec("downsample", 1.0, 32.0, 4.0),
    spec("mix", 0.0, 1.0, 1.0, ""),
    spec("output", -24.0, 6.0, 0.0, "dB"),
];

const STEREO_PARAMS: &[ParamSpec] = &[
    spec("width", 0.0, 2.0, 1.0, ""),
    spec("balance", -1.0, 1.0, 0.0, ""),
    int_spec("invert_left", 0.0, 1.0, 0.0),
    int_spec("invert_right", 0.0, 1.0, 0.0),
];

const GATE_PARAMS: &[ParamSpec] = &[
    int_spec("rate", 0.0, 5.0, 1.0),
    spec("duty", 0.05, 1.0, 0.5, ""),
    spec("depth", 0.0, 1.0, 1.0, ""),
    spec("smooth", 0.0, 20.0, 3.0, "ms"),
    int_spec("steps", 0.0, 65_535.0, 65_535.0),
];

const AUTOPAN_PARAMS: &[ParamSpec] = &[
    int_spec("rate", 0.0, 5.0, 3.0),
    spec("depth", 0.0, 1.0, 0.7, ""),
    int_spec("shape", 0.0, 2.0, 0.0),
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
    int_spec("sidechain", 0.0, 127.0, 0.0),
];

const CHORUS_PARAMS: &[ParamSpec] = &[
    spec("rate", 0.05, 5.0, 0.8, "Hz"),
    spec("depth", 0.0, 10.0, 3.0, "ms"),
    spec("delay", 5.0, 30.0, 12.0, "ms"),
    spec("mix", 0.0, 1.0, 0.4, ""),
];

const LIMITER_PARAMS: &[ParamSpec] = &[
    spec("input", 0.0, 24.0, 0.0, "dB"),
    spec("ceiling", -12.0, 0.0, -0.3, "dB"),
    spec("release", 10.0, 1_000.0, 80.0, "ms"),
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
            Self::Chorus => "chorus",
            Self::Limiter => "limiter",
            Self::Phaser => "phaser",
            Self::Flanger => "flanger",
            Self::Bitcrusher => "bitcrusher",
            Self::Stereo => "stereo",
            Self::Gate => "gate",
            Self::Autopan => "autopan",
            Self::Plugin => "plugin",
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
            Self::Chorus => CHORUS_PARAMS,
            Self::Limiter => LIMITER_PARAMS,
            Self::Phaser => PHASER_PARAMS,
            Self::Flanger => FLANGER_PARAMS,
            Self::Bitcrusher => BITCRUSHER_PARAMS,
            Self::Stereo => STEREO_PARAMS,
            Self::Gate => GATE_PARAMS,
            Self::Autopan => AUTOPAN_PARAMS,
            Self::Plugin => &[],
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
        assert_eq!(EffectKind::parse("vocoder"), None);
        assert_eq!(EffectKind::parse("plugin"), None);
    }

    #[test]
    fn unit_mappings_round_trip() {
        let cutoff = INSTRUMENT_PARAMS[inst::CUTOFF];
        for value in [20.0, 632.5, 1_000.0, 20_000.0] {
            assert!((cutoff.from_unit(cutoff.to_unit(value)) - value).abs() / value < 1e-3);
        }
        assert!((cutoff.to_unit(632.5) - 0.5).abs() < 0.01);
        assert_eq!(cutoff.format(2_500.0), "2.50 kHz");
        for gain in [0.0, 0.01, 0.5, 1.0, 1.25] {
            let back = volume_from_unit(volume_to_unit(gain, 1.25), 1.25);
            assert!((back - gain).abs() < 1e-3, "{gain} -> {back}");
        }
        assert!((volume_to_unit(1.0, 1.25) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn sanitize_clamps_and_rounds() {
        let wave = INSTRUMENT_PARAMS[inst::WAVE1];
        assert!((wave.sanitize(2.6) - 3.0).abs() < f32::EPSILON);
        assert!((wave.sanitize(99.0) - 4.0).abs() < f32::EPSILON);
        assert_eq!(index_of(INSTRUMENT_PARAMS, "release"), Some(inst::RELEASE));
    }
}
