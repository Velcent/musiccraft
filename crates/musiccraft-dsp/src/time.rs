use crate::effect::ProcessContext;

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
            self.damp_l = dl + (self.damp_l - dl) * damping;
            self.damp_r = dr + (self.damp_r - dr) * damping;
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
                *slot = in_l + fb_l;
            }
            if let Some(slot) = self.right.get_mut(self.write) {
                *slot = in_r + fb_r;
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
