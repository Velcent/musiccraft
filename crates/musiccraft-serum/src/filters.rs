//! Serum filter types onto Antiserum's (`FILTER_TYPES`). The Serum 1 names
//! are the menu as serum2vital read it; the Serum 2 names follow the patterns
//! serum2vital's `filter_map.py` decodes. Which Antiserum type stands in is
//! our own choice.

/// Antiserum's filter types, by index.
const LP12: usize = 0;
const LP24: usize = 1;
const HP12: usize = 2;
const HP24: usize = 3;
const BP12: usize = 4;
const BP24: usize = 5;
const NOTCH12: usize = 6;
const NOTCH24: usize = 7;
const PEAK12: usize = 8;
const PEAK24: usize = 9;
const LADDER: usize = 10;
const DIODE: usize = 11;
const COMB_PLUS: usize = 12;
const COMB_MINUS: usize = 13;
const FORMANT: usize = 14;
const PHASER: usize = 16;
const FLANGER: usize = 17;

/// The stand-in for one filter family letter at a slope, and whether the
/// slope matches.
fn family(letter: char, slope: u32) -> Option<(usize, bool)> {
    let (twelve, twenty_four) = match letter {
        'L' => (LP12, LP24),
        'H' => (HP12, HP24),
        'B' => (BP12, BP24),
        'P' => (PEAK12, PEAK24),
        'N' => (NOTCH12, NOTCH24),
        _ => return None,
    };
    Some(match slope {
        12 => (twelve, true),
        24 => (twenty_four, true),
        s if s < 12 => (twelve, false),
        s if s < 24 => (twenty_four, false),
        _ => (twenty_four, false),
    })
}

/// Antiserum's type for a Serum 1 filter menu name: (type, exact).
/// `None`: no counterpart (the filter is turned off).
#[must_use]
pub fn serum1(name: &str) -> Option<(usize, bool)> {
    let slope = |s: &str| s.trim().parse::<u32>().ok();
    if name.contains(" EQ ") {
        // Shelves and an EQ band: a peak at the cutoff is closest.
        return Some((PEAK12, false));
    }
    if let Some(rest) = name.strip_prefix("MG Low ") {
        let s = slope(rest)?;
        return Some(match s {
            12 => (LP12, true),
            24 => (LADDER, true),
            6 => (LP12, false),
            _ => (LADDER, false),
        });
    }
    for (word, letter) in [
        ("Low ", 'L'),
        ("High ", 'H'),
        ("Band ", 'B'),
        ("Peak ", 'P'),
        ("Notch ", 'N'),
    ] {
        if let Some(rest) = name.strip_prefix(word) {
            return family(letter, slope(rest)?);
        }
    }
    let first = name.chars().next()?;
    // Dual responses ("LB 12", "HN 12") and morphing ones ("L/B/H 24"): the
    // first response stands in.
    if let Some((letters, rest)) = name.split_once(' ')
        && letters.chars().all(|c| "LHBPN/".contains(c))
        && let Some(s) = slope(rest)
    {
        return family(first, s).map(|(t, _)| (t, false));
    }
    Some(match name {
        "Cmb +" => (COMB_PLUS, true),
        "Cmb -" => (COMB_MINUS, true),
        n if n.starts_with("Cmb") => (
            if n.ends_with('-') {
                COMB_MINUS
            } else {
                COMB_PLUS
            },
            false,
        ),
        n if n.starts_with("Flg") => (FLANGER, n == "Flg +"),
        n if n.starts_with("Phs") || n.starts_with("FPhs") => (PHASER, false),
        "French LP" => (LP24, false),
        "German LP" => (LADDER, false),
        "Formant-I" => (FORMANT, true),
        "Formant-II" | "Formant-III" => (FORMANT, false),
        "Bandreject" => (NOTCH24, false),
        n if n.starts_with("Dist.Comb") => (COMB_PLUS, false),
        "Scream LP" => (DIODE, false),
        "Scream BP" => (BP24, false),
        _ => return None,
    })
}

/// Antiserum's type for a Serum 2 filter type name (`MgL24`, `LB12`,
/// `CombHL6P`, …), as [`serum1`].
#[must_use]
pub fn serum2(name: &str) -> Option<(usize, bool)> {
    let raw = name.trim();
    let digits = |s: &str| s.parse::<u32>().ok();
    if let Some(s) = raw
        .strip_prefix("MgL")
        .or_else(|| raw.strip_prefix("mgl"))
        .and_then(digits)
    {
        return serum1(&format!("MG Low {s}"));
    }
    let split = raw.find(|c: char| c.is_ascii_digit()).unwrap_or(raw.len());
    let (letters, number) = raw.split_at(split);
    if let Some(slope) = digits(number) {
        let mut chars = letters.chars();
        let first = chars.next();
        let all_family = !letters.is_empty() && letters.chars().all(|c| "LHBPN".contains(c));
        if all_family && let Some(first) = first {
            // One letter: a plain response; more: dual or morphing.
            return family(first, slope).map(|(t, exact)| (t, exact && letters.len() == 1));
        }
        if let Some(kind) = letters.strip_suffix("EQ") {
            return ["L", "B", "H"].contains(&kind).then_some((PEAK12, false));
        }
    }
    if let Some(rest) = raw.strip_prefix("Comb") {
        return Some(match rest {
            "P" => (COMB_PLUS, true),
            "N" => (COMB_MINUS, true),
            r if r.ends_with('N') => (COMB_MINUS, false),
            _ => (COMB_PLUS, false),
        });
    }
    if raw.starts_with("FlangePhase") || raw.starts_with("Phase") {
        return Some((PHASER, false));
    }
    if let Some(rest) = raw.strip_prefix("Flange") {
        return Some((FLANGER, rest == "P"));
    }
    if raw.starts_with("DistComb") {
        return Some((COMB_PLUS, false));
    }
    if let Some(rest) = raw.strip_prefix("Formant") {
        let exact = matches!(rest.to_lowercase().as_str(), "one" | "i" | "1");
        return Some((FORMANT, exact));
    }
    let key = raw.to_lowercase().replace([' ', '-'], "");
    let special = match key.as_str() {
        "bandreject" => "Bandreject",
        "french" | "frenchlp" => "French LP",
        "german" | "germanlp" => "German LP",
        "scream" | "screamlp" | "scream3lp" => "Scream LP",
        "screambp" => "Scream BP",
        "comb2" => "Cmb -",
        "combp" => "Cmb +",
        "combn" => "Cmb -",
        "flangep" => "Flg +",
        "flangen" => "Flg -",
        "ladderems" | "ladderacid" | "laddermg" | "dirtymg" => return Some((LADDER, false)),
        "wsp" => return Some((DIODE, false)),
        "exp" => return Some((LP24, false)),
        "expbpf" => return Some((BP24, false)),
        "djmixer" | "pz_svf" | "pzsvf" => return Some((LP12, false)),
        // Ring modulation, sample and hold, reverbs, all-passes, add-bass.
        _ => return None,
    };
    serum1(special)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menus::FILTER_NAMES;

    #[test]
    fn every_serum1_type_is_decided() {
        let none: Vec<&str> = FILTER_NAMES
            .iter()
            .copied()
            .filter(|n| serum1(n).is_none())
            .collect();
        assert_eq!(
            none,
            [
                "Ring Mod",
                "Ring Modx2",
                "SampHold",
                "SampHold-",
                "Combs",
                "Allpasses",
                "Reverb",
                "Add Bass"
            ]
        );
        assert_eq!(serum1("MG Low 12"), Some((LP12, true)));
        assert_eq!(serum1("High 24"), Some((HP24, true)));
        assert_eq!(serum1("LB 12"), Some((LP12, false)));
        assert_eq!(serum1("B/P/N 24"), Some((BP24, false)));
        assert_eq!(serum1("Cmb H6-"), Some((COMB_MINUS, false)));
    }

    #[test]
    fn serum2_names_decode() {
        assert_eq!(serum2("MgL12"), Some((LP12, true)));
        assert_eq!(serum2("MgL24"), Some((LADDER, true)));
        assert_eq!(serum2("L24"), Some((LP24, true)));
        assert_eq!(serum2("N12"), Some((NOTCH12, true)));
        assert_eq!(serum2("HB12"), Some((HP12, false)));
        assert_eq!(serum2("LBH24"), Some((LP24, false)));
        assert_eq!(serum2("CombP"), Some((COMB_PLUS, true)));
        assert_eq!(serum2("CombHL6N"), Some((COMB_MINUS, false)));
        assert_eq!(serum2("Phase48L6P"), Some((PHASER, false)));
        assert_eq!(serum2("BEQ12"), Some((PEAK12, false)));
        assert_eq!(serum2("FormantII"), Some((FORMANT, false)));
        assert_eq!(serum2("GermanLP"), Some((LADDER, false)));
        assert_eq!(serum2("RingMod"), None);
    }
}
