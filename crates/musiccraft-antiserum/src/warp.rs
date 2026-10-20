//! Oscillator warps: reshape the read phase (or the level) of a wavetable
//! oscillator. Pure functions, run per sample.

/// A warp mode, by `oscN_warpM` value (see `params::WARPS`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Warp {
    #[default]
    Off,
    Sync,
    BendPlus,
    BendMinus,
    Pwm,
    AsymPlus,
    AsymMinus,
    Mirror,
    Flip,
    Quantize,
    Remap,
    Fm,
    Am,
    Rm,
}

const ALL: [Warp; 14] = [
    Warp::Off,
    Warp::Sync,
    Warp::BendPlus,
    Warp::BendMinus,
    Warp::Pwm,
    Warp::AsymPlus,
    Warp::AsymMinus,
    Warp::Mirror,
    Warp::Flip,
    Warp::Quantize,
    Warp::Remap,
    Warp::Fm,
    Warp::Am,
    Warp::Rm,
];

impl Warp {
    /// The mode for a parameter value.
    #[must_use]
    pub fn from_param(value: f32) -> Self {
        ALL.get(value.max(0.0).round() as usize)
            .copied()
            .unwrap_or_default()
    }

    /// Whether it reads the oscillator's mod source.
    #[must_use]
    pub fn uses_source(self) -> bool {
        matches!(self, Self::Fm | Self::Am | Self::Rm)
    }

    /// How much faster than the oscillator's pitch the warped wave moves at
    /// most, so the oscillator picks a band-limited level with fewer
    /// harmonics.
    #[must_use]
    pub fn speed(self, amount: f32) -> f32 {
        let a = amount.clamp(0.0, 1.0);
        match self {
            Self::Sync => 1.0 + a * 15.0,
            Self::BendPlus | Self::BendMinus => 1.0 + a * 3.0,
            Self::Pwm => 1.0 / (1.0 - a * 0.98),
            Self::AsymPlus | Self::AsymMinus => 0.5 / (0.5 - a * 0.49),
            Self::Mirror => 1.0 + a,
            Self::Remap => 1.0 + a * 0.5,
            Self::Fm => 1.0 + a * 4.0,
            _ => 1.0,
        }
    }

    /// Warps `phase` (0..1) by `amount` (0..1), with `source` (−1..1) for the
    /// cross-modulating modes. Returns the phase to read and a gain.
    #[must_use]
    pub fn apply(self, amount: f32, phase: f32, source: f32) -> (f32, f32) {
        let a = amount.clamp(0.0, 1.0);
        let p = phase;
        match self {
            Self::Off => (p, 1.0),
            Self::Sync => ((p * (1.0 + a * 15.0)).fract(), 1.0),
            Self::BendPlus => (bend(p, a), 1.0),
            Self::BendMinus => (1.0 - bend(1.0 - p, a), 1.0),
            Self::Pwm => {
                let w = 1.0 - a * 0.98;
                if p < w { (p / w, 1.0) } else { (0.0, 0.0) }
            }
            Self::AsymPlus => (asym(p, 0.5 + a * 0.49), 1.0),
            Self::AsymMinus => (asym(p, 0.5 - a * 0.49), 1.0),
            Self::Mirror => {
                let mirrored = if p < 0.5 { 2.0 * p } else { 2.0 - 2.0 * p };
                (p + (mirrored - p) * a, 1.0)
            }
            Self::Flip => (p, if p > 1.0 - a { -1.0 } else { 1.0 }),
            Self::Quantize => {
                let steps = 2.0 + (1.0 - a) * (1.0 - a) * 254.0;
                ((p * steps).floor() / steps, 1.0)
            }
            Self::Remap => {
                let s = p * p * (3.0 - 2.0 * p);
                (p + (s - p) * a, 1.0)
            }
            Self::Fm => ((p + a * 2.0 * source).rem_euclid(1.0), 1.0),
            Self::Am => (p, 1.0 - a + a * (source * 0.5 + 0.5)),
            Self::Rm => (p, 1.0 - a + a * source),
        }
    }
}

/// Pushes the phase later: `p(1−k)/(1−kp)` with `k = 0.75·a`, so the end
/// moves up to 4 times faster (cheaper than a power curve).
fn bend(p: f32, a: f32) -> f32 {
    let k = 0.75 * a;
    p * (1.0 - k) / (1.0 - k * p)
}

/// Moves the cycle's midpoint to `m`.
fn asym(p: f32, m: f32) -> f32 {
    if p < m {
        p * 0.5 / m
    } else {
        0.5 + (p - m) * 0.5 / (1.0 - m)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_amount_leaves_the_phase_alone() {
        for warp in ALL {
            for p in [0.0, 0.2, 0.5, 0.9] {
                let (q, gain) = warp.apply(0.0, p, 0.7);
                assert!((q - p).abs() < 1e-6 || warp == Warp::Quantize, "{warp:?}");
                assert!((gain - 1.0).abs() < 1e-6, "{warp:?}");
            }
        }
    }

    #[test]
    fn warps_shape_the_phase() {
        assert!((Warp::Sync.apply(1.0, 0.5, 0.0).0 - 0.0).abs() < 1e-6);
        assert!((Warp::BendPlus.apply(1.0, 0.5, 0.0).0 - 0.2).abs() < 1e-6);
        assert!((Warp::BendMinus.apply(1.0, 0.5, 0.0).0 - 0.8).abs() < 1e-6);
        assert!((Warp::BendPlus.apply(1.0, 1.0, 0.0).0 - 1.0).abs() < 1e-6);
        assert_eq!(Warp::Pwm.apply(0.5, 0.8, 0.0), (0.0, 0.0));
        assert!((Warp::AsymPlus.apply(1.0, 0.99, 0.0).0 - 0.5).abs() < 1e-3);
        assert_eq!(Warp::Mirror.apply(1.0, 0.75, 0.0).0, 0.5);
        assert_eq!(Warp::Flip.apply(0.5, 0.75, 0.0).1, -1.0);
        assert_eq!(Warp::Quantize.apply(1.0, 0.7, 0.0).0, 0.5);
        assert!((Warp::Fm.apply(0.5, 0.25, 0.5).0 - 0.75).abs() < 1e-6);
        assert_eq!(Warp::Rm.apply(1.0, 0.3, -0.5).1, -0.5);
        assert_eq!(Warp::Am.apply(1.0, 0.3, -1.0).1, 0.0);
        assert_eq!(Warp::from_param(13.0), Warp::Rm);
        assert_eq!(Warp::from_param(99.0), Warp::Off);
    }
}
