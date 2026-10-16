//! VST3 plugin hosting for MusicCraft.
//!
//! Like `musiccraft-clap`, this crate is exempt from the workspace's `unsafe`
//! ban (see `ARCHITECTURE.md`): hosting a VST3 plugin means loading a shared
//! library and calling COM-style interfaces through raw pointers. All of it
//! sits behind safe types — [`Vst3Host`] implements
//! `musiccraft_render::PluginFactory`, and each plugin is split into a
//! main-thread `PluginInstance` and an audio-thread
//! `musiccraft_engine::External`.
//!
//! Conventions: every `unsafe` block states why it is sound; reference counts
//! are managed by `ComPtr`/`ComWrapper`; main-thread interfaces are called
//! only from the main thread and `setProcessing`/`process` only from the
//! processor; the audio-thread half never allocates or locks.
#![allow(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]
// VST3 enum types are `int` on Windows and `unsigned int` elsewhere, so the
// same cast is needed on one platform and redundant on the other.
#![allow(clippy::unnecessary_cast)]
// Interface methods keep the VST3 names.
#![allow(non_snake_case)]

mod editor;
#[cfg(target_os = "linux")]
mod fd;
mod host;
mod instance;
mod module;
mod processor;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use musiccraft_render::{LivePlugin, PluginFactory, PluginInfo, PluginKind, PluginRequest};
use vst3::Steinberg::TUID;

pub use crate::module::GetFactory;
use crate::module::{ClassInfo, Module, class_id};

/// Folders searched for `.vst3` modules on this platform, plus `VST3_PATH`.
#[must_use]
pub fn search_paths() -> Vec<PathBuf> {
    let mut paths: Vec<PathBuf> = std::env::var_os("VST3_PATH")
        .map(|p| std::env::split_paths(&p).collect())
        .unwrap_or_default();
    let home = std::env::var_os("HOME").map(PathBuf::from);
    if cfg!(target_os = "macos") {
        if let Some(home) = &home {
            paths.push(home.join("Library/Audio/Plug-Ins/VST3"));
        }
        paths.push("/Library/Audio/Plug-Ins/VST3".into());
    } else if cfg!(windows) {
        if let Some(common) = std::env::var_os("COMMONPROGRAMFILES") {
            paths.push(PathBuf::from(common).join("VST3"));
        }
    } else {
        if let Some(home) = &home {
            paths.push(home.join(".vst3"));
        }
        paths.push("/usr/lib/vst3".into());
        paths.push("/usr/local/lib/vst3".into());
    }
    paths
}

/// Every `.vst3` bundle or file under `dir`, a few levels deep. A bundle's
/// insides are not searched.
fn find_modules(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    let mut entries: Vec<PathBuf> = entries.filter_map(|e| e.ok().map(|e| e.path())).collect();
    entries.sort();
    for path in entries {
        let is_vst3 = path
            .extension()
            .is_some_and(|x| x.eq_ignore_ascii_case("vst3"));
        if is_vst3 {
            out.push(path);
        } else if path.is_dir() && depth > 0 {
            find_modules(&path, depth - 1, out);
        }
    }
}

/// What a class is for, if it is a plugin at all.
fn kind(class: &ClassInfo) -> Option<PluginKind> {
    if class.category != "Audio Module Class" {
        return None;
    }
    let instrument = class
        .sub_categories
        .split('|')
        .any(|c| c.eq_ignore_ascii_case("Instrument"));
    Some(if instrument {
        PluginKind::Instrument
    } else {
        PluginKind::Effect
    })
}

struct Entry {
    info: PluginInfo,
    cid: TUID,
    module: Arc<Module>,
}

#[derive(Default)]
struct State {
    scanned: bool,
    /// Folders searched besides the host's own.
    extra: Vec<PathBuf>,
    /// Loaded modules by file; kept for the life of the host.
    modules: BTreeMap<PathBuf, Arc<Module>>,
    catalog: Vec<Entry>,
    errors: Vec<String>,
}

/// Finds and creates VST3 plugins. Scans lazily, on first use.
pub struct Vst3Host {
    folders: Vec<PathBuf>,
    /// Modules linked into the program (tests), with a display path.
    linked: Vec<(GetFactory, PathBuf)>,
    state: Mutex<State>,
}

impl Default for Vst3Host {
    fn default() -> Self {
        Self::new()
    }
}

impl Vst3Host {
    /// A host scanning the standard folders and `VST3_PATH`.
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

    /// A host offering only the plugins of a module linked into this program,
    /// given by its `GetPluginFactory` function.
    #[must_use]
    pub fn with_linked(get_factory: GetFactory, name: &str) -> Self {
        Self {
            folders: Vec::new(),
            linked: vec![(get_factory, PathBuf::from(name))],
            state: Mutex::new(State::default()),
        }
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }

    fn scan(&self, state: &mut State) {
        state.catalog.clear();
        state.errors.clear();
        let mut modules: Vec<Arc<Module>> = Vec::new();
        for (get, name) in &self.linked {
            let cached = state.modules.get(name).cloned();
            match cached.map_or_else(|| Module::from_linked(*get, name).map(Arc::new), Ok) {
                Ok(module) => {
                    state.modules.insert(name.clone(), Arc::clone(&module));
                    modules.push(module);
                }
                Err(err) => state.errors.push(err),
            }
        }
        let mut files = Vec::new();
        for folder in self.folders.iter().chain(&state.extra.clone()) {
            find_modules(folder, 4, &mut files);
        }
        files.sort();
        files.dedup();
        for file in files {
            let cached = state.modules.get(&file).cloned();
            match cached.map_or_else(|| Module::load(&file).map(Arc::new), Ok) {
                Ok(module) => {
                    state.modules.insert(file, Arc::clone(&module));
                    modules.push(module);
                }
                Err(err) => state.errors.push(err),
            }
        }
        for module in modules {
            for class in module.classes() {
                let Some(kind) = kind(&class) else {
                    continue;
                };
                let id = class_id(&class.cid);
                if state.catalog.iter().any(|e| e.info.id == id) {
                    continue;
                }
                state.catalog.push(Entry {
                    info: PluginInfo {
                        id,
                        name: class.name,
                        vendor: class.vendor,
                        version: class.version,
                        kind,
                        path: module.path().to_path_buf(),
                    },
                    cid: class.cid,
                    module: Arc::clone(&module),
                });
            }
        }
        state.catalog.sort_by_key(|e| e.info.name.to_lowercase());
        state.scanned = true;
    }

    /// Like `instantiate`, returning the concrete halves (window tests).
    #[cfg(all(test, target_os = "linux"))]
    fn build(
        &self,
        id: &str,
    ) -> Result<(instance::Vst3Instance, processor::Vst3Processor), String> {
        let (module, cid) = self
            .ensure_scanned()
            .catalog
            .iter()
            .find(|e| e.info.id == id)
            .map(|e| (Arc::clone(&e.module), e.cid))
            .ok_or("not installed")?;
        instance::build(module, &cid, 48_000, 256, None)
    }

    fn ensure_scanned(&self) -> std::sync::MutexGuard<'_, State> {
        let mut state = self.lock();
        if !state.scanned {
            self.scan(&mut state);
        }
        state
    }
}

impl PluginFactory for Vst3Host {
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
        let (module, cid) = self
            .ensure_scanned()
            .catalog
            .iter()
            .find(|e| e.info.id == request.id)
            .map(|e| (Arc::clone(&e.module), e.cid))
            .ok_or("not installed")?;
        instance::create(
            module,
            &cid,
            request.sample_rate,
            request.max_frames,
            request.state,
        )
    }
}

#[cfg(test)]
mod tests;
