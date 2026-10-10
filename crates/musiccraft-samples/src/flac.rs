//! FLAC encoder (lossless), following the published format specification
//! (Xiph.Org; RFC 9639). Each 4096-frame block stores each channel as a constant
//! or as the best fixed polynomial predictor (orders 0–4) with partitioned Rice
//! coding of the residual.

use std::path::Path;

use crate::SampleError;

const BLOCK_SIZE: usize = 4096;
const MAX_PARTITION_ORDER: u32 = 6;

/// Appends bits most-significant first.
#[derive(Default)]
struct BitWriter {
    bytes: Vec<u8>,
    acc: u64,
    bits: u32,
}

impl BitWriter {
    fn put(&mut self, value: u64, bits: u32) {
        debug_assert!(bits <= 32);
        if bits == 0 {
            return;
        }
        let mask = if bits == 64 {
            u64::MAX
        } else {
            (1 << bits) - 1
        };
        self.acc = (self.acc << bits) | (value & mask);
        self.bits += bits;
        while self.bits >= 8 {
            self.bits -= 8;
            self.bytes.push((self.acc >> self.bits) as u8);
        }
        self.acc &= (1 << self.bits) - 1;
    }

    fn put_signed(&mut self, value: i64, bits: u32) {
        self.put(value as u64, bits);
    }

    /// Unary zeros terminated by a one.
    fn put_unary(&mut self, mut zeros: u64) {
        while zeros >= 32 {
            self.put(0, 32);
            zeros -= 32;
        }
        self.put(1, zeros as u32 + 1);
    }

    fn align(&mut self) {
        if self.bits > 0 {
            self.put(0, 8 - self.bits);
        }
    }
}

fn crc8(data: &[u8]) -> u8 {
    let mut crc = 0u8;
    for byte in data {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 {
                (crc << 1) ^ 0x07
            } else {
                crc << 1
            };
        }
    }
    crc
}

fn crc16(data: &[u8]) -> u16 {
    let mut crc = 0u16;
    for byte in data {
        crc ^= u16::from(*byte) << 8;
        for _ in 0..8 {
            crc = if crc & 0x8000 != 0 {
                (crc << 1) ^ 0x8005
            } else {
                crc << 1
            };
        }
    }
    crc
}

/// FLAC's UTF-8-like coding of the frame number.
fn utf8_number(value: u64, out: &mut BitWriter) {
    if value < 0x80 {
        out.put(value, 8);
        return;
    }
    let mut bytes = Vec::new();
    let mut v = value;
    let mut payload_bits = 6;
    while v >= 1 << payload_bits {
        bytes.push(0x80 | (v & 0x3f) as u8);
        v >>= 6;
        payload_bits -= 1;
    }
    let count = bytes.len() + 1;
    let lead = (0xff_u16 << (8 - count)) as u8 | v as u8;
    out.put(u64::from(lead), 8);
    for byte in bytes.iter().rev() {
        out.put(u64::from(*byte), 8);
    }
}

fn zigzag(r: i64) -> u64 {
    ((r << 1) ^ (r >> 63)) as u64
}

/// Residual of the fixed predictor of `order` (FLAC's orders 0–4).
fn fixed_residual(samples: &[i64], order: usize) -> Vec<i64> {
    (order..samples.len())
        .map(|i| {
            let s = |k: usize| samples[i - k];
            match order {
                0 => s(0),
                1 => s(0) - s(1),
                2 => s(0) - 2 * s(1) + s(2),
                3 => s(0) - 3 * s(1) + 3 * s(2) - s(3),
                _ => s(0) - 4 * s(1) + 6 * s(2) - 4 * s(3) + s(4),
            }
        })
        .collect()
}

/// Best Rice parameter and its cost in bits for a run of residuals.
fn rice_parameter(residual: &[i64]) -> (u32, u64) {
    let sum: u64 = residual.iter().map(|r| zigzag(*r)).sum();
    let n = residual.len().max(1) as u64;
    let mean = sum / n;
    let guess = 64 - mean.leading_zeros();
    let mut best = (0, u64::MAX);
    for k in guess.saturating_sub(1).min(14)..=(guess + 1).min(14) {
        let cost = n * (u64::from(k) + 1) + residual.iter().map(|r| zigzag(*r) >> k).sum::<u64>();
        if cost < best.1 {
            best = (k, cost);
        }
    }
    best
}

/// Partition order, Rice parameter per partition, and total cost in bits.
type Plan = (u32, Vec<u32>, u64);

/// Chooses the partition order and Rice parameters for a residual.
fn partition(residual: &[i64], block: usize, order: usize) -> (u32, Vec<u32>, u64) {
    let mut best = (0, Vec::new(), u64::MAX);
    for p in 0..=MAX_PARTITION_ORDER {
        let parts = 1usize << p;
        if block % parts != 0 || block / parts <= order {
            break;
        }
        let size = block / parts;
        let mut params = Vec::with_capacity(parts);
        let mut cost = 0;
        let mut start = 0;
        for i in 0..parts {
            let len = if i == 0 { size - order } else { size };
            let (k, c) = rice_parameter(&residual[start..start + len]);
            params.push(k);
            cost += c + 4;
            start += len;
        }
        if cost < best.2 {
            best = (p, params, cost);
        }
    }
    best
}

fn write_subframe(out: &mut BitWriter, samples: &[i64], bps: u32) {
    if samples.iter().all(|s| *s == samples[0]) {
        out.put(0, 1);
        out.put(0b000000, 6);
        out.put(0, 1);
        out.put_signed(samples[0], bps);
        return;
    }
    let block = samples.len();
    let mut best: Option<(usize, Vec<i64>, Plan)> = None;
    for order in 0..=4.min(block - 1) {
        let residual = fixed_residual(samples, order);
        let plan = partition(&residual, block, order);
        let total = plan.2 + order as u64 * u64::from(bps);
        if best
            .as_ref()
            .is_none_or(|b| total < b.2.2 + b.0 as u64 * u64::from(bps))
        {
            best = Some((order, residual, plan));
        }
    }
    let Some((order, residual, (partition_order, params, cost))) = best else {
        return;
    };
    let verbatim = block as u64 * u64::from(bps);
    if cost + order as u64 * u64::from(bps) >= verbatim {
        out.put(0, 1);
        out.put(0b000001, 6);
        out.put(0, 1);
        for s in samples {
            out.put_signed(*s, bps);
        }
        return;
    }
    out.put(0, 1);
    out.put(0b001000 | order as u64, 6);
    out.put(0, 1);
    for s in &samples[..order] {
        out.put_signed(*s, bps);
    }
    out.put(0b00, 2);
    out.put(u64::from(partition_order), 4);
    let size = block >> partition_order;
    let mut start = 0;
    for (i, k) in params.iter().enumerate() {
        let len = if i == 0 { size - order } else { size };
        out.put(u64::from(*k), 4);
        for r in &residual[start..start + len] {
            let z = zigzag(*r);
            out.put_unary(z >> k);
            out.put(z & ((1 << k) - 1), *k);
        }
        start += len;
    }
}

fn encode_frame(number: u64, channels: &[Vec<i64>], bps: u32) -> Vec<u8> {
    let block = channels[0].len();
    let mut header = BitWriter::default();
    header.put(0b1111_1111_1111_1000, 16);
    header.put(0b0111, 4);
    header.put(0b0000, 4);
    header.put(0b0001, 4);
    header.put(if bps == 16 { 0b100 } else { 0b110 }, 3);
    header.put(0, 1);
    utf8_number(number, &mut header);
    header.put(block as u64 - 1, 16);
    let crc = crc8(&header.bytes);
    header.put(u64::from(crc), 8);
    let mut frame = header;
    for channel in channels {
        write_subframe(&mut frame, channel, bps);
    }
    frame.align();
    let crc = crc16(&frame.bytes);
    frame.put(u64::from(crc), 16);
    frame.bytes
}

/// Bit depths [`write_flac`] supports.
pub const FLAC_DEPTHS: [u32; 2] = [16, 24];

/// Encodes stereo audio as FLAC bytes at 16 or 24 bits.
#[must_use]
pub fn encode_flac(left: &[f32], right: &[f32], sample_rate: u32, bits: u32) -> Vec<u8> {
    let bits = if bits == 24 { 24 } else { 16 };
    let scale = f64::from((1u32 << (bits - 1)) - 1);
    let quantize = |x: f32| {
        let x = if x.is_finite() {
            f64::from(x).clamp(-1.0, 1.0)
        } else {
            0.0
        };
        (x * scale).round() as i64
    };
    let frames = left.len().min(right.len());
    let mut out = b"fLaC".to_vec();
    let mut info = BitWriter::default();
    info.put(1, 1);
    info.put(0, 7);
    info.put(34, 24);
    info.put(BLOCK_SIZE as u64, 16);
    info.put(BLOCK_SIZE as u64, 16);
    info.put(0, 24);
    info.put(0, 24);
    info.put(u64::from(sample_rate), 20);
    info.put(1, 3);
    info.put(u64::from(bits - 1), 5);
    info.put(frames as u64 >> 32, 4);
    info.put(frames as u64 & 0xffff_ffff, 32);
    for _ in 0..4 {
        info.put(0, 32);
    }
    out.extend(info.bytes);
    for (number, start) in (0..frames).step_by(BLOCK_SIZE).enumerate() {
        let end = (start + BLOCK_SIZE).min(frames);
        let channels = [
            left[start..end]
                .iter()
                .map(|x| quantize(*x))
                .collect::<Vec<_>>(),
            right[start..end]
                .iter()
                .map(|x| quantize(*x))
                .collect::<Vec<_>>(),
        ];
        out.extend(encode_frame(number as u64, &channels, bits));
    }
    out
}

/// Writes stereo audio to a FLAC file at 16 or 24 bits.
///
/// # Errors
///
/// Returns [`SampleError::Io`] if the file cannot be written.
pub fn write_flac(
    path: &Path,
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
    bits: u32,
) -> Result<(), SampleError> {
    std::fs::write(path, encode_flac(left, right, sample_rate, bits))
        .map_err(|e| SampleError::Io(format!("cannot write {}: {e}", path.display())))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_signal(frames: usize) -> (Vec<f32>, Vec<f32>) {
        let left: Vec<f32> = (0..frames)
            .map(|i| (i as f32 * 0.03).sin() * 0.6 + ((i * 7919) % 13) as f32 * 0.001)
            .collect();
        let mut right: Vec<f32> = left.iter().map(|x| -x * 0.5).collect();
        // A silent stretch exercises constant subframes.
        for x in right.iter_mut().take(5_000) {
            *x = 0.0;
        }
        (left, right)
    }

    #[test]
    fn checksums_match_reference_values() {
        assert_eq!(crc8(b"123456789"), 0xf4);
        assert_eq!(crc16(b"123456789"), 0xfee8);
    }

    #[test]
    fn frame_numbers_use_flac_utf8() {
        for (value, bytes) in [
            (0x7f_u64, vec![0x7f]),
            (0x80, vec![0xc2, 0x80]),
            (0x800, vec![0xe0, 0xa0, 0x80]),
        ] {
            let mut w = BitWriter::default();
            utf8_number(value, &mut w);
            assert_eq!(w.bytes, bytes, "{value:#x}");
        }
    }

    #[test]
    fn output_starts_with_streaminfo_and_compresses() {
        let (left, right) = test_signal(20_000);
        let data = encode_flac(&left, &right, 44_100, 16);
        assert_eq!(&data[..4], b"fLaC");
        assert_eq!(data[4], 0x80, "last metadata block, STREAMINFO");
        assert!(data.len() < 20_000 * 4, "smaller than raw 16-bit PCM");
    }

    /// Decodes with the reference `flac` tool, when installed, and compares
    /// every sample with what was encoded.
    #[test]
    fn reference_decoder_reproduces_the_samples() {
        if !Path::new("/usr/bin/flac").exists() {
            return;
        }
        let dir = std::env::temp_dir().join(format!("musiccraft-flac-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        for bits in FLAC_DEPTHS {
            let (left, right) = test_signal(10_000);
            let flac = dir.join(format!("t{bits}.flac"));
            let raw = dir.join(format!("t{bits}.raw"));
            write_flac(&flac, &left, &right, 48_000, bits).unwrap();
            let status = std::process::Command::new("/usr/bin/flac")
                .args([
                    "-s",
                    "-d",
                    "-f",
                    "--force-raw-format",
                    "--endian=little",
                    "--sign=signed",
                    "-o",
                ])
                .arg(&raw)
                .arg(&flac)
                .status()
                .unwrap();
            assert!(status.success(), "flac rejected the {bits}-bit stream");
            let bytes = std::fs::read(&raw).unwrap();
            let width = (bits / 8) as usize;
            let decoded: Vec<i64> = bytes
                .chunks_exact(width)
                .map(|c| {
                    let mut v = 0i64;
                    for (i, b) in c.iter().enumerate() {
                        v |= i64::from(*b) << (8 * i);
                    }
                    let shift = 64 - 8 * width as u32;
                    (v << shift) >> shift
                })
                .collect();
            let scale = f64::from((1u32 << (bits - 1)) - 1);
            let expected: Vec<i64> = left
                .iter()
                .zip(&right)
                .flat_map(|(l, r)| [l, r])
                .map(|x| (f64::from(*x) * scale).round() as i64)
                .collect();
            assert_eq!(decoded, expected, "{bits}-bit round trip");
        }
        std::fs::remove_dir_all(dir).unwrap();
    }
}
