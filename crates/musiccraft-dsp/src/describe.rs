//! What an effect knob shows: the names of a choice parameter's values and
//! the display text of a value. The mixer's effect windows and the exported
//! plugins (as host display text and typed-text parsing) both use this, so a
//! value reads the same everywhere.

use musiccraft_params::{
    CORRECTION_SCALES, EffectKind, FILTER_TYPES, LFO_SHAPES, NOTE_NAMES, ParamSpec,
};

use crate::beat::BEAT_CYCLE_NAMES;
use crate::blocks::BLOCK_NAMES;
use crate::modulation::SYNC_RATE_NAMES;
use crate::scratch::SCRATCH_PATTERNS;

const WAVESHAPES: [&str; 7] = [
    "tanh",
    "hard clip",
    "fold",
    "sine",
    "asymmetric",
    "rectify",
    "drawn",
];
const OFF_ON: [&str; 2] = ["off", "on"];
const VOWELS: [&str; 5] = ["a", "e", "i", "o", "u"];

/// Whether `kind`'s `rate`-style parameters count tempo-synced divisions.
fn synced(kind: EffectKind, name: &str) -> bool {
    match kind {
        EffectKind::Gate | EffectKind::Autopan | EffectKind::AutoFilter | EffectKind::Scratch => {
            name == "rate"
        }
        EffectKind::Stutter => matches!(name, "slice" | "every"),
        _ => false,
    }
}

/// The names of a choice parameter's values (index = value), if parameter
/// `name` of `kind` is one.
#[must_use]
pub fn choices(kind: EffectKind, name: &str) -> Option<Vec<&'static str>> {
    let list: Vec<&'static str> = match name {
        _ if synced(kind, name) => SYNC_RATE_NAMES.to_vec(),
        "shape" if kind == EffectKind::Waveshaper => WAVESHAPES.to_vec(),
        "shape" => vec!["sine", "triangle", "square"],
        "lfo_shape" => LFO_SHAPES.to_vec(),
        "mode" if kind == EffectKind::Distortion => vec!["soft", "hard"],
        "mode" => FILTER_TYPES.to_vec(),
        "drive_type" => vec!["soft", "hard", "fuzz"],
        "cycle" => BEAT_CYCLE_NAMES.to_vec(),
        "pattern" => SCRATCH_PATTERNS.to_vec(),
        "hold" if kind == EffectKind::Scratch => vec!["off", "held"],
        "time_mode" => vec!["steps", "ramps"],
        name if name.ends_with("block") => BLOCK_NAMES.to_vec(),
        "cabinet" => vec!["off", "small", "large"],
        "key" => NOTE_NAMES.to_vec(),
        "scale" => CORRECTION_SCALES.iter().map(|(name, _)| *name).collect(),
        "invert_left" | "invert_right" => OFF_ON.to_vec(),
        _ => return None,
    };
    Some(list)
}

/// Display text for `value` of parameter `spec` of `kind`.
#[must_use]
pub fn text(kind: EffectKind, spec: &ParamSpec, value: f32) -> String {
    if let Some(list) = choices(kind, spec.name) {
        return list
            .get(value.round().max(0.0) as usize)
            .copied()
            .unwrap_or("?")
            .to_owned();
    }
    match spec.name {
        "sidechain" if value < 0.5 => "off".to_owned(),
        "sidechain" => format!("from {value:.0}"),
        "follow" if value < 0.5 => "off".to_owned(),
        "follow" => format!("ch {value:.0}"),
        "harmony_1" | "harmony_2" if value.abs() < 0.5 => "off".to_owned(),
        "harmony_1" | "harmony_2" => format!("{value:+.0} st"),
        "target" if value < 0.5 => "off".to_owned(),
        "target" => format!("to {value:.0}"),
        name if name.ends_with("_target") && value < 0.5 => "stays".to_owned(),
        name if name.ends_with("_target") => format!("to {value:.0}"),
        "vowel" => {
            let i = (value.floor().max(0.0) as usize).min(4);
            let next = (i + 1).min(4);
            if value.fract() < 0.05 || i == next {
                VOWELS[i].to_owned()
            } else {
                format!("{}→{}", VOWELS[i], VOWELS[next])
            }
        }
        _ => spec.format(value),
    }
}

/// Parses text typed for parameter `spec` of `kind`: a choice name, `off`
/// for parameters that show it, or a number with an optional unit (see
/// [`musiccraft_params::describe::parse`]).
#[must_use]
pub fn parse(kind: EffectKind, spec: &ParamSpec, typed: &str) -> Option<f32> {
    let typed = typed.trim();
    if let Some(list) = choices(kind, spec.name) {
        if let Some(i) = list.iter().position(|c| c.eq_ignore_ascii_case(typed)) {
            return Some(spec.sanitize(i as f32));
        }
    }
    if typed.eq_ignore_ascii_case(&text(kind, spec, 0.0)) && spec.accepts(0.0) {
        return Some(0.0);
    }
    if spec.name == "vowel" {
        if let Some(i) = VOWELS.iter().position(|v| v.eq_ignore_ascii_case(typed)) {
            return Some(spec.sanitize(i as f32));
        }
    }
    // Routing values read "from 3", "ch 2", "to 5"; harmonies "+7 st".
    let number = ["from ", "ch ", "to "]
        .iter()
        .find_map(|prefix| typed.strip_prefix(prefix))
        .unwrap_or(typed);
    let number = number.strip_suffix(" st").unwrap_or(number);
    musiccraft_params::describe::parse(spec, number.trim_start_matches('+'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_params::EFFECT_KINDS;

    fn spec(kind: EffectKind, name: &str) -> ParamSpec {
        *kind.params().iter().find(|s| s.name == name).unwrap()
    }

    #[test]
    fn choices_name_values() {
        let delay_free = text(EffectKind::Gate, &spec(EffectKind::Gate, "rate"), 2.0);
        assert_eq!(delay_free, "1/8");
        let mode = spec(EffectKind::Filter, "mode");
        assert_eq!(text(EffectKind::Filter, &mode, 0.0), FILTER_TYPES[0]);
        let shape = spec(EffectKind::Waveshaper, "shape");
        assert_eq!(text(EffectKind::Waveshaper, &shape, 2.0), "fold");
        assert_eq!(parse(EffectKind::Waveshaper, &shape, "Fold"), Some(2.0));
    }

    #[test]
    fn plain_values_use_their_unit() {
        let cutoff = spec(EffectKind::Filter, "cutoff");
        assert_eq!(text(EffectKind::Filter, &cutoff, 2_500.0), "2.50 kHz");
        assert_eq!(parse(EffectKind::Filter, &cutoff, "2.5 kHz"), Some(2_500.0));
    }

    #[test]
    fn every_choice_fits_its_range() {
        for kind in EFFECT_KINDS {
            for spec in kind.params() {
                if let Some(list) = choices(kind, spec.name) {
                    assert!(spec.min >= 0.0, "{kind}/{}", spec.name);
                    assert!(
                        list.len() as f32 > spec.max,
                        "{kind}/{} has {} names for max {}",
                        spec.name,
                        list.len(),
                        spec.max
                    );
                }
            }
        }
    }

    #[test]
    fn every_text_parses_back() {
        for kind in EFFECT_KINDS {
            for spec in kind.params() {
                for value in [spec.min, spec.default, spec.max] {
                    let shown = text(kind, spec, value);
                    let parsed = parse(kind, spec, &shown)
                        .unwrap_or_else(|| panic!("{kind}/{}: {shown:?}", spec.name));
                    assert_eq!(
                        text(kind, spec, parsed),
                        shown,
                        "{kind}/{} {value}",
                        spec.name
                    );
                }
            }
        }
    }
}
