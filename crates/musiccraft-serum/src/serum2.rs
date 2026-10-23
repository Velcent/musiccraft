// Ported from serum2vital's `serum2.py`
// (https://github.com/btesser/serum2vital), Copyright (C) 2026 Ben Tesser,
// GPL-3.0-or-later; see `lib.rs`.

//! Serum 2 presets (`.SerumPreset`), little-endian throughout:
//!
//! ```text
//! b"XferJson\0"
//! u64     length of the JSON header
//! bytes   JSON header (presetName, presetAuthor, tags, productVersion, …)
//! u32     size of the payload once decompressed
//! u32     payload encoding (2: zstd)
//! bytes   a zstd frame holding a CBOR document
//! ```
//!
//! The CBOR document maps module names (`Env0`, `LFO3`, `Oscillator1`,
//! `VoiceFilter0`, `ModSlot17`, `FXRack0`, …) to module state. A module's
//! `plainParams` is the text `"default"` or a map of only the parameters that
//! differ from their defaults, in real units (seconds, Hz, dB, percent).

use std::io::Read;

use crate::Error;
use crate::cbor::{self, Value};

const MAGIC: &[u8; 9] = b"XferJson\0";
const ENCODING_ZSTD: u32 = 2;
/// The largest payload accepted (Serum 2 states are well under this).
const MAX_PAYLOAD: usize = 256 << 20;

/// A parsed Serum 2 preset.
#[derive(Debug, Clone, PartialEq)]
pub struct Serum2Patch {
    pub name: String,
    pub author: String,
    pub description: String,
    pub tags: Vec<String>,
    pub product_version: String,
    /// The CBOR state: module name to module.
    pub state: Value,
}

impl Serum2Patch {
    /// Module `name`, if present.
    #[must_use]
    pub fn module(&self, name: &str) -> Option<&Value> {
        self.state.get(name)
    }

    /// The non-default parameters of `module` (empty when untouched).
    #[must_use]
    pub fn plain_params(&self, module: &str) -> &[(String, Value)] {
        self.module(module)
            .and_then(|m| m.get("plainParams"))
            .and_then(Value::as_map)
            .unwrap_or_default()
    }

    /// A number parameter of `module`, or `default`.
    #[must_use]
    pub fn param(&self, module: &str, key: &str, default: f64) -> f64 {
        lookup(self.plain_params(module), key)
            .and_then(Value::as_f64)
            .unwrap_or(default)
    }

    /// A text parameter of `module` (a menu choice), if set.
    #[must_use]
    pub fn text(&self, module: &str, key: &str) -> Option<&str> {
        lookup(self.plain_params(module), key).and_then(Value::as_str)
    }

    /// Whether `module` sets `key`.
    #[must_use]
    pub fn has(&self, module: &str, key: &str) -> bool {
        lookup(self.plain_params(module), key).is_some()
    }

    /// Oscillator `index`'s engine, e.g. `kOsc_Wavetable`.
    #[must_use]
    pub fn osc_type(&self, index: usize) -> &str {
        self.text(&format!("Oscillator{index}"), "kParamType")
            .unwrap_or("kOsc_Wavetable")
    }
}

/// The value under `key` in a parameter map.
#[must_use]
pub fn lookup<'a>(params: &'a [(String, Value)], key: &str) -> Option<&'a Value> {
    params.iter().find(|(k, _)| k == key).map(|(_, v)| v)
}

/// Whether `data` starts like a Serum 2 preset.
#[must_use]
pub fn looks_like(data: &[u8]) -> bool {
    data.starts_with(MAGIC)
}

fn u32_at(data: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(data.get(at..at + 4)?.try_into().ok()?))
}

/// Parses a Serum 2 preset; `stem` names it when the file does not.
///
/// # Errors
///
/// No `XferJson` header, a bad JSON header, an unknown encoding, or a payload
/// that does not decompress to a CBOR map.
pub fn read(data: &[u8], stem: &str) -> Result<Serum2Patch, Error> {
    if !looks_like(data) {
        return Err(Error::NotSerum("no XferJson header".to_owned()));
    }
    let cut = || Error::Bad("the preset is cut off".to_owned());
    let json_len = data
        .get(MAGIC.len()..MAGIC.len() + 8)
        .and_then(|b| b.try_into().ok())
        .map(u64::from_le_bytes)
        .ok_or_else(cut)?;
    let header_end = MAGIC.len() + 8;
    let json_end = usize::try_from(json_len)
        .ok()
        .and_then(|n| header_end.checked_add(n))
        .filter(|&end| end <= data.len())
        .ok_or_else(cut)?;
    let meta: serde_json::Value = serde_json::from_slice(&data[header_end..json_end])
        .map_err(|e| Error::Bad(format!("bad JSON header: {e}")))?;
    let raw_size = u32_at(data, json_end).ok_or_else(cut)? as usize;
    let encoding = u32_at(data, json_end + 4).ok_or_else(cut)?;
    if encoding != ENCODING_ZSTD {
        return Err(Error::Bad(format!(
            "unsupported payload encoding {encoding}"
        )));
    }
    if raw_size > MAX_PAYLOAD {
        return Err(Error::Bad("the preset payload is too large".to_owned()));
    }
    let payload = data.get(json_end + 8..).ok_or_else(cut)?;
    let mut decoder = ruzstd::decoding::StreamingDecoder::new(payload)
        .map_err(|e| Error::Bad(format!("the payload is not zstd data: {e}")))?;
    let mut blob = Vec::with_capacity(raw_size);
    decoder
        .by_ref()
        .take(MAX_PAYLOAD as u64)
        .read_to_end(&mut blob)
        .map_err(|e| Error::Bad(format!("the payload does not decompress: {e}")))?;
    let state = cbor::decode(&blob).map_err(Error::Bad)?;
    if state.as_map().is_none() {
        return Err(Error::Bad("the payload is not a CBOR map".to_owned()));
    }
    let text = |key: &str| {
        meta.get(key)
            .and_then(serde_json::Value::as_str)
            .unwrap_or_default()
            .to_owned()
    };
    let mut name = text("presetName");
    if name.is_empty() {
        name = state
            .get("presetName")
            .and_then(Value::as_str)
            .unwrap_or(stem)
            .to_owned();
    }
    Ok(Serum2Patch {
        name,
        author: text("presetAuthor"),
        description: text("presetDescription"),
        tags: meta
            .get("tags")
            .and_then(serde_json::Value::as_array)
            .map(|tags| {
                tags.iter()
                    .filter_map(serde_json::Value::as_str)
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
        product_version: text("productVersion"),
        state,
    })
}
