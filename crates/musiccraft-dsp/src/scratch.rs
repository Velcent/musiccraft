//! Scratch: freezes the last stretch of audio like a record under a hand and
//! moves the play point through it, either from the `position` control
//! (automate it to scratch by hand) or with a tempo-synced pattern. All
//! allocation happens in `new`.

use std::f32::consts::TAU;

use crate::modulation::SYNC_RATES;
use crate::{ProcessContext, flush_denormal};

fn param(p: &[f32], i: usize, default: f32) -> f32 {
    p.get(i).copied().unwrap_or(default)
}

/// Names of the scratch patterns, by `pattern` value.
pub const SCRATCH_PATTERNS: [&str; 5] = ["manual", "baby", "chirp", "tear", "transform"];
/// Longest stretch that can be held, in seconds.
const MAX_SECONDS: f32 = 8.0;

/// Where the record is (0..1 of the held span) and how loud, for a pattern
/// at `phase` (0..1 through a cycle).
#[must_use]
pub fn scratch_pattern(pattern: f32, phase: f32) -> (f32, f32) {
    let phase = phase.rem_euclid(1.0);
    let baby = 0.5 - 0.5 * (phase * TAU).cos();
    match pattern.round() as i32 {
        1 => (baby, 1.0),
        // Forward stroke heard, the return cut.
        2 => (baby, if phase < 0.5 { 1.0 } else { 0.0 }),
        // The forward stroke in three jerks, a smooth return.
        3 => {
            if phase < 0.5 {
                let t = phase * 6.0;
                let step = t.floor();
                let f = t - step;
                ((step + f * f * (3.0 - 2.0 * f)) / 3.0, 1.0)
            } else {
                (baby, 1.0)
            }
        }
        // A baby scratch chopped on and off eight times per cycle.
        4 => (
            baby,
            if (phase * 16.0) as u32 % 2 == 0 {
                1.0
            } else {
                0.0
            },
        ),
        _ => (0.0, 1.0),
    }
}

/// Scratch effect.
///
/// Parameters: `[hold, position, span beats, pattern, rate, depth, inertia
/// ms, mix]`. While `hold` is off audio passes and is recorded; turning it on
/// freezes the last `span` beats, played from `position` (manual pattern) or
/// moved by the pattern, `depth` scaling how far.
#[derive(Debug, Clone)]
pub struct Scratch {
    lines: [Vec<f32>; 2],
    write: usize,
    holding: bool,
    /// Smoothed play point, samples behind the write point at the freeze.
    platter: f32,
    gain: f32,
}

impl Scratch {
    /// Allocates the record buffer. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (MAX_SECONDS * sample_rate.max(1.0)) as usize + 4;
        Self {
            lines: [vec![0.0; len], vec![0.0; len]],
            write: 0,
            holding: false,
            platter: 0.0,
            gain: 1.0,
        }
    }

    /// Clears the record.
    pub fn reset(&mut self) {
        for line in &mut self.lines {
            line.fill(0.0);
        }
        self.holding = false;
        self.platter = 0.0;
        self.gain = 1.0;
    }

    fn read(&self, ch: usize, back: f32) -> f32 {
        let Some(line) = self.lines.get(ch) else {
            return 0.0;
        };
        let len = line.len();
        let back = back.clamp(1.0, (len - 3) as f32);
        let whole = back as usize;
        let frac = back - whole as f32;
        let at = |b: usize| {
            line.get((self.write + len - b) % len)
                .copied()
                .unwrap_or(0.0)
        };
        let a = at(whole);
        a + (at(whole + 1) - a) * frac
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let len = self.lines[0].len();
        if len < 8 {
            return;
        }
        let sr = ctx.sample_rate.max(1.0);
        let hold = param(p, 0, 0.0) >= 0.5;
        let position = param(p, 1, 0.0).clamp(0.0, 1.0);
        let samples_per_beat = 60.0 / ctx.tempo_bpm.max(1.0) as f32 * sr;
        let span = (param(p, 2, 1.0).clamp(0.0625, 8.0) * samples_per_beat).min((len - 4) as f32);
        let pattern = param(p, 3, 0.0);
        let rate = SYNC_RATES
            .get(param(p, 4, 3.0).round().max(0.0) as usize)
            .copied()
            .unwrap_or(1.0);
        let depth = param(p, 5, 1.0).clamp(0.0, 1.0);
        let inertia = 1.0 - (-1.0 / (param(p, 6, 5.0).max(0.1) * 0.001 * sr)).exp();
        let mix = param(p, 7, 1.0).clamp(0.0, 1.0);
        let smooth = 1.0 - (-1.0 / (0.001 * sr)).exp();
        let beats_per_sample = ctx.tempo_bpm / 60.0 / f64::from(sr);
        let mut beats = ctx.beats;
        if hold && !self.holding {
            // Drop the needle at the start of the held span.
            self.platter = span;
        }
        self.holding = hold;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if !hold {
                for (line, x) in self.lines.iter_mut().zip([*l, *r]) {
                    if let Some(slot) = line.get_mut(self.write) {
                        *slot = if x.is_finite() {
                            flush_denormal(x)
                        } else {
                            0.0
                        };
                    }
                }
                self.write = (self.write + 1) % len;
                beats += beats_per_sample;
                continue;
            }
            let (place, level) = if pattern.round() as i32 == 0 {
                (position, 1.0)
            } else {
                let (at, level) = scratch_pattern(pattern, (beats / rate).fract() as f32);
                ((position + at * depth).min(1.0), level)
            };
            // `place` 0 is the start of the held span, 1 its end (the freeze).
            let target = span * (1.0 - place);
            self.platter += (target - self.platter) * inertia;
            self.gain += (level - self.gain) * smooth;
            let wet = [self.read(0, self.platter), self.read(1, self.platter)];
            *l += (wet[0] * self.gain - *l) * mix;
            *r += (wet[1] * self.gain - *r) * mix;
            beats += beats_per_sample;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_params::EffectKind;

    /// 120 BPM at 1 kHz: 500 samples a beat.
    fn ctx() -> ProcessContext {
        ProcessContext {
            sample_rate: 1_000.0,
            tempo_bpm: 120.0,
            beats: 0.0,
        }
    }

    #[test]
    fn passes_audio_until_held() {
        let mut fx = Scratch::new(1_000.0);
        let input: Vec<f32> = (0..1_000).map(|i| i as f32).collect();
        let (mut l, mut r) = (input.clone(), input.clone());
        fx.process(&mut l, &mut r, &EffectKind::Scratch.defaults(), ctx());
        assert_eq!(l, input);
    }

    #[test]
    fn manual_position_plays_from_the_held_span() {
        let mut fx = Scratch::new(1_000.0);
        let mut p = EffectKind::Scratch.defaults();
        p[6] = 0.1; // almost no inertia
        let ramp: Vec<f32> = (0..1_000).map(|i| i as f32).collect();
        let (mut l, mut r) = (ramp.clone(), ramp.clone());
        fx.process(&mut l, &mut r, &p, ctx());
        // Hold with the needle halfway through the last beat (samples 500..1000).
        p[0] = 1.0;
        p[1] = 0.5;
        let (mut l, mut r) = (vec![0.0; 200], vec![0.0; 200]);
        fx.process(&mut l, &mut r, &p, ctx());
        assert!((l[150] - 749.0).abs() < 2.0, "{}", l[150]);
    }

    #[test]
    fn patterns_stay_in_range() {
        for pattern in 0..SCRATCH_PATTERNS.len() {
            for i in 0..=100 {
                let (at, level) = scratch_pattern(pattern as f32, i as f32 / 100.0);
                assert!((0.0..=1.0).contains(&at) && (0.0..=1.0).contains(&level));
            }
        }
        assert_eq!(scratch_pattern(2.0, 0.75).1, 0.0, "chirp cuts the return");
    }

    #[test]
    fn a_pattern_moves_the_record() {
        let mut fx = Scratch::new(1_000.0);
        let mut p = EffectKind::Scratch.defaults();
        let ramp: Vec<f32> = (0..1_000).map(|i| i as f32).collect();
        let (mut l, mut r) = (ramp.clone(), ramp);
        fx.process(&mut l, &mut r, &p, ctx());
        p[0] = 1.0;
        p[3] = 1.0; // baby scratch
        let (mut l, mut r) = (vec![0.0; 500], vec![0.0; 500]);
        fx.process(&mut l, &mut r, &p, ctx());
        let (lo, hi) = l
            .iter()
            .fold((f32::MAX, f32::MIN), |(a, b), s| (a.min(*s), b.max(*s)));
        assert!(
            hi - lo > 200.0,
            "the play point sweeps the record: {lo}..{hi}"
        );
    }
}
