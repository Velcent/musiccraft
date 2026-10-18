//! The noise oscillator's generated tables (see `params::NOISE_TYPES`).

/// Samples in each noise table.
pub const NOISE_LEN: usize = 65_536;
/// The rate a noise table plays at naturally.
pub const NOISE_RATE: f32 = 48_000.0;

/// Generated noise tables, one per type. Built on the main thread.
#[derive(Debug, Clone)]
pub struct NoiseBank {
    tables: Vec<Vec<f32>>,
}

impl Default for NoiseBank {
    fn default() -> Self {
        Self::new()
    }
}

impl NoiseBank {
    /// Generates every table (deterministically).
    #[must_use]
    pub fn new() -> Self {
        let tables = vec![
            normalize(white(1)),
            normalize(pink()),
            normalize(brown()),
            normalize(crackle()),
            normalize(digital()),
        ];
        Self { tables }
    }

    /// One sample of table `kind` at position `pos` (wrapping), interpolated.
    #[must_use]
    pub fn read(&self, kind: usize, pos: f32) -> f32 {
        let Some(table) = self.tables.get(kind) else {
            return 0.0;
        };
        let x = pos.rem_euclid(NOISE_LEN as f32);
        let i = (x as usize) % NOISE_LEN;
        let t = x - x.floor();
        let a = table.get(i).copied().unwrap_or(0.0);
        let b = table.get((i + 1) % NOISE_LEN).copied().unwrap_or(0.0);
        a + (b - a) * t
    }
}

struct Rng(u32);

impl Rng {
    fn next(&mut self) -> f32 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 17;
        self.0 ^= self.0 << 5;
        (self.0 >> 8) as f32 / (1 << 23) as f32 - 1.0
    }
}

fn white(seed: u32) -> Vec<f32> {
    let mut rng = Rng(0x9e37_79b9 ^ seed);
    (0..NOISE_LEN).map(|_| rng.next()).collect()
}

/// Paul Kellet's well-known economy pink filter over white noise.
fn pink() -> Vec<f32> {
    let (mut b0, mut b1, mut b2) = (0.0_f32, 0.0_f32, 0.0_f32);
    white(2)
        .into_iter()
        .map(|w| {
            b0 = 0.99765 * b0 + w * 0.099_046;
            b1 = 0.96300 * b1 + w * 0.296_516_4;
            b2 = 0.57000 * b2 + w * 1.052_691_3;
            b0 + b1 + b2 + w * 0.1848
        })
        .collect()
}

/// Integrated white noise with a leak.
fn brown() -> Vec<f32> {
    let mut y = 0.0_f32;
    white(3)
        .into_iter()
        .map(|w| {
            y = 0.995 * y + 0.05 * w;
            y
        })
        .collect()
}

/// Sparse random clicks.
fn crackle() -> Vec<f32> {
    let mut rng = Rng(0x1234_5671);
    (0..NOISE_LEN)
        .map(|_| {
            let r = rng.next();
            if r.abs() > 0.995 {
                r.signum() * (0.3 + rng.next().abs())
            } else {
                0.0
            }
        })
        .collect()
}

/// Random steps held for 1 to 16 samples, quantized to 16 levels.
fn digital() -> Vec<f32> {
    let mut rng = Rng(0x0bad_cafe);
    let mut out = Vec::with_capacity(NOISE_LEN);
    while out.len() < NOISE_LEN {
        let value = (rng.next() * 8.0).round() / 8.0;
        let hold = 1 + ((rng.next() + 1.0) * 7.5) as usize;
        out.extend(std::iter::repeat_n(value, hold));
    }
    out.truncate(NOISE_LEN);
    out
}

/// Removes DC and scales to a peak of 1.
fn normalize(mut table: Vec<f32>) -> Vec<f32> {
    let mean = table.iter().sum::<f32>() / table.len().max(1) as f32;
    let peak = table
        .iter()
        .map(|x| (x - mean).abs())
        .fold(0.0_f32, f32::max)
        .max(1e-9);
    for x in &mut table {
        *x = (*x - mean) / peak;
    }
    table
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_are_full_scale_and_distinct() {
        let bank = NoiseBank::new();
        assert_eq!(bank.tables.len(), 5);
        for table in &bank.tables {
            assert_eq!(table.len(), NOISE_LEN);
            let peak = table.iter().fold(0.0_f32, |m, x| m.max(x.abs()));
            assert!((peak - 1.0).abs() < 1e-4);
        }
        // Brown noise changes slowly, white noise does not.
        let roughness = |t: &[f32]| t.windows(2).map(|w| (w[1] - w[0]).abs()).sum::<f32>();
        assert!(roughness(&bank.tables[2]) < roughness(&bank.tables[0]) / 10.0);
        assert_eq!(bank.read(9, 0.0), 0.0);
    }
}
