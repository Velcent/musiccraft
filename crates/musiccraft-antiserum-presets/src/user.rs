//! User presets (`docs/spec/antiserum-app.md`): patch files in a folder of
//! their own, shared by the app and the Antiserum plugin. Main thread only.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use musiccraft_patch::Patch;

/// Names another folder for the user presets (portable setups).
pub const FOLDER_VARIABLE: &str = "MUSICCRAFT_USER_PRESETS";

/// A folder set by the program, which wins over everything else.
static OVERRIDE: Mutex<Option<PathBuf>> = Mutex::new(None);

/// Uses `folder` for the user presets from now on (`None`: the usual one).
pub fn set_folder(folder: Option<PathBuf>) {
    if let Ok(mut slot) = OVERRIDE.lock() {
        *slot = folder;
    }
}

/// The user presets' folder: one set with [`set_folder`], else
/// [`FOLDER_VARIABLE`], else `presets/antiserum` in MusicCraft's per-user data
/// folder.
#[must_use]
pub fn folder() -> Option<PathBuf> {
    if let Some(set) = OVERRIDE.lock().ok().and_then(|s| s.clone()) {
        return Some(set);
    }
    std::env::var_os(FOLDER_VARIABLE)
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
        .or_else(|| musiccraft_paths::data_dir().map(|d| d.join("presets").join("antiserum")))
}

/// The file a preset called `name` is saved in.
fn file(folder: &Path, name: &str) -> PathBuf {
    let safe: String = name
        .chars()
        .map(|c| {
            if matches!(c, '/' | '\\' | ':' | '*' | '?' | '"' | '<' | '>' | '|') {
                '-'
            } else {
                c
            }
        })
        .collect();
    folder.join(format!("{}.json", safe.trim()))
}

/// Reads one preset file; its name comes from the file when the patch has
/// none.
fn read(path: &Path) -> Option<Patch> {
    let text = std::fs::read_to_string(path).ok()?;
    let mut patch = Patch::from_json(&text).ok()?;
    if patch.name.is_empty() {
        patch.name = path.file_stem()?.to_string_lossy().into_owned();
    }
    Some(patch)
}

/// Every user preset in `folder`, by name.
#[must_use]
pub fn list_in(folder: &Path) -> Vec<Patch> {
    let Ok(entries) = std::fs::read_dir(folder) else {
        return Vec::new();
    };
    let mut presets: Vec<Patch> = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .filter(|p| {
            p.extension()
                .is_some_and(|x| x.eq_ignore_ascii_case("json"))
        })
        .filter_map(|p| read(&p))
        .collect();
    presets.sort_by_key(|p| p.name.to_lowercase());
    presets
}

/// Every user preset, by name.
#[must_use]
pub fn list() -> Vec<Patch> {
    folder().map_or_else(Vec::new, |f| list_in(&f))
}

/// How long [`cached`] trusts its last reading of the folder.
const FRESH: std::time::Duration = std::time::Duration::from_secs(3);

/// The last reading of the folder: when, which folder and what.
type Reading = (
    std::time::Instant,
    Option<PathBuf>,
    std::sync::Arc<Vec<Patch>>,
);

static CACHE: Mutex<Option<Reading>> = Mutex::new(None);

/// The user presets as of at most a few seconds ago, for lists drawn every
/// frame (the browser, the preset bars). Saving and deleting refresh it.
#[must_use]
pub fn cached() -> std::sync::Arc<Vec<Patch>> {
    let now = std::time::Instant::now();
    let where_ = folder();
    let Ok(mut cache) = CACHE.lock() else {
        return std::sync::Arc::new(list());
    };
    if let Some((at, dir, presets)) = cache.as_ref()
        && now.duration_since(*at) < FRESH
        && *dir == where_
    {
        return std::sync::Arc::clone(presets);
    }
    let presets = std::sync::Arc::new(where_.as_deref().map_or_else(Vec::new, list_in));
    *cache = Some((now, where_, std::sync::Arc::clone(&presets)));
    presets
}

/// Makes the next [`cached`] read the folder again.
fn forget() {
    if let Ok(mut cache) = CACHE.lock() {
        *cache = None;
    }
}

/// The user preset called `name` (case-insensitive).
#[must_use]
pub fn find(name: &str) -> Option<Patch> {
    list()
        .into_iter()
        .find(|p| p.name.eq_ignore_ascii_case(name))
}

/// Saves `patch` as a user preset under its name, replacing one of that
/// name. Returns the file.
pub fn save(patch: &Patch) -> Result<PathBuf, String> {
    let name = patch.name.trim();
    if name.is_empty() {
        return Err("a preset needs a name".to_owned());
    }
    if crate::find(name).is_some() {
        return Err(format!("`{name}` is a factory preset; choose another name"));
    }
    let folder = folder().ok_or("there is no folder for user presets")?;
    std::fs::create_dir_all(&folder)
        .map_err(|e| format!("cannot create {}: {e}", folder.display()))?;
    let path = file(&folder, name);
    let json = patch.to_json()?;
    std::fs::write(&path, json).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    forget();
    Ok(path)
}

/// Deletes the user preset called `name`.
pub fn delete(name: &str) -> Result<PathBuf, String> {
    let folder = folder().ok_or("there is no folder for user presets")?;
    let entries = std::fs::read_dir(&folder).map_err(|e| e.to_string())?;
    let path = entries
        .filter_map(Result::ok)
        .map(|e| e.path())
        .find(|p| read(p).is_some_and(|patch| patch.name.eq_ignore_ascii_case(name)))
        .ok_or_else(|| format!("no user preset `{name}`"))?;
    std::fs::remove_file(&path).map_err(|e| format!("cannot delete {}: {e}", path.display()))?;
    forget();
    Ok(path)
}

/// A factory preset or, failing that, a user preset called `name`.
#[must_use]
pub fn find_any(name: &str) -> Option<Patch> {
    crate::find(name).cloned().or_else(|| find(name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presets_save_list_and_delete_in_their_folder() {
        let dir = std::env::temp_dir().join(format!("mc-user-presets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        set_folder(Some(dir.clone()));
        assert!(list().is_empty());
        let mut patch = crate::find("Glass Reese").unwrap().clone();
        assert!(save(&patch).is_err(), "factory names are taken");
        patch.name = "My Reese".to_owned();
        let path = save(&patch).unwrap();
        assert!(path.ends_with("My Reese.json"));
        let other = Patch {
            name: "a/b".to_owned(),
            ..Patch::default()
        };
        assert!(save(&other).unwrap().ends_with("a-b.json"));
        let names: Vec<String> = list().into_iter().map(|p| p.name).collect();
        assert_eq!(names, ["a/b", "My Reese"]);
        assert_eq!(cached().len(), 2, "saving refreshed the cache");
        assert_eq!(
            find_any("my reese").map(|p| p.tables),
            Some(patch.tables.clone())
        );
        assert_eq!(
            find_any("Glass Reese").map(|p| p.name),
            Some("Glass Reese".to_owned())
        );
        delete("My Reese").unwrap();
        assert!(find("My Reese").is_none());
        assert!(delete("My Reese").is_err());
        set_folder(None);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
