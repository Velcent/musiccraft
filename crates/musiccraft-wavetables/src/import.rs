//! Tables from WAV files: wavetables, single cycles and sliced audio.

use std::io::Cursor;
use std::path::Path;

use musiccraft_dsp::pitch::PitchDetector;

use crate::{FRAME_LEN, MAX_FRAMES, Table, resample};

/// A file of at most this many samples is a single cycle under [`Slicing::Auto`].
const SINGLE_CYCLE_MAX: usize = 4096;

/// How a file is cut into frames.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Slicing {
    /// Decide from the file (see the spec).
    #[default]
    Auto,
    /// Consecutive frames of this many samples.
    Wavetable(usize),
    /// Frames of this many samples, at most 256 spread across the file.
    Fixed(usize),
    /// One period of the detected pitch per frame.
    Pitch,
    /// The whole file is one frame.
    SingleCycle,
}

impl Slicing {
    /// Parses `auto`, `single`, `pitch`, `fixed:<len>` or `frames:<len>`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        let len = |s: &str| s.parse::<usize>().ok().filter(|&n| n >= 2);
        match text {
            "auto" => Some(Self::Auto),
            "single" => Some(Self::SingleCycle),
            "pitch" => Some(Self::Pitch),
            _ => {
                if let Some(n) = text.strip_prefix("fixed:") {
                    len(n).map(Self::Fixed)
                } else {
                    text.strip_prefix("frames:")
                        .and_then(len)
                        .map(Self::Wavetable)
                }
            }
        }
    }
}

/// Reads and slices a WAV file.
///
/// # Errors
///
/// The file cannot be read or decoded, or yields no frame.
pub fn from_wav_file(path: &Path, slicing: Slicing) -> Result<Table, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    from_wav(&bytes, slicing)
}

/// Slices a WAV file held in memory.
///
/// # Errors
///
/// The bytes are not a WAV file `hound` decodes, or yield no frame.
pub fn from_wav(bytes: &[u8], slicing: Slicing) -> Result<Table, String> {
    let (mono, rate) = decode(bytes)?;
    let slicing = match slicing {
        Slicing::Auto => auto(bytes, mono.len()),
        other => other,
    };
    slice(&mono, rate, slicing)
}

/// Slices mono audio at `rate` (used only for pitch detection).
///
/// # Errors
///
/// No whole frame, or no detectable pitch.
pub fn slice(mono: &[f32], rate: f32, slicing: Slicing) -> Result<Table, String> {
    let frames: Vec<Vec<f32>> = match slicing {
        Slicing::Auto | Slicing::Wavetable(_) => {
            let len = match slicing {
                Slicing::Wavetable(len) => len.max(2),
                _ => FRAME_LEN,
            };
            mono.chunks_exact(len)
                .take(MAX_FRAMES)
                .map(|chunk| resample::frame(chunk, 0.0, len as f64, true))
                .collect()
        }
        Slicing::Fixed(len) => spread(mono, len.max(2) as f64, false),
        Slicing::Pitch => {
            let hz = detect_pitch(mono, rate).ok_or("no pitch found in the file")?;
            spread(mono, f64::from(rate / hz), false)
        }
        Slicing::SingleCycle => {
            if mono.len() < 2 {
                Vec::new()
            } else {
                vec![resample::frame(mono, 0.0, mono.len() as f64, true)]
            }
        }
    };
    if frames.is_empty() {
        return Err("the file is too short for one frame".to_owned());
    }
    Table::new(&frames.concat()).ok_or_else(|| "no whole frame".to_owned())
}

/// Frames of `len` samples: one per `len` up to 256, evenly spaced.
fn spread(mono: &[f32], len: f64, wrap: bool) -> Vec<Vec<f32>> {
    let total = mono.len() as f64;
    if len < 2.0 || total < len {
        return Vec::new();
    }
    let count = ((total / len).floor() as usize).clamp(1, MAX_FRAMES);
    let room = total - len;
    (0..count)
        .map(|i| {
            let start = if count > 1 {
                room * i as f64 / (count - 1) as f64
            } else {
                0.0
            };
            resample::frame(mono, start, len, wrap)
        })
        .collect()
}

/// The median pitch estimate over the file, in hertz.
fn detect_pitch(mono: &[f32], rate: f32) -> Option<f32> {
    let mut detector = PitchDetector::new(rate);
    let mut estimates: Vec<f32> = mono
        .iter()
        .filter_map(|&x| {
            if detector.push(x) {
                detector.pitch()
            } else {
                None
            }
        })
        .collect();
    if estimates.is_empty() {
        return None;
    }
    estimates.sort_by(f32::total_cmp);
    estimates.get(estimates.len() / 2).copied()
}

/// The [`Slicing::Auto`] decision.
fn auto(bytes: &[u8], len: usize) -> Slicing {
    if let Some(frame) = clm_frame_len(bytes) {
        Slicing::Wavetable(frame)
    } else if len % FRAME_LEN == 0 && (1..=MAX_FRAMES).contains(&(len / FRAME_LEN)) {
        Slicing::Wavetable(FRAME_LEN)
    } else if len <= SINGLE_CYCLE_MAX {
        Slicing::SingleCycle
    } else {
        Slicing::Pitch
    }
}

/// Decodes to mono (channels averaged) and the sample rate.
fn decode(bytes: &[u8]) -> Result<(Vec<f32>, f32), String> {
    let reader =
        hound::WavReader::new(Cursor::new(bytes)).map_err(|e| format!("not a WAV file: {e}"))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels.max(1));
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .collect::<Result<_, _>>()
            .map_err(|e| e.to_string())?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1_i64 << (spec.bits_per_sample.clamp(1, 32) - 1)) as f32;
            reader
                .into_samples::<i32>()
                .map(|s| s.map(|v| v as f32 * scale))
                .collect::<Result<_, _>>()
                .map_err(|e| e.to_string())?
        }
    };
    let mono = interleaved
        .chunks_exact(channels)
        .map(|frame| frame.iter().sum::<f32>() / channels as f32)
        .collect();
    Ok((mono, spec.sample_rate as f32))
}

/// The frame length in a `clm ` chunk (`<!>` and decimal digits), if any.
#[must_use]
pub fn clm_frame_len(bytes: &[u8]) -> Option<usize> {
    let mut at = 12; // after "RIFF", the size and "WAVE"
    if bytes.get(0..4)? != b"RIFF" || bytes.get(8..12)? != b"WAVE" {
        return None;
    }
    while let Some(header) = bytes.get(at..at + 8) {
        let (id, size) = header.split_at(4);
        let size = u32::from_le_bytes(size.try_into().ok()?) as usize;
        let body = bytes.get(at + 8..(at + 8).saturating_add(size))?;
        if id == b"clm " {
            let digits: String = body
                .strip_prefix(b"<!>")?
                .iter()
                .take_while(|b| b.is_ascii_digit())
                .map(|&b| char::from(b))
                .collect();
            return digits.parse().ok().filter(|&n: &usize| n >= 2);
        }
        // Chunks are padded to an even length.
        at += 8 + size + size % 2;
    }
    None
}
