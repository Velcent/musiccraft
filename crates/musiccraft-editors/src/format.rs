//! Value text shared by hints and captions (`docs/spec/controls.md`, "Value
//! text"): one formatter per kind of value.

/// A linear gain as a level: dB with one decimal and a true minus sign,
/// "−∞ dB" at silence.
#[must_use]
pub fn level(gain: f32) -> String {
    if gain <= 1e-6 {
        return "−∞ dB".to_owned();
    }
    let db = 20.0 * gain.log10();
    let text = format!("{db:+.1} dB");
    // "-0.0" reads as a level change; show it as "0.0".
    if text == "-0.0 dB" || text == "+0.0 dB" {
        return "0.0 dB".to_owned();
    }
    text.replacen('-', "−", 1)
}

/// A pan or balance from −1 (left) to 1 (right): percent with a side word,
/// "Centre" at zero.
#[must_use]
pub fn pan(pan: f32) -> String {
    let percent = (pan * 100.0).round();
    if percent == 0.0 {
        "Centre".to_owned()
    } else if percent < 0.0 {
        format!("{:.0}% left", -percent)
    } else {
        format!("{percent:.0}% right")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn levels_read_in_db_with_a_true_minus() {
        assert_eq!(level(1.0), "0.0 dB");
        assert_eq!(level(0.5), "−6.0 dB");
        assert_eq!(level(1.2), "+1.6 dB");
        assert_eq!(level(0.0), "−∞ dB");
    }

    #[test]
    fn pans_name_their_side() {
        assert_eq!(pan(0.0), "Centre");
        assert_eq!(pan(0.001), "Centre");
        assert_eq!(pan(-0.25), "25% left");
        assert_eq!(pan(1.0), "100% right");
    }
}
