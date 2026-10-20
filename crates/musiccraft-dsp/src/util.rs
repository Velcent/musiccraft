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

/// Below this magnitude a fed-back value is treated as silence (about -300 dB).
const DENORMAL_FLOOR: f32 = 1e-15;

/// Returns 0 for values too small to hear. Applied wherever a value is fed back
/// into filter or delay state, so decaying tails reach exact silence instead of
/// lingering as subnormal floats, which are very slow on many CPUs. Non-finite
/// values pass through unchanged (the engine replaces those).
#[inline]
#[must_use]
pub fn flush_denormal(x: f32) -> f32 {
    if x.abs() < DENORMAL_FLOOR { 0.0 } else { x }
}

/// Replaces subnormal samples with zero, for signals coming in from outside
/// (a mixer track's input, a plugin's input or sidechain): the first
/// operations on subnormal values are slow even where flush-to-zero is on.
/// The test is a bit check, so it is cheap on subnormals too.
pub fn flush_subnormals(samples: &mut [f32]) {
    for s in samples.iter_mut().filter(|s| s.is_subnormal()) {
        *s = 0.0;
    }
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

/// How a channel or mixer pan splits a signal between left and right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PanLaw {
    /// Equal power: -3 dB on each side at centre.
    #[default]
    Circular,
    /// Equal amplitude: -6 dB on each side at centre.
    Triangular,
}

impl PanLaw {
    /// Left and right gains for `pan` in -1..=1 (non-finite values centre).
    /// A hard pan is unity on its side and silent on the other.
    #[must_use]
    pub fn gains(self, pan: f32) -> (f32, f32) {
        let pan = if pan.is_finite() {
            pan.clamp(-1.0, 1.0)
        } else {
            0.0
        };
        match self {
            Self::Circular => {
                let angle = (pan + 1.0) * std::f32::consts::FRAC_PI_4;
                (angle.cos(), angle.sin())
            }
            Self::Triangular => ((1.0 - pan) * 0.5, (1.0 + pan) * 0.5),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pan_laws_meet_their_centre_and_edge_levels() {
        let close = |a: f32, b: f32| (a - b).abs() < 1e-4;
        let (l, r) = PanLaw::Circular.gains(0.0);
        assert!(close(gain_to_db(l), -3.0103) && close(l, r));
        let (l, r) = PanLaw::Triangular.gains(0.0);
        assert!(close(l, 0.5) && close(r, 0.5));
        for law in [PanLaw::Circular, PanLaw::Triangular] {
            let (l, r) = law.gains(-1.0);
            assert!(close(l, 1.0) && r.abs() < 1e-6, "{law:?} hard left");
            let (l, r) = law.gains(1.0);
            assert!(l.abs() < 1e-6 && close(r, 1.0), "{law:?} hard right");
            assert_eq!(law.gains(f32::NAN), law.gains(0.0));
            // Circular keeps the power constant; triangular the amplitude.
            let (l, r) = law.gains(0.3);
            match law {
                PanLaw::Circular => assert!(close(l * l + r * r, 1.0)),
                PanLaw::Triangular => assert!(close(l + r, 1.0)),
            }
        }
    }

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
    fn tiny_values_flush_to_zero() {
        assert_eq!(flush_denormal(1e-20), 0.0);
        assert_eq!(flush_denormal(-1e-20), 0.0);
        assert_eq!(flush_denormal(f32::MIN_POSITIVE / 4.0), 0.0);
        assert_eq!(flush_denormal(1e-6), 1e-6);
        assert_eq!(flush_denormal(-0.5), -0.5);
        assert!(flush_denormal(f32::NAN).is_nan());
    }

    #[test]
    fn pan_law_is_balance() {
        assert_eq!(pan_gains(0.0), (1.0, 1.0));
        assert_eq!(pan_gains(-1.0), (1.0, 0.0));
        assert_eq!(pan_gains(1.0), (0.0, 1.0));
        assert_eq!(pan_gains(f32::NAN), (1.0, 1.0));
    }
}
