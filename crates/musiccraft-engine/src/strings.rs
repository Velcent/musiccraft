//! Karplus–Strong strings for the pluck instrument: a tuned delay line with
//! a damping filter. Allocation happens only in `new`.

/// Capacity of a string's delay line, in frames.
pub(crate) const STRING_LEN: usize = 4096;
/// Most strings a pluck voice sounds together.
pub(crate) const MAX_STRINGS: usize = 3;

/// One string.
#[derive(Debug, Clone)]
pub(crate) struct KsString {
    line: Vec<f32>,
    write: usize,
    /// Period in frames.
    pub delay: f32,
    last: f32,
}

impl KsString {
    pub fn new() -> Self {
        Self {
            line: vec![0.0; STRING_LEN],
            write: 0,
            delay: 100.0,
            last: 0.0,
        }
    }

    /// Tunes the string to `delay` frames and fills one period with `burst`.
    pub fn excite(&mut self, delay: f32, mut burst: impl FnMut() -> f32) {
        self.delay = delay.clamp(2.0, (STRING_LEN - 4) as f32);
        for (i, slot) in self.line.iter_mut().enumerate() {
            let value = burst();
            *slot = if (i as f32) < self.delay { value } else { 0.0 };
        }
        self.write = self.delay.ceil() as usize % STRING_LEN;
        self.last = 0.0;
    }

    /// Silences the string.
    pub fn clear(&mut self) {
        self.line.fill(0.0);
        self.last = 0.0;
    }

    /// The output `delay` frames back.
    fn read(&self, delay: f32) -> f32 {
        let len = STRING_LEN as f32;
        let position = (self.write as f32 - delay).rem_euclid(len);
        let whole = position as usize;
        let frac = position - whole as f32;
        let a = self.line.get(whole % STRING_LEN).copied().unwrap_or(0.0);
        let b = self
            .line
            .get((whole + 1) % STRING_LEN)
            .copied()
            .unwrap_or(0.0);
        a + (b - a) * frac
    }

    /// Advances one frame: reads the string, damps it, scales it by the
    /// per-period `feedback`, adds `inject` (bowing) and returns the output.
    pub fn step(&mut self, damping: f32, feedback: f32, inject: f32) -> f32 {
        // The damping filter delays by `damping` frames; shorten the line to
        // match so the pitch stays true.
        let read = self.read((self.delay - damping).max(1.0));
        let filtered = read * (1.0 - damping) + self.last * damping;
        self.last = read;
        if let Some(slot) = self.line.get_mut(self.write) {
            // Each sample passes once per period, so the per-period factor
            // applies directly.
            let value = filtered * feedback + inject;
            *slot = if value.is_finite() {
                value.clamp(-4.0, 4.0)
            } else {
                0.0
            };
        }
        self.write = (self.write + 1) % STRING_LEN;
        read
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_excited_string_repeats_each_period() {
        let mut s = KsString::new();
        let mut n = 0.0;
        s.excite(100.0, || {
            n += 0.01;
            n
        });
        let out: Vec<f32> = (0..300).map(|_| s.step(0.0, 1.0, 0.0)).collect();
        assert!((out[0] - out[100]).abs() < 1e-4 && (out[5] - out[205]).abs() < 1e-4);
    }

    #[test]
    fn injection_keeps_a_lossy_string_ringing() {
        let mut s = KsString::new();
        s.excite(50.0, || 0.0);
        let mut last = 0.0_f32;
        for i in 0..20_000 {
            let push = if i % 50 == 0 { 0.05 } else { 0.0 };
            last = last.max(s.step(0.2, 0.9, push).abs() * f32::from(i > 19_000));
        }
        assert!(last > 0.05, "a steady push sustains it: {last}");
    }
}
