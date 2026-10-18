//! Controllers on the audio thread: each block every lane computes a value
//! from its source and writes its target (see `docs/spec/controllers.md`).
//! Lanes are built on the main thread and travel inside the [`Sequence`];
//! their running state lives in the engine's fixed-size array.

use musiccraft_formula::Formula;
use musiccraft_params::{ParamSpec, volume_from_unit};

use crate::graph::Graph;
use crate::{AutomationTarget, Sequence};
use musiccraft_instruments::Voice;

/// Most controllers the engine runs; later ones are ignored.
pub const MAX_CONTROLLERS: usize = 64;

/// How a control position (0..1) becomes the target's real units.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Convert {
    /// A volume fader with this maximum gain.
    Volume(f32),
    /// Pan, −1..1.
    Pan,
    /// A parameter table entry.
    Spec(ParamSpec),
}

impl Convert {
    /// The real value for a control position.
    #[must_use]
    pub fn apply(self, position: f32) -> f32 {
        match self {
            Self::Volume(max) => volume_from_unit(position, max),
            Self::Pan => position * 2.0 - 1.0,
            Self::Spec(spec) => spec.from_unit(position),
        }
    }
}

/// Where a controller's value comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
// Lanes are plain `Copy` data in a vector built on the main thread; boxing
// the compiled formula would put an allocation behind every lane.
#[allow(clippy::large_enum_variant)]
pub enum ControllerSource {
    /// Shape (sine, triangle, saw, square, random), beats per cycle, phase.
    Lfo { shape: u8, period: f32, phase: f32 },
    /// Triggered by the channel's notes; times in seconds.
    Envelope {
        channel: u16,
        attack: f32,
        decay: f32,
        sustain: f32,
        release: f32,
    },
    /// The channel's highest held key mapped from `low` to `high`.
    Keyboard { channel: u16, low: u8, high: u8 },
    /// The track's last peak times `gain`; times in milliseconds.
    Peak {
        track: u16,
        gain: f32,
        attack: f32,
        release: f32,
    },
    /// An expression of the song position.
    Formula(Formula),
    /// A fixed value.
    Macro(f32),
}

/// One controller as the engine runs it.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ControllerLane {
    /// What it writes.
    pub target: AutomationTarget,
    /// Control position to real units.
    pub convert: Convert,
    /// Control positions for outputs 0 and 1.
    pub min: f32,
    pub max: f32,
    /// The source.
    pub source: ControllerSource,
}

/// A controller's running state.
#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ControllerState {
    level: f32,
    /// 0 idle, 1 attack, 2 decay and sustain, 3 release.
    stage: u8,
    gate: bool,
    value: f32,
    random: f32,
    cycle: i64,
    seed: u32,
}

/// What a block of controllers needs to know.
pub(crate) struct ControlContext<'a> {
    /// Beat position: the song while playing, a free clock while stopped.
    pub beats: f64,
    /// Seconds this block lasts.
    pub seconds: f32,
    pub voices: &'a [Voice],
}

impl ControllerState {
    /// The controller's value for this block, 0..1.
    fn update(&mut self, source: &ControllerSource, cx: &ControlContext<'_>, peaks: &[f32]) -> f32 {
        let value = match *source {
            ControllerSource::Lfo {
                shape,
                period,
                phase,
            } => {
                let cycles = cx.beats / f64::from(period.max(1e-3)) + f64::from(phase);
                let p = cycles.rem_euclid(1.0) as f32;
                let cycle = cycles.floor() as i64;
                if cycle != self.cycle {
                    self.cycle = cycle;
                    if self.seed == 0 {
                        self.seed = 0x2545_f491;
                    }
                    self.seed ^= self.seed << 13;
                    self.seed ^= self.seed >> 17;
                    self.seed ^= self.seed << 5;
                    self.random = self.seed as f32 / u32::MAX as f32;
                }
                let bipolar = match shape {
                    1 => 1.0 - 4.0 * (p - 0.5).abs(),
                    2 => p * 2.0 - 1.0,
                    3 => {
                        if p < 0.5 {
                            1.0
                        } else {
                            -1.0
                        }
                    }
                    4 => self.random * 2.0 - 1.0,
                    _ => (p * std::f32::consts::TAU).sin(),
                };
                0.5 + 0.5 * bipolar
            }
            ControllerSource::Envelope {
                channel,
                attack,
                decay,
                sustain,
                release,
            } => {
                let held = cx.voices.iter().any(|v| v.held() && v.channel == channel);
                if held && !self.gate {
                    self.stage = 1;
                } else if !held && self.gate {
                    self.stage = 3;
                }
                self.gate = held;
                let rate = |time: f32| cx.seconds / time.max(1e-3);
                match self.stage {
                    1 => {
                        self.level += rate(attack);
                        if self.level >= 1.0 {
                            self.level = 1.0;
                            self.stage = 2;
                        }
                    }
                    2 => {
                        let s = sustain.clamp(0.0, 1.0);
                        self.level = (self.level - rate(decay) * (1.0 - s)).max(s);
                    }
                    3 => {
                        self.level -= rate(release);
                        if self.level <= 0.0 {
                            self.level = 0.0;
                            self.stage = 0;
                        }
                    }
                    _ => {}
                }
                self.level
            }
            ControllerSource::Keyboard { channel, low, high } => {
                let key = cx
                    .voices
                    .iter()
                    .filter(|v| v.held() && v.channel == channel)
                    .map(|v| v.key)
                    .max();
                if let Some(key) = key {
                    let span = (f32::from(high) - f32::from(low)).abs().max(1.0);
                    let t = (f32::from(key) - f32::from(low.min(high))) / span;
                    self.value = if high >= low { t } else { 1.0 - t }.clamp(0.0, 1.0);
                }
                self.value
            }
            ControllerSource::Peak {
                track,
                gain,
                attack,
                release,
            } => {
                let target =
                    (peaks.get(usize::from(track)).copied().unwrap_or(0.0) * gain).min(1.0);
                let time = if target > self.level { attack } else { release };
                let coeff = 1.0 - (-cx.seconds / (time.max(0.01) * 0.001)).exp();
                self.level += (target - self.level) * coeff;
                self.level
            }
            ControllerSource::Formula(formula) => formula.eval(cx.beats),
            ControllerSource::Macro(value) => value,
        };
        if value.is_finite() {
            value.clamp(0.0, 1.0)
        } else {
            0.0
        }
    }
}

/// Runs every controller of `sequence` and writes its target in `graph`.
pub(crate) fn apply_controllers(
    graph: &mut Graph,
    sequence: &Sequence,
    states: &mut [ControllerState; MAX_CONTROLLERS],
    cx: &ControlContext<'_>,
) {
    // Peaks from the previous block, per track.
    let mut peaks = [0.0_f32; crate::graph::MAX_PEAK_TRACKS];
    for (peak, track) in peaks.iter_mut().zip(&graph.tracks) {
        *peak = track.last_peak;
    }
    for (lane, state) in sequence.controllers().iter().zip(states.iter_mut()) {
        let value = state.update(&lane.source, cx, &peaks);
        let position = lane.min + (lane.max - lane.min) * value;
        crate::engine::write_target(graph, lane.target, lane.convert.apply(position));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cx(beats: f64) -> ControlContext<'static> {
        ControlContext {
            beats,
            seconds: 0.005,
            voices: &[],
        }
    }

    #[test]
    fn lfo_follows_the_beat() {
        let mut state = ControllerState::default();
        let lfo = ControllerSource::Lfo {
            shape: 2,
            period: 4.0,
            phase: 0.0,
        };
        assert!((state.update(&lfo, &cx(0.0), &[]) - 0.0).abs() < 1e-6);
        assert!((state.update(&lfo, &cx(2.0), &[]) - 0.5).abs() < 1e-6);
        assert!((state.update(&lfo, &cx(3.0), &[]) - 0.75).abs() < 1e-6);
    }

    #[test]
    fn peak_follows_its_track() {
        let mut state = ControllerState::default();
        let peak = ControllerSource::Peak {
            track: 1,
            gain: 2.0,
            attack: 0.1,
            release: 100.0,
        };
        let v = state.update(&peak, &cx(0.0), &[0.0, 0.3]);
        assert!((v - 0.6).abs() < 1e-3, "{v}");
        let falling = state.update(&peak, &cx(0.0), &[0.0, 0.0]);
        assert!(falling > 0.5 && falling < 0.6, "releases slowly: {falling}");
    }

    #[test]
    fn formula_and_macro_are_clamped() {
        let mut state = ControllerState::default();
        let formula = ControllerSource::Formula(Formula::parse("t - 1").unwrap_or_default());
        assert_eq!(state.update(&formula, &cx(0.0), &[]), 0.0);
        assert!((state.update(&formula, &cx(1.25), &[]) - 0.25).abs() < 1e-6);
        assert_eq!(state.update(&formula, &cx(9.0), &[]), 1.0);
        assert_eq!(
            state.update(&ControllerSource::Macro(0.4), &cx(0.0), &[]),
            0.4
        );
    }
}
