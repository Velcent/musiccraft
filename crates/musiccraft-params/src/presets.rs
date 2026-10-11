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
pub const KEEP: [&str; 8] = [
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
