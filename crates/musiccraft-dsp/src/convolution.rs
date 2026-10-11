//! Convolution reverb: uniformly partitioned overlap-save convolution with a
//! stereo impulse response, plus generated impulse responses for when no
//! file is loaded. Everything is allocated in [`Convolver::new`].

use crate::fft::{Complex, Fft};
use crate::{ProcessContext, Svf, SvfMode, db_to_gain};

/// Samples per partition (and the reverb's added latency).
const BLOCK: usize = 512;
/// Longest impulse response used, in seconds; longer ones are cut.
pub const MAX_IR_SECONDS: f32 = 6.0;
/// Longest pre-delay in milliseconds.
const MAX_PREDELAY_MS: f32 = 250.0;

/// Names of the generated impulse responses, usable as `ir:<name>`.
pub const BUILTIN_IRS: [&str; 5] = musiccraft_params::IMPULSE_RESPONSES;

/// A generated stereo impulse response: decaying noise that darkens over
/// time, with a few early reflections. `None` for unknown names.
#[must_use]
pub fn builtin_ir(name: &str, sample_rate: f32) -> Option<(Vec<f32>, Vec<f32>)> {
    // (decay seconds to -60 dB, brightness, early reflection spread ms)
    let (rt60, bright, early): (f32, f32, f32) = match name {
        "room" => (0.6, 0.5, 12.0),
        "chamber" => (1.1, 0.6, 20.0),
        "plate" => (1.6, 0.9, 2.0),
        "hall" => (2.4, 0.55, 35.0),
        "cathedral" => (4.5, 0.45, 60.0),
        _ => return None,
    };
    let sr = sample_rate.max(1.0);
    let len = ((rt60 * 1.1).min(MAX_IR_SECONDS) * sr) as usize;
    let mut seed = 0x2545_f491_u32;
    let mut noise = move || {
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 5;
        seed as f32 / u32::MAX as f32 * 2.0 - 1.0
    };
    let mut channels = [vec![0.0; len], vec![0.0; len]];
    for (ch, ir) in channels.iter_mut().enumerate() {
        let mut low = 0.0_f32;
        for (i, s) in ir.iter_mut().enumerate() {
            let t = i as f32 / sr;
            // The tail gets darker as it decays.
            let cutoff = (bright * 0.9 * (-t / rt60 * 2.0).exp()).clamp(0.02, 0.95);
            low += (noise() - low) * cutoff;
            let fade_in = (t / 0.004).min(1.0);
            *s = low * (-6.9 * t / rt60).exp() * fade_in;
        }
        for k in 0..6_usize {
            let at = ((early * (k as f32 + 1.0) * (1.0 + 0.13 * ch as f32)) * 0.001 * sr) as usize;
            if let Some(s) = ir.get_mut(at) {
                *s += 0.5 * 0.75_f32.powi(k as i32) * if (k + ch) % 2 == 0 { 1.0 } else { -1.0 };
            }
        }
    }
    let [left, right] = channels;
    Some((left, right))
}

/// Bins kept per half spectrum of a real signal.
const BINS: usize = BLOCK + 1;

/// Splits the spectrum `z` of `left + i·right` (both real) into the half
/// spectra of each, written to `left` and `right`.
fn split(z: &[Complex], left: &mut [Complex], right: &mut [Complex]) {
    let n = z.len();
    for (k, (l, r)) in left.iter_mut().zip(right.iter_mut()).enumerate() {
        let a = z.get(k).copied().unwrap_or_default();
        let b = z.get((n - k) % n.max(1)).copied().unwrap_or_default();
        // L = (Z[k] + conj Z[n−k]) / 2, R = (Z[k] − conj Z[n−k]) / 2i.
        *l = ((a.0 + b.0) * 0.5, (a.1 - b.1) * 0.5);
        *r = ((a.1 + b.1) * 0.5, (b.0 - a.0) * 0.5);
    }
}

/// Builds the full spectrum of `left + i·right` from their half spectra.
fn join(left: &[Complex], right: &[Complex], z: &mut [Complex]) {
    let n = z.len();
    for (k, out) in z.iter_mut().enumerate() {
        let (l, r) = if k < BINS {
            (
                left.get(k).copied().unwrap_or_default(),
                right.get(k).copied().unwrap_or_default(),
            )
        } else {
            let (l, r) = (
                left.get(n - k).copied().unwrap_or_default(),
                right.get(n - k).copied().unwrap_or_default(),
            );
            ((l.0, -l.1), (r.0, -r.1))
        };
        *out = (l.0 - r.1, l.1 + r.0);
    }
}

/// Multiply-accumulates `x · h` into `acc`, bin by bin.
fn mac(acc: &mut [Complex], x: &[Complex], h: &[Complex]) {
    for ((a, x), h) in acc.iter_mut().zip(x).zip(h) {
        a.0 += x.0 * h.0 - x.1 * h.1;
        a.1 += x.0 * h.1 + x.1 * h.0;
    }
}

/// Stereo convolution reverb.
///
/// Parameters: `[mix, predelay ms, low cut Hz, high cut Hz, output dB]`.
#[derive(Debug, Clone)]
pub struct Convolver {
    fft: Fft,
    /// Half spectra of the impulse response, `BINS` per partition.
    ir: [Vec<Complex>; 2],
    /// Half spectra of past input blocks, newest at partition `head`.
    history: [Vec<Complex>; 2],
    partitions: usize,
    head: usize,
    /// The previous and current input blocks (overlap-save).
    window: Vec<Complex>,
    scratch: Vec<Complex>,
    accumulator: [Vec<Complex>; 2],
    input: Vec<Complex>,
    output: Vec<Complex>,
    fill: usize,
    /// Consecutive silent input blocks; once the whole response has passed,
    /// blocks are skipped.
    quiet_blocks: usize,
    predelay: [Vec<f32>; 2],
    predelay_write: usize,
    filters: [[Svf; 2]; 2],
}

impl Convolver {
    /// Prepares the impulse response. Main thread only.
    #[must_use]
    pub fn new(left: &[f32], right: &[f32], sample_rate: f32) -> Self {
        let fft = Fft::new(BLOCK * 2);
        let n = fft.len();
        let max_len = (MAX_IR_SECONDS * sample_rate.max(1.0)) as usize;
        let len = left.len().max(right.len()).min(max_len);
        // Normalize so the response has unit energy (noise in, same level out).
        let energy: f32 = left[..left.len().min(len)]
            .iter()
            .chain(&right[..right.len().min(len)])
            .map(|s| s * s)
            .sum();
        let norm = if energy > 1e-9 {
            (2.0 / energy).sqrt()
        } else {
            0.0
        };
        let at = |v: &[f32], i: usize| v.get(i).copied().unwrap_or(0.0) * norm;
        let partitions = len.div_ceil(BLOCK).max(1);
        let mut ir = [
            vec![(0.0, 0.0); partitions * BINS],
            vec![(0.0, 0.0); partitions * BINS],
        ];
        let mut z = vec![(0.0, 0.0); n];
        let [ir_l, ir_r] = &mut ir;
        for (p, (hl, hr)) in ir_l
            .chunks_exact_mut(BINS)
            .zip(ir_r.chunks_exact_mut(BINS))
            .enumerate()
        {
            z.fill((0.0, 0.0));
            for (i, slot) in z.iter_mut().take(BLOCK).enumerate() {
                let k = p * BLOCK + i;
                if k < len {
                    *slot = (at(left, k), at(right, k));
                }
            }
            fft.forward(&mut z);
            split(&z, hl, hr);
        }
        let predelay_len = (MAX_PREDELAY_MS * 0.001 * sample_rate.max(1.0)) as usize + 2;
        Self {
            fft,
            ir,
            history: [
                vec![(0.0, 0.0); partitions * BINS],
                vec![(0.0, 0.0); partitions * BINS],
            ],
            partitions,
            head: 0,
            window: vec![(0.0, 0.0); n],
            scratch: vec![(0.0, 0.0); n],
            accumulator: [vec![(0.0, 0.0); BINS], vec![(0.0, 0.0); BINS]],
            input: vec![(0.0, 0.0); BLOCK],
            output: vec![(0.0, 0.0); BLOCK],
            fill: 0,
            quiet_blocks: 0,
            predelay: [vec![0.0; predelay_len], vec![0.0; predelay_len]],
            predelay_write: 0,
            filters: Default::default(),
        }
    }

    /// A convolver for a generated impulse response (`hall` if unknown).
    #[must_use]
    pub fn builtin(name: &str, sample_rate: f32) -> Self {
        let (l, r) = builtin_ir(name, sample_rate)
            .or_else(|| builtin_ir("hall", sample_rate))
            .unwrap_or_default();
        Self::new(&l, &r, sample_rate)
    }

    /// Clears the tail.
    pub fn reset(&mut self) {
        for spectra in &mut self.history {
            spectra.fill((0.0, 0.0));
        }
        self.window.fill((0.0, 0.0));
        self.output.fill((0.0, 0.0));
        self.input.fill((0.0, 0.0));
        for line in &mut self.predelay {
            line.fill(0.0);
        }
        self.filters = Default::default();
    }

    /// Convolves the newest full input block.
    fn run_block(&mut self) {
        let silent = self
            .input
            .iter()
            .all(|(l, r)| l.abs() < 1e-9 && r.abs() < 1e-9);
        self.quiet_blocks = if silent { self.quiet_blocks + 1 } else { 0 };
        if self.quiet_blocks > self.partitions + 1 {
            // Everything has rung out and nothing new came in.
            self.output.fill((0.0, 0.0));
            return;
        }
        // Slide the window: [previous block, new block].
        self.window.copy_within(BLOCK.., 0);
        if let Some(tail) = self.window.get_mut(BLOCK..) {
            tail.copy_from_slice(&self.input);
        }
        let count = self.partitions;
        self.head = (self.head + 1) % count;
        self.scratch.copy_from_slice(&self.window);
        self.fft.forward(&mut self.scratch);
        let range = self.head * BINS..(self.head + 1) * BINS;
        let [hl, hr] = &mut self.history;
        if let (Some(l), Some(r)) = (hl.get_mut(range.clone()), hr.get_mut(range)) {
            split(&self.scratch, l, r);
        }
        let [acc_l, acc_r] = &mut self.accumulator;
        acc_l.fill((0.0, 0.0));
        acc_r.fill((0.0, 0.0));
        for p in 0..count {
            let index = (self.head + count - p) % count;
            let x = index * BINS..(index + 1) * BINS;
            let h = p * BINS..(p + 1) * BINS;
            let [xl, xr] = &self.history;
            let [il, ir] = &self.ir;
            if let (Some(xl), Some(xr), Some(il), Some(ir)) =
                (xl.get(x.clone()), xr.get(x), il.get(h.clone()), ir.get(h))
            {
                mac(acc_l, xl, il);
                mac(acc_r, xr, ir);
            }
        }
        join(acc_l, acc_r, &mut self.scratch);
        self.fft.inverse(&mut self.scratch);
        if let Some(valid) = self.scratch.get(BLOCK..) {
            self.output.copy_from_slice(valid);
        }
    }

    /// Processes a block in place.
    pub fn process(&mut self, left: &mut [f32], right: &mut [f32], p: &[f32], ctx: ProcessContext) {
        let param = |i: usize, default: f32| p.get(i).copied().unwrap_or(default);
        let mix = param(0, 0.3).clamp(0.0, 1.0);
        let len = self.predelay[0].len();
        let delay = ((param(1, 0.0).clamp(0.0, MAX_PREDELAY_MS) * 0.001 * ctx.sample_rate)
            as usize)
            .min(len.saturating_sub(1));
        let output = db_to_gain(param(4, 0.0));
        let [high_pass, low_pass] = &mut self.filters;
        for f in high_pass.iter_mut() {
            f.set(param(2, 20.0), 0.1, ctx.sample_rate);
        }
        for f in low_pass.iter_mut() {
            f.set(param(3, 20_000.0), 0.1, ctx.sample_rate);
        }
        for (l, r) in left.iter_mut().zip(right.iter_mut()) {
            // Pre-delay, then into the convolution.
            for (line, x) in self.predelay.iter_mut().zip([*l, *r]) {
                if let Some(slot) = line.get_mut(self.predelay_write) {
                    *slot = x;
                }
            }
            let read = (self.predelay_write + len - delay) % len.max(1);
            let delayed = [
                self.predelay[0].get(read).copied().unwrap_or(0.0),
                self.predelay[1].get(read).copied().unwrap_or(0.0),
            ];
            self.predelay_write = (self.predelay_write + 1) % len.max(1);
            if let Some(slot) = self.input.get_mut(self.fill) {
                *slot = (delayed[0], delayed[1]);
            }
            let (wl, wr) = self.output.get(self.fill).copied().unwrap_or((0.0, 0.0));
            self.fill += 1;
            if self.fill >= BLOCK {
                self.fill = 0;
                self.run_block();
            }
            let [[hl, hr], [ll, lr]] = &mut self.filters;
            let wl = ll.process(hl.process(wl, SvfMode::HighPass), SvfMode::LowPass);
            let wr = lr.process(hr.process(wr, SvfMode::HighPass), SvfMode::LowPass);
            *l = (*l * (1.0 - mix) + wl * mix) * output;
            *r = (*r * (1.0 - mix) + wr * mix) * output;
        }
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

    #[test]
    fn convolving_with_a_delayed_impulse_delays_the_input() {
        // IR: a single tap 700 samples in, left only scaled 1, right 0.5.
        let mut l_ir = vec![0.0; 1_000];
        let mut r_ir = vec![0.0; 1_000];
        l_ir[700] = 1.0;
        r_ir[700] = 0.5;
        let mut conv = Convolver::new(&l_ir, &r_ir, 48_000.0);
        let mut l = vec![0.0; 4_096];
        let mut r = vec![0.0; 4_096];
        l[10] = 1.0;
        r[10] = 1.0;
        conv.process(&mut l, &mut r, &[1.0, 0.0, 10.0, 20_000.0, 0.0], ctx());
        let peak = |v: &[f32]| {
            v.iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(i, x)| (i, *x))
                .unwrap_or_default()
        };
        let (li, lv) = peak(&l);
        let (ri, rv) = peak(&r);
        // Block latency plus the tap.
        assert_eq!(li, 10 + 700 + BLOCK);
        assert_eq!(ri, li);
        assert!((rv / lv - 0.5).abs() < 0.05, "right is half: {lv} {rv}");
    }

    #[test]
    fn builtin_rooms_have_tails() {
        for name in BUILTIN_IRS {
            let mut conv = Convolver::builtin(name, 48_000.0);
            let mut l = vec![0.0; 48_000];
            let mut r = vec![0.0; 48_000];
            l[0] = 1.0;
            r[0] = 1.0;
            conv.process(&mut l, &mut r, &[1.0, 0.0, 20.0, 20_000.0, 0.0], ctx());
            let energy: f32 = l[2_000..].iter().map(|s| s * s).sum();
            assert!(energy > 1e-4, "{name} has a tail");
            assert!(
                l.iter().chain(&r).all(|s| s.is_finite() && s.abs() < 2.0),
                "{name}"
            );
        }
    }
}
