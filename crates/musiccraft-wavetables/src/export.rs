//! Tables as WAV files, readable by our import and by other synths that
//! read the `clm ` frame-length convention.

use std::path::Path;

use crate::{FRAME_LEN, Table};

/// The sample rate written (a wavetable has no real rate).
const RATE: u32 = 48_000;

/// A mono 32-bit float WAV of every frame, with a `clm ` chunk.
#[must_use]
pub fn to_wav(table: &Table) -> Vec<u8> {
    let mut clm = format!("<!>{FRAME_LEN} MusicCraft").into_bytes();
    if clm.len() % 2 == 1 {
        clm.push(b' ');
    }
    let samples = table.samples();
    let data_len = samples.len() * 4;
    let mut out = Vec::with_capacity(64 + clm.len() + data_len);
    let chunk = |out: &mut Vec<u8>, id: &[u8; 4], len: usize| {
        out.extend_from_slice(id);
        out.extend_from_slice(&(len as u32).to_le_bytes());
    };
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&((4 + 8 + 16 + 8 + clm.len() + 8 + data_len) as u32).to_le_bytes());
    out.extend_from_slice(b"WAVE");
    chunk(&mut out, b"fmt ", 16);
    out.extend_from_slice(&3_u16.to_le_bytes()); // IEEE float
    out.extend_from_slice(&1_u16.to_le_bytes()); // mono
    out.extend_from_slice(&RATE.to_le_bytes());
    out.extend_from_slice(&(RATE * 4).to_le_bytes());
    out.extend_from_slice(&4_u16.to_le_bytes());
    out.extend_from_slice(&32_u16.to_le_bytes());
    chunk(&mut out, b"clm ", clm.len());
    out.extend_from_slice(&clm);
    chunk(&mut out, b"data", data_len);
    for x in samples {
        out.extend_from_slice(&x.to_le_bytes());
    }
    out
}

/// Writes [`to_wav`] to `path`.
///
/// # Errors
///
/// The file cannot be written.
pub fn to_wav_file(table: &Table, path: &Path) -> Result<(), String> {
    std::fs::write(path, to_wav(table)).map_err(|e| format!("cannot write {}: {e}", path.display()))
}
