//! Note names and musical time helpers.

use musiccraft_params::{TICKS_PER_BAR, TICKS_PER_STEP};

const NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Name of a MIDI key, with key 60 = `C5` (octave = key / 12).
#[must_use]
pub fn key_name(key: u8) -> String {
    let name = NAMES.get(usize::from(key % 12)).copied().unwrap_or("?");
    format!("{name}{}", key / 12)
}

/// Parses a key given as a MIDI number (`60`) or a name (`C5`, `F#4`, `Bb3`).
#[must_use]
pub fn parse_key(text: &str) -> Option<u8> {
    if let Ok(number) = text.parse::<u8>() {
        return (number <= 127).then_some(number);
    }
    let mut chars = text.chars();
    let letter = chars.next()?.to_ascii_uppercase();
    let base: i32 = match letter {
        'C' => 0,
        'D' => 2,
        'E' => 4,
        'F' => 5,
        'G' => 7,
        'A' => 9,
        'B' => 11,
        _ => return None,
    };
    let rest = chars.as_str();
    let (accidental, octave) = if let Some(rest) = rest.strip_prefix('#') {
        (1, rest)
    } else if let Some(rest) = rest.strip_prefix('b') {
        (-1, rest)
    } else {
        (0, rest)
    };
    let octave: i32 = octave.parse().ok()?;
    let key = octave * 12 + base + accidental;
    u8::try_from(key).ok().filter(|k| *k <= 127)
}

/// Converts ticks to steps for display.
#[must_use]
pub fn ticks_to_steps(ticks: u32) -> f64 {
    f64::from(ticks) / f64::from(TICKS_PER_STEP)
}

/// Converts ticks to bars for display.
#[must_use]
pub fn ticks_to_bars(ticks: u32) -> f64 {
    f64::from(ticks) / f64::from(TICKS_PER_BAR)
}

/// Rounds `ticks` up to a whole number of bars (at least one).
#[must_use]
pub fn round_up_to_bar(ticks: u32) -> u32 {
    ticks.div_ceil(TICKS_PER_BAR).max(1) * TICKS_PER_BAR
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn key_names_round_trip() {
        for key in 0..=127 {
            assert_eq!(parse_key(&key_name(key)), Some(key));
        }
        assert_eq!(key_name(60), "C5");
        assert_eq!(key_name(69), "A5");
    }

    #[test]
    fn parses_numbers_names_and_flats() {
        assert_eq!(parse_key("60"), Some(60));
        assert_eq!(parse_key("c5"), Some(60));
        assert_eq!(parse_key("Bb4"), Some(58));
        assert_eq!(parse_key("F#4"), Some(54));
        assert_eq!(parse_key("H2"), None);
        assert_eq!(parse_key("128"), None);
        assert_eq!(parse_key("G10"), Some(127));
        assert_eq!(parse_key("G#10"), None);
        assert_eq!(parse_key("Cb0"), None);
    }

    #[test]
    fn bar_rounding() {
        assert_eq!(round_up_to_bar(0), TICKS_PER_BAR);
        assert_eq!(round_up_to_bar(1), TICKS_PER_BAR);
        assert_eq!(round_up_to_bar(TICKS_PER_BAR + 1), 2 * TICKS_PER_BAR);
    }
}
