use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use musiccraft_dsp::SampleBuffer;
use musiccraft_params::BUILTIN_PREFIX;

use crate::{SampleError, builtin, load_wav};

/// Decoded samples keyed by their resolved location, so each file is decoded
/// once and shared between channels.
#[derive(Debug, Default)]
pub struct SampleCache {
    entries: HashMap<String, Arc<SampleBuffer>>,
}

/// Resolves a relative sample path against the project directory, falling back
/// to the path as given (relative to the working directory).
fn resolve(sample: &str, base_dir: Option<&Path>) -> PathBuf {
    let path = Path::new(sample);
    if path.is_absolute() {
        return path.to_path_buf();
    }
    match base_dir.map(|dir| dir.join(path)) {
        Some(candidate) if candidate.exists() => candidate,
        _ => path.to_path_buf(),
    }
}

impl SampleCache {
    /// An empty cache.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Returns the decoded sample for a project sample reference: a
    /// `builtin:<name>` sound rendered at `sample_rate`, or a WAV file.
    ///
    /// # Errors
    ///
    /// Returns [`SampleError`] if the sound is unknown or the file cannot be
    /// decoded.
    pub fn get(
        &mut self,
        sample: &str,
        base_dir: Option<&Path>,
        sample_rate: u32,
    ) -> Result<Arc<SampleBuffer>, SampleError> {
        let key = match sample.strip_prefix(BUILTIN_PREFIX) {
            Some(name) => format!("{BUILTIN_PREFIX}{name}@{sample_rate}"),
            None => resolve(sample, base_dir).to_string_lossy().into_owned(),
        };
        if let Some(buffer) = self.entries.get(&key) {
            return Ok(Arc::clone(buffer));
        }
        let buffer = match sample.strip_prefix(BUILTIN_PREFIX) {
            Some(name) => builtin(name, sample_rate)
                .ok_or_else(|| SampleError::UnknownBuiltin(name.to_owned()))?,
            None => load_wav(Path::new(&key))?,
        };
        let buffer = Arc::new(buffer);
        self.entries.insert(key, Arc::clone(&buffer));
        Ok(buffer)
    }

    /// Number of cached samples.
    #[must_use]
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether nothing is cached.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Forgets every cached sample (for example after files changed on disk).
    pub fn clear(&mut self) {
        self.entries.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{WavFormat, write_wav};

    #[test]
    fn caches_builtins_per_rate() {
        let mut cache = SampleCache::new();
        let a = cache.get("builtin:kick", None, 44_100).unwrap();
        let b = cache.get("builtin:kick", None, 44_100).unwrap();
        assert!(Arc::ptr_eq(&a, &b));
        cache.get("builtin:kick", None, 48_000).unwrap();
        assert_eq!(cache.len(), 2);
        assert!(cache.get("builtin:nope", None, 44_100).is_err());
    }

    #[test]
    fn resolves_relative_paths_against_the_project() {
        let dir = std::env::temp_dir().join(format!("musiccraft-cache-{}", std::process::id()));
        std::fs::create_dir_all(dir.join("sounds")).unwrap();
        write_wav(
            &dir.join("sounds/one.wav"),
            &[0.5],
            &[0.5],
            8_000,
            WavFormat::Float32,
        )
        .unwrap();
        let mut cache = SampleCache::new();
        let buffer = cache.get("sounds/one.wav", Some(&dir), 44_100).unwrap();
        assert_eq!(buffer.len(), 1);
        assert!(cache.get("sounds/one.wav", None, 44_100).is_err());
        std::fs::remove_dir_all(dir).unwrap();
    }
}
