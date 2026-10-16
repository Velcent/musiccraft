//! Beat shaper: drawn time and volume envelopes over a tempo-synced cycle.
//! The time envelope says how far behind the live input playback is, so
//! held values repeat earlier audio, rising ramps slow it down or stop it,
//! and steep ramps play it backwards. All allocation happens in `new`.

use crate::{ProcessContext, flush_denormal};

fn param(p: &[f32], i: usize, default: f32) -> f32 {
    p.get(i).copied().unwrap_or(default)
}

/// Points per envelope.
pub const BEAT_POINTS: usize = 8;
/// Cycle lengths in beats, by `cycle` value.
pub const BEAT_CYCLES: [f64; 4] = [1.0, 2.0, 4.0, 8.0];
/// Names of [`BEAT_CYCLES`].
pub const BEAT_CYCLE_NAMES: [&str; 4] = ["1 beat", "2 beats", "1 bar", "2 bars"];
/// Index of the first time point.
pub const FIRST_TIME: usize = 2;
/// Index of the first volume point.
pub const FIRST_VOLUME: usize = FIRST_TIME + BEAT_POINTS;
/// Longest history kept, in seconds; longer cycles reach back at most this far.
const MAX_SECONDS: f32 = 8.0;
/// Jumps in the time envelope crossfade over this long.
const FADE_SECONDS: f32 = 0.003;

/// The envelope through `points` (evenly spaced over the cycle) at `phase`
/// (0..1): held flat over each step, or ramped linearly to the next point
/// (wrapping to the first).
#[must_use]
pub fn envelope(points: &[f32], phase: f32, ramps: bool) -> f32 {
    let n = points.len();
    if n == 0 {
        return 0.0;
    }
    let pos = phase.rem_euclid(1.0) * n as f32;
    let k = (pos as usize).min(n - 1);
    let a = points.get(k).copied().unwrap_or(0.0);
    if !ramps {
        return a;
    }
    let b = points.get((k + 1) % n).copied().unwrap_or(a);
    a + (b - a) * (pos - k as f32)
}

/// Beat shaper.
///
/// Parameters: `[cycle, time mode (0 steps, 1 ramps), time 1 … time 8 (how
/// far behind the input playback is, as a share of the cycle), volume 1 …
/// volume 8, mix]`.
#[derive(Debug, Clone)]
pub struct BeatShaper {
    lines: [Vec<f32>; 2],
    write: usize,
    last_delay: f32,
    fade_from: f32,
    fade: f32,
    gain: f32,
}

impl BeatShaper {
    /// Allocates the history buffer. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (MAX_SECONDS * sample_rate.max(1.0)) as usize + 4;
        Self {
            lines: [vec![0.0; len], vec![0.0; len]],
            write: 0,
            last_delay: 0.0,
            fade_from: 0.0,
            fade: 0.0,
            gain: 1.0,
        }
    }

    /// Clears the history.
    pub fn reset(&mut self) {
        for line in &mut self.lines {
            line.fill(0.0);
        }
        self.last_delay = 0.0;
        self.fade = 0.0;
        self.gain = 1.0;
    }

    fn read(&self, ch: usize, delay: f32) -> f32 {
        let Some(line) = self.lines.get(ch) else {
            return 0.0;
        };
        let len = line.len();
        let delay = delay.clamp(0.0, (len - 3) as f32);
        let whole = delay as usize;
        let frac = delay - whole as f32;
        let at = |back: usize| {
            line.get((self.write + len - back) % len)
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
        let cycle = BEAT_CYCLES
            .get(param(p, 0, 2.0).round().max(0.0) as usize)
            .copied()
            .unwrap_or(4.0);
        let ramps = param(p, 1, 0.0) >= 0.5;
        let times = p.get(FIRST_TIME..FIRST_TIME + BEAT_POINTS).unwrap_or(&[]);
        let volumes = p
            .get(FIRST_VOLUME..FIRST_VOLUME + BEAT_POINTS)
            .unwrap_or(&[]);
        let mix = param(p, FIRST_VOLUME + BEAT_POINTS, 1.0).clamp(0.0, 1.0);
        let samples_per_beat = 60.0 / ctx.tempo_bpm.max(1.0) as f32 * sr;
        let cycle_samples = (cycle as f32 * samples_per_beat).min((len - 4) as f32);
        let beats_per_sample = ctx.tempo_bpm / 60.0 / f64::from(sr);
        let fade_step = 1.0 / (FADE_SECONDS * sr).max(1.0);
        let smooth = 1.0 - (-1.0 / (0.002 * sr)).exp();
        let mut beats = ctx.beats;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            for (line, x) in self.lines.iter_mut().zip([*l, *r]) {
                if let Some(slot) = line.get_mut(self.write) {
                    *slot = if x.is_finite() {
                        flush_denormal(x)
                    } else {
                        0.0
                    };
                }
            }
            let phase = (beats / cycle).fract() as f32;
            let delay = envelope(times, phase, ramps).clamp(0.0, 1.0) * cycle_samples;
            // A jump (not a slope) in the time envelope crossfades from the
            // old read point so it does not click.
            if (delay - self.last_delay).abs() > 64.0 {
                self.fade_from = self.last_delay;
                self.fade = 1.0;
            }
            self.last_delay = delay;
            let target = envelope(volumes, phase, true).clamp(0.0, 1.0);
            self.gain += (target - self.gain) * smooth;
            let (dry_l, dry_r) = (*l, *r);
            let mut wet = [self.read(0, delay), self.read(1, delay)];
            if self.fade > 0.0 {
                let old = [self.read(0, self.fade_from), self.read(1, self.fade_from)];
                for (w, o) in wet.iter_mut().zip(old) {
                    *w += (o - *w) * self.fade;
                }
                self.fade = (self.fade - fade_step).max(0.0);
            }
            *l = dry_l + (wet[0] * self.gain - dry_l) * mix;
            *r = dry_r + (wet[1] * self.gain - dry_r) * mix;
            self.write = (self.write + 1) % len;
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

    fn ramp(n: usize) -> Vec<f32> {
        (0..n).map(|i| i as f32 / n as f32).collect()
    }

    #[test]
    fn defaults_pass_audio_through() {
        let mut fx = BeatShaper::new(1_000.0);
        let input = ramp(4_000);
        let (mut l, mut r) = (input.clone(), input.clone());
        fx.process(&mut l, &mut r, &EffectKind::BeatShaper.defaults(), ctx());
        for (a, b) in l.iter().zip(&input) {
            assert!((a - b).abs() < 1e-6);
        }
    }

    #[test]
    fn held_time_point_repeats_earlier_audio() {
        // One-beat cycle (500 samples); the second half plays the first half
        // again (half a cycle behind).
        let mut p = EffectKind::BeatShaper.defaults();
        p[0] = 0.0;
        for k in 4..8 {
            p[FIRST_TIME + k] = 0.5;
        }
        let mut fx = BeatShaper::new(1_000.0);
        let input = ramp(1_000);
        let (mut l, mut r) = (input.clone(), input.clone());
        fx.process(&mut l, &mut r, &p, ctx());
        // Past the crossfade (3 samples), sample 300 is sample 50.
        assert!(
            (l[300] - input[50]).abs() < 1e-6,
            "{} vs {}",
            l[300],
            input[50]
        );
        assert!((l[100] - input[100]).abs() < 1e-6, "first half is live");
    }

    #[test]
    fn rising_ramp_freezes_the_audio() {
        // Delay rising exactly as fast as time: playback stands still.
        let mut p = EffectKind::BeatShaper.defaults();
        p[0] = 0.0;
        p[1] = 1.0;
        for k in 0..8 {
            p[FIRST_TIME + k] = k as f32 / 8.0;
        }
        let mut fx = BeatShaper::new(1_000.0);
        let input = ramp(500);
        let (mut l, mut r) = (input.clone(), input.clone());
        fx.process(&mut l, &mut r, &p, ctx());
        assert!((l[100] - l[300]).abs() < 1e-6 && (l[300] - l[430]).abs() < 1e-6);
    }

    #[test]
    fn volume_points_shape_the_level() {
        let mut p = EffectKind::BeatShaper.defaults();
        p[0] = 0.0;
        for k in 0..8 {
            p[FIRST_VOLUME + k] = if k % 2 == 0 { 1.0 } else { 0.0 };
        }
        let mut fx = BeatShaper::new(1_000.0);
        let (mut l, mut r) = (vec![1.0; 500], vec![1.0; 500]);
        fx.process(&mut l, &mut r, &p, ctx());
        // Volume ramps 1 → 0 over the first step (62.5 samples).
        assert!(l[5] > 0.8 && l[60] < 0.2, "{} {}", l[5], l[60]);
    }

    #[test]
    fn envelope_steps_and_ramps() {
        let pts = [0.0, 1.0];
        assert_eq!(envelope(&pts, 0.25, false), 0.0);
        assert!((envelope(&pts, 0.25, true) - 0.5).abs() < 1e-6);
        assert!(
            (envelope(&pts, 0.75, true) - 0.5).abs() < 1e-6,
            "wraps to the first"
        );
    }
}
