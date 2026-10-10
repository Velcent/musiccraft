//! Schroeder-Moorer reverb in the style of the public-domain "Freeverb" design by
//! Jezar at Dreampoint: eight parallel damped combs and four series all-passes per
//! channel.

const COMB_TUNING: [usize; 8] = [1116, 1188, 1277, 1356, 1422, 1491, 1557, 1617];
const ALLPASS_TUNING: [usize; 4] = [556, 441, 341, 225];
const STEREO_SPREAD: usize = 23;
const FIXED_GAIN: f32 = 0.015;
const SCALE_ROOM: f32 = 0.28;
const OFFSET_ROOM: f32 = 0.7;
const SCALE_DAMP: f32 = 0.4;

#[derive(Debug, Clone)]
struct Comb {
    buffer: Vec<f32>,
    index: usize,
    store: f32,
}

impl Comb {
    fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len.max(1)],
            index: 0,
            store: 0.0,
        }
    }

    fn process(&mut self, input: f32, feedback: f32, damp: f32) -> f32 {
        let len = self.buffer.len();
        let Some(slot) = self.buffer.get_mut(self.index) else {
            return 0.0;
        };
        let output = *slot;
        self.store = output * (1.0 - damp) + self.store * damp;
        *slot = input + self.store * feedback;
        self.index = (self.index + 1) % len;
        output
    }
}

#[derive(Debug, Clone)]
struct Allpass {
    buffer: Vec<f32>,
    index: usize,
}

impl Allpass {
    fn new(len: usize) -> Self {
        Self {
            buffer: vec![0.0; len.max(1)],
            index: 0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let len = self.buffer.len();
        let Some(slot) = self.buffer.get_mut(self.index) else {
            return input;
        };
        let buffered = *slot;
        *slot = input + buffered * 0.5;
        self.index = (self.index + 1) % len;
        buffered - input
    }
}

/// Stereo algorithmic reverb.
///
/// Parameters: `[size, damping, width, mix]`, all 0..=1.
#[derive(Debug, Clone)]
pub struct Reverb {
    combs_l: Vec<Comb>,
    combs_r: Vec<Comb>,
    allpass_l: Vec<Allpass>,
    allpass_r: Vec<Allpass>,
}

impl Reverb {
    /// Allocates the delay lines, scaled for the sample rate. Main thread only.
    #[must_use]
    pub fn new(sample_rate: f32) -> Self {
        let scale = sample_rate.max(1.0) / 44_100.0;
        let sized = |n: usize| ((n as f32) * scale) as usize;
        Self {
            combs_l: COMB_TUNING.iter().map(|&n| Comb::new(sized(n))).collect(),
            combs_r: COMB_TUNING
                .iter()
                .map(|&n| Comb::new(sized(n + STEREO_SPREAD)))
                .collect(),
            allpass_l: ALLPASS_TUNING
                .iter()
                .map(|&n| Allpass::new(sized(n)))
                .collect(),
            allpass_r: ALLPASS_TUNING
                .iter()
                .map(|&n| Allpass::new(sized(n + STEREO_SPREAD)))
                .collect(),
        }
    }

    /// Clears the reverb tail.
    pub fn reset(&mut self) {
        for comb in self.combs_l.iter_mut().chain(&mut self.combs_r) {
            comb.buffer.fill(0.0);
            comb.store = 0.0;
        }
        for ap in self.allpass_l.iter_mut().chain(&mut self.allpass_r) {
            ap.buffer.fill(0.0);
        }
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32]) {
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default).clamp(0.0, 1.0);
        let feedback = param(0, 0.6) * SCALE_ROOM + OFFSET_ROOM;
        let damp = param(1, 0.5) * SCALE_DAMP;
        let width = param(2, 1.0);
        let mix = param(3, 0.25);
        let wet1 = width / 2.0 + 0.5;
        let wet2 = (1.0 - width) / 2.0;

        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            let input = (*l + *r) * FIXED_GAIN;
            let mut out_l = 0.0;
            let mut out_r = 0.0;
            for comb in &mut self.combs_l {
                out_l += comb.process(input, feedback, damp);
            }
            for comb in &mut self.combs_r {
                out_r += comb.process(input, feedback, damp);
            }
            for ap in &mut self.allpass_l {
                out_l = ap.process(out_l);
            }
            for ap in &mut self.allpass_r {
                out_r = ap.process(out_r);
            }
            let wet_l = out_l * wet1 + out_r * wet2;
            let wet_r = out_r * wet1 + out_l * wet2;
            // Wet level is scaled up so mix = 1 is roughly as loud as the input.
            *l = *l * (1.0 - mix) + wet_l * mix * 3.0;
            *r = *r * (1.0 - mix) + wet_r * mix * 3.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn impulse_produces_a_decaying_tail() {
        let mut reverb = Reverb::new(44_100.0);
        let params = [0.5, 0.5, 1.0, 1.0];
        let mut l = vec![0.0; 44_100];
        let mut r = vec![0.0; 44_100];
        l[0] = 1.0;
        r[0] = 1.0;
        reverb.process(&mut l, &mut r, &params);
        let energy = |s: &[f32]| s.iter().map(|x| x * x).sum::<f32>();
        let early = energy(&l[1_000..11_000]);
        let late = energy(&l[30_000..40_000]);
        assert!(early > 0.0);
        assert!(late < early / 10.0, "early {early}, late {late}");
        assert!(l.iter().chain(&r).all(|s| s.is_finite()));
    }

    #[test]
    fn dry_mix_is_transparent() {
        let mut reverb = Reverb::new(44_100.0);
        let mut l = vec![0.5; 64];
        let mut r = vec![-0.5; 64];
        reverb.process(&mut l, &mut r, &[0.5, 0.5, 1.0, 0.0]);
        assert!(l.iter().all(|s| (*s - 0.5).abs() < 1e-6));
        assert!(r.iter().all(|s| (*s + 0.5).abs() < 1e-6));
    }
}
