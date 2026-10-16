//! Factory presets for the built-in synth and effects. A preset lists only
//! the values that differ from the parameter defaults; applying it sets every
//! parameter of the table (defaults for the rest), except those listed in
//! [`KEEP`].

use crate::{EffectKind, INSTRUMENT_PARAMS, ParamSpec};

/// A named set of parameter values.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Preset {
    /// Display name.
    pub name: &'static str,
    /// Values that differ from the defaults, by parameter name.
    pub values: &'static [(&'static str, f32)],
}

/// Parameters presets never change (routing and sample settings).
pub const KEEP: [&str; 9] = [
    "slices",
    "sidechain",
    "one_shot",
    "root_key",
    "pitch",
    "arp_mode",
    "arp_rate",
    "arp_range",
    "arp_gate",
];

impl Preset {
    /// Every value of `specs` this preset sets, defaults filled in, without
    /// the parameters in [`KEEP`].
    #[must_use]
    pub fn resolve(&self, specs: &[ParamSpec]) -> Vec<(&'static str, f32)> {
        specs
            .iter()
            .filter(|spec| !KEEP.contains(&spec.name))
            .map(|spec| {
                let value = self
                    .values
                    .iter()
                    .find(|(name, _)| *name == spec.name)
                    .map_or(spec.default, |(_, v)| *v);
                (spec.name, spec.sanitize(value))
            })
            .collect()
    }

    /// Whether `values` (indexed like `specs`) are this preset's.
    #[must_use]
    pub fn matches(&self, specs: &[ParamSpec], values: &[f32]) -> bool {
        self.resolve(specs).iter().all(|(name, value)| {
            specs
                .iter()
                .position(|s| s.name == *name)
                .and_then(|i| values.get(i))
                .is_some_and(|v| (v - value).abs() <= 1e-3 * value.abs().max(1.0))
        })
    }
}

const fn preset(name: &'static str, values: &'static [(&'static str, f32)]) -> Preset {
    Preset { name, values }
}

/// Synth presets. Waves: 0 sine, 1 saw, 2 square, 3 triangle, 4 noise.
pub const SYNTH: &[Preset] = &[
    preset("Init", &[]),
    preset(
        "Analog bass",
        &[
            ("wave1", 1.0),
            ("wave2", 2.0),
            ("detune", -1200.0),
            ("osc_mix", 0.4),
            ("cutoff", 600.0),
            ("resonance", 0.35),
            ("env_amount", 2.5),
            ("decay", 0.25),
            ("sustain", 0.4),
            ("release", 0.08),
            ("sub", 0.5),
            ("drive", 0.3),
        ],
    ),
    preset(
        "Sub bass",
        &[
            ("wave1", 0.0),
            ("osc_mix", 0.0),
            ("sub", 0.4),
            ("cutoff", 2_000.0),
            ("attack", 0.005),
            ("decay", 0.5),
            ("sustain", 1.0),
            ("release", 0.1),
            ("glide", 0.05),
        ],
    ),
    preset(
        "Wobble bass",
        &[
            ("wave1", 1.0),
            ("wave2", 2.0),
            ("detune", -1200.0),
            ("osc_mix", 0.5),
            ("cutoff", 400.0),
            ("resonance", 0.3),
            ("lfo_target", 1.0),
            ("lfo_depth", 0.8),
            ("lfo_rate", 4.0),
            ("sub", 0.25),
            ("drive", 0.2),
            ("sustain", 0.7),
        ],
    ),
    preset(
        "Supersaw lead",
        &[
            ("wave1", 1.0),
            ("wave2", 1.0),
            ("detune", 12.0),
            ("osc_mix", 0.5),
            ("unison", 7.0),
            ("spread", 25.0),
            ("cutoff", 7_000.0),
            ("resonance", 0.1),
            ("attack", 0.01),
            ("decay", 0.4),
            ("sustain", 0.8),
            ("release", 0.25),
        ],
    ),
    preset(
        "Square lead",
        &[
            ("wave1", 2.0),
            ("wave2", 1.0),
            ("detune", 5.0),
            ("osc_mix", 0.3),
            ("cutoff", 4_000.0),
            ("resonance", 0.25),
            ("env_amount", 1.0),
            ("lfo_depth", 0.12),
            ("lfo_rate", 5.5),
            ("glide", 0.04),
            ("sustain", 0.9),
        ],
    ),
    preset(
        "Warm pad",
        &[
            ("wave1", 1.0),
            ("wave2", 1.0),
            ("detune", 9.0),
            ("osc_mix", 0.5),
            ("unison", 5.0),
            ("spread", 30.0),
            ("cutoff", 1_800.0),
            ("resonance", 0.15),
            ("attack", 0.8),
            ("decay", 1.5),
            ("sustain", 0.8),
            ("release", 1.8),
            ("lfo_target", 1.0),
            ("lfo_depth", 0.2),
            ("lfo_rate", 0.3),
        ],
    ),
    preset(
        "Pluck",
        &[
            ("wave1", 1.0),
            ("wave2", 2.0),
            ("detune", 0.0),
            ("osc_mix", 0.3),
            ("cutoff", 900.0),
            ("resonance", 0.3),
            ("env_amount", 4.0),
            ("attack", 0.001),
            ("decay", 0.25),
            ("sustain", 0.0),
            ("release", 0.2),
        ],
    ),
    preset(
        "Electric keys",
        &[
            ("wave1", 0.0),
            ("wave2", 3.0),
            ("detune", 1200.0),
            ("osc_mix", 0.25),
            ("cutoff", 5_000.0),
            ("decay", 1.2),
            ("sustain", 0.3),
            ("release", 0.4),
            ("lfo_target", 2.0),
            ("lfo_depth", 0.2),
            ("lfo_rate", 4.5),
        ],
    ),
    preset(
        "Brass",
        &[
            ("wave1", 1.0),
            ("wave2", 1.0),
            ("detune", 7.0),
            ("osc_mix", 0.5),
            ("unison", 3.0),
            ("spread", 12.0),
            ("cutoff", 1_200.0),
            ("env_amount", 2.5),
            ("resonance", 0.1),
            ("attack", 0.06),
            ("decay", 0.3),
            ("sustain", 0.7),
            ("release", 0.2),
        ],
    ),
    preset(
        "Acid bass",
        &[
            ("wave1", 1.0),
            ("osc_mix", 0.0),
            ("cutoff", 500.0),
            ("resonance", 0.6),
            ("env_amount", 3.5),
            ("decay", 0.2),
            ("sustain", 0.2),
            ("release", 0.05),
            ("glide", 0.08),
            ("drive", 0.35),
        ],
    ),
    preset(
        "Three-oscillator lead",
        &[
            ("wave1", 1.0),
            ("wave2", 2.0),
            ("detune", 7.0),
            ("osc_mix", 0.4),
            ("wave3", 1.0),
            ("osc3_level", 0.5),
            ("osc3_coarse", -12.0),
            ("cutoff", 5_000.0),
            ("sustain", 0.9),
        ],
    ),
    preset(
        "Noise riser",
        &[
            ("wave1", 4.0),
            ("osc_mix", 0.0),
            ("filter_type", 1.0),
            ("cutoff", 1_500.0),
            ("resonance", 0.3),
            ("attack", 2.0),
            ("sustain", 1.0),
            ("release", 1.0),
            ("lfo_target", 1.0),
            ("lfo_depth", 0.6),
            ("lfo_rate", 0.2),
        ],
    ),
];

/// Drum synth presets. Types: 0 kick, 1 snare, 2 hat, 3 tom, 4 clap.
pub const DRUM: &[Preset] = &[
    preset(
        "Punchy kick",
        &[
            ("drum_decay", 0.35),
            ("drum_sweep", 30.0),
            ("drum_click", 0.5),
        ],
    ),
    preset(
        "Deep kick",
        &[
            ("drum_decay", 0.9),
            ("drum_sweep", 18.0),
            ("drum_tune", -3.0),
            ("drum_click", 0.1),
        ],
    ),
    preset(
        "Tight kick",
        &[
            ("drum_decay", 0.18),
            ("drum_sweep", 36.0),
            ("drum_click", 0.7),
        ],
    ),
    preset(
        "Snare",
        &[("drum_type", 1.0), ("drum_decay", 0.25), ("drum_tone", 0.6)],
    ),
    preset(
        "Crack snare",
        &[
            ("drum_type", 1.0),
            ("drum_decay", 0.15),
            ("drum_tone", 0.8),
            ("drum_sweep", 12.0),
        ],
    ),
    preset(
        "Closed hat",
        &[("drum_type", 2.0), ("drum_decay", 0.06), ("drum_tone", 0.6)],
    ),
    preset("Open hat", &[("drum_type", 2.0), ("drum_decay", 0.35)]),
    preset(
        "Low tom",
        &[
            ("drum_type", 3.0),
            ("drum_decay", 0.5),
            ("drum_sweep", 10.0),
            ("drum_tune", -5.0),
        ],
    ),
    preset(
        "High tom",
        &[
            ("drum_type", 3.0),
            ("drum_decay", 0.35),
            ("drum_sweep", 10.0),
            ("drum_tune", 5.0),
        ],
    ),
    preset(
        "Clap",
        &[("drum_type", 4.0), ("drum_decay", 0.25), ("drum_tone", 0.4)],
    ),
];

/// Pluck presets.
pub const PLUCK: &[Preset] = &[
    preset(
        "Nylon guitar",
        &[
            ("pluck_decay", 2.5),
            ("pluck_damping", 0.45),
            ("pluck_tone", 0.55),
        ],
    ),
    preset(
        "Steel string",
        &[
            ("pluck_decay", 4.0),
            ("pluck_damping", 0.15),
            ("pluck_tone", 0.85),
        ],
    ),
    preset(
        "Harp",
        &[
            ("pluck_decay", 3.0),
            ("pluck_damping", 0.3),
            ("pluck_tone", 0.6),
        ],
    ),
    preset(
        "Pluck bass",
        &[
            ("pluck_decay", 1.5),
            ("pluck_damping", 0.6),
            ("pluck_tone", 0.4),
        ],
    ),
    preset(
        "Muted",
        &[
            ("pluck_decay", 0.3),
            ("pluck_damping", 0.8),
            ("pluck_tone", 0.5),
        ],
    ),
];

/// FM synth presets.
pub const FM: &[Preset] = &[
    preset(
        "E-piano",
        &[
            ("fm_algo", 2.0),
            ("op1_ratio", 1.0),
            ("op1_level", 1.0),
            ("op1_decay", 1.5),
            ("op1_sustain", 0.3),
            ("op2_ratio", 1.0),
            ("op2_level", 0.45),
            ("op2_decay", 0.6),
            ("op2_sustain", 0.1),
            ("op3_ratio", 1.0),
            ("op3_level", 0.6),
            ("op3_decay", 2.0),
            ("op3_sustain", 0.2),
            ("op4_ratio", 14.0),
            ("op4_level", 0.25),
            ("op4_decay", 0.1),
            ("op4_sustain", 0.0),
        ],
    ),
    preset(
        "Bell",
        &[
            ("fm_algo", 2.0),
            ("op1_ratio", 1.0),
            ("op1_level", 1.0),
            ("op1_decay", 3.0),
            ("op1_sustain", 0.0),
            ("op2_ratio", 3.5),
            ("op2_level", 0.6),
            ("op2_decay", 2.0),
            ("op2_sustain", 0.0),
            ("op3_ratio", 2.0),
            ("op3_level", 0.6),
            ("op3_decay", 2.5),
            ("op3_sustain", 0.0),
            ("op4_ratio", 5.0),
            ("op4_level", 0.4),
            ("op4_decay", 1.0),
            ("op4_sustain", 0.0),
            ("release", 1.5),
            ("sustain", 0.0),
            ("decay", 3.0),
        ],
    ),
    preset(
        "FM bass",
        &[
            ("fm_algo", 0.0),
            ("op1_ratio", 1.0),
            ("op1_level", 1.0),
            ("op1_decay", 0.6),
            ("op1_sustain", 0.6),
            ("op2_ratio", 1.0),
            ("op2_level", 0.7),
            ("op2_decay", 0.25),
            ("op2_sustain", 0.2),
            ("op3_ratio", 2.0),
            ("op3_level", 0.3),
            ("op3_decay", 0.15),
            ("op3_sustain", 0.0),
            ("op4_ratio", 1.0),
            ("op4_level", 0.0),
            ("op4_decay", 0.3),
            ("op4_sustain", 0.0),
            ("release", 0.08),
        ],
    ),
    preset(
        "Brass",
        &[
            ("fm_algo", 1.0),
            ("op1_ratio", 1.0),
            ("op1_level", 1.0),
            ("op1_decay", 1.0),
            ("op1_sustain", 0.8),
            ("op2_ratio", 1.0),
            ("op2_level", 0.6),
            ("op2_decay", 0.4),
            ("op2_sustain", 0.5),
            ("op3_ratio", 1.0),
            ("op3_level", 0.3),
            ("op3_decay", 0.5),
            ("op3_sustain", 0.3),
            ("op4_ratio", 1.0),
            ("op4_level", 0.0),
            ("op4_decay", 0.3),
            ("op4_sustain", 0.0),
            ("attack", 0.05),
        ],
    ),
    preset(
        "Organ",
        &[
            ("fm_algo", 5.0),
            ("op1_ratio", 1.0),
            ("op1_level", 1.0),
            ("op1_decay", 5.0),
            ("op1_sustain", 1.0),
            ("op2_ratio", 2.0),
            ("op2_level", 0.7),
            ("op2_decay", 5.0),
            ("op2_sustain", 1.0),
            ("op3_ratio", 3.0),
            ("op3_level", 0.5),
            ("op3_decay", 5.0),
            ("op3_sustain", 1.0),
            ("op4_ratio", 4.0),
            ("op4_level", 0.4),
            ("op4_decay", 5.0),
            ("op4_sustain", 1.0),
            ("sustain", 1.0),
            ("release", 0.05),
        ],
    ),
    preset(
        "FM pluck",
        &[
            ("fm_algo", 0.0),
            ("op1_ratio", 1.0),
            ("op1_level", 1.0),
            ("op1_decay", 0.4),
            ("op1_sustain", 0.0),
            ("op2_ratio", 2.0),
            ("op2_level", 0.5),
            ("op2_decay", 0.15),
            ("op2_sustain", 0.0),
            ("op3_ratio", 1.0),
            ("op3_level", 0.0),
            ("op3_decay", 0.4),
            ("op3_sustain", 0.0),
            ("op4_ratio", 1.0),
            ("op4_level", 0.0),
            ("op4_decay", 0.3),
            ("op4_sustain", 0.0),
            ("sustain", 0.0),
            ("decay", 0.4),
        ],
    ),
    preset(
        "Metallic",
        &[
            ("fm_algo", 3.0),
            ("op1_ratio", 1.0),
            ("op1_level", 1.0),
            ("op1_decay", 2.0),
            ("op1_sustain", 0.0),
            ("op2_ratio", 1.41),
            ("op2_level", 0.6),
            ("op2_decay", 2.0),
            ("op2_sustain", 0.0),
            ("op3_ratio", 2.8),
            ("op3_level", 0.5),
            ("op3_decay", 2.0),
            ("op3_sustain", 0.0),
            ("op4_ratio", 3.5),
            ("op4_level", 0.7),
            ("op4_decay", 1.0),
            ("op4_sustain", 0.2),
            ("fm_feedback", 0.4),
            ("sustain", 0.0),
            ("decay", 2.0),
        ],
    ),
];

/// Keys presets (types: 0 piano, 1 electric piano, 2 organ).
pub const KEYS: &[Preset] = &[
    preset(
        "Grand piano",
        &[
            ("keys_tone", 0.55),
            ("keys_decay", 4.0),
            ("keys_hammer", 0.35),
        ],
    ),
    preset(
        "Bright piano",
        &[
            ("keys_tone", 0.85),
            ("keys_decay", 3.0),
            ("keys_hammer", 0.45),
        ],
    ),
    preset(
        "Soft piano",
        &[
            ("keys_tone", 0.2),
            ("keys_decay", 5.0),
            ("keys_hammer", 0.1),
        ],
    ),
    preset(
        "Electric piano",
        &[
            ("keys_type", 1.0),
            ("keys_decay", 3.0),
            ("keys_tremolo", 0.25),
        ],
    ),
    preset(
        "Bell e-piano",
        &[("keys_type", 1.0), ("keys_tone", 0.8), ("keys_hammer", 0.8)],
    ),
    preset(
        "Rock organ",
        &[
            ("keys_type", 2.0),
            ("keys_tone", 0.9),
            ("keys_hammer", 0.6),
            ("keys_tremolo", 0.3),
            ("sustain", 1.0),
            ("release", 0.05),
        ],
    ),
    preset(
        "Jazz organ",
        &[
            ("keys_type", 2.0),
            ("keys_tone", 0.4),
            ("keys_hammer", 0.5),
            ("keys_tremolo", 0.15),
            ("sustain", 1.0),
            ("release", 0.05),
        ],
    ),
];

/// Additive presets.
pub const ADDITIVE: &[Preset] = &[
    preset(
        "Soft pad",
        &[
            ("add_bright", 0.3),
            ("attack", 0.6),
            ("release", 1.5),
            ("sustain", 1.0),
        ],
    ),
    preset("Bright saw", &[("add_bright", 1.0), ("add_partials", 32.0)]),
    preset("Hollow", &[("add_even", 0.0), ("add_bright", 0.6)]),
    preset(
        "Glass",
        &[
            ("add_inharm", 0.5),
            ("add_decay", 0.5),
            ("add_partials", 24.0),
            ("sustain", 0.4),
        ],
    ),
    preset(
        "Pipe",
        &[
            ("add_partials", 8.0),
            ("add_bright", 0.2),
            ("add_even", 0.5),
            ("sustain", 1.0),
        ],
    ),
];

const FILTER: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Low-pass sweep",
        &[("mode", 0.0), ("cutoff", 800.0), ("resonance", 0.5)],
    ),
    preset(
        "Telephone",
        &[("mode", 1.0), ("cutoff", 1_500.0), ("resonance", 0.6)],
    ),
    preset(
        "Thin out",
        &[("mode", 2.0), ("cutoff", 400.0), ("resonance", 0.2)],
    ),
];

const EQ: &[Preset] = &[
    preset("Flat", &[]),
    preset(
        "Warmth",
        &[("low_gain", 3.0), ("low_freq", 120.0), ("high_gain", -2.0)],
    ),
    preset(
        "Presence",
        &[
            ("mid_gain", 3.0),
            ("mid_freq", 3_000.0),
            ("mid_q", 1.0),
            ("high_gain", 2.0),
        ],
    ),
    preset("Low cut", &[("low_gain", -18.0), ("low_freq", 150.0)]),
    preset(
        "Smile",
        &[
            ("low_gain", 4.0),
            ("mid_gain", -3.0),
            ("mid_freq", 800.0),
            ("high_gain", 4.0),
        ],
    ),
];

const DELAY: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Dotted eighth",
        &[("time", 0.75), ("feedback", 0.4), ("mix", 0.3)],
    ),
    preset(
        "Slapback",
        &[
            ("time", 0.125),
            ("feedback", 0.1),
            ("mix", 0.3),
            ("ping_pong", 0.0),
        ],
    ),
    preset(
        "Quarter ping-pong",
        &[("time", 1.0), ("feedback", 0.5), ("mix", 0.35)],
    ),
    preset(
        "Dub echo",
        &[
            ("time", 0.75),
            ("feedback", 0.7),
            ("mix", 0.35),
            ("damping", 0.7),
        ],
    ),
    preset(
        "Tape echo",
        &[
            ("time", 0.75),
            ("feedback", 0.55),
            ("ping_pong", 0.0),
            ("damping", 0.5),
            ("mod_rate", 0.7),
            ("mod_depth", 1.5),
            ("low_cut", 150.0),
            ("saturation", 0.5),
        ],
    ),
    preset(
        "Diffused",
        &[
            ("feedback", 0.6),
            ("diffusion", 0.7),
            ("mix", 0.35),
            ("low_cut", 250.0),
        ],
    ),
    preset(
        "Wide slap",
        &[
            ("time", 0.25),
            ("feedback", 0.1),
            ("ping_pong", 0.0),
            ("offset", 15.0),
        ],
    ),
];

const REVERB: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Small room",
        &[
            ("size", 0.3),
            ("damping", 0.6),
            ("width", 0.8),
            ("mix", 0.18),
        ],
    ),
    preset("Hall", &[("size", 0.75), ("damping", 0.4), ("mix", 0.3)]),
    preset("Plate", &[("size", 0.6), ("damping", 0.2), ("mix", 0.25)]),
    preset(
        "Huge space",
        &[("size", 0.95), ("damping", 0.3), ("mix", 0.4)],
    ),
];

const COMPRESSOR: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Gentle glue",
        &[
            ("threshold", -12.0),
            ("ratio", 2.0),
            ("attack", 30.0),
            ("release", 200.0),
            ("makeup", 2.0),
        ],
    ),
    preset(
        "Punchy drums",
        &[
            ("threshold", -18.0),
            ("ratio", 4.0),
            ("attack", 10.0),
            ("release", 100.0),
            ("makeup", 4.0),
        ],
    ),
    preset(
        "Vocal",
        &[
            ("threshold", -20.0),
            ("ratio", 3.0),
            ("attack", 5.0),
            ("release", 150.0),
            ("makeup", 4.0),
        ],
    ),
    preset(
        "Squash",
        &[
            ("threshold", -30.0),
            ("ratio", 10.0),
            ("attack", 1.0),
            ("release", 60.0),
            ("makeup", 10.0),
        ],
    ),
];

const DISTORTION: &[Preset] = &[
    preset("Default", &[]),
    preset("Warm", &[("drive", 6.0), ("mix", 0.5), ("output", -2.0)]),
    preset("Crunch", &[("drive", 18.0), ("output", -6.0)]),
    preset("Fuzz", &[("drive", 36.0), ("output", -12.0)]),
    preset(
        "Hard clip",
        &[
            ("mode", 1.0),
            ("drive", 6.0),
            ("threshold", -6.0),
            ("output", -6.0),
        ],
    ),
    preset(
        "Square-ish",
        &[
            ("mode", 1.0),
            ("drive", 24.0),
            ("threshold", -18.0),
            ("output", -12.0),
        ],
    ),
];

const CHORUS: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Subtle",
        &[("rate", 0.4), ("depth", 2.0), ("delay", 12.0), ("mix", 0.3)],
    ),
    preset(
        "Wide",
        &[("rate", 0.8), ("depth", 4.0), ("delay", 15.0), ("mix", 0.5)],
    ),
    preset(
        "Vibrato",
        &[("rate", 5.0), ("depth", 3.0), ("delay", 7.0), ("mix", 1.0)],
    ),
    preset(
        "Vintage ensemble",
        &[
            ("rate", 0.6),
            ("depth", 3.5),
            ("delay", 14.0),
            ("mix", 0.5),
            ("voices", 3.0),
            ("tone", 6_000.0),
        ],
    ),
    preset(
        "Dark duo",
        &[("voices", 2.0), ("tone", 3_000.0), ("mix", 0.45)],
    ),
];

const LIMITER: &[Preset] = &[
    preset("Safety", &[]),
    preset("Loud master", &[("input", 6.0), ("release", 60.0)]),
    preset(
        "Brickwall",
        &[("input", 3.0), ("ceiling", -1.0), ("release", 30.0)],
    ),
];

const PHASER: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Slow swirl",
        &[
            ("rate", 0.2),
            ("depth", 0.9),
            ("feedback", 0.6),
            ("stages", 8.0),
        ],
    ),
    preset(
        "Fast wobble",
        &[
            ("rate", 4.0),
            ("depth", 0.5),
            ("feedback", 0.3),
            ("stages", 4.0),
        ],
    ),
    preset(
        "Deep",
        &[
            ("rate", 0.4),
            ("depth", 1.0),
            ("feedback", 0.85),
            ("stages", 12.0),
            ("mix", 0.6),
        ],
    ),
    preset(
        "Vintage",
        &[
            ("rate", 0.3),
            ("depth", 0.8),
            ("feedback", 0.7),
            ("stages", 4.0),
            ("warm", 0.8),
        ],
    ),
];

const FLANGER: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Jet",
        &[
            ("rate", 0.1),
            ("depth", 4.0),
            ("delay", 1.0),
            ("feedback", 0.85),
        ],
    ),
    preset(
        "Metallic",
        &[
            ("rate", 0.5),
            ("depth", 1.0),
            ("delay", 3.0),
            ("feedback", -0.8),
        ],
    ),
    preset(
        "Gentle",
        &[
            ("rate", 0.2),
            ("depth", 1.5),
            ("feedback", 0.2),
            ("mix", 0.35),
        ],
    ),
];

const BITCRUSHER: &[Preset] = &[
    preset("Default", &[]),
    preset("Retro console", &[("bits", 6.0), ("downsample", 6.0)]),
    preset(
        "Light grit",
        &[("bits", 12.0), ("downsample", 2.0), ("mix", 0.5)],
    ),
    preset(
        "Destroyed",
        &[("bits", 3.0), ("downsample", 16.0), ("output", -6.0)],
    ),
    preset(
        "Squeezed",
        &[
            ("bits", 10.0),
            ("downsample", 1.0),
            ("squeeze", 0.6),
            ("output", -6.0),
        ],
    ),
];

const STEREO: &[Preset] = &[
    preset("Default", &[]),
    preset("Mono", &[("width", 0.0)]),
    preset("Wide", &[("width", 1.6)]),
    preset("Extra wide", &[("width", 2.0)]),
];

const GATE: &[Preset] = &[
    preset("All steps", &[]),
    preset("Eighths", &[("rate", 2.0), ("duty", 0.6)]),
    preset("Offbeat chop", &[("steps", 43_690.0), ("duty", 0.9)]),
    preset(
        "Trance stutter",
        &[("steps", 56_797.0), ("duty", 0.7), ("smooth", 2.0)],
    ),
    preset("Half depth", &[("depth", 0.5), ("duty", 0.4)]),
];

const AUTOPAN: &[Preset] = &[
    preset("Default", &[]),
    preset("Slow sweep", &[("rate", 5.0), ("depth", 0.8)]),
    preset(
        "Eighth bounce",
        &[("rate", 2.0), ("depth", 0.6), ("shape", 1.0)],
    ),
    preset(
        "Hard switch",
        &[("rate", 3.0), ("depth", 1.0), ("shape", 2.0)],
    ),
];

const BASS_BOOST: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Sub weight",
        &[("freq", 50.0), ("boost", 9.0), ("drive", 0.1)],
    ),
    preset("Warm", &[("freq", 120.0), ("boost", 4.0), ("drive", 0.4)]),
    preset("Pumped", &[("freq", 70.0), ("boost", 12.0), ("drive", 0.7)]),
];

const OVERDRIVE: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Crunch",
        &[("drive", 24.0), ("tone", 4_000.0), ("bias", 0.2)],
    ),
    preset(
        "Warm tube",
        &[
            ("drive", 10.0),
            ("tone", 2_000.0),
            ("bias", 0.3),
            ("output", -3.0),
        ],
    ),
    preset(
        "Fuzz",
        &[
            ("drive", 40.0),
            ("tone", 6_000.0),
            ("bias", 0.4),
            ("output", -12.0),
        ],
    ),
];

const SOFT_CLIPPER: &[Preset] = &[
    preset("Default", &[]),
    preset("Gentle", &[("threshold", -1.0), ("knee", 0.8)]),
    preset(
        "Hard",
        &[("threshold", -6.0), ("knee", 0.1), ("output", 3.0)],
    ),
];

const WAVESHAPER: &[Preset] = &[
    preset("Tanh", &[]),
    preset(
        "Hard clip",
        &[("shape", 1.0), ("drive", 18.0), ("output", -9.0)],
    ),
    preset(
        "Wavefolder",
        &[("shape", 2.0), ("drive", 18.0), ("output", -6.0)],
    ),
    preset("Sine fold", &[("shape", 3.0), ("drive", 12.0)]),
    preset("Asymmetric", &[("shape", 4.0), ("drive", 15.0)]),
    preset("Rectifier", &[("shape", 5.0), ("drive", 6.0), ("mix", 0.5)]),
    preset(
        "Drawn: soft knee",
        &[
            ("shape", 6.0),
            ("drive", 6.0),
            ("point_1", -0.9),
            ("point_2", -0.85),
            ("point_3", -0.7),
            ("point_4", -0.4),
            ("point_6", 0.4),
            ("point_7", 0.7),
            ("point_8", 0.85),
            ("point_9", 0.9),
        ],
    ),
    preset(
        "Drawn: stairs",
        &[
            ("shape", 6.0),
            ("drive", 3.0),
            ("point_2", -1.0),
            ("point_3", -0.5),
            ("point_4", -0.5),
            ("point_6", 0.5),
            ("point_7", 0.5),
            ("point_8", 1.0),
        ],
    ),
];

const DC_BLOCK: &[Preset] = &[preset("Default", &[]), preset("Steep", &[("freq", 30.0)])];

const RING_MOD: &[Preset] = &[
    preset("Default", &[]),
    preset("Robot", &[("freq", 60.0)]),
    preset("Bell", &[("freq", 1_250.0), ("mix", 0.6)]),
    preset("Tremolo", &[("freq", 6.0), ("mix", 0.5)]),
];

const FREQ_SHIFT: &[Preset] = &[
    preset("Default", &[]),
    preset("Detune", &[("shift", 3.0), ("mix", 0.5)]),
    preset("Down", &[("shift", -200.0)]),
    preset("Metallic", &[("shift", 700.0), ("mix", 0.7)]),
];

const FORMANT: &[Preset] = &[
    preset("A", &[]),
    preset("E", &[("vowel", 1.0)]),
    preset("I", &[("vowel", 2.0)]),
    preset("O", &[("vowel", 3.0)]),
    preset("U", &[("vowel", 4.0)]),
    preset("Talking", &[("vowel", 1.5), ("resonance", 0.9)]),
];

const AUTO_FILTER: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Slow sweep",
        &[("rate", 5.0), ("depth", 3.0), ("cutoff", 600.0)],
    ),
    preset(
        "Wobble",
        &[
            ("rate", 1.0),
            ("depth", 2.5),
            ("resonance", 0.7),
            ("cutoff", 400.0),
        ],
    ),
    preset(
        "Envelope follower",
        &[
            ("depth", 0.0),
            ("envelope", 3.0),
            ("cutoff", 300.0),
            ("resonance", 0.6),
        ],
    ),
    preset(
        "High-pass swell",
        &[
            ("mode", 2.0),
            ("rate", 5.0),
            ("depth", 2.0),
            ("cutoff", 800.0),
        ],
    ),
    preset(
        "Sidechain pump",
        &[
            ("cutoff", 20_000.0),
            ("depth", 0.0),
            ("lfo_shape", 2.0),
            ("volume", 0.8),
        ],
    ),
    preset(
        "Drawn wah",
        &[
            ("mode", 1.0),
            ("resonance", 0.7),
            ("lfo_shape", 5.0),
            ("step_1", -1.0),
            ("step_2", 1.0),
            ("step_3", -0.5),
            ("step_4", 0.5),
            ("step_5", -1.0),
            ("step_6", 1.0),
            ("step_7", 0.0),
            ("step_8", -0.5),
        ],
    ),
    preset(
        "Moving pan",
        &[
            ("depth", 1.0),
            ("lfo_shape", 1.0),
            ("pan", 0.8),
            ("rate", 4.0),
        ],
    ),
];

const NOISE_GATE: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Tight drums",
        &[("threshold", -30.0), ("hold", 5.0), ("release", 40.0)],
    ),
    preset(
        "Soft",
        &[("threshold", -50.0), ("release", 300.0), ("range", -20.0)],
    ),
];

const TRANSIENT: &[Preset] = &[
    preset("Neutral", &[]),
    preset("Punch", &[("attack", 0.7), ("output", -4.0)]),
    preset("Soften", &[("attack", -0.6)]),
    preset("Tighten", &[("attack", 0.3), ("sustain", -0.7)]),
    preset("Sustain", &[("sustain", 0.8), ("output", -3.0)]),
];

const MULTI_TAP: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Dotted",
        &[
            ("tap1_time", 0.75),
            ("tap2_time", 1.5),
            ("tap3_time", 2.25),
            ("tap4_time", 3.0),
        ],
    ),
    preset(
        "Rhythmic",
        &[
            ("tap1_time", 0.25),
            ("tap2_time", 0.75),
            ("tap3_time", 1.25),
            ("tap4_time", 2.0),
            ("feedback", 0.1),
        ],
    ),
    preset(
        "Wash",
        &[("feedback", 0.7), ("damping", 0.6), ("mix", 0.45)],
    ),
    preset(
        "Fading lo-fi",
        &[
            ("tap1_high_cut", 8_000.0),
            ("tap2_high_cut", 4_000.0),
            ("tap3_high_cut", 2_000.0),
            ("tap4_high_cut", 1_000.0),
            ("tap3_low_cut", 200.0),
            ("tap4_low_cut", 400.0),
        ],
    ),
    preset(
        "Sweep across",
        &[
            ("tap1_pan", -1.0),
            ("tap2_pan", -0.33),
            ("tap3_pan", 0.33),
            ("tap4_pan", 1.0),
            ("feedback", 0.0),
        ],
    ),
];

const GRAPHIC_EQ: &[Preset] = &[
    preset("Flat", &[]),
    preset(
        "Smile",
        &[
            ("band_63", 5.0),
            ("band_160", 3.0),
            ("band_1k", -2.0),
            ("band_6k3", 3.0),
            ("band_16k", 5.0),
        ],
    ),
    preset(
        "Telephone",
        &[
            ("band_63", -18.0),
            ("band_160", -12.0),
            ("band_1k", 8.0),
            ("band_6k3", -12.0),
            ("band_16k", -18.0),
        ],
    ),
    preset(
        "Warm",
        &[
            ("band_160", 3.0),
            ("band_400", 2.0),
            ("band_6k3", -2.0),
            ("band_16k", -3.0),
        ],
    ),
];

const PARAMETRIC_EQ: &[Preset] = &[
    preset("Flat", &[]),
    preset(
        "Vocal",
        &[
            ("band1_gain", -9.0),
            ("band1_freq", 90.0),
            ("band3_gain", -3.0),
            ("band5_gain", 3.0),
            ("band7_gain", 4.0),
        ],
    ),
    preset(
        "Kick",
        &[
            ("band1_gain", 4.0),
            ("band3_gain", -6.0),
            ("band3_freq", 350.0),
            ("band5_gain", 4.0),
            ("band5_freq", 3_500.0),
        ],
    ),
    preset("Air", &[("band7_gain", 6.0), ("band7_freq", 10_000.0)]),
];

const MULTIBAND: &[Preset] = &[
    preset("Gentle glue", &[]),
    preset(
        "Master polish",
        &[
            ("low_thresh", -20.0),
            ("low_ratio", 2.0),
            ("mid_thresh", -16.0),
            ("mid_ratio", 2.0),
            ("high_thresh", -22.0),
            ("high_ratio", 2.5),
            ("high_gain", 1.5),
        ],
    ),
    preset(
        "Tame lows",
        &[
            ("low_thresh", -30.0),
            ("low_ratio", 6.0),
            ("mid_ratio", 1.0),
            ("high_ratio", 1.0),
        ],
    ),
    preset(
        "De-ess",
        &[
            ("high_freq", 5_000.0),
            ("high_thresh", -30.0),
            ("high_ratio", 8.0),
            ("low_ratio", 1.0),
            ("mid_ratio", 1.0),
        ],
    ),
    preset(
        "Upfront",
        &[
            ("low_thresh", -30.0),
            ("mid_thresh", -30.0),
            ("high_thresh", -30.0),
            ("low_ratio", 6.0),
            ("mid_ratio", 6.0),
            ("high_ratio", 6.0),
            ("output", 6.0),
        ],
    ),
];

const VOCODER: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Robot",
        &[("bandwidth", 0.8), ("release", 30.0), ("noise", 0.0)],
    ),
    preset("Whisper", &[("noise", 0.8), ("bandwidth", 0.3)]),
    preset("Deep", &[("shift", -5.0)]),
    preset("Chipmunk", &[("shift", 7.0)]),
];

const PITCH_SHIFT: &[Preset] = &[
    preset("Octave up", &[]),
    preset("Octave down", &[("pitch", -12.0)]),
    preset("Fifth", &[("pitch", 7.0), ("mix", 0.5)]),
    preset(
        "Doubler",
        &[
            ("pitch", 0.0),
            ("fine", 12.0),
            ("window", 30.0),
            ("mix", 0.5),
        ],
    ),
];

const PITCH_CORRECT: &[Preset] = &[
    preset("Natural", &[]),
    preset("Hard tune", &[("speed", 0.0)]),
    preset("Gentle", &[("speed", 120.0), ("amount", 0.6)]),
    preset("C major", &[("scale", 1.0), ("speed", 10.0)]),
    preset("A minor", &[("key", 9.0), ("scale", 2.0), ("speed", 10.0)]),
    preset("Thirds", &[("harmony_1", 4.0), ("harmony_level", 0.6)]),
    preset(
        "Choir",
        &[
            ("harmony_1", 7.0),
            ("harmony_2", -5.0),
            ("harmony_level", 0.5),
        ],
    ),
];

const TUNER: &[Preset] = &[
    preset("A = 440 Hz", &[]),
    preset("A = 432 Hz", &[("reference", 432.0)]),
    preset("A = 442 Hz", &[("reference", 442.0)]),
];

const STUTTER: &[Preset] = &[
    preset("Bar end", &[]),
    preset("Build", &[("slice", 0.0), ("portion", 0.5)]),
    preset(
        "Half-bar echo",
        &[("slice", 2.0), ("every", 4.0), ("decay", 0.7)],
    ),
    preset(
        "Glitch",
        &[
            ("slice", 0.0),
            ("every", 3.0),
            ("portion", 0.5),
            ("mix", 0.8),
        ],
    ),
];

const CONVOLUTION: &[Preset] = &[
    preset("Default", &[]),
    preset("Subtle", &[("mix", 0.15), ("high_cut", 8_000.0)]),
    preset(
        "Distant",
        &[("mix", 0.6), ("predelay", 40.0), ("low_cut", 200.0)],
    ),
    preset("Dark", &[("mix", 0.35), ("high_cut", 3_000.0)]),
];

const DYNAMICS: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Glue",
        &[
            ("comp_threshold", -18.0),
            ("comp_ratio", 2.0),
            ("comp_attack", 30.0),
            ("comp_release", 200.0),
            ("comp_makeup", 2.0),
        ],
    ),
    preset(
        "Loud",
        &[
            ("comp_threshold", -12.0),
            ("comp_ratio", 4.0),
            ("comp_attack", 5.0),
            ("comp_makeup", 3.0),
            ("limit_gain", 6.0),
            ("limit_ceiling", -0.1),
        ],
    ),
    preset(
        "Tight drums",
        &[
            ("gate_threshold", -40.0),
            ("gate_hold", 10.0),
            ("gate_release", 60.0),
            ("comp_threshold", -20.0),
            ("comp_ratio", 3.0),
            ("comp_attack", 15.0),
            ("comp_release", 80.0),
        ],
    ),
    preset(
        "Clean vocal",
        &[
            ("gate_threshold", -50.0),
            ("gate_range", -20.0),
            ("comp_threshold", -24.0),
            ("comp_ratio", 3.0),
            ("comp_makeup", 6.0),
        ],
    ),
];

const STEREO_SHAPER: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Swap sides",
        &[
            ("left_to_left", 0.0),
            ("right_to_left", 1.0),
            ("left_to_right", 1.0),
            ("right_to_right", 0.0),
        ],
    ),
    preset(
        "Mono",
        &[
            ("left_to_left", 0.5),
            ("right_to_left", 0.5),
            ("left_to_right", 0.5),
            ("right_to_right", 0.5),
        ],
    ),
    preset(
        "Left only",
        &[("left_to_right", 1.0), ("right_to_right", 0.0)],
    ),
    preset("Haas wide", &[("delay", 12.0)]),
    preset(
        "Wider",
        &[
            ("left_to_left", 0.8),
            ("right_to_left", -0.3),
            ("left_to_right", -0.3),
            ("right_to_right", 0.8),
        ],
    ),
];

const MULTIBAND_LIMITER: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Enhance",
        &[
            ("low_drive", 6.0),
            ("low_ceiling", -6.0),
            ("mid_drive", 4.0),
            ("mid_ceiling", -6.0),
            ("high_drive", 6.0),
            ("high_ceiling", -8.0),
            ("high_level", 2.0),
            ("amount", 0.5),
        ],
    ),
    preset(
        "Enhance hard",
        &[
            ("low_drive", 12.0),
            ("low_ceiling", -9.0),
            ("mid_drive", 9.0),
            ("mid_ceiling", -9.0),
            ("high_drive", 12.0),
            ("high_ceiling", -10.0),
            ("high_level", 3.0),
            ("release", 60.0),
        ],
    ),
    preset(
        "Warm loud",
        &[
            ("low_drive", 8.0),
            ("low_ceiling", -6.0),
            ("low_level", 2.0),
            ("mid_drive", 5.0),
            ("mid_ceiling", -6.0),
            ("high_level", -2.0),
            ("release", 150.0),
        ],
    ),
    preset(
        "Tame lows",
        &[
            ("low_ceiling", -12.0),
            ("low_level", 4.0),
            ("low_freq", 120.0),
        ],
    ),
    preset(
        "Brickwall",
        &[
            ("mid_drive", 6.0),
            ("low_drive", 6.0),
            ("high_drive", 6.0),
            ("ceiling", -1.0),
        ],
    ),
];

const SEND: &[Preset] = &[
    preset("Default", &[]),
    preset("Half", &[("level", 0.5)]),
    preset("Send only", &[("dry", 0.0)]),
];

const HALL_REVERB: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Small room",
        &[
            ("decay", 0.6),
            ("size", 0.2),
            ("predelay", 5.0),
            ("early", 0.7),
        ],
    ),
    preset(
        "Concert hall",
        &[
            ("decay", 3.5),
            ("size", 0.85),
            ("predelay", 30.0),
            ("damping", 5_000.0),
        ],
    ),
    preset(
        "Huge space",
        &[
            ("decay", 12.0),
            ("size", 1.0),
            ("predelay", 60.0),
            ("mod_depth", 0.6),
            ("mix", 0.4),
        ],
    ),
    preset(
        "Bright plate",
        &[
            ("decay", 1.8),
            ("size", 0.4),
            ("damping", 14_000.0),
            ("early", 0.1),
            ("low_cut", 250.0),
        ],
    ),
    preset("Dark", &[("damping", 2_000.0), ("decay", 3.0)]),
];

const GUITAR_CHAIN: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Clean",
        &[
            ("drive", 0.0),
            ("compress", 0.5),
            ("chorus", 0.4),
            ("output", 0.0),
        ],
    ),
    preset(
        "Crunch",
        &[
            ("drive", 24.0),
            ("mid", 3.0),
            ("treble", 2.0),
            ("output", -9.0),
        ],
    ),
    preset(
        "High gain",
        &[
            ("drive", 40.0),
            ("drive_type", 1.0),
            ("bass", 4.0),
            ("mid", -5.0),
            ("treble", 4.0),
            ("cabinet", 2.0),
            ("output", -12.0),
        ],
    ),
    preset(
        "Fuzz",
        &[
            ("drive", 36.0),
            ("drive_type", 2.0),
            ("treble", -3.0),
            ("output", -10.0),
        ],
    ),
];

const MULTI_FX: &[Preset] = &[
    preset("Default", &[]),
    preset("Sweep down", &[("block", 1.0), ("x", 0.2), ("y", 0.6)]),
    preset("Thin out", &[("block", 2.0), ("x", 0.6), ("y", 0.3)]),
    preset("Lo-fi", &[("block", 4.0), ("x", 0.6), ("y", 0.3)]),
    preset("Robot", &[("block", 5.0), ("x", 0.25), ("y", 0.8)]),
    preset("Wash", &[("block", 9.0), ("x", 0.9), ("y", 0.6)]),
];

const FX_CHAIN: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Dirty space",
        &[
            ("stage1_block", 4.0),
            ("stage1_x", 0.5),
            ("stage1_y", 0.2),
            ("stage2_block", 7.0),
            ("stage2_x", 0.2),
            ("stage2_y", 0.7),
            ("stage3_block", 9.0),
            ("stage3_x", 0.8),
            ("stage3_y", 0.4),
        ],
    ),
    preset(
        "Radio",
        &[
            ("stage1_block", 2.0),
            ("stage1_x", 0.45),
            ("stage1_y", 0.3),
            ("stage2_block", 1.0),
            ("stage2_x", 0.65),
            ("stage2_y", 0.3),
            ("stage3_block", 3.0),
            ("stage3_x", 0.4),
            ("stage3_y", 0.6),
        ],
    ),
    preset(
        "Swirl",
        &[
            ("stage1_block", 6.0),
            ("stage1_x", 0.3),
            ("stage1_y", 0.6),
            ("stage2_block", 8.0),
            ("stage2_x", 0.4),
            ("stage2_y", 0.5),
            ("stage3_block", 10.0),
            ("stage3_x", 0.75),
            ("stage3_y", 0.5),
        ],
    ),
];

const BEAT_SHAPER: &[Preset] = &[
    preset("Default", &[]),
    preset(
        "Half repeat",
        &[
            ("time_5", 0.5),
            ("time_6", 0.5),
            ("time_7", 0.5),
            ("time_8", 0.5),
        ],
    ),
    preset(
        "Tape stop",
        &[
            ("time_mode", 1.0),
            ("time_5", 0.0),
            ("time_6", 0.15),
            ("time_7", 0.32),
            ("time_8", 0.45),
        ],
    ),
    preset(
        "Stutter end",
        &[("time_7", 0.125), ("time_8", 0.25), ("cycle", 1.0)],
    ),
    preset(
        "Pump",
        &[
            ("volume_1", 0.0),
            ("volume_2", 0.6),
            ("volume_3", 0.0),
            ("volume_4", 0.6),
            ("volume_5", 0.0),
            ("volume_6", 0.6),
            ("volume_7", 0.0),
            ("volume_8", 0.6),
        ],
    ),
    preset(
        "Gate halves",
        &[
            ("volume_2", 0.0),
            ("volume_4", 0.0),
            ("volume_6", 0.0),
            ("volume_8", 0.0),
        ],
    ),
];

const BAND_SPLIT: &[Preset] = &[
    preset("Default", &[]),
    preset("Bass split", &[("low_freq", 120.0), ("high_freq", 6_000.0)]),
    preset(
        "Vocal split",
        &[("low_freq", 300.0), ("high_freq", 4_000.0)],
    ),
];

const SCRATCH: &[Preset] = &[
    preset("Default", &[]),
    preset("Baby scratch", &[("pattern", 1.0), ("depth", 0.5)]),
    preset("Chirps", &[("pattern", 2.0), ("rate", 2.0), ("depth", 0.3)]),
    preset("Tear", &[("pattern", 3.0), ("depth", 0.6)]),
    preset(
        "Transformer",
        &[("pattern", 4.0), ("rate", 4.0), ("depth", 0.4)],
    ),
];

/// Presets for a built-in effect.
#[must_use]
pub fn effect(kind: EffectKind) -> &'static [Preset] {
    match kind {
        EffectKind::Filter => FILTER,
        EffectKind::Eq => EQ,
        EffectKind::Delay => DELAY,
        EffectKind::Reverb => REVERB,
        EffectKind::Compressor => COMPRESSOR,
        EffectKind::Distortion => DISTORTION,
        EffectKind::Chorus => CHORUS,
        EffectKind::Limiter => LIMITER,
        EffectKind::Phaser => PHASER,
        EffectKind::Flanger => FLANGER,
        EffectKind::Bitcrusher => BITCRUSHER,
        EffectKind::Stereo => STEREO,
        EffectKind::Gate => GATE,
        EffectKind::Autopan => AUTOPAN,
        EffectKind::BassBoost => BASS_BOOST,
        EffectKind::Overdrive => OVERDRIVE,
        EffectKind::SoftClipper => SOFT_CLIPPER,
        EffectKind::Waveshaper => WAVESHAPER,
        EffectKind::DcBlock => DC_BLOCK,
        EffectKind::RingMod => RING_MOD,
        EffectKind::FreqShift => FREQ_SHIFT,
        EffectKind::Formant => FORMANT,
        EffectKind::AutoFilter => AUTO_FILTER,
        EffectKind::NoiseGate => NOISE_GATE,
        EffectKind::Transient => TRANSIENT,
        EffectKind::MultiTap => MULTI_TAP,
        EffectKind::GraphicEq => GRAPHIC_EQ,
        EffectKind::ParametricEq => PARAMETRIC_EQ,
        EffectKind::Multiband => MULTIBAND,
        EffectKind::Vocoder => VOCODER,
        EffectKind::PitchShift => PITCH_SHIFT,
        EffectKind::Stutter => STUTTER,
        EffectKind::Convolution => CONVOLUTION,
        EffectKind::PitchCorrect => PITCH_CORRECT,
        EffectKind::Tuner => TUNER,
        EffectKind::Dynamics => DYNAMICS,
        EffectKind::StereoShaper => STEREO_SHAPER,
        EffectKind::MultibandLimiter => MULTIBAND_LIMITER,
        EffectKind::Send => SEND,
        EffectKind::HallReverb => HALL_REVERB,
        EffectKind::GuitarChain => GUITAR_CHAIN,
        EffectKind::MultiFx => MULTI_FX,
        EffectKind::FxChain => FX_CHAIN,
        EffectKind::BeatShaper => BEAT_SHAPER,
        EffectKind::BandSplit => BAND_SPLIT,
        EffectKind::Scratch => SCRATCH,
        EffectKind::Plugin => &[],
    }
}

/// The synth parameter table, for [`Preset::resolve`] with [`SYNTH`].
#[must_use]
pub fn synth_specs() -> &'static [ParamSpec] {
    INSTRUMENT_PARAMS
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::EFFECT_KINDS;

    #[test]
    fn every_preset_names_real_parameters_in_range() {
        let mut tables: Vec<(&[ParamSpec], &[Preset])> = vec![
            (INSTRUMENT_PARAMS, SYNTH),
            (INSTRUMENT_PARAMS, DRUM),
            (INSTRUMENT_PARAMS, PLUCK),
            (INSTRUMENT_PARAMS, FM),
            (INSTRUMENT_PARAMS, KEYS),
            (INSTRUMENT_PARAMS, ADDITIVE),
        ];
        tables.extend(EFFECT_KINDS.iter().map(|k| (k.params(), effect(*k))));
        for (specs, presets) in tables {
            assert!(!presets.is_empty());
            for preset in presets {
                for (name, value) in preset.values {
                    let spec = specs
                        .iter()
                        .find(|s| s.name == *name)
                        .unwrap_or_else(|| panic!("{}: no parameter `{name}`", preset.name));
                    assert!(spec.accepts(*value), "{}: {name} = {value}", preset.name);
                }
                let values: Vec<f32> = specs
                    .iter()
                    .map(|s| {
                        preset
                            .resolve(specs)
                            .iter()
                            .find(|(n, _)| *n == s.name)
                            .map_or(s.default, |(_, v)| *v)
                    })
                    .collect();
                assert!(preset.matches(specs, &values), "{}", preset.name);
            }
        }
        assert!(
            !SYNTH[1]
                .resolve(INSTRUMENT_PARAMS)
                .iter()
                .any(|(n, _)| *n == "root_key")
        );
    }
}
