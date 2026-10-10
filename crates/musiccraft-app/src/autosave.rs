//! Periodic backup of the open project for crash recovery.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use musiccraft_project::Project;

/// Minimum time between autosaves.
const INTERVAL: Duration = Duration::from_secs(60);

/// The platform's per-user data folder for MusicCraft.
fn data_dir() -> Option<PathBuf> {
    let env = |key: &str| {
        std::env::var_os(key)
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
    };
    if cfg!(target_os = "windows") {
        env("APPDATA").map(|p| p.join("MusicCraft"))
    } else if cfg!(target_os = "macos") {
        env("HOME").map(|p| p.join("Library/Application Support/MusicCraft"))
    } else {
        env("XDG_DATA_HOME")
            .or_else(|| env("HOME").map(|h| h.join(".local/share")))
            .map(|p| p.join("musiccraft"))
    }
}

/// Where the autosave file lives.
#[must_use]
pub fn path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("autosave.mcproj"))
}

/// Writes the project to `path` atomically.
fn write(project: &Project, path: &Path) -> Result<(), String> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    }
    let json = project.to_json().map_err(|e| e.to_string())?;
    let temp = path.with_extension("tmp");
    std::fs::write(&temp, json)
        .and_then(|()| std::fs::rename(&temp, path))
        .map_err(|e| e.to_string())
}

pub struct Autosave {
    path: Option<PathBuf>,
    saved_revision: u64,
    last: Instant,
}

impl Autosave {
    pub fn new(revision: u64) -> Self {
        Self {
            path: path(),
            saved_revision: revision,
            last: Instant::now(),
        }
    }

    /// Saves if the project changed and the interval has passed. Returns a
    /// status message when something was written or failed.
    pub fn tick(&mut self, project: &Project, revision: u64) -> Option<Result<String, String>> {
        if revision == self.saved_revision || self.last.elapsed() < INTERVAL {
            return None;
        }
        let path = self.path.as_ref()?;
        self.last = Instant::now();
        self.saved_revision = revision;
        Some(write(project, path).map(|()| format!("autosaved to {}", path.display())))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn writes_only_after_changes_and_interval() {
        let dir = std::env::temp_dir().join(format!("musiccraft-autosave-{}", std::process::id()));
        let file = dir.join("autosave.mcproj");
        let mut autosave = Autosave {
            path: Some(file.clone()),
            saved_revision: 1,
            last: Instant::now(),
        };
        let project = Project::new("Backup");
        assert!(autosave.tick(&project, 1).is_none(), "unchanged");
        assert!(autosave.tick(&project, 2).is_none(), "too soon");
        autosave.last = Instant::now().checked_sub(INTERVAL).unwrap();
        assert!(autosave.tick(&project, 2).unwrap().is_ok());
        let loaded = Project::from_json(&std::fs::read_to_string(&file).unwrap()).unwrap();
        assert_eq!(loaded.name(), "Backup");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
