//! Synthetic Serum presets for tests: the layouts of [`crate::fxp`] and
//! [`crate::serum2`] written back out. Nothing here comes from a file Serum
//! made.

use crate::cbor::{self, Value};
use crate::params::{self, PARAMS, stored_default};

const FXP_HEADER: usize = 60;
/// Big enough that every parameter of the second block is trusted.
const BLOB_LEN: usize = 0x84D8;

/// A modulation slot for [`serum1`]: (slot 0..15, source id, destination
/// parameter name, amount −1..1).
pub type Serum1Mod<'a> = (usize, u16, &'a str, f32);

/// A Serum 1 `.fxp` named `name` with every parameter at its default except
/// `params` (stored 0..1 values), the given oscillator A table and
/// modulation slots.
#[must_use]
pub fn serum1(name: &str, params: &[(&str, f32)], table_a: &str, mods: &[Serum1Mod]) -> Vec<u8> {
    let mut blob = vec![0_u8; BLOB_LEN];
    let mut values: Vec<f32> = (0..PARAMS.len()).map(stored_default).collect();
    for &(param, value) in params {
        let index = params::index(param).unwrap_or_else(|| panic!("unknown parameter {param}"));
        values[index] = value;
    }
    for (i, v) in values.iter().enumerate() {
        let at = if i < params::LOW_BLOCK {
            0x3460 + 4 * i
        } else {
            0x4AE0 + 4 * (i - params::LOW_BLOCK)
        };
        blob[at..at + 4].copy_from_slice(&v.to_le_bytes());
    }
    put_text(&mut blob, 0x4972, name);
    put_text(&mut blob, 0x3C08, table_a);
    for (i, order) in (0_i32..10).enumerate() {
        let at = 0x3BE0 + 4 * i;
        blob[at..at + 4].copy_from_slice(&order.to_le_bytes());
    }
    for &(slot, source, dest, amount) in mods {
        let base = slot * 40;
        let dest = params::index(dest).unwrap_or_else(|| panic!("unknown parameter {dest}")) as u16;
        blob[base + 0x04..base + 0x08].copy_from_slice(&amount.to_le_bytes());
        blob[base + 0x08..base + 0x0C].copy_from_slice(&1.0_f32.to_le_bytes());
        blob[base + 0x14..base + 0x16].copy_from_slice(&source.to_le_bytes());
        blob[base + 0x1A..base + 0x1C].copy_from_slice(&dest.to_le_bytes());
        blob[base + 0x21] = 0x80;
        blob[base + 0x22] = slot as u8;
        blob[base + 0x23] = 0xFF;
    }
    let chunk = miniz_oxide::deflate::compress_to_vec_zlib(&blob, 6);
    let mut data = Vec::with_capacity(FXP_HEADER + chunk.len());
    data.extend_from_slice(b"CcnK");
    data.extend_from_slice(&((FXP_HEADER - 8 + chunk.len()) as u32).to_be_bytes());
    data.extend_from_slice(b"FPCh");
    data.extend_from_slice(&1_u32.to_be_bytes());
    data.extend_from_slice(b"XfsX");
    data.extend_from_slice(&1_u32.to_be_bytes());
    data.extend_from_slice(&1_u32.to_be_bytes());
    let mut program = [0_u8; 28];
    for (b, c) in program.iter_mut().zip(name.bytes()) {
        *b = c;
    }
    data.extend_from_slice(&program);
    data.extend_from_slice(&(chunk.len() as u32).to_be_bytes());
    data.extend_from_slice(&chunk);
    data
}

fn put_text(blob: &mut [u8], at: usize, text: &str) {
    blob[at..at + text.len()].copy_from_slice(text.as_bytes());
}

/// A Serum 2 `.SerumPreset` named `name` holding `state` (a CBOR map of
/// modules), tagged `tags`.
#[must_use]
pub fn serum2(name: &str, tags: &[&str], state: &Value) -> Vec<u8> {
    let meta = serde_json::json!({
        "fileType": "SerumPreset",
        "presetName": name,
        "presetAuthor": "MusicCraft tests",
        "tags": tags,
        "productVersion": "2.0.0",
    })
    .to_string();
    let raw = cbor::encode(state);
    let packed = ruzstd::encoding::compress_to_vec(
        raw.as_slice(),
        ruzstd::encoding::CompressionLevel::Fastest,
    );
    let mut data = b"XferJson\0".to_vec();
    data.extend_from_slice(&(meta.len() as u64).to_le_bytes());
    data.extend_from_slice(meta.as_bytes());
    data.extend_from_slice(&(raw.len() as u32).to_le_bytes());
    data.extend_from_slice(&2_u32.to_le_bytes());
    data.extend_from_slice(&packed);
    data
}

/// A CBOR map from string keys.
#[must_use]
pub fn map(entries: Vec<(&str, Value)>) -> Value {
    Value::Map(
        entries
            .into_iter()
            .map(|(k, v)| (k.to_owned(), v))
            .collect(),
    )
}

/// A module `{"plainParams": {…}}`, plus extra entries.
#[must_use]
pub fn module(params: Vec<(&str, Value)>, extra: Vec<(&str, Value)>) -> Value {
    let mut entries = vec![("plainParams", map(params))];
    entries.extend(extra);
    map(entries)
}

/// A number.
#[must_use]
pub fn n(value: f64) -> Value {
    Value::Float(value)
}

/// A text.
#[must_use]
pub fn t(value: &str) -> Value {
    Value::Text(value.to_owned())
}
