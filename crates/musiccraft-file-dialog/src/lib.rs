//! Native file dialogs (`docs/spec/style/dialogs.md`, "File dialogs are the
//! operating system's own"): the desktop portal on Linux, which also works
//! inside the Flatpak, and the system panels on macOS and Windows.
//!
//! A [`Picker`] shows one dialog at a time without blocking the window that
//! asked for it: the dialog runs on its own thread, and the window polls for
//! the answer each frame. macOS only allows its panels on the main thread, so
//! there the dialog runs in place, as a modal panel.

use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, TryRecvError};

pub use rfd::FileDialog;

/// What a dialog chooses.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// One existing file.
    File,
    /// One or more existing files.
    Files,
    /// A file to write (the dialog asks before replacing one).
    Save,
    /// A folder.
    Folder,
}

/// Shows `dialog` and waits for it: the chosen paths, or none when it was
/// cancelled.
#[must_use]
pub fn run(dialog: FileDialog, mode: Mode) -> Vec<PathBuf> {
    match mode {
        Mode::File => dialog.pick_file().into_iter().collect(),
        Mode::Files => dialog.pick_files().unwrap_or_default(),
        Mode::Save => dialog.save_file().into_iter().collect(),
        Mode::Folder => dialog.pick_folder().into_iter().collect(),
    }
}

/// One dialog at a time, with a tag saying what its answer is for.
pub struct Picker<T> {
    open: Option<(T, Receiver<Vec<PathBuf>>)>,
}

impl<T> Default for Picker<T> {
    fn default() -> Self {
        Self { open: None }
    }
}

impl<T> Picker<T> {
    /// Whether a dialog is waiting for an answer.
    #[must_use]
    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    /// Shows `dialog`; `wake` runs once it is answered (to repaint the
    /// window). Returns `false`, and shows nothing, while another dialog is
    /// open.
    pub fn show(
        &mut self,
        dialog: FileDialog,
        mode: Mode,
        tag: T,
        wake: impl FnOnce() + Send + 'static,
    ) -> bool {
        self.start(tag, move || run(dialog, mode), wake)
    }

    /// Runs `chooser` the way [`Picker::show`] runs a dialog.
    pub fn start(
        &mut self,
        tag: T,
        chooser: impl FnOnce() -> Vec<PathBuf> + Send + 'static,
        wake: impl FnOnce() + Send + 'static,
    ) -> bool {
        if self.is_open() {
            return false;
        }
        let (tx, rx) = mpsc::channel();
        let job = move || {
            let _ = tx.send(chooser());
            wake();
        };
        if cfg!(target_os = "macos") {
            job();
        } else if std::thread::Builder::new()
            .name("file-dialog".to_owned())
            .spawn(job)
            .is_err()
        {
            return false;
        }
        self.open = Some((tag, rx));
        true
    }

    /// The answer, once there is one: the tag and the chosen paths (none
    /// when the dialog was cancelled).
    pub fn poll(&mut self) -> Option<(T, Vec<PathBuf>)> {
        let (_, rx) = self.open.as_ref()?;
        let paths = match rx.try_recv() {
            Ok(paths) => paths,
            Err(TryRecvError::Empty) => return None,
            Err(TryRecvError::Disconnected) => Vec::new(),
        };
        self.open.take().map(|(tag, _)| (tag, paths))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    fn wait<T>(picker: &mut Picker<T>) -> (T, Vec<PathBuf>) {
        let start = Instant::now();
        loop {
            if let Some(answer) = picker.poll() {
                return answer;
            }
            assert!(start.elapsed() < Duration::from_secs(5), "no answer");
            std::thread::yield_now();
        }
    }

    #[test]
    fn an_answer_comes_back_with_its_tag_and_wakes_the_window() {
        let mut picker = Picker::default();
        let (woke, woken) = mpsc::channel();
        assert!(picker.start(
            "open",
            || vec![PathBuf::from("/songs/a.mcproj")],
            move || woke.send(()).unwrap(),
        ));
        assert!(picker.is_open());
        assert_eq!(
            wait(&mut picker),
            ("open", vec![PathBuf::from("/songs/a.mcproj")])
        );
        assert!(woken.recv_timeout(Duration::from_secs(5)).is_ok());
        assert!(!picker.is_open());
        assert_eq!(picker.poll(), None);
    }

    #[test]
    fn one_dialog_at_a_time() {
        let mut picker = Picker::default();
        let (release, gate) = mpsc::channel::<()>();
        assert!(picker.start(
            1,
            move || {
                let _ = gate.recv();
                Vec::new()
            },
            || {},
        ));
        assert!(!picker.start(2, Vec::new, || {}));
        release.send(()).unwrap();
        // Cancelled: no paths.
        assert_eq!(wait(&mut picker), (1, Vec::new()));
        assert!(picker.start(2, Vec::new, || {}));
    }
}
