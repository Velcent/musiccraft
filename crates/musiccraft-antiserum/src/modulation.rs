//! Envelopes and LFOs, run at the control rate. Values are unipolar (0..1).

/// Bends a 0..1 progress `t`: 0 is linear, positive moves fast at first,
/// negative slowly at first.
#[must_use]
pub fn shape(t: f32, curve: f32) -> f32 {
    let t = t.clamp(0.0, 1.0);
    let k = -curve.clamp(-1.0, 1.0) * 6.0;
    if k.abs() < 1e-3 {
        t
    } else {
        (k * t).exp_m1() / k.exp_m1()
    }
}

/// The mod matrix curve: `s^(e^(−2.5·curve))` for a unipolar `s`.
#[must_use]
pub fn slot_curve(s: f32, curve: f32) -> f32 {
    let s = s.clamp(0.0, 1.0);
    if curve == 0.0 {
        s
    } else {
        s.powf((-2.5 * curve.clamp(-1.0, 1.0)).exp())
    }
}

/// An envelope's settings, in seconds and −1..1 curves.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct EnvParams {
    pub attack: f32,
    pub hold: f32,
    pub decay: f32,
    pub sustain: f32,
    pub release: f32,
    pub attack_curve: f32,
    pub decay_curve: f32,
    pub release_curve: f32,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum Stage {
    #[default]
    Idle,
    Attack,
    Hold,
    Decay,
    Sustain,
    Release,
}

/// An attack-hold-decay-sustain-release envelope with curves.
#[derive(Debug, Clone, Copy, Default)]
pub struct Envelope {
    stage: Stage,
    /// Progress through the stage, 0..1.
    t: f32,
    /// Level the stage started from.
    from: f32,
    level: f32,
}

impl Envelope {
    /// Starts (or restarts, from the current level) the attack.
    pub fn gate_on(&mut self) {
        self.stage = Stage::Attack;
        self.t = 0.0;
        self.from = self.level;
    }

    /// Starts the release from the current level.
    pub fn gate_off(&mut self) {
        if self.stage != Stage::Idle {
            self.stage = Stage::Release;
            self.t = 0.0;
            self.from = self.level;
        }
    }

    /// Silences it at once.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Whether it has finished its release (or never started).
    #[must_use]
    pub fn is_idle(&self) -> bool {
        self.stage == Stage::Idle
    }

    /// The current level.
    #[must_use]
    pub fn level(&self) -> f32 {
        self.level
    }

    /// Advances by `dt` seconds and returns the new level.
    pub fn advance(&mut self, dt: f32, p: &EnvParams) -> f32 {
        let sustain = p.sustain.clamp(0.0, 1.0);
        let mut dt = dt.max(0.0);
        // A block can finish several stages; leftover time carries over.
        for _ in 0..5 {
            let (time, next) = match self.stage {
                Stage::Idle => {
                    self.level = 0.0;
                    return 0.0;
                }
                Stage::Sustain => {
                    self.level = sustain;
                    return sustain;
                }
                Stage::Attack => (p.attack.max(0.0), Stage::Hold),
                Stage::Hold => (p.hold.max(0.0), Stage::Decay),
                Stage::Decay => (p.decay.max(0.0), Stage::Sustain),
                Stage::Release => (p.release.max(0.0), Stage::Idle),
            };
            let remaining = (1.0 - self.t) * time;
            let finished = dt >= remaining;
            if finished {
                dt -= remaining;
                self.t = 1.0;
            } else {
                self.t += dt / time;
            }
            self.level = match self.stage {
                Stage::Attack => self.from + (1.0 - self.from) * shape(self.t, p.attack_curve),
                Stage::Hold => 1.0,
                Stage::Decay => 1.0 + (sustain - 1.0) * shape(self.t, p.decay_curve),
                _ => self.from * (1.0 - shape(self.t, p.release_curve)),
            };
            if !finished {
                return self.level;
            }
            self.stage = next;
            self.t = 0.0;
            self.from = self.level;
        }
        self.level
    }
}

/// Most points in a custom LFO shape.
pub const MAX_POINTS: usize = 32;

/// One point of a drawn LFO shape: position (0..1), value (0..1) and the
/// curve of the segment that leaves it (−1..1).
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LfoPoint {
    pub x: f32,
    pub y: f32,
    pub curve: f32,
}

/// A drawn LFO shape: up to [`MAX_POINTS`] points, sorted by position.
/// Plain `Copy` data, so it can travel to the audio thread in a message.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LfoShape {
    points: [LfoPoint; MAX_POINTS],
    len: usize,
}

impl Default for LfoShape {
    /// A triangle.
    fn default() -> Self {
        Self::new(&[
            LfoPoint {
                x: 0.0,
                y: 0.0,
                curve: 0.0,
            },
            LfoPoint {
                x: 0.5,
                y: 1.0,
                curve: 0.0,
            },
        ])
    }
}

impl LfoShape {
    /// A shape through `points` (clamped, sorted; extra points dropped). The
    /// last point's segment runs back to the first point's value at 1.
    #[must_use]
    pub fn new(points: &[LfoPoint]) -> Self {
        let mut out = [LfoPoint::default(); MAX_POINTS];
        let mut len = 0;
        for (slot, p) in out.iter_mut().zip(points) {
            *slot = LfoPoint {
                x: clamp01(p.x),
                y: clamp01(p.y),
                curve: if p.curve.is_finite() {
                    p.curve.clamp(-1.0, 1.0)
                } else {
                    0.0
                },
            };
            len += 1;
        }
        if len == 0 {
            len = 1;
        }
        out[..len].sort_by(|a, b| a.x.total_cmp(&b.x));
        Self { points: out, len }
    }

    /// The points.
    #[must_use]
    pub fn points(&self) -> &[LfoPoint] {
        self.points.get(..self.len).unwrap_or(&[])
    }

    /// The value at phase `p` (0..1).
    #[must_use]
    pub fn value(&self, p: f32) -> f32 {
        let points = self.points();
        let Some(first) = points.first() else {
            return 0.0;
        };
        let wrap = LfoPoint {
            x: first.x + 1.0,
            ..*first
        };
        let mut p = p.rem_euclid(1.0);
        if p < first.x {
            p += 1.0;
        }
        for (i, a) in points.iter().enumerate() {
            let b = points.get(i + 1).unwrap_or(&wrap);
            if p <= b.x {
                let span = b.x - a.x;
                let t = if span > 0.0 { (p - a.x) / span } else { 1.0 };
                return a.y + (b.y - a.y) * shape(t, a.curve);
            }
        }
        first.y
    }
}

fn clamp01(x: f32) -> f32 {
    if x.is_finite() {
        x.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

/// An LFO's settings for one block.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct LfoParams {
    /// Shape number (see `params::LFO_SHAPES`).
    pub shape: u8,
    /// 0 trigger, 1 envelope, 2 free.
    pub mode: u8,
    /// Cycles per second.
    pub rate: f32,
    pub delay: f32,
    pub rise: f32,
    pub phase: f32,
}

/// One voice's LFO.
#[derive(Debug, Clone, Copy, Default)]
pub struct Lfo {
    phase: f32,
    elapsed: f32,
    done: bool,
    held: f32,
    next: f32,
    seed: u32,
}

impl Lfo {
    /// Restarts it for a new note, seeding its random shapes.
    pub fn start(&mut self, seed: u32) {
        self.phase = 0.0;
        self.elapsed = 0.0;
        self.done = false;
        self.seed = seed | 1;
        self.held = self.random();
        self.next = self.random();
    }

    fn random(&mut self) -> f32 {
        self.seed ^= self.seed << 13;
        self.seed ^= self.seed >> 17;
        self.seed ^= self.seed << 5;
        (self.seed >> 8) as f32 / (1 << 24) as f32
    }

    /// Advances by `dt` seconds and returns the value. `free_phase` is the
    /// synth's shared phase for free-running LFOs.
    pub fn advance(&mut self, dt: f32, p: &LfoParams, free_phase: f32, custom: &LfoShape) -> f32 {
        self.elapsed += dt;
        let phase = if p.mode == 2 {
            let previous = self.phase;
            self.phase = free_phase;
            if free_phase < previous {
                self.new_cycle();
            }
            free_phase
        } else if self.done {
            1.0
        } else {
            let next = self.phase + p.rate.max(0.0) * dt;
            if next >= 1.0 {
                if p.mode == 1 {
                    self.done = true;
                    self.phase = 1.0;
                } else {
                    self.phase = next.fract();
                    self.new_cycle();
                }
            } else {
                self.phase = next;
            }
            self.phase
        };
        let p_shifted = (phase + p.phase).rem_euclid(1.0);
        let p_shifted = if self.done && p.mode == 1 {
            // Hold the cycle's end value.
            (1.0 + p.phase - 1e-6).rem_euclid(1.0)
        } else {
            p_shifted
        };
        let value = match p.shape {
            0 => 0.5 - 0.5 * (p_shifted * std::f32::consts::TAU).cos(),
            1 => 1.0 - (2.0 * p_shifted - 1.0).abs(),
            2 => p_shifted,
            3 => 1.0 - p_shifted,
            4 => {
                if p_shifted < 0.5 {
                    1.0
                } else {
                    0.0
                }
            }
            5 => self.held,
            6 => {
                let t = p_shifted;
                let s = t * t * (3.0 - 2.0 * t);
                self.held + (self.next - self.held) * s
            }
            _ => custom.value(p_shifted),
        };
        let fade = if self.elapsed < p.delay {
            0.0
        } else if p.rise > 0.0 {
            ((self.elapsed - p.delay) / p.rise).min(1.0)
        } else {
            1.0
        };
        value * fade
    }

    fn new_cycle(&mut self) {
        self.held = self.next;
        self.next = self.random();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn params() -> EnvParams {
        EnvParams {
            attack: 0.1,
            hold: 0.1,
            decay: 0.1,
            sustain: 0.5,
            release: 0.1,
            ..EnvParams::default()
        }
    }

    #[test]
    fn envelope_runs_its_stages() {
        let mut env = Envelope::default();
        env.gate_on();
        assert!((env.advance(0.05, &params()) - 0.5).abs() < 1e-5);
        assert!((env.advance(0.05, &params()) - 1.0).abs() < 1e-5);
        assert_eq!(env.advance(0.05, &params()), 1.0, "hold");
        env.advance(0.05, &params());
        env.advance(0.1, &params());
        assert!((env.advance(0.01, &params()) - 0.5).abs() < 1e-5, "sustain");
        env.gate_off();
        assert!((env.advance(0.05, &params()) - 0.25).abs() < 1e-5);
        env.advance(0.06, &params());
        assert!(env.is_idle());
    }

    #[test]
    fn zero_times_jump_and_curves_bend() {
        let mut env = Envelope::default();
        env.gate_on();
        let p = EnvParams {
            sustain: 0.3,
            ..EnvParams::default()
        };
        assert!((env.advance(0.001, &p) - 0.3).abs() < 1e-6);
        assert!(shape(0.5, 1.0) > 0.9);
        assert!(shape(0.5, -1.0) < 0.1);
        assert_eq!(shape(0.5, 0.0), 0.5);
        assert!(slot_curve(0.5, 1.0) > 0.9 && slot_curve(0.5, -1.0) < 0.01);
    }

    #[test]
    fn lfo_shapes_delay_and_envelope_mode() {
        let shape = LfoShape::default();
        assert!((shape.value(0.5) - 1.0).abs() < 1e-6);
        assert!((shape.value(0.75) - 0.5).abs() < 1e-6);
        let mut lfo = Lfo::default();
        lfo.start(7);
        let p = LfoParams {
            shape: 2,
            rate: 1.0,
            ..LfoParams::default()
        };
        assert!((lfo.advance(0.25, &p, 0.0, &shape) - 0.25).abs() < 1e-6);
        let delayed = LfoParams { delay: 1.0, ..p };
        let mut lfo = Lfo::default();
        lfo.start(7);
        assert_eq!(lfo.advance(0.5, &delayed, 0.0, &shape), 0.0);
        let once = LfoParams { mode: 1, ..p };
        let mut lfo = Lfo::default();
        lfo.start(7);
        lfo.advance(1.5, &once, 0.0, &shape);
        assert!(
            lfo.advance(0.25, &once, 0.0, &shape) > 0.99,
            "holds the end"
        );
    }
}
