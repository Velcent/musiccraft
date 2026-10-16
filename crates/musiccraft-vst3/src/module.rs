//! Opening VST3 modules (bundles or single files) and reading their class
//! lists.

use std::ffi::c_char;
use std::path::{Path, PathBuf};

use vst3::ComPtr;
use vst3::Steinberg::{
    IPluginFactory, IPluginFactory2, IPluginFactory2Trait, IPluginFactoryTrait, PClassInfo,
    PClassInfo2, PFactoryInfo, TUID, kResultOk,
};

/// `GetPluginFactory`, the one function every VST3 module exports.
pub type GetFactory = unsafe extern "system" fn() -> *mut IPluginFactory;

/// Reads a fixed-size, nul-terminated C string field.
pub(crate) fn field(chars: &[c_char]) -> String {
    let bytes: Vec<u8> = chars
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Reads a fixed-size, nul-terminated UTF-16 field.
pub(crate) fn wide(chars: &[u16]) -> String {
    let end = chars.iter().position(|c| *c == 0).unwrap_or(chars.len());
    String::from_utf16_lossy(chars.get(..end).unwrap_or_default())
}

/// What a module says about one of its classes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ClassInfo {
    pub cid: TUID,
    pub category: String,
    pub name: String,
    pub sub_categories: String,
    pub vendor: String,
    pub version: String,
}

/// The id MusicCraft gives a class: `vst3:` and 32 hex digits.
pub(crate) fn class_id(cid: &TUID) -> String {
    let hex: String = cid.iter().map(|b| format!("{:02X}", *b as u8)).collect();
    format!("vst3:{hex}")
}

/// A loaded VST3 module and its factory. Stays loaded while any instance
/// created from it is alive; each instance holds an `Arc` to it.
pub(crate) struct Module {
    path: PathBuf,
    /// Dropped (released) before the module exit function runs.
    factory: Option<ComPtr<IPluginFactory>>,
    exit: Option<unsafe extern "system" fn() -> bool>,
    // Unloaded last, after `exit` runs in `Drop`.
    _library: Option<libloading::Library>,
}

impl std::fmt::Debug for Module {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Module").field("path", &self.path).finish()
    }
}

/// The architecture folder name inside a bundle.
fn arch() -> &'static str {
    match std::env::consts::ARCH {
        "x86" => "x86",
        "aarch64" => "aarch64",
        "arm" => "armv7l",
        _ => "x86_64",
    }
}

/// The shared object inside a plugin file: bundles keep it under `Contents`.
pub(crate) fn binary_path(path: &Path) -> PathBuf {
    if !path.is_dir() {
        return path.to_path_buf();
    }
    let stem = path.file_stem().unwrap_or_default().to_string_lossy();
    let contents = path.join("Contents");
    if cfg!(target_os = "macos") {
        contents.join("MacOS").join(stem.as_ref())
    } else if cfg!(windows) {
        contents
            .join(format!("{}-win", arch()))
            .join(format!("{stem}.vst3"))
    } else {
        contents
            .join(format!("{}-linux", arch()))
            .join(format!("{stem}.so"))
    }
}

impl Module {
    /// Loads a `.vst3` bundle or file and runs its entry function.
    pub fn load(path: &Path) -> Result<Self, String> {
        if cfg!(target_os = "macos") {
            return Err(format!(
                "{}: loading VST3 bundles on macOS is not supported yet",
                path.display()
            ));
        }
        let binary = binary_path(path);
        let library = open(&binary)?;
        // SAFETY: every VST3 module exports `GetPluginFactory` with this
        // signature.
        let get = unsafe { library.get::<GetFactory>(b"GetPluginFactory\0") }
            .map(|symbol| *symbol)
            .map_err(|_| format!("{} is not a VST3 module", path.display()))?;
        let exit = exit_function(&library);
        let mut module = Self {
            path: path.to_path_buf(),
            factory: None,
            exit,
            _library: Some(library),
        };
        module.factory = Some(factory(get, path)?);
        Ok(module)
    }

    /// Wraps a module linked into this program (tests).
    pub fn from_linked(get: GetFactory, path: &Path) -> Result<Self, String> {
        Ok(Self {
            path: path.to_path_buf(),
            factory: Some(factory(get, path)?),
            exit: None,
            _library: None,
        })
    }

    /// The file this module came from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    pub fn factory(&self) -> Option<&ComPtr<IPluginFactory>> {
        self.factory.as_ref()
    }

    /// Every class the factory offers.
    pub fn classes(&self) -> Vec<ClassInfo> {
        let Some(factory) = &self.factory else {
            return Vec::new();
        };
        // SAFETY: an all-zero info is a valid value of this plain struct.
        let mut info: PFactoryInfo = unsafe { std::mem::zeroed() };
        // SAFETY: main thread; `info` is writable.
        let vendor = if unsafe { factory.getFactoryInfo(&mut info) } == kResultOk {
            field(&info.vendor)
        } else {
            String::new()
        };
        let factory2 = factory.cast::<IPluginFactory2>();
        // SAFETY: main thread.
        let count = unsafe { factory.countClasses() };
        (0..count)
            .filter_map(|i| {
                if let Some(factory2) = &factory2 {
                    // SAFETY: an all-zero info is a valid value.
                    let mut info: PClassInfo2 = unsafe { std::mem::zeroed() };
                    // SAFETY: `i` is below the count; `info` is writable.
                    if unsafe { factory2.getClassInfo2(i, &mut info) } == kResultOk {
                        let class_vendor = field(&info.vendor);
                        return Some(ClassInfo {
                            cid: info.cid,
                            category: field(&info.category),
                            name: field(&info.name),
                            sub_categories: field(&info.subCategories),
                            vendor: if class_vendor.is_empty() {
                                vendor.clone()
                            } else {
                                class_vendor
                            },
                            version: field(&info.version),
                        });
                    }
                }
                // SAFETY: an all-zero info is a valid value.
                let mut info: PClassInfo = unsafe { std::mem::zeroed() };
                // SAFETY: `i` is below the count; `info` is writable.
                (unsafe { factory.getClassInfo(i, &mut info) } == kResultOk).then(|| ClassInfo {
                    cid: info.cid,
                    category: field(&info.category),
                    name: field(&info.name),
                    sub_categories: String::new(),
                    vendor: vendor.clone(),
                    version: String::new(),
                })
            })
            .collect()
    }
}

/// Calls `GetPluginFactory`.
fn factory(get: GetFactory, path: &Path) -> Result<ComPtr<IPluginFactory>, String> {
    // SAFETY: the module is loaded and its entry function (if any) has run;
    // the factory comes with one reference, which `ComPtr` takes over.
    unsafe { ComPtr::from_raw(get()) }
        .ok_or_else(|| format!("{}: the module has no plugin factory", path.display()))
}

/// Opens the shared library and runs the platform entry function.
#[cfg(unix)]
fn open(binary: &Path) -> Result<libloading::Library, String> {
    use libloading::os::unix::{Library, RTLD_LOCAL, RTLD_NOW};
    // SAFETY: loading a plugin library runs its initializers. That is the
    // point of hosting plugins; the user installed this file as a plugin.
    let library = unsafe { Library::open(Some(binary), RTLD_NOW | RTLD_LOCAL) }
        .map_err(|e| format!("cannot load {}: {e}", binary.display()))?;
    // SAFETY: Linux VST3 modules may export `ModuleEntry`, which takes the
    // library's own handle.
    let entry = unsafe {
        library.get::<unsafe extern "system" fn(*mut std::ffi::c_void) -> bool>(b"ModuleEntry\0")
    }
    .ok()
    .map(|symbol| *symbol);
    let handle = library.into_raw();
    if let Some(entry) = entry {
        // SAFETY: called once, before anything else, with the handle the
        // library was opened with.
        if !unsafe { entry(handle) } {
            // SAFETY: `handle` came from `into_raw` above.
            drop(unsafe { Library::from_raw(handle) });
            return Err(format!("{}: the module failed to start", binary.display()));
        }
    }
    // SAFETY: `handle` came from `into_raw` above and is used once.
    Ok(unsafe { Library::from_raw(handle) }.into())
}

#[cfg(windows)]
fn open(binary: &Path) -> Result<libloading::Library, String> {
    // SAFETY: as on Unix: loading a plugin runs its initializers.
    let library = unsafe { libloading::Library::new(binary) }
        .map_err(|e| format!("cannot load {}: {e}", binary.display()))?;
    // SAFETY: Windows VST3 modules may export `InitDll`, taking nothing.
    if let Ok(entry) = unsafe { library.get::<unsafe extern "system" fn() -> bool>(b"InitDll\0") } {
        // SAFETY: called once, before anything else.
        if !unsafe { entry() } {
            return Err(format!("{}: the module failed to start", binary.display()));
        }
    }
    Ok(library)
}

/// The module's exit function, if it has one.
fn exit_function(library: &libloading::Library) -> Option<unsafe extern "system" fn() -> bool> {
    let name: &[u8] = if cfg!(windows) {
        b"ExitDll\0"
    } else {
        b"ModuleExit\0"
    };
    // SAFETY: the exit functions take nothing and return a bool.
    unsafe { library.get::<unsafe extern "system" fn() -> bool>(name) }
        .ok()
        .map(|symbol| *symbol)
}

impl Drop for Module {
    fn drop(&mut self) {
        // Release the factory first: the module may not be used after exit.
        self.factory = None;
        if let Some(exit) = self.exit {
            // SAFETY: the entry function ran in `load`, and every instance
            // holds an `Arc` to this module, so none is alive now.
            unsafe { exit() };
        }
    }
}
