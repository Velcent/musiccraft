//! Modulation and character effects: phaser, flanger, bitcrusher, stereo
//! tool, trance gate and auto-pan. All allocation happens in `new`.

use std::f32::consts::{PI, TAU};

use crate::{ProcessContext, db_to_gain};

/// Beats per step (or per cycle) by rate index: 1/32, 1/16, 1/8, 1/4, 1/2
/// note, one bar.
pub const SYNC_RATES: [f64; 6] = [0.125, 0.25, 0.5, 1.0, 2.0, 4.0];

/// Names of [`SYNC_RATES`].
pub const SYNC_RATE_NAMES: [&str; 6] = ["1/32", "1/16", "1/8", "1/4", "1/2", "1 bar"];

fn param(p: &[f32], i: usize, default: f32) -> f32 {
    p.get(i).copied().unwrap_or(default)
}

fn sync_rate(value: f32) -> f64 {
    SYNC_RATES
        .get(value.round().max(0.0) as usize)
        .copied()
        .unwrap_or(0.25)
}

/// Beats advanced per sample at the context's tempo.
fn beats_per_sample(ctx: ProcessContext) -> f64 {
    ctx.tempo_bpm / 60.0 / f64::from(ctx.sample_rate.max(1.0))
}

/// Phaser: a chain of first-order all-pass filters whose corner sweeps
/// between 200 Hz and 4 kHz, mixed with the dry signal.
///
/// Parameters: `[rate Hz, depth, feedback, stages, mix]`.
#[derive(Debug, Clone, Default)]
pub struct Phaser {
    /// Per channel: all-pass states (input and output memories).
    state: [[(f32, f32); 12]; 2],
    feedback: [f32; 2],
    phase: f32,
}

impl Phaser {
    /// Clears the filters.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let sr = ctx.sample_rate.max(1.0);
        let rate = param(p, 0, 0.5).max(0.0);
        let depth = param(p, 1, 0.7).clamp(0.0, 1.0);
        let feedback = param(p, 2, 0.5).clamp(0.0, 0.9);
        let stages = (param(p, 3, 6.0).round() as usize).clamp(2, 12);
        let mix = param(p, 4, 0.5).clamp(0.0, 1.0);
        let step = rate / sr;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            for (ch, sample) in [l, r].into_iter().enumerate() {
                let offset = if ch == 0 { 0.0 } else { 0.25 };
                let lfo = 0.5 + 0.5 * ((self.phase + offset) * TAU).sin();
                let freq = 200.0 * 20.0_f32.powf(lfo * depth);
                let t = (PI * freq.min(sr * 0.45) / sr).tan();
                let a = (t - 1.0) / (t + 1.0);
                let mut x = *sample + self.feedback[ch] * feedback;
                for (x1, y1) in self.state[ch].iter_mut().take(stages) {
                    let y = a * x + *x1 - a * *y1;
                    *x1 = x;
                    *y1 = y;
                    x = y;
                }
                if !x.is_finite() {
                    x = 0.0;
                    self.state[ch] = [(0.0, 0.0); 12];
                }
                self.feedback[ch] = x;
                *sample += (x - *sample) * mix;
            }
            self.phase = (self.phase + step).fract();
        }
    }
}

/// Flanger: a very short modulated delay with feedback.
///
/// Parameters: `[rate Hz, depth ms, delay ms, feedback, mix]`.
#[derive(Debug, Clone)]
pub struct Flanger {
    lines: [Vec<f32>; 2],
    write: usize,
    phase: f32,
}

impl Flanger {
    /// Allocates 20 ms of delay. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (sample_rate.max(1.0) * 0.02) as usize + 4;
        Self {
            lines: [vec![0.0; len], vec![0.0; len]],
            write: 0,
            phase: 0.0,
        }
    }

    /// Clears the delay memory.
    pub fn reset(&mut self) {
        for line in &mut self.lines {
            line.fill(0.0);
        }
    }

    fn read(line: &[f32], write: usize, delay: f32) -> f32 {
        let len = line.len();
        let delay = delay.clamp(1.0, (len - 2) as f32);
        let whole = delay as usize;
        let frac = delay - whole as f32;
        let a = line
            .get((write + len - whole) % len)
            .copied()
            .unwrap_or(0.0);
        let b = line
            .get((write + len - whole - 1) % len)
            .copied()
            .unwrap_or(0.0);
        a + (b - a) * frac
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let len = self.lines[0].len();
        if len < 4 {
            return;
        }
        let sr = ctx.sample_rate.max(1.0);
        let rate = param(p, 0, 0.25).max(0.0);
        let depth = param(p, 1, 2.0).max(0.0) * 0.001 * sr;
        let base = param(p, 2, 2.0).max(0.0) * 0.001 * sr;
        let feedback = param(p, 3, 0.5).clamp(-0.95, 0.95);
        let mix = param(p, 4, 0.5).clamp(0.0, 1.0);
        let step = rate / sr;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            for (ch, sample) in [l, r].into_iter().enumerate() {
                let offset = if ch == 0 { 0.0 } else { 0.25 };
                let lfo = 0.5 + 0.5 * ((self.phase + offset) * TAU).sin();
                let line = &mut self.lines[ch];
                let wet = Self::read(line, self.write, base + depth * lfo);
                let input = *sample + wet * feedback;
                if let Some(slot) = line.get_mut(self.write) {
                    *slot = if input.is_finite() { input } else { 0.0 };
                }
                *sample += (wet - *sample) * mix;
            }
            self.write = (self.write + 1) % len;
            self.phase = (self.phase + step).fract();
        }
    }
}

/// Bitcrusher: fewer bits and a lower effective sample rate.
///
/// Parameters: `[bits, downsample, mix, output dB]`.
#[derive(Debug, Clone, Default)]
pub struct Bitcrusher {
    held: [f32; 2],
    counter: u32,
}

impl Bitcrusher {
    /// Clears the held sample.
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32]) {
        let bits = param(p, 0, 8.0).round().clamp(1.0, 16.0);
        let hold = (param(p, 1, 4.0).round() as u32).clamp(1, 32);
        let mix = param(p, 2, 1.0).clamp(0.0, 1.0);
        let output = db_to_gain(param(p, 3, 0.0));
        let levels = 2.0_f32.powf(bits - 1.0);
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if self.counter == 0 {
                self.held = [
                    (*l * levels).round() / levels,
                    (*r * levels).round() / levels,
                ];
            }
            self.counter = (self.counter + 1) % hold;
            *l = (*l + (self.held[0] - *l) * mix) * output;
            *r = (*r + (self.held[1] - *r) * mix) * output;
        }
    }
}

/// Stereo tool: mid/side width, balance and per-side phase inversion.
///
/// Parameters: `[width, balance, invert left, invert right]`.
pub fn stereo(left: &mut [f32], right: &mut [f32], p: &[f32]) {
    let width = param(p, 0, 1.0).clamp(0.0, 2.0);
    let balance = param(p, 1, 0.0).clamp(-1.0, 1.0);
    let flip_l = if param(p, 2, 0.0) >= 0.5 { -1.0 } else { 1.0 };
    let flip_r = if param(p, 3, 0.0) >= 0.5 { -1.0 } else { 1.0 };
    let gain_l = (1.0 - balance).min(1.0);
    let gain_r = (1.0 + balance).min(1.0);
    for (l, r) in left.iter_mut().zip(right.iter_mut()) {
        let mid = (*l + *r) * 0.5;
        let side = (*l - *r) * 0.5 * width;
        *l = (mid + side) * gain_l * flip_l;
        *r = (mid - side) * gain_r * flip_r;
    }
}

/// The beat clock effects synchronise to, advanced per sample.
#[derive(Debug, Clone, Copy)]
struct Clock {
    beats: f64,
    step: f64,
}

impl Clock {
    fn new(ctx: ProcessContext) -> Self {
        Self {
            beats: ctx.beats,
            step: beats_per_sample(ctx),
        }
    }

    fn tick(&mut self) -> f64 {
        let now = self.beats;
        self.beats += self.step;
        now
    }
}

/// Trance gate: opens and closes on a 16-step pattern in time with the beat.
///
/// Parameters: `[rate, duty, depth, smooth ms, steps]`.
#[derive(Debug, Clone, Default)]
pub struct Gate {
    gain: f32,
}

/// Whether step `index` (0..16) is on in a gate pattern.
#[must_use]
pub fn gate_step_on(pattern: f32, index: usize) -> bool {
    let bits = pattern.round().clamp(0.0, 65_535.0) as u32;
    bits & (1 << (index % 16)) != 0
}

impl Gate {
    /// Resets the smoothed gain.
    pub fn reset(&mut self) {
        self.gain = 0.0;
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let rate = sync_rate(param(p, 0, 1.0));
        let duty = f64::from(param(p, 1, 0.5).clamp(0.05, 1.0));
        let depth = param(p, 2, 1.0).clamp(0.0, 1.0);
        let smooth_ms = param(p, 3, 3.0).max(0.0);
        let pattern = param(p, 4, 65_535.0);
        let coeff = if smooth_ms <= 0.0 {
            1.0
        } else {
            1.0 - (-1.0 / (smooth_ms * 0.001 * ctx.sample_rate.max(1.0))).exp()
        };
        let mut clock = Clock::new(ctx);
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let beats = clock.tick();
            let position = beats / rate;
            let index = (position.floor() as i64).rem_euclid(16) as usize;
            let open = gate_step_on(pattern, index) && position.fract() < duty;
            let target = if open { 1.0 } else { 1.0 - depth };
            self.gain += (target - self.gain) * coeff;
            *l *= self.gain;
            *r *= self.gain;
        }
    }
}

/// The auto-pan LFO's value (−1..1) at `phase` (0..1) for a shape index.
#[must_use]
pub fn autopan_shape(shape: f32, phase: f64) -> f32 {
    let phase = phase.rem_euclid(1.0) as f32;
    match shape.round() as i32 {
        1 => 1.0 - 4.0 * (phase - 0.5).abs(),
        2 => {
            if phase < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        _ => (phase * TAU).sin(),
    }
}

/// Auto-pan: moves the sound between left and right in time with the beat.
///
/// Parameters: `[rate, depth, shape]`.
pub fn autopan(left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
    let rate = sync_rate(param(p, 0, 3.0));
    let depth = param(p, 1, 0.7).clamp(0.0, 1.0);
    let shape = param(p, 2, 0.0);
    let mut clock = Clock::new(ctx);
    for (l, r) in left.iter_mut().zip(right.iter_mut()) {
        let pan = autopan_shape(shape, clock.tick() / rate) * depth;
        *l *= (1.0 - pan).min(1.0);
        *r *= (1.0 + pan).min(1.0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx() -> ProcessContext {
        ProcessContext {
            sample_rate: 48_000.0,
            tempo_bpm: 120.0,
            beats: 0.0,
        }
    }

    fn tone(n: usize) -> (Vec<f32>, Vec<f32>) {
        let l: Vec<f32> = (0..n).map(|i| (i as f32 * 0.05).sin() * 0.5).collect();
        (l.clone(), l)
    }

    #[test]
    fn phaser_and_flanger_stay_bounded_and_change_the_sound() {
        let (mut l, mut r) = tone(48_000);
        let dry = l.clone();
        let mut phaser = Phaser::default();
        phaser.process(&mut l, &mut r, &[1.0, 1.0, 0.8, 12.0, 0.5], ctx());
        assert!(l.iter().chain(&r).all(|s| s.is_finite() && s.abs() < 2.0));
        assert!(l.iter().zip(&dry).any(|(a, b)| (a - b).abs() > 0.05));
        let (mut l, mut r) = tone(48_000);
        let mut flanger = Flanger::new(48_000.0);
        flanger.process(&mut l, &mut r, &[0.5, 3.0, 2.0, 0.9, 0.5], ctx());
        assert!(l.iter().chain(&r).all(|s| s.is_finite() && s.abs() < 3.0));
        assert!(l.iter().zip(&dry).any(|(a, b)| (a - b).abs() > 0.05));
    }

    #[test]
    fn bitcrusher_quantizes_and_holds() {
        let mut l: Vec<f32> = (0..8).map(|i| i as f32 * 0.1).collect();
        let mut r = l.clone();
        Bitcrusher::default().process(&mut l, &mut r, &[2.0, 4.0, 1.0, 0.0]);
        // Two bits: levels at multiples of 0.5; held for four samples.
        assert_eq!(&l[..4], &[0.0; 4]);
        assert!(l[4..].iter().all(|s| (*s - 0.5).abs() < 1e-6), "{l:?}");
    }

    #[test]
    fn stereo_width_and_balance() {
        let (mut l, mut r) = (vec![1.0, 0.5], vec![0.0, 0.5]);
        stereo(&mut l, &mut r, &[0.0, 0.0, 0.0, 0.0]);
        assert_eq!((l[0], r[0]), (0.5, 0.5), "width 0 is mono");
        let (mut l, mut r) = (vec![1.0], vec![1.0]);
        stereo(&mut l, &mut r, &[1.0, 1.0, 0.0, 1.0]);
        assert_eq!((l[0], r[0]), (0.0, -1.0), "balance right, right inverted");
    }

    #[test]
    fn gate_follows_the_beat_and_its_pattern() {
        // 1/16 steps at 120 BPM = 6000 samples; only the first step is on.
        let mut gate = Gate::default();
        let mut l = vec![1.0; 24_000];
        let mut r = l.clone();
        gate.process(&mut l, &mut r, &[1.0, 0.5, 1.0, 0.0, 1.0], ctx());
        assert!((l[100] - 1.0).abs() < 1e-6, "open at the start of step 1");
        assert!(l[3_100].abs() < 1e-6, "closed after the duty");
        assert!(l[6_100].abs() < 1e-6, "step 2 is off");
        assert!(gate_step_on(5.0, 2) && !gate_step_on(5.0, 1));
        // Starting mid-song, it lines up with the song position.
        let mut gate = Gate::default();
        let mut l = vec![1.0; 100];
        let mut r = l.clone();
        let later = ProcessContext {
            beats: 4.0,
            ..ctx()
        };
        gate.process(&mut l, &mut r, &[1.0, 0.5, 1.0, 0.0, 1.0], later);
        assert!((l[50] - 1.0).abs() < 1e-6, "bar 2 starts on step 1 again");
    }

    #[test]
    fn autopan_moves_between_sides() {
        let mut l = vec![1.0; 24_000];
        let mut r = l.clone();
        autopan(&mut l, &mut r, &[3.0, 1.0, 2.0], ctx());
        // Square, one cycle per beat: first half right (left down).
        assert!(l[100] < 0.01 && (r[100] - 1.0).abs() < 1e-6);
        assert!((l[12_100] - 1.0).abs() < 1e-6 && r[12_100] < 0.01);
    }
}
