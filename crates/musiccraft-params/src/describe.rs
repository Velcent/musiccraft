//! What a host needs to show and automate a parameter: a label, the names of
//! a choice parameter's values, display text and parsing typed text (used by
//! plugin wrappers and generic parameter lists).
//!
//! Parameter names are the stable identifiers: project files, commands and
//! plugin parameter IDs all use them, so they are never renamed (see the
//! `frozen_names` test).

use crate::{
    ARP_MODES, ARP_RATES, DRUM_TYPES, FILTER_TYPES, FM_ALGOS, FM_WAVES, KEYS_TYPES, LFO_TARGETS,
    MOD_DESTS, MOD_SOURCES, ParamSpec, WAVEFORMS,
};

/// Second filter types, by `filter2_type` value.
pub const FILTER2_TYPES: [&str; 4] = ["off", "low-pass", "band-pass", "high-pass"];
/// Filter routings, by `filter_routing` value.
pub const FILTER_ROUTINGS: [&str; 3] = ["series", "parallel", "split"];
/// Second LFO shapes, by `lfo2_shape` value.
pub const LFO2_SHAPES: [&str; 5] = ["sine", "triangle", "saw", "square", "random"];
/// Voice modes, by `mono` value.
pub const VOICE_MODES: [&str; 2] = ["poly", "mono"];
/// Switches.
pub const OFF_ON: [&str; 2] = ["off", "on"];
/// Arpeggiator rate labels, by `arp_rate` value.
pub const ARP_RATE_LABELS: [&str; 5] = [
    ARP_RATES[0].0,
    ARP_RATES[1].0,
    ARP_RATES[2].0,
    ARP_RATES[3].0,
    ARP_RATES[4].0,
];

/// The names of a choice parameter's values (index = value), if it is one.
#[must_use]
pub fn choices(name: &str) -> Option<&'static [&'static str]> {
    let list: &'static [&'static str] = match name {
        "wave1" | "wave2" | "wave3" => &WAVEFORMS,
        "filter_type" => &FILTER_TYPES,
        "filter2_type" => &FILTER2_TYPES,
        "filter_routing" => &FILTER_ROUTINGS,
        "lfo_target" => &LFO_TARGETS,
        "lfo2_shape" => &LFO2_SHAPES,
        "fm_algo" => &FM_ALGOS,
        "keys_type" => &KEYS_TYPES,
        "drum_type" => &DRUM_TYPES,
        "arp_mode" => &ARP_MODES,
        "arp_rate" => &ARP_RATE_LABELS,
        "mono" => &VOICE_MODES,
        "one_shot" | "reverse" | "resyn_loop" | "swing_truncate" => &OFF_ON,
        _ => {
            let slot = |prefix: &str, suffix: &str| {
                name.strip_prefix(prefix)
                    .and_then(|rest| rest.strip_suffix(suffix))
                    .is_some_and(|n| n.len() == 1 && n.chars().all(|c| c.is_ascii_digit()))
            };
            if slot("mod", "_source") {
                &MOD_SOURCES
            } else if slot("mod", "_dest") {
                &MOD_DESTS
            } else if slot("op", "_wave") {
                &FM_WAVES
            } else {
                return None;
            }
        }
    };
    Some(list)
}

/// A readable label from a parameter name: `cutoff2` → "Cutoff 2",
/// `op3_ratio` → "Op 3 ratio", `add_bright_b` → "Add bright B".
#[must_use]
pub fn label(name: &str) -> String {
    let mut out = String::new();
    let mut previous: Option<char> = None;
    for c in name.chars() {
        match c {
            '_' => out.push(' '),
            c if c.is_ascii_digit() && previous.is_some_and(|p| p.is_ascii_alphabetic()) => {
                out.push(' ');
                out.push(c);
            }
            c => out.push(c),
        }
        previous = Some(c);
    }
    // A lone trailing letter (the B spectrum) reads as a capital.
    if let Some((head, last)) = out.rsplit_once(' ') {
        if last.len() == 1 && last.chars().all(|c| c.is_ascii_alphabetic()) {
            out = format!("{head} {}", last.to_ascii_uppercase());
        }
    }
    let mut chars = out.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().chain(chars).collect(),
        None => out,
    }
}

/// Display text for `value`: the choice's name, or the value with its unit.
#[must_use]
pub fn display(spec: &ParamSpec, value: f32) -> String {
    if let Some(list) = choices(spec.name) {
        let index = spec.sanitize(value) - spec.min;
        if let Some(text) = list.get(index.max(0.0) as usize) {
            return (*text).to_owned();
        }
    }
    spec.format(value)
}

/// Parses text a user typed for `spec`: a choice name, or a number with an
/// optional unit (`2.5 kHz`, `120 ms`, `40%`). The result is sanitized.
#[must_use]
pub fn parse(spec: &ParamSpec, text: &str) -> Option<f32> {
    let text = text.trim();
    if let Some(list) = choices(spec.name) {
        if let Some(i) = list.iter().position(|c| c.eq_ignore_ascii_case(text)) {
            return Some(spec.sanitize(spec.min + i as f32));
        }
    }
    let lower = text.to_ascii_lowercase();
    let split = lower
        .find(|c: char| !(c.is_ascii_digit() || c == '.' || c == '-' || c == '+'))
        .unwrap_or(lower.len());
    let number: f32 = lower[..split].trim().parse().ok()?;
    if !number.is_finite() {
        return None;
    }
    let unit = lower[split..].trim();
    let value = match (spec.unit, unit) {
        ("Hz", "khz") => number * 1_000.0,
        ("s", "ms") => number / 1_000.0,
        ("ms", "s") => number * 1_000.0,
        ("", "%") if !spec.integer => number / 100.0,
        (_, "") => number,
        (expected, given) if expected.eq_ignore_ascii_case(given) => number,
        _ => return None,
    };
    Some(spec.sanitize(value))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{INSTRUMENT_PARAMS, index_of};

    fn spec(name: &str) -> &'static ParamSpec {
        &INSTRUMENT_PARAMS[index_of(INSTRUMENT_PARAMS, name).unwrap()]
    }

    #[test]
    fn choice_lists_cover_their_ranges() {
        for spec in INSTRUMENT_PARAMS {
            if let Some(list) = choices(spec.name) {
                assert!(spec.integer, "{}", spec.name);
                assert_eq!(
                    spec.max - spec.min + 1.0,
                    list.len() as f32,
                    "{}",
                    spec.name
                );
            }
        }
        assert_eq!(choices("mod3_dest"), Some(&MOD_DESTS[..]));
        assert_eq!(choices("op6_wave"), Some(&FM_WAVES[..]));
        assert_eq!(choices("cutoff"), None);
    }

    /// Every parameter name, one `table name` line each (`instrument` or the
    /// effect's name).
    fn all_names() -> String {
        let mut out = String::new();
        let tables = std::iter::once(("instrument", INSTRUMENT_PARAMS)).chain(
            crate::EFFECT_KINDS
                .iter()
                .map(|kind| (kind.name(), kind.params())),
        );
        for (table, params) in tables {
            for spec in params {
                out.push_str(&format!("{table} {}\n", spec.name));
            }
        }
        out
    }

    /// Names are stable IDs (project files, commands, plugin parameter IDs):
    /// none may be renamed or removed. A new parameter is added to
    /// `frozen-names.txt` in the same change (`MUSICCRAFT_BLESS_NAMES=1`
    /// rewrites it).
    #[test]
    fn frozen_names() {
        let path = concat!(env!("CARGO_MANIFEST_DIR"), "/frozen-names.txt");
        let current = all_names();
        if std::env::var_os("MUSICCRAFT_BLESS_NAMES").is_some() {
            std::fs::write(path, &current).unwrap();
        }
        let frozen = std::fs::read_to_string(path).unwrap();
        let lines: std::collections::HashSet<&str> = current.lines().collect();
        for name in frozen.lines() {
            assert!(
                lines.contains(name),
                "parameter `{name}` was renamed or removed"
            );
        }
        let frozen: std::collections::HashSet<&str> = frozen.lines().collect();
        for name in current.lines() {
            assert!(
                frozen.contains(name),
                "new parameter `{name}`: add it to frozen-names.txt"
            );
        }
    }

    #[test]
    fn labels_read_naturally() {
        assert_eq!(label("cutoff2"), "Cutoff 2");
        assert_eq!(label("op3_ratio"), "Op 3 ratio");
        assert_eq!(label("add_bright_b"), "Add bright B");
        assert_eq!(label("attack"), "Attack");
    }

    #[test]
    fn values_display_and_parse_back() {
        assert_eq!(display(spec("filter_type"), 1.0), "band-pass");
        assert_eq!(parse(spec("filter_type"), "High-Pass"), Some(2.0));
        assert_eq!(parse(spec("cutoff"), "2.5 kHz"), Some(2_500.0));
        assert_eq!(parse(spec("attack"), "120 ms"), Some(0.12));
        assert_eq!(parse(spec("osc_mix"), "40%"), Some(0.4));
        assert_eq!(parse(spec("unison"), "99"), Some(spec("unison").max));
        assert_eq!(parse(spec("cutoff"), "loud"), None);
        assert_eq!(parse(spec("cutoff"), "3 dB"), None);
        for spec in INSTRUMENT_PARAMS {
            let text = display(spec, spec.default);
            if choices(spec.name).is_some() {
                assert_eq!(
                    parse(spec, &text),
                    Some(spec.default),
                    "{}: {text}",
                    spec.name
                );
            }
        }
    }
}
