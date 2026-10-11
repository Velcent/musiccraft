//! CLAP plugin hosting for MusicCraft.
//!
//! This is the one MusicCraft crate allowed to use `unsafe` (see
//! `ARCHITECTURE.md`): hosting a plugin means calling a shared library through
//! the C ABI. All of it sits behind safe types — [`ClapHost`] implements
//! `musiccraft_render::PluginFactory`, and each plugin is split into a
//! main-thread `PluginInstance` and an audio-thread `musiccraft_engine::External`.
//!
//! Conventions: every `unsafe` block states why it is sound; plugin function
//! pointers are checked before use; main-thread CLAP functions are called
//! only from the thread that created the instance and audio-thread functions
//! only from the processor; the audio-thread half never allocates.
#![allow(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

#[cfg(unix)]
mod fd;
mod gui;
mod host;
mod instance;
mod library;
mod processor;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use clap_sys::entry::clap_plugin_entry;
use musiccraft_render::{LivePlugin, PluginFactory, PluginInfo, PluginKind, PluginRequest};

use crate::library::{Descriptor, Library};

/// Folders searched for `.clap` files on this platform, plus `CLAP_PATH`.
#[must_use]
pub fn search_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::env::var_os("CLAP_PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        if let Some(home) = &home {
            paths.push(home.join("Library/Audio/Plug-Ins/CLAP"));
        }
        paths.push("/Library/Audio/Plug-Ins/CLAP".into());
    } else if cfg!(windows) {
        if let Some(common) = std::env::var_os("COMMONPROGRAMFILES") {
            paths.push(PathBuf::from(common).join("CLAP"));
        }
        if let Some(local) = std::env::var_os("LOCALAPPDATA") {
            paths.push(PathBuf::from(local).join("Programs/Common/CLAP"));
        }
    } else {
        if let Some(home) = &home {
            paths.push(home.join(".clap"));
        }
        paths.push("/usr/lib/clap".into());
        paths.push("/usr/local/lib/clap".into());
    }
    paths
}

/// Every `.clap` file (or bundle) under `dir`, a few levels deep.
fn find_plugins(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for path in entries {
        let is_clap = path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("clap"));
        if is_clap {
            out.push(path);
        } else if path.is_dir() && depth > 0 {
            find_plugins(&path, depth - 1, out);
        }
    }
}

fn kind(features: &[String]) -> Option<PluginKind> {
    let has = |f: &str| features.iter().any(|x| x == f);
    if has("instrument") {
        Some(PluginKind::Instrument)
    } else if has("audio-effect") || has("analyzer") {
        Some(PluginKind::Effect)
    } else {
        None
    }
}

struct Entry {
    info: PluginInfo,
    library: Arc<Library>,
}

#[derive(Default)]
struct State {
    scanned: bool,
    /// Folders searched besides the host's own.
    extra: Vec<PathBuf>,
    /// Loaded libraries by file; kept for the life of the host.
    libraries: BTreeMap<PathBuf, Arc<Library>>,
    catalog: Vec<Entry>,
    errors: Vec<String>,
}

/// Finds and creates CLAP plugins. Scans lazily, on first use.
pub struct ClapHost {
    folders: Vec<PathBuf>,
    /// Entries linked into the program (tests), with a display path.
    linked: Vec<(&'static clap_plugin_entry, PathBuf)>,
    state: Mutex<State>,
}

impl Default for ClapHost {
    fn default() -> Self {
        Self::new()
    }
}

impl ClapHost {
    /// A host scanning the standard folders and `CLAP_PATH`.
    #[must_use]
    pub fn new() -> Self {
        Self::with_folders(search_paths())
    }

    /// A host scanning only `folders`.
    #[must_use]
    pub fn with_folders(folders: Vec<PathBuf>) -> Self {
        Self {
            folders,
            linked: Vec::new(),
            state: Mutex::new(State::default()),
        }
    }

    /// A host offering only the plugins of an entry linked into this program.
    #[must_use]
    pub fn with_linked(entry: &'static clap_plugin_entry, name: &str) -> Self {
        Self {
            folders: Vec::new(),
            linked: vec![(entry, PathBuf::from(name))],
            state: Mutex::new(State::default()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn scan(&self, state: &mut State) {
        state.catalog.clear();
        state.errors.clear();
        let mut libraries: Vec<Arc<Library>> = Vec::new();
        for (entry, name) in &self.linked {
            let cached = state.libraries.get(name).cloned();
            match cached.map_or_else(|| Library::from_static(entry, name).map(Arc::new), Ok) {
                Ok(library) => {
                    state.libraries.insert(name.clone(), Arc::clone(&library));
                    libraries.push(library);
                }
                Err(err) => state.errors.push(err),
            }
        }
        let mut files = Vec::new();
        for folder in self.folders.iter().chain(&state.extra.clone()) {
            find_plugins(folder, 4, &mut files);
        }
        files.sort();
        files.dedup();
        for file in files {
            let cached = state.libraries.get(&file).cloned();
            match cached.map_or_else(|| Library::load(&file).map(Arc::new), Ok) {
                Ok(library) => {
                    state.libraries.insert(file, Arc::clone(&library));
                    libraries.push(library);
                }
                Err(err) => state.errors.push(err),
            }
        }
        for library in libraries {
            for Descriptor {
                id,
                name,
                vendor,
                version,
                features,
            } in library.descriptors()
            {
                let Some(kind) = kind(&features) else {
                    continue;
                };
                if state.catalog.iter().any(|e| e.info.id == id) {
                    continue;
                }
                state.catalog.push(Entry {
                    info: PluginInfo {
                        id,
                        name,
                        vendor,
                        version,
                        kind,
                        path: library.path().to_path_buf(),
                    },
                    library: Arc::clone(&library),
                });
            }
        }
        state.catalog.sort_by_key(|e| e.info.name.to_lowercase());
        state.scanned = true;
    }

    fn ensure_scanned(&self) -> std::sync::MutexGuard<'_, State> {
        let mut state = self.lock();
        if !state.scanned {
            self.scan(&mut state);
        }
        state
    }
}

impl PluginFactory for ClapHost {
    fn catalog(&self) -> Vec<PluginInfo> {
        self.ensure_scanned()
            .catalog
            .iter()
            .map(|e| e.info.clone())
            .collect()
    }

    fn rescan(&self) {
        let mut state = self.lock();
        self.scan(&mut state);
    }

    fn set_extra_folders(&self, folders: Vec<PathBuf>) {
        self.lock().extra = folders;
    }

    fn scan_errors(&self) -> Vec<String> {
        self.lock().errors.clone()
    }

    fn instantiate(&self, request: &PluginRequest<'_>) -> Result<LivePlugin, String> {
        let library = self
            .ensure_scanned()
            .catalog
            .iter()
            .find(|e| e.info.id == request.id)
            .map(|e| Arc::clone(&e.library))
            .ok_or("not installed")?;
        instance::create(
            library,
            request.id,
            request.sample_rate,
            request.max_frames,
            request.state,
        )
    }
}

#[cfg(test)]
mod tests;
