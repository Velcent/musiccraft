//! Opening `.clap` files and reading their plugin factories.

use std::ffi::{CStr, CString, c_char};
use std::path::{Path, PathBuf};

use clap_sys::entry::clap_plugin_entry;
use clap_sys::factory::plugin_factory::{CLAP_PLUGIN_FACTORY_ID, clap_plugin_factory};
use clap_sys::plugin::clap_plugin_descriptor;

/// Reads a C string the plugin owns.
///
/// # Safety
///
/// `ptr` must be null or point at a nul-terminated string that stays valid
/// for the call.
pub(crate) unsafe fn text(ptr: *const c_char) -> String {
    if ptr.is_null() {
        return String::new();
    }
    // SAFETY: guaranteed by the caller.
    unsafe { CStr::from_ptr(ptr) }
        .to_string_lossy()
        .into_owned()
}

/// Reads a fixed-size, nul-terminated C string field.
pub(crate) fn field(chars: &[c_char]) -> String {
    let bytes: Vec<u8> = chars
        .iter()
        .take_while(|c| **c != 0)
        .map(|c| *c as u8)
        .collect();
    String::from_utf8_lossy(&bytes).into_owned()
}

/// What a library says about one of its plugins.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Descriptor {
    pub id: String,
    pub name: String,
    pub vendor: String,
    pub version: String,
    pub features: Vec<String>,
}

/// An initialized CLAP library. Stays loaded (and initialized) while any
/// instance created from it is alive; each instance holds an `Arc` to it.
pub(crate) struct Library {
    path: PathBuf,
    entry: *const clap_plugin_entry,
    // Dropped after `deinit` runs in `Drop`.
    _library: Option<libloading::Library>,
}

// SAFETY: `entry` points at the library's static entry structure, which is
// immutable and lives while `_library` is loaded. CLAP entry and factory
// functions may be called from any thread; MusicCraft calls them from the main
// thread only.
unsafe impl Send for Library {}
// SAFETY: as above; `Library` has no interior mutability.
unsafe impl Sync for Library {}

impl std::fmt::Debug for Library {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Library").field("path", &self.path).finish()
    }
}

/// The shared object inside a plugin file: macOS plugins are bundles.
fn binary_path(path: &Path) -> PathBuf {
    if path.is_dir() {
        let stem = path.file_stem().unwrap_or_default();
        return path.join("Contents").join("MacOS").join(stem);
    }
    path.to_path_buf()
}

impl Library {
    /// Loads and initializes a `.clap` file.
    pub fn load(path: &Path) -> Result<Self, String> {
        let binary = binary_path(path);
        // SAFETY: loading a plugin library runs its initializers. That is the
        // point of hosting plugins; the user installed this file as a plugin.
        let library = unsafe { libloading::Library::new(&binary) }
            .map_err(|e| format!("cannot load {}: {e}", binary.display()))?;
        // SAFETY: the CLAP ABI requires a `clap_entry` data symbol of type
        // `clap_plugin_entry`; asking for `*const T` yields its address.
        let entry = unsafe { library.get::<*const clap_plugin_entry>(b"clap_entry\0") }
            .map(|symbol| *symbol)
            .map_err(|_| format!("{} is not a CLAP plugin (no clap_entry)", path.display()))?;
        Self::init(entry, path, Some(library))
    }

    /// Wraps an entry linked into this program (tests).
    pub fn from_static(entry: &'static clap_plugin_entry, path: &Path) -> Result<Self, String> {
        Self::init(std::ptr::from_ref(entry), path, None)
    }

    fn init(
        entry: *const clap_plugin_entry,
        path: &Path,
        library: Option<libloading::Library>,
    ) -> Result<Self, String> {
        // SAFETY: `entry` points at the library's entry structure (or a
        // static one), valid while the library stays loaded.
        let Some(table) = (unsafe { entry.as_ref() }) else {
            return Err(format!("{}: null clap_entry", path.display()));
        };
        if table.clap_version.major < 1 {
            return Err(format!("{}: CLAP version too old", path.display()));
        }
        let c_path = CString::new(path.to_string_lossy().into_owned())
            .map_err(|_| format!("{}: path contains a nul byte", path.display()))?;
        let init = table
            .init
            .ok_or_else(|| format!("{}: no init function", path.display()))?;
        // SAFETY: `init` comes from the entry table; CLAP requires calling it
        // once with the plugin's path before anything else.
        if !unsafe { init(c_path.as_ptr()) } {
            return Err(format!(
                "{}: plugin library failed to initialize",
                path.display()
            ));
        }
        Ok(Self {
            path: path.to_path_buf(),
            entry,
            _library: library,
        })
    }

    /// The file this library came from.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// The plugin factory, if the library has one.
    pub fn factory(&self) -> Option<&clap_plugin_factory> {
        // SAFETY: `entry` is valid while `self` lives (see `init`).
        let entry = unsafe { self.entry.as_ref() }?;
        let get = entry.get_factory?;
        // SAFETY: `get_factory` comes from the entry table and takes a
        // nul-terminated factory id; the result is a static factory or null.
        let factory = unsafe { get(CLAP_PLUGIN_FACTORY_ID.as_ptr()) };
        // SAFETY: the plugin factory id promises a `clap_plugin_factory`,
        // valid while the library is loaded.
        unsafe { factory.cast::<clap_plugin_factory>().as_ref() }
    }

    /// Every plugin the factory offers.
    pub fn descriptors(&self) -> Vec<Descriptor> {
        let Some(factory) = self.factory() else {
            return Vec::new();
        };
        let (Some(count), Some(get)) = (factory.get_plugin_count, factory.get_plugin_descriptor)
        else {
            return Vec::new();
        };
        // SAFETY: factory functions take the factory pointer itself.
        let count = unsafe { count(factory) };
        (0..count)
            .filter_map(|i| {
                // SAFETY: `i` is below the reported count.
                let desc = unsafe { get(factory, i) };
                // SAFETY: a non-null descriptor is valid while the library
                // is loaded.
                let desc: &clap_plugin_descriptor = unsafe { desc.as_ref() }?;
                // SAFETY: descriptor strings are nul-terminated and static.
                let (id, name, vendor, version) = unsafe {
                    (
                        text(desc.id),
                        text(desc.name),
                        text(desc.vendor),
                        text(desc.version),
                    )
                };
                let mut features = Vec::new();
                let mut feature = desc.features;
                while !feature.is_null() {
                    // SAFETY: `features` is a null-terminated array of
                    // strings, read until the terminator.
                    let item = unsafe { *feature };
                    if item.is_null() {
                        break;
                    }
                    // SAFETY: as above.
                    features.push(unsafe { text(item) });
                    // SAFETY: not past the terminator yet.
                    feature = unsafe { feature.add(1) };
                }
                (!id.is_empty()).then_some(Descriptor {
                    id,
                    name,
                    vendor,
                    version,
                    features,
                })
            })
            .collect()
    }
}

impl Drop for Library {
    fn drop(&mut self) {
        // SAFETY: `entry` is still valid: the library unloads after this.
        if let Some(deinit) = unsafe { self.entry.as_ref() }.and_then(|e| e.deinit) {
            // SAFETY: `init` succeeded, and every instance holds an `Arc` to
            // this library, so none is alive when it is deinitialized.
            unsafe { deinit() };
        }
    }
}
