// musiccraft-serum: reads Xfer Serum 1 and Serum 2 presets and converts them
// into Antiserum patches. The reading code, parameter table, menu orders and
// unit laws are a Rust port of serum2vital.
//
// serum2vital is Copyright (C) 2026 Ben Tesser
// (https://github.com/btesser/serum2vital). This port is part of MusicCraft.
// Both are free software: you can redistribute them and/or modify them under
// the terms of the GNU General Public License as published by the Free
// Software Foundation, either version 3 of the License, or (at your option)
// any later version. They are distributed in the hope that they will be
// useful, but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the GNU General
// Public License (`LICENSE`) for more details.

//! Serum preset import (`docs/spec/serum-import.md`).
//!
//! Everything known about the Serum formats comes from serum2vital, whose
//! author worked them out; this crate follows it and is not checked against
//! Serum itself (see `docs/clean-room.md`). The mapping onto Antiserum is our
//! own.
//!
//! ```no_run
//! let path = std::path::Path::new("Bass 01.fxp");
//! let converted = musiccraft_serum::import_file(path, &musiccraft_serum::Assets::detect(path))?;
//! println!("{} ({} notes)", converted.patch.name, converted.notes.len());
//! # Ok::<(), musiccraft_serum::Error>(())
//! ```

pub mod assets;
pub mod cbor;
mod convert1;
mod convert2;
mod filters;
#[doc(hidden)]
pub mod fixtures;
mod fx1;
mod fx2;
pub mod fxp;
mod mapping;
pub mod menus;
mod out;
pub mod params;
pub mod serum2;

#[cfg(test)]
mod tests;

use std::path::Path;

pub use assets::Assets;
use musiccraft_patch::Patch;

/// Why a file was not converted.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The file is not a Serum preset (another plugin's `.fxp`, a patch, …).
    NotSerum(String),
    /// A Serum preset that cannot be read.
    Bad(String),
    /// The file cannot be read at all.
    Io(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotSerum(why) => write!(f, "not a Serum preset: {why}"),
            Self::Bad(why) => write!(f, "unreadable Serum preset: {why}"),
            Self::Io(why) => f.write_str(why),
        }
    }
}

impl std::error::Error for Error {}

/// A parsed preset of either version.
#[derive(Debug, Clone, PartialEq)]
pub enum Preset {
    Serum1(Box<fxp::Serum1Patch>),
    Serum2(serum2::Serum2Patch),
}

/// A converted preset: the patch and what could not be carried over exactly.
#[derive(Debug, Clone, PartialEq)]
pub struct Conversion {
    pub patch: Patch,
    /// One line per approximated, dropped or missing feature, no repeats.
    pub notes: Vec<String>,
}

/// Whether `data` starts like a Serum 1 or Serum 2 preset. A Serum 1 check
/// only looks at the VST2 header, so another plugin's `.fxp` passes too
/// (and [`read`] then refuses it).
#[must_use]
pub fn looks_like(data: &[u8]) -> bool {
    fxp::looks_like(data) || serum2::looks_like(data)
}

/// Parses a preset of either version; `stem` names it when the file does not.
///
/// # Errors
///
/// Not a Serum preset, or one that cannot be read.
pub fn read(data: &[u8], stem: &str) -> Result<Preset, Error> {
    if serum2::looks_like(data) {
        serum2::read(data, stem).map(Preset::Serum2)
    } else if fxp::looks_like(data) {
        fxp::read(data, stem).map(|p| Preset::Serum1(Box::new(p)))
    } else {
        Err(Error::NotSerum(
            "it starts with neither CcnK nor XferJson".to_owned(),
        ))
    }
}

/// Converts a parsed preset, finding its wavetables through `assets`.
#[must_use]
pub fn convert(preset: &Preset, assets: &Assets) -> Conversion {
    let out = match preset {
        Preset::Serum1(p) => convert1::convert(p, assets),
        Preset::Serum2(p) => convert2::convert(p, assets),
    };
    out.finish()
}

/// Reads and converts the preset file at `path`.
///
/// # Errors
///
/// The file cannot be read, is not a Serum preset, or cannot be parsed.
pub fn import_file(path: &Path, assets: &Assets) -> Result<Conversion, Error> {
    let data = std::fs::read(path)
        .map_err(|e| Error::Io(format!("cannot read {}: {e}", path.display())))?;
    let stem = path
        .file_stem()
        .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    Ok(convert(&read(&data, &stem)?, assets))
}
