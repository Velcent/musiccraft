//! Which instrument parameters each instrument actually uses, so a plugin
//! wrapper or a generic editor shows only those (the table itself is shared;
//! see `musiccraft_params::INSTRUMENT_PARAMS`).

use crate::Instrument;

/// The amplitude envelope.
const ENVELOPE: [&str; 4] = ["attack", "decay", "sustain", "release"];

/// Played by the host rather than a voice: arpeggiator, swing, timing,
/// echo, cut groups and polyphony. A plugin wrapper implements these around
/// the voices; every sounding instrument has them.
pub const NOTE_FUNCTIONS: &[&str] = &[
    "arp_mode",
    "arp_rate",
    "arp_range",
    "arp_gate",
    "swing_mix",
    "swing_truncate",
    "time_shift",
    "gate",
    "echo_count",
    "echo_time",
    "echo_feed",
    "echo_pitch",
    "cut_group",
    "cut_by",
    "mono",
    "poly_max",
];

const SYNTH: &[&str] = &[
    "wave1",
    "wave2",
    "detune",
    "osc_mix",
    "unison",
    "spread",
    "sub",
    "wave3",
    "osc3_level",
    "osc3_coarse",
    "cutoff",
    "resonance",
    "env_amount",
    "filter_type",
    "drive",
    "filter2_type",
    "cutoff2",
    "resonance2",
    "filter_routing",
    "lfo_rate",
    "lfo_depth",
    "lfo_target",
    "lfo2_rate",
    "lfo2_shape",
    "env2_attack",
    "env2_decay",
    "mod1_source",
    "mod1_dest",
    "mod1_amount",
    "mod2_source",
    "mod2_dest",
    "mod2_amount",
    "mod3_source",
    "mod3_dest",
    "mod3_amount",
    "pitch",
    "glide",
];

const FM: &[&str] = &[
    "fm_algo",
    "fm_feedback",
    "op1_ratio",
    "op1_level",
    "op1_decay",
    "op1_sustain",
    "op1_wave",
    "op2_ratio",
    "op2_level",
    "op2_decay",
    "op2_sustain",
    "op2_wave",
    "op3_ratio",
    "op3_level",
    "op3_decay",
    "op3_sustain",
    "op3_wave",
    "op4_ratio",
    "op4_level",
    "op4_decay",
    "op4_sustain",
    "op4_wave",
    "op5_ratio",
    "op5_level",
    "op5_decay",
    "op5_sustain",
    "op5_wave",
    "op6_ratio",
    "op6_level",
    "op6_decay",
    "op6_sustain",
    "op6_wave",
    "fm_cutoff",
    "fm_resonance",
    "pitch",
    "glide",
];

const KEYS: &[&str] = &[
    "keys_type",
    "keys_tone",
    "keys_decay",
    "keys_tremolo",
    "keys_hammer",
    "pitch",
    "glide",
];

const ADDITIVE: &[&str] = &[
    "add_partials",
    "add_bright",
    "add_even",
    "add_inharm",
    "add_decay",
    "add_bright_b",
    "add_even_b",
    "add_inharm_b",
    "add_morph",
    "add_morph_time",
    "add_morph_rate",
    "add_morph_depth",
    "pitch",
    "glide",
];

const DRUM: &[&str] = &[
    "drum_type",
    "drum_tune",
    "drum_decay",
    "drum_sweep",
    "drum_tone",
    "drum_click",
    "pitch",
];

const PLUCK: &[&str] = &[
    "pluck_decay",
    "pluck_damping",
    "pluck_tone",
    "pluck_bow",
    "pluck_strings",
    "pluck_detune",
    "pitch",
];

const SAMPLER: &[&str] = &[
    "root_key", "pitch", "glide", "one_shot", "slices", "reverse", "fade_in", "fade_out",
];

const KIT: &[&str] = &["root_key", "pitch", "fade_in", "fade_out"];

const ZONES: &[&str] = &[
    "pitch", "glide", "one_shot", "reverse", "fade_in", "fade_out",
];

const GRANULAR: &[&str] = &[
    "gran_size",
    "gran_density",
    "gran_position",
    "gran_spread",
    "gran_pitch_rand",
    "gran_scan",
    "root_key",
    "pitch",
    "glide",
];

const RESYNTH: &[&str] = &[
    "resyn_speed",
    "resyn_position",
    "resyn_blur",
    "resyn_loop",
    "add_partials",
    "add_inharm",
    "pitch",
    "glide",
];

const MIDI_OUT: &[&str] = &[
    "midi_cc1",
    "midi_cc2",
    "midi_cc3",
    "midi_cc4",
    "midi_cc5",
    "midi_cc6",
    "midi_cc7",
    "midi_cc8",
    "midi_cc1_number",
    "midi_cc2_number",
    "midi_cc3_number",
    "midi_cc4_number",
    "midi_cc5_number",
    "midi_cc6_number",
    "midi_cc7_number",
    "midi_cc8_number",
    "midi_program",
];

/// An instrument without its data (samples, zones, routing).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Sampler,
    Synth,
    DrumSynth,
    Pluck,
    Fm,
    Keys,
    Additive,
    Granular,
    Kit,
    Zones,
    Resynth,
    Layer,
    External,
    MidiOut,
}

/// Every kind, in [`Kind`] order.
pub const KINDS: [Kind; 14] = [
    Kind::Sampler,
    Kind::Synth,
    Kind::DrumSynth,
    Kind::Pluck,
    Kind::Fm,
    Kind::Keys,
    Kind::Additive,
    Kind::Granular,
    Kind::Kit,
    Kind::Zones,
    Kind::Resynth,
    Kind::Layer,
    Kind::External,
    Kind::MidiOut,
];

impl Kind {
    /// The kind's own sound parameters (not the [`NOTE_FUNCTIONS`]).
    #[must_use]
    pub fn sound_params(self) -> &'static [&'static str] {
        match self {
            Self::Sampler => SAMPLER,
            Self::Synth => SYNTH,
            Self::DrumSynth => DRUM,
            Self::Pluck => PLUCK,
            Self::Fm => FM,
            Self::Keys => KEYS,
            Self::Additive => ADDITIVE,
            Self::Granular => GRANULAR,
            Self::Kit => KIT,
            Self::Zones => ZONES,
            Self::Resynth => RESYNTH,
            Self::MidiOut => MIDI_OUT,
            Self::Layer | Self::External => &[],
        }
    }

    /// Whether its voices follow the amplitude envelope.
    #[must_use]
    pub fn uses_envelope(self) -> bool {
        !matches!(
            self,
            Self::DrumSynth | Self::Layer | Self::External | Self::MidiOut
        )
    }

    /// Whether it makes sound itself (so the [`NOTE_FUNCTIONS`] apply).
    #[must_use]
    pub fn sounds(self) -> bool {
        !matches!(self, Self::Layer | Self::External | Self::MidiOut)
    }

    /// Every parameter the kind uses, in display order: envelope, sound,
    /// note functions.
    pub fn params(self) -> impl Iterator<Item = &'static str> {
        let envelope: &'static [&'static str] = if self.uses_envelope() { &ENVELOPE } else { &[] };
        let notes = if self.sounds() { NOTE_FUNCTIONS } else { &[] };
        envelope
            .iter()
            .chain(self.sound_params())
            .chain(notes)
            .copied()
    }
}

impl Instrument {
    /// This instrument's kind.
    #[must_use]
    pub fn kind(&self) -> Kind {
        match self {
            Self::Sampler(_) => Kind::Sampler,
            Self::Synth => Kind::Synth,
            Self::DrumSynth => Kind::DrumSynth,
            Self::Pluck => Kind::Pluck,
            Self::Fm => Kind::Fm,
            Self::Keys => Kind::Keys,
            Self::Additive => Kind::Additive,
            Self::Granular(_) => Kind::Granular,
            Self::Kit(_) => Kind::Kit,
            Self::Zones(_) => Kind::Zones,
            Self::Resynth(_) => Kind::Resynth,
            Self::Layer(_) => Kind::Layer,
            Self::External(_) => Kind::External,
            Self::MidiOut(_) => Kind::MidiOut,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_params::{INSTRUMENT_PARAMS, index_of};

    #[test]
    fn every_listed_name_is_a_real_parameter_once() {
        for kind in KINDS {
            let names: Vec<_> = kind.params().collect();
            for name in &names {
                assert!(
                    index_of(INSTRUMENT_PARAMS, name).is_some(),
                    "{kind:?}: {name}"
                );
                assert_eq!(
                    names.iter().filter(|n| *n == name).count(),
                    1,
                    "{kind:?} lists {name} twice"
                );
            }
        }
    }

    #[test]
    fn every_parameter_belongs_somewhere() {
        for spec in INSTRUMENT_PARAMS {
            assert!(
                KINDS.iter().any(|k| k.params().any(|n| n == spec.name)),
                "{} is used by no instrument",
                spec.name
            );
        }
    }

    #[test]
    fn instruments_report_their_kind() {
        assert_eq!(Instrument::Synth.kind(), Kind::Synth);
        assert_eq!(Instrument::Layer(vec![1]).kind(), Kind::Layer);
        assert!(Kind::Synth.params().any(|n| n == "attack"));
        assert!(!Kind::DrumSynth.params().any(|n| n == "attack"));
        assert!(Kind::DrumSynth.params().any(|n| n == "arp_mode"));
        assert_eq!(Kind::External.params().count(), 0);
    }
}
