use crate::effect::ProcessContext;
use crate::util::flush_denormal;

/// Longest delay time supported, in seconds. The buffer is sized for this.
const MAX_DELAY_SECONDS: f32 = 4.0;

/// Tempo-synced stereo delay with optional ping-pong and feedback damping.
///
/// Parameters: `[time (beats), feedback, mix, ping_pong, damping]`.
#[derive(Debug, Clone)]
pub struct Delay {
    left: Vec<f32>,
    right: Vec<f32>,
    write: usize,
    damp_l: f32,
    damp_r: f32,
}

impl Delay {
    /// Allocates the delay lines. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (sample_rate.max(1.0) * MAX_DELAY_SECONDS) as usize + 1;
        Self {
            left: vec![0.0; len],
            right: vec![0.0; len],
            write: 0,
            damp_l: 0.0,
            damp_r: 0.0,
        }
    }

    /// Clears the delay memory.
    pub fn reset(&mut self) {
        self.left.fill(0.0);
        self.right.fill(0.0);
        self.damp_l = 0.0;
        self.damp_r = 0.0;
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let len = self.left.len();
        if len < 2 || self.right.len() != len {
            return;
        }
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let seconds_per_beat = 60.0 / ctx.tempo_bpm.max(1.0) as f32;
        let delay_seconds = (param(0, 0.75) * seconds_per_beat).clamp(0.0, MAX_DELAY_SECONDS);
        let delay = ((delay_seconds * ctx.sample_rate) as usize).clamp(1, len - 1);
        let feedback = param(1, 0.4).clamp(0.0, 0.95);
        let mix = param(2, 0.3).clamp(0.0, 1.0);
        let ping_pong = param(3, 1.0) >= 0.5;
        let damping = param(4, 0.3).clamp(0.0, 0.99);

        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let read = (self.write + len - delay) % len;
            let dl = self.left.get(read).copied().unwrap_or(0.0);
            let dr = self.right.get(read).copied().unwrap_or(0.0);
            self.damp_l = flush_denormal(dl + (self.damp_l - dl) * damping);
            self.damp_r = flush_denormal(dr + (self.damp_r - dr) * damping);
            let (fb_l, fb_r) = if ping_pong {
                (self.damp_r * feedback, self.damp_l * feedback)
            } else {
                (self.damp_l * feedback, self.damp_r * feedback)
            };
            let (in_l, in_r) = if ping_pong {
                ((*l + *r) * 0.5, 0.0)
            } else {
                (*l, *r)
            };
            if let Some(slot) = self.left.get_mut(self.write) {
                *slot = flush_denormal(in_l + fb_l);
            }
            if let Some(slot) = self.right.get_mut(self.write) {
                *slot = flush_denormal(in_r + fb_r);
            }
            self.write = (self.write + 1) % len;
            *l += (dl - *l) * mix;
            *r += (dr - *r) * mix;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_repeats_after_one_beat() {
        let sample_rate = 1_000.0;
        let mut delay = Delay::new(sample_rate);
        let ctx = ProcessContext {
            beats: 0.0,
            sample_rate,
            tempo_bpm: 60.0,
        };
        // 1 beat at 60 BPM = 1 s = 1000 samples; full wet, no feedback, no ping-pong.
        let params = [1.0, 0.0, 1.0, 0.0, 0.0];
        let mut l = vec![0.0; 1_500];
        let mut r = vec![0.0; 1_500];
        l[0] = 1.0;
        r[0] = 1.0;
        delay.process(&mut l, &mut r, &params, ctx);
        assert!(l[0].abs() < 1e-6);
        assert!((l[1_000] - 1.0).abs() < 1e-6);
        assert!((r[1_000] - 1.0).abs() < 1e-6);
    }

    #[test]
    fn feedback_decays() {
        let sample_rate = 100.0;
        let mut delay = Delay::new(sample_rate);
        let ctx = ProcessContext {
            beats: 0.0,
            sample_rate,
            tempo_bpm: 60.0,
        };
        let params = [0.1, 0.5, 1.0, 0.0, 0.0];
        let mut l = vec![0.0; 100];
        let mut r = vec![0.0; 100];
        l[0] = 1.0;
        delay.process(&mut l, &mut r, &params, ctx);
        assert!((l[10] - 1.0).abs() < 1e-6);
        assert!((l[20] - 0.5).abs() < 1e-6);
        assert!((l[30] - 0.25).abs() < 1e-6);
    }
}

/// Longest chorus delay (base plus depth) in seconds.
const MAX_CHORUS_SECONDS: f32 = 0.05;

/// Stereo chorus: a short delay per channel modulated by sine LFOs a quarter
/// cycle apart, mixed with the dry signal.
///
/// Parameters: `[rate Hz, depth ms, delay ms, mix, voices, tone Hz]`.
#[derive(Debug, Clone)]
pub struct Chorus {
    left: Vec<f32>,
    right: Vec<f32>,
    write: usize,
    phase: f32,
    tone: [f32; 2],
}

impl Chorus {
    /// Allocates the delay lines. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let len = (sample_rate.max(1.0) * MAX_CHORUS_SECONDS) as usize + 2;
        Self {
            left: vec![0.0; len],
            right: vec![0.0; len],
            write: 0,
            phase: 0.0,
            tone: [0.0; 2],
        }
    }

    /// Clears the delay memory.
    pub fn reset(&mut self) {
        self.left.fill(0.0);
        self.right.fill(0.0);
        self.tone = [0.0; 2];
    }

    fn read(buffer: &[f32], write: usize, delay: f32) -> f32 {
        let len = buffer.len();
        let delay = delay.clamp(1.0, (len - 2) as f32);
        let whole = delay as usize;
        let frac = delay - whole as f32;
        let a = buffer
            .get((write + len - whole) % len)
            .copied()
            .unwrap_or(0.0);
        let b = buffer
            .get((write + len - whole - 1) % len)
            .copied()
            .unwrap_or(0.0);
        a + (b - a) * frac
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let len = self.left.len();
        if len < 4 || self.right.len() != len {
            return;
        }
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let sr = ctx.sample_rate.max(1.0);
        let rate = param(0, 0.8).max(0.0);
        let depth = param(1, 3.0).max(0.0) * 0.001 * sr;
        let base = param(2, 12.0).max(0.0) * 0.001 * sr;
        let mix = param(3, 0.4).clamp(0.0, 1.0);
        let voices = (param(4, 1.0).round() as usize).clamp(1, 3);
        let tone_hz = param(5, 20_000.0);
        let tone =
            (tone_hz < 20_000.0).then(|| 1.0 - (-std::f32::consts::TAU * tone_hz / sr).exp());
        let step = rate / sr;
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            if let Some(slot) = self.left.get_mut(self.write) {
                *slot = *l;
            }
            if let Some(slot) = self.right.get_mut(self.write) {
                *slot = *r;
            }
            let mut wet = [0.0_f32; 2];
            for v in 0..voices {
                let phase = self.phase + v as f32 / voices as f32;
                for (ch, (w, line)) in wet.iter_mut().zip([&self.left, &self.right]).enumerate() {
                    let offset = if ch == 0 { 0.0 } else { 0.25 };
                    let lfo = ((phase + offset) * std::f32::consts::TAU).sin();
                    *w += Self::read(line, self.write, base + depth * (0.5 + 0.5 * lfo));
                }
            }
            for (w, state) in wet.iter_mut().zip(&mut self.tone) {
                *w /= voices as f32;
                if let Some(a) = tone {
                    *state = crate::flush_denormal(*state + (*w - *state) * a);
                    *w = *state;
                }
            }
            let [wet_l, wet_r] = wet;
            *l += (wet_l - *l) * mix;
            *r += (wet_r - *r) * mix;
            self.write = (self.write + 1) % len;
            self.phase = (self.phase + step).fract();
        }
    }
}

#[cfg(test)]
mod chorus_tests {
    use super::*;

    #[test]
    fn chorus_delays_and_modulates() {
        let ctx = ProcessContext {
            beats: 0.0,
            sample_rate: 10_000.0,
            tempo_bpm: 120.0,
        };
        let mut chorus = Chorus::new(ctx.sample_rate);
        // Fully wet, no modulation: a pure 10 ms delay.
        let mut l = vec![0.0; 300];
        let mut r = vec![0.0; 300];
        l[0] = 1.0;
        r[0] = 1.0;
        chorus.process(&mut l, &mut r, &[0.0, 0.0, 10.0, 1.0], ctx);
        assert!(l[0].abs() < 1e-6);
        assert!((l[100] - 1.0).abs() < 1e-4, "got {}", l[100]);
        // With modulation the output stays bounded and finite.
        let mut l: Vec<f32> = (0..10_000).map(|i| (i as f32 * 0.03).sin()).collect();
        let mut r = l.clone();
        chorus.process(&mut l, &mut r, &[5.0, 10.0, 30.0, 0.5], ctx);
        assert!(l.iter().chain(&r).all(|s| s.is_finite() && s.abs() <= 1.01));
    }

    #[test]
    fn chorus_voices_average_and_tone_darkens() {
        let ctx = ProcessContext {
            beats: 0.0,
            sample_rate: 10_000.0,
            tempo_bpm: 120.0,
        };
        // Three voices with no modulation are three identical copies: the
        // average is the same 10 ms delay.
        let mut chorus = Chorus::new(ctx.sample_rate);
        let mut l = vec![0.0; 300];
        let mut r = vec![0.0; 300];
        l[0] = 1.0;
        chorus.process(&mut l, &mut r, &[0.0, 0.0, 10.0, 1.0, 3.0, 20_000.0], ctx);
        assert!((l[100] - 1.0).abs() < 1e-4, "got {}", l[100]);
        // A low tone smears the echo's click into a gentler bump.
        let mut chorus = Chorus::new(ctx.sample_rate);
        let mut l = vec![0.0; 300];
        let mut r = vec![0.0; 300];
        l[0] = 1.0;
        chorus.process(&mut l, &mut r, &[0.0, 0.0, 10.0, 1.0, 1.0, 500.0], ctx);
        assert!(l[100] < 0.5 && l[101] > 0.05, "{} {}", l[100], l[101]);
    }
}
