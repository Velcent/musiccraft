//! Putting the plugin library Cargo built where our hosts look for plugins.

use std::env::consts::{DLL_PREFIX, DLL_SUFFIX};
use std::path::{Path, PathBuf};

/// The plugin library Cargo built for these tests.
pub fn library() -> PathBuf {
    let exe = std::env::current_exe().unwrap();
    let deps = exe.parent().unwrap();
    let name = format!("{DLL_PREFIX}musiccraft_plugin_export{DLL_SUFFIX}");
    [deps.join(&name), deps.parent().unwrap().join(&name)]
        .into_iter()
        .find(|p| p.is_file())
        .unwrap_or_else(|| panic!("no {name} next to {}", deps.display()))
}

/// Puts the library at `to`: a hard link (debug builds are hundreds of
/// megabytes), or a copy across file systems.
fn place(to: &Path) {
    if std::fs::hard_link(library(), to).is_err() {
        std::fs::copy(library(), to).unwrap();
    }
}

/// A fresh folder, next to the library, holding it as `<bundle>.clap` and as
/// a `<bundle>.vst3` bundle.
pub fn install(tag: &str, bundle: &str) -> PathBuf {
    let target = library().parent().unwrap().to_path_buf();
    let dir = target.join(format!("mc-export-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    place(&dir.join(format!("{bundle}.clap")));
    let arch = format!("{}-linux", std::env::consts::ARCH);
    let binary = dir
        .join(format!("{bundle}.vst3/Contents"))
        .join(arch)
        .join(format!("{bundle}.so"));
    std::fs::create_dir_all(binary.parent().unwrap()).unwrap();
    place(&binary);
    dir
}

/// Our VST3 host's ID for a class ID.
pub fn vst3_id(class: [u8; 16]) -> String {
    let hex: String = class.iter().map(|b| format!("{b:02X}")).collect();
    format!("vst3:{hex}")
}
