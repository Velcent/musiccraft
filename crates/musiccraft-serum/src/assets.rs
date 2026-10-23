//! Finding the wavetables a Serum preset names (`docs/spec/serum-import.md`,
//! Wavetables). The search order is our own design; the folder names are
//! where Serum keeps its data, as serum2vital's `convert.py` looks for them.

use std::cell::RefCell;
use std::collections::HashMap;
use std::f32::consts::TAU;
use std::path::{Path, PathBuf};

use musiccraft_patch::TableRef;
use musiccraft_wavetables::import::{Slicing, from_wav_file};
use musiccraft_wavetables::{FRAME_LEN, Table};

/// Names more Serum data folders (several, separated like `PATH`).
pub const FOLDER_VARIABLE: &str = "MUSICCRAFT_SERUM_FOLDER";

/// Where tables live under a Serum data folder.
const TABLE_DIRS: [&str; 2] = ["Tables", "Serum 2 Presets/Tables"];
/// What an empty table name means: Serum's default table.
const DEFAULT_TABLE: &str = "S2 Tables/Default Shapes.wav";
/// Most files indexed under one folder.
const MAX_INDEXED: usize = 50_000;

/// Where to look for wavetables, with caches shared by every preset of a
/// batch.
#[derive(Debug, Default)]
pub struct Assets {
    /// `Tables` folders next to or above the preset (searched first).
    local: Vec<PathBuf>,
    /// Serum data folders (holding `Tables/`).
    roots: Vec<PathBuf>,
    /// File name (lower case) to path, per folder.
    index: RefCell<HashMap<PathBuf, HashMap<String, PathBuf>>>,
    /// Loaded tables by path, as patch table text.
    loaded: RefCell<HashMap<PathBuf, Result<String, String>>>,
}

/// A table for an oscillator, or why it is a stand-in.
pub struct Found {
    /// `data:` table text.
    pub table: String,
    /// Set when the table is not the one the preset names.
    pub note: Option<String>,
}

impl Assets {
    /// No folders: every named table becomes a stand-in.
    #[must_use]
    pub fn none() -> Self {
        Self::default()
    }

    /// The given Serum data folders only.
    #[must_use]
    pub fn with_roots(roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            ..Self::default()
        }
    }

    /// The folders for a preset at `preset`: `Tables` folders beside it or
    /// above it, [`FOLDER_VARIABLE`], then Serum's usual data folders.
    #[must_use]
    pub fn detect(preset: &Path) -> Self {
        let mut assets = Self::with_roots(default_roots());
        assets.add_local(preset);
        assets
    }

    /// Adds the `Tables` folders beside or above `preset` (searched before
    /// the data folders), once each.
    pub fn add_local(&mut self, preset: &Path) {
        for dir in preset.ancestors().skip(1) {
            for name in ["Tables", "tables", "Wavetables", "wavetables"] {
                let tables = dir.join(name);
                if tables.is_dir() && !self.local.contains(&tables) {
                    self.local.push(tables);
                }
            }
        }
    }

    /// The table for `reference` (Serum's name for it, such as
    /// `Analog/Basic Shapes.wav`).
    #[must_use]
    pub fn table(&self, reference: &str) -> Found {
        let reference = reference.replace('\\', "/");
        let reference = reference.trim().trim_start_matches('/');
        let (wanted, default) = if reference.is_empty() {
            (DEFAULT_TABLE, true)
        } else {
            (reference, false)
        };
        if let Some(path) = self.find(wanted) {
            match self.load(&path) {
                Ok(table) => return Found { table, note: None },
                Err(why) => {
                    return Found {
                        table: builtin("saw"),
                        note: Some(format!(
                            "missing: wavetable {wanted} could not be read ({why}); a saw stands in"
                        )),
                    };
                }
            }
        }
        let stem = Path::new(wanted)
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().to_lowercase());
        if let Some(shape) = basic_shape(&stem) {
            return Found {
                table: builtin(shape),
                note: None,
            };
        }
        Found {
            table: builtin("saw"),
            // Serum's default table starts on a saw: no note for it.
            note: (!default)
                .then(|| format!("missing: wavetable {wanted} was not found; a saw stands in")),
        }
    }

    fn bases(&self) -> Vec<PathBuf> {
        let mut bases = self.local.clone();
        for root in &self.roots {
            for dir in TABLE_DIRS {
                let base = root.join(dir);
                if base.is_dir() {
                    bases.push(base);
                }
            }
        }
        bases
    }

    fn find(&self, reference: &str) -> Option<PathBuf> {
        let bases = self.bases();
        for base in &bases {
            let candidate = base.join(reference);
            if candidate.is_file() {
                return Some(candidate);
            }
        }
        // Serum records a path relative to the library the table came
        // from; tables move between packs, so fall back to the file name.
        let name = Path::new(reference)
            .file_name()?
            .to_string_lossy()
            .to_lowercase();
        let mut index = self.index.borrow_mut();
        for base in bases {
            let files = index
                .entry(base.clone())
                .or_insert_with(|| wav_files(&base));
            if let Some(found) = files.get(&name) {
                return Some(found.clone());
            }
        }
        None
    }

    fn load(&self, path: &Path) -> Result<String, String> {
        self.loaded
            .borrow_mut()
            .entry(path.to_path_buf())
            .or_insert_with(|| {
                from_wav_file(path, Slicing::Auto)
                    .map(|table| TableRef::Data(table.samples().to_vec()).to_text())
            })
            .clone()
    }
}

/// Every `.wav` file below `base`, by lower-case file name (the first one
/// found wins).
fn wav_files(base: &Path) -> HashMap<String, PathBuf> {
    let mut files = HashMap::new();
    let mut pending = vec![base.to_path_buf()];
    while let Some(dir) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        let mut entries: Vec<PathBuf> = entries.flatten().map(|e| e.path()).collect();
        entries.sort();
        for path in entries {
            if files.len() >= MAX_INDEXED {
                return files;
            }
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("wav"))
                && let Some(name) = path.file_name()
            {
                files
                    .entry(name.to_string_lossy().to_lowercase())
                    .or_insert(path);
            }
        }
    }
    files
}

/// Serum's usual data folders that exist on this computer, after the ones
/// [`FOLDER_VARIABLE`] names.
#[must_use]
pub fn default_roots() -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = std::env::var_os(FOLDER_VARIABLE)
        .map(|v| std::env::split_paths(&v).collect())
        .unwrap_or_default();
    let mut documents = Vec::new();
    if let Some(home) = std::env::var_os("HOME").or_else(|| std::env::var_os("USERPROFILE")) {
        let home = PathBuf::from(home);
        documents.push(home.join("Documents"));
        // Serum under Wine (yabridge on Linux).
        let mut prefixes = vec![home.join(".wine")];
        if let Some(prefix) = std::env::var_os("WINEPREFIX") {
            prefixes.insert(0, PathBuf::from(prefix));
        }
        for prefix in prefixes {
            if let Ok(users) = std::fs::read_dir(prefix.join("drive_c/users")) {
                let mut users: Vec<PathBuf> = users.flatten().map(|e| e.path()).collect();
                users.sort();
                documents.extend(users.into_iter().map(|u| u.join("Documents")));
            }
        }
    }
    for docs in documents {
        for name in ["Serum Presets", "Serum 2 Presets"] {
            roots.push(docs.join("Xfer").join(name));
        }
    }
    for name in ["Serum Presets", "Serum 2 Presets"] {
        roots.push(Path::new("/Library/Audio/Presets/Xfer Records").join(name));
    }
    roots.retain(|r| r.is_dir());
    roots.dedup();
    roots
}

/// Serum's built-in basic shape for a table's file stem, if it names one.
fn basic_shape(stem: &str) -> Option<&'static str> {
    Some(match stem.trim() {
        "sin" | "sine" => "sine",
        "tri" | "triangle" => "triangle",
        "square" | "sqr" => "square",
        "saw" | "sawtooth" | "saw up" | "default shapes" => "saw",
        "saw down" | "ramp down" => "saw down",
        "pulse" | "pulse width" => "pulse",
        "roundrect" | "round rect" | "rounded square" => "roundrect",
        _ => return None,
    })
}

/// One frame of a basic shape, as table text.
#[must_use]
pub fn builtin(shape: &str) -> String {
    let n = FRAME_LEN as f32;
    let frame: Vec<f32> = (0..FRAME_LEN)
        .map(|i| {
            let x = i as f32 / n;
            match shape {
                "sine" => (TAU * x).sin(),
                "triangle" => 4.0 * ((x + 0.25).fract() - 0.5).abs() - 1.0,
                "square" => {
                    if x < 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                "pulse" => {
                    if x < 0.25 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                "roundrect" => (4.0 * (TAU * x).sin()).tanh() / 4.0_f32.tanh(),
                "saw down" => 1.0 - 2.0 * x,
                _ => 2.0 * x - 1.0,
            }
        })
        .collect();
    let table = Table::new(&frame).unwrap_or_default();
    TableRef::Data(table.samples().to_vec()).to_text()
}
