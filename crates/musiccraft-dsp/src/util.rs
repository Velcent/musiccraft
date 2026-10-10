/// Converts decibels to linear gain.
#[must_use]
pub fn db_to_gain(db: f32) -> f32 {
    10.0_f32.powf(db / 20.0)
}

/// Converts linear gain to decibels. Silence maps to -144 dB.
#[must_use]
pub fn gain_to_db(gain: f32) -> f32 {
    if gain <= 1e-7 {
        -144.0
    } else {
        20.0 * gain.log10()
    }
}

/// Frequency in hertz of a (possibly fractional) MIDI key, with key 69 = 440 Hz.
#[must_use]
pub fn key_to_hz(key: f32) -> f32 {
    440.0 * 2.0_f32.powf((key - 69.0) / 12.0)
}

/// Left and right gains for a balance pan in -1..=1. Centre is unity on both sides.
#[must_use]
pub fn pan_gains(pan: f32) -> (f32, f32) {
    let pan = if pan.is_finite() {
        pan.clamp(-1.0, 1.0)
    } else {
        0.0
    };
    ((1.0 - pan).min(1.0), (1.0 + pan).min(1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn db_round_trip() {
        assert!((db_to_gain(0.0) - 1.0).abs() < 1e-6);
        assert!((db_to_gain(-6.0206) - 0.5).abs() < 1e-4);
        assert!((gain_to_db(0.5) + 6.0206).abs() < 1e-3);
        assert!(gain_to_db(0.0) <= -144.0);
    }

    #[test]
    fn a4_is_440() {
        assert!((key_to_hz(69.0) - 440.0).abs() < 1e-3);
        assert!((key_to_hz(57.0) - 220.0).abs() < 1e-3);
    }

    #[test]
    fn pan_law_is_balance() {
        assert_eq!(pan_gains(0.0), (1.0, 1.0));
        assert_eq!(pan_gains(-1.0), (1.0, 0.0));
        assert_eq!(pan_gains(1.0), (0.0, 1.0));
        assert_eq!(pan_gains(f32::NAN), (1.0, 1.0));
    }
}
