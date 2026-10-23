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
mod probe;
mod processor;

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, PoisonError};

use musiccraft_render::{LivePlugin, PluginFactory, PluginInfo, PluginKind, PluginRequest};
use vst3::Steinberg::TUID;

pub use crate::module::GetFactory;
use crate::module::{ClassInfo, Module, class_id};
pub use crate::probe::serve_probe;

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

/// The module at `file`, loading it on first use.
fn load(state: &mut State, file: &Path) -> Result<Arc<Module>, String> {
    if let Some(module) = state.modules.get(file) {
        return Ok(Arc::clone(module));
    }
    let module = Arc::new(Module::load(file)?);
    state
        .modules
        .insert(file.to_path_buf(), Arc::clone(&module));
    Ok(module)
}

/// Probes `files` in child processes, a few at a time. `None` where a child
/// could not be started.
fn probe_all(
    command: &ProbeCommand,
    files: &[&PathBuf],
) -> Vec<Option<Result<Vec<ClassInfo>, String>>> {
    let next = AtomicUsize::new(0);
    let results = Mutex::new((0..files.len()).map(|_| None).collect::<Vec<_>>());
    let workers = std::thread::available_parallelism()
        .map_or(1, std::num::NonZero::get)
        .clamp(1, 8)
        .min(files.len());
    std::thread::scope(|scope| {
        for _ in 0..workers {
            scope.spawn(|| {
                loop {
                    let i = next.fetch_add(1, Ordering::Relaxed);
                    let Some(file) = files.get(i) else {
                        break;
                    };
                    let result = probe::probe(command(file), file).ok();
                    let mut results = results.lock().unwrap_or_else(PoisonError::into_inner);
                    if let Some(slot) = results.get_mut(i) {
                        *slot = result;
                    }
                }
            });
        }
    });
    results.into_inner().unwrap_or_else(PoisonError::into_inner)
}

struct Entry {
    info: PluginInfo,
    cid: TUID,
    /// The module's file (or a linked module's display path), the key into
    /// `State::modules`.
    file: PathBuf,
}

/// Builds the command for a probe child (the module path is added as
/// `MUSICCRAFT_VST3_PROBE`).
type ProbeCommand = Box<dyn Fn(&Path) -> Command + Send + Sync>;

/// How modules are read during a scan.
enum Scanner {
    /// Loaded into this process (tests, and programs that do not call
    /// [`serve_probe`]).
    InProcess,
    /// Probed in child processes; loaded here only when first used.
    Child(ProbeCommand),
}

#[derive(Default)]
struct State {
    scanned: bool,
    /// Folders searched besides the host's own.
    extra: Vec<PathBuf>,
    /// Loaded modules by file; kept for the life of the host. With a child
    /// scanner a module is loaded only when one of its plugins is created.
    modules: BTreeMap<PathBuf, Arc<Module>>,
    catalog: Vec<Entry>,
    errors: Vec<String>,
}

/// Finds and creates VST3 plugins. Scans lazily, on first use.
pub struct Vst3Host {
    folders: Vec<PathBuf>,
    scanner: Scanner,
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
    /// A host scanning the standard folders and `VST3_PATH`. In a program
    /// that calls [`serve_probe`] first thing, each module is scanned in a
    /// child process (this program again), so one that crashes is skipped.
    #[must_use]
    pub fn new() -> Self {
        let mut host = Self::with_folders(search_paths());
        if probe::served() {
            if let Ok(exe) = std::env::current_exe() {
                host.scanner = Scanner::Child(Box::new(move |_| Command::new(&exe)));
            }
        }
        host
    }

    /// Scans modules in child processes started from `command` (tests).
    #[cfg(test)]
    fn with_probe(mut self, command: impl Fn(&Path) -> Command + Send + Sync + 'static) -> Self {
        self.scanner = Scanner::Child(Box::new(command));
        self
    }

    /// A host scanning only `folders`.
    #[must_use]
    pub fn with_folders(folders: Vec<PathBuf>) -> Self {
        Self {
            folders,
            scanner: Scanner::InProcess,
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
            scanner: Scanner::InProcess,
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
        let mut found: Vec<(PathBuf, Vec<ClassInfo>)> = Vec::new();
        for (get, name) in &self.linked {
            let cached = state.modules.get(name).cloned();
            match cached.map_or_else(|| Module::from_linked(*get, name).map(Arc::new), Ok) {
                Ok(module) => {
                    found.push((name.clone(), module.classes()));
                    state.modules.insert(name.clone(), module);
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
        let results = self.read_modules(state, &files);
        for (file, result) in files.into_iter().zip(results) {
            match result {
                Ok(classes) => found.push((file, classes)),
                Err(err) => state.errors.push(err),
            }
        }
        for (file, classes) in found {
            for class in classes {
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
                        path: file.clone(),
                    },
                    cid: class.cid,
                    file: file.clone(),
                });
            }
        }
        state.catalog.sort_by_key(|e| e.info.name.to_lowercase());
        state.scanned = true;
    }

    /// The classes of each file, in order. Modules already loaded answer in
    /// process; the others go to the scanner.
    fn read_modules(
        &self,
        state: &mut State,
        files: &[PathBuf],
    ) -> Vec<Result<Vec<ClassInfo>, String>> {
        let probed = match &self.scanner {
            Scanner::Child(command) => {
                let fresh: Vec<&PathBuf> = files
                    .iter()
                    .filter(|f| !state.modules.contains_key(*f))
                    .collect();
                fresh
                    .iter()
                    .copied()
                    .zip(probe_all(command, &fresh))
                    .collect()
            }
            Scanner::InProcess => BTreeMap::new(),
        };
        files
            .iter()
            .map(|file| match probed.get(file) {
                Some(Some(result)) => result.clone(),
                // Not probed, or the child could not be started.
                _ => load(state, file).map(|module| module.classes()),
            })
            .collect()
    }

    /// Like `instantiate`, returning the concrete halves (window tests).
    #[cfg(all(test, target_os = "linux"))]
    fn build(
        &self,
        id: &str,
    ) -> Result<(instance::Vst3Instance, processor::Vst3Processor), String> {
        let (module, cid) = self.module_for(id)?;
        instance::build(module, &cid, 48_000, 256, None)
    }

    /// The module and class id of plugin `id`. A module that was probed in a
    /// child process is loaded here now.
    fn module_for(&self, id: &str) -> Result<(Arc<Module>, TUID), String> {
        let mut state = self.ensure_scanned();
        let (file, cid) = state
            .catalog
            .iter()
            .find(|e| e.info.id == id)
            .map(|e| (e.file.clone(), e.cid))
            .ok_or("not installed")?;
        Ok((load(&mut state, &file)?, cid))
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
        let (module, cid) = self.module_for(request.id)?;
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
