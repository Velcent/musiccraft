//! Periodic backup of the open project for crash recovery.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use musiccraft_project::Project;

/// Minimum time between autosaves.
const INTERVAL: Duration = Duration::from_secs(60);

use musiccraft_paths::data_dir;

/// Where the autosave file lives.
#[must_use]
pub fn path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("autosave.mcproj"))
}

/// Where the backup of a session that did not close properly is kept.
#[must_use]
pub fn recovery_path() -> Option<PathBuf> {
    data_dir().map(|dir| dir.join("recovery.mcproj"))
}

/// At start-up: an autosave left behind means the last session did not close
/// properly (a clean exit deletes it). Moves it to [`recovery_path`] so this
/// session's autosaves cannot overwrite it, and returns where it went.
pub fn take_crash_backup() -> Option<PathBuf> {
    move_aside(&path()?, &recovery_path()?)
}

fn move_aside(from: &Path, to: &Path) -> Option<PathBuf> {
    if !from.is_file() {
        return None;
    }
    std::fs::rename(from, to).ok()?;
    Some(to.to_path_buf())
}

/// Deletes this session's autosave: called when the app closes with nothing
/// unsaved, so the next start-up does not mistake it for a crash.
pub fn discard() {
    if let Some(path) = path() {
        let _ = std::fs::remove_file(path);
    }
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
        let result = write(project, path);
        // Only a successful write counts; a failed one is retried after the
        // interval instead of waiting for the next edit.
        if result.is_ok() {
            self.saved_revision = revision;
        }
        Some(result.map(|()| format!("autosaved to {}", path.display())))
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

    #[test]
    fn failed_writes_are_retried() {
        let dir = std::env::temp_dir().join(format!("musiccraft-retry-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // A directory where the file should be makes the write fail.
        let file = dir.join("autosave.mcproj");
        std::fs::create_dir_all(file.with_extension("tmp")).unwrap();
        let mut autosave = Autosave {
            path: Some(file.clone()),
            saved_revision: 1,
            last: Instant::now().checked_sub(INTERVAL).unwrap(),
        };
        let project = Project::new("Retry");
        assert!(autosave.tick(&project, 2).unwrap().is_err());
        std::fs::remove_dir_all(file.with_extension("tmp")).unwrap();
        autosave.last = Instant::now().checked_sub(INTERVAL).unwrap();
        assert!(
            autosave.tick(&project, 2).unwrap().is_ok(),
            "the same revision is written once the folder is fixed"
        );
        std::fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn a_crash_backup_is_moved_out_of_the_way() {
        let dir = std::env::temp_dir().join(format!("musiccraft-crash-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let autosave = dir.join("autosave.mcproj");
        let recovery = dir.join("recovery.mcproj");
        assert_eq!(move_aside(&autosave, &recovery), None, "nothing to recover");
        std::fs::write(&autosave, "backup").unwrap();
        std::fs::write(&recovery, "older backup").unwrap();
        assert_eq!(move_aside(&autosave, &recovery), Some(recovery.clone()));
        assert!(!autosave.exists());
        assert_eq!(std::fs::read_to_string(&recovery).unwrap(), "backup");
        std::fs::remove_dir_all(dir).unwrap();
    }
}
