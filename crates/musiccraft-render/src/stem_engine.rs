//! Which splitter `sample.split_stems` uses (`docs/spec/stems.md`): Demucs in
//! ONNX Runtime when the runtime and the model are there (downloading the
//! model the first time), else the built-in splitter.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::time::Duration;

use musiccraft_stems::Stems;

/// Where the Demucs model is published (`.github/workflows/stem-model.yml`).
pub const MODEL_URL: &str =
    "https://github.com/bhimio1/musiccraft/releases/download/stem-model-v1/htdemucs-v1.onnx";

/// The model's file name in the data folder's `models/`.
pub const MODEL_FILE: &str = "htdemucs-v1.onnx";

/// The splitter a split asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Engine {
    /// Demucs when it can run, else the built-in splitter.
    #[default]
    Auto,
    /// Demucs, or fail.
    Demucs,
    /// The built-in splitter.
    Builtin,
}

impl Engine {
    /// `auto`, `demucs` or `builtin`.
    #[must_use]
    pub fn parse(raw: &str) -> Option<Self> {
        match raw {
            "auto" => Some(Self::Auto),
            "demucs" => Some(Self::Demucs),
            "builtin" => Some(Self::Builtin),
            _ => None,
        }
    }
}

/// What a split is doing, for progress.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Phase {
    /// Fetching the model (first use only).
    Download,
    /// Splitting.
    Split,
}

/// The Demucs model: `MUSICCRAFT_STEM_MODEL` when set, else `models/` in the
/// per-user data folder.
#[must_use]
pub fn model_path() -> Option<PathBuf> {
    std::env::var_os("MUSICCRAFT_STEM_MODEL")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| musiccraft_paths::data_dir().map(|d| d.join("models").join(MODEL_FILE)))
}

/// The size `url` reports, if it does.
fn remote_size(url: &str) -> Option<u64> {
    let out = Command::new("curl")
        .args(["-sfIL", "--max-time", "20", url])
        .stdin(Stdio::null())
        .output()
        .ok()?;
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|l| {
            let (key, value) = l.split_once(':')?;
            key.eq_ignore_ascii_case("content-length")
                .then(|| value.trim().parse::<u64>().ok())
                .flatten()
        })
        .next_back()
}

/// Downloads `url` to `path` with the system's `curl` (on every supported
/// system), through a temporary file. `Ok(false)` when cancelled.
fn download(
    url: &str,
    path: &Path,
    progress: &mut dyn FnMut(Phase, f32) -> bool,
) -> Result<bool, String> {
    let folder = path.parent().ok_or("the model has no folder")?;
    std::fs::create_dir_all(folder).map_err(|e| e.to_string())?;
    let part = path.with_extension("part");
    let total = remote_size(url);
    let mut child = Command::new("curl")
        .args(["-sfL", "--retry", "3", "-o"])
        .arg(&part)
        .arg(url)
        .stdin(Stdio::null())
        .spawn()
        .map_err(|e| format!("cannot run curl to fetch the stem model: {e}"))?;
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break status;
        }
        let done = std::fs::metadata(&part).map_or(0, |m| m.len());
        let fraction = total.map_or(0.0, |t| done as f32 / t.max(1) as f32);
        if !progress(Phase::Download, fraction.min(1.0)) {
            let _ = child.kill();
            let _ = child.wait();
            let _ = std::fs::remove_file(&part);
            return Ok(false);
        }
        std::thread::sleep(Duration::from_millis(200));
    };
    if !status.success() {
        let _ = std::fs::remove_file(&part);
        return Err(format!("could not download the stem model from {url}"));
    }
    std::fs::rename(&part, path).map_err(|e| e.to_string())?;
    Ok(true)
}

/// Splits with Demucs: finds the runtime, fetches the model if needed.
/// `Ok(None)` when cancelled.
fn demucs(
    left: &[f32],
    right: &[f32],
    rate: f64,
    progress: &mut dyn FnMut(Phase, f32) -> bool,
) -> Result<Option<(Stems, String)>, String> {
    // Fails early, before any download, when there is no runtime.
    musiccraft_demucs::runtime()?;
    let model = model_path().ok_or("there is no data folder for the stem model")?;
    if !model.is_file() && !download(MODEL_URL, &model, progress)? {
        return Ok(None);
    }
    let mut net = musiccraft_demucs::Demucs::load(&model)?;
    let backend = net.backend();
    match net.split(left, right, rate, &mut |p| progress(Phase::Split, p)) {
        Ok(stems) => Ok(Some((stems, format!("Demucs on {backend}")))),
        Err(musiccraft_demucs::Error::Cancelled) => Ok(None),
        Err(musiccraft_demucs::Error::Failed(why)) => Err(why),
    }
}

/// Splits a sound with `engine`. Returns the stems and how they were made
/// (`Demucs on CUDA`, `the built-in splitter`, …), or `Ok(None)` when
/// `progress` cancelled it.
///
/// # Errors
///
/// Only for [`Engine::Demucs`]: why Demucs could not run.
pub fn split(
    engine: Engine,
    left: &[f32],
    right: &[f32],
    rate: f64,
    progress: &mut dyn FnMut(Phase, f32) -> bool,
) -> Result<Option<(Stems, String)>, String> {
    let note = match engine {
        Engine::Builtin => None,
        Engine::Demucs => return demucs(left, right, rate, progress),
        Engine::Auto => match demucs(left, right, rate, progress) {
            Ok(done) => return Ok(done),
            Err(why) => Some(why),
        },
    };
    let stems = musiccraft_stems::split(left, right, rate, &mut |p| progress(Phase::Split, p));
    Ok(stems.ok().map(|stems| {
        let how = match note {
            Some(why) => format!("the built-in splitter; Demucs: {why}"),
            None => "the built-in splitter".to_owned(),
        };
        (stems, how)
    }))
}

/// `sample.stems_info`: which splitter `sample.split_stems` would use.
#[derive(Default)]
pub struct StemsInfoCommand;

impl musiccraft_commands::Command for StemsInfoCommand {
    fn name(&self) -> &'static str {
        "sample.stems_info"
    }

    fn summary(&self) -> &'static str {
        "Say which stem splitter is ready: the ONNX Runtime found, its GPU support and the Demucs model."
    }

    fn usage(&self) -> &'static str {
        "sample.stems_info"
    }

    fn execute(
        &self,
        _project: &mut musiccraft_project::Project,
        _args: &[String],
    ) -> Result<musiccraft_commands::Outcome, musiccraft_commands::CommandError> {
        let runtime = match musiccraft_demucs::runtime() {
            Ok(r) => {
                let gpu = if r.gpu.is_empty() {
                    "no GPU support, CPU only".to_owned()
                } else {
                    format!("GPU first: {}, then CPU", r.gpu.join(", "))
                };
                let version = r.version.map(|v| format!(" {v}")).unwrap_or_default();
                format!("ONNX Runtime{version} at {} ({gpu})", r.path.display())
            }
            Err(why) => format!("{why}; splits use the built-in splitter"),
        };
        let model = match model_path() {
            Some(path) if path.is_file() => format!("Demucs model at {}", path.display()),
            Some(path) => format!(
                "Demucs model not yet at {} (fetched on first use from {MODEL_URL})",
                path.display()
            ),
            None => "no data folder for the Demucs model".to_owned(),
        };
        Ok(musiccraft_commands::Outcome::done(format!(
            "{runtime}; {model}"
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn engines_parse() {
        assert_eq!(Engine::parse("auto"), Some(Engine::Auto));
        assert_eq!(Engine::parse("demucs"), Some(Engine::Demucs));
        assert_eq!(Engine::parse("builtin"), Some(Engine::Builtin));
        assert_eq!(Engine::parse("gpu"), None);
    }

    #[test]
    fn the_model_downloads_through_a_temporary_file() {
        let dir = std::env::temp_dir().join(format!("musiccraft-model-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let source = dir.join("source.onnx");
        std::fs::write(&source, vec![7_u8; 100_000]).unwrap();
        let url = format!("file://{}", source.display());
        let target = dir.join("models").join(MODEL_FILE);
        let mut phases = Vec::new();
        let done = download(&url, &target, &mut |phase, _| {
            phases.push(phase);
            true
        });
        assert_eq!(done, Ok(true));
        assert_eq!(std::fs::read(&target).unwrap().len(), 100_000);
        assert!(!target.with_extension("part").exists());
        assert!(phases.iter().all(|p| *p == Phase::Download));
        let missing = format!("file://{}", dir.join("nothing.onnx").display());
        assert!(download(&missing, &dir.join("other.onnx"), &mut |_, _| true).is_err());
        assert!(!dir.join("other.onnx").exists());
        std::fs::remove_dir_all(&dir).ok();
    }

    #[test]
    fn the_built_in_splitter_says_so() {
        let tone: Vec<f32> = (0..4_000).map(|i| (i as f32 * 0.05).sin()).collect();
        let (_, how) = split(Engine::Builtin, &tone, &tone, 22_050.0, &mut |_, _| true)
            .unwrap()
            .unwrap();
        assert_eq!(how, "the built-in splitter");
        let cancelled = split(Engine::Builtin, &tone, &tone, 22_050.0, &mut |_, _| false);
        assert_eq!(cancelled.map(|d| d.is_none()), Ok(true));
    }
}
