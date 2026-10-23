//! Splits a sound into stems with Demucs (HTDemucs by Alexandre Défossez and
//! Meta, MIT licence) running in ONNX Runtime (`docs/spec/stems.md`).
//!
//! The network comes from `tools/stem-model/export.py`, which leaves
//! everything but the network itself to this crate, as Demucs's own
//! `apply_model` does it: normalising, cutting the song into overlapping
//! segments, the transforms around the network and the cross-fades between
//! segments.
//!
//! ONNX Runtime is loaded at run time from a shared library
//! ([`runtime_candidates`]); a session tries the GPU execution providers in turn
//! and falls back to the CPU ([`Demucs::load`]). Main or worker thread only.

mod spectrum;

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use musiccraft_stems::{Audio, Stems};
use ort::ep::{self, ExecutionProvider, ExecutionProviderDispatch};
use ort::session::Session;
use ort::value::Tensor;
use serde::Deserialize;

use spectrum::Transform;

/// What the exporter wrote into the model's metadata (key `musiccraft`).
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Info {
    /// Source names in output order (`drums`, `bass`, `other`, `vocals`).
    pub sources: Vec<String>,
    /// Rate the network works at.
    pub sample_rate: u32,
    /// Samples per segment.
    pub segment: usize,
    /// Transform size.
    pub nfft: usize,
    /// Transform hop.
    pub hop: usize,
}

/// One run of the network over a segment: `mix` is `2 × segment` samples,
/// `spec` is `4 × bins × frames`; returns `sources × 4 × bins × frames`
/// spectrograms and `sources × 2 × segment` waveforms.
pub trait Network {
    /// Runs the network.
    ///
    /// # Errors
    ///
    /// Why it failed.
    fn run(&mut self, mix: Vec<f32>, spec: Vec<f32>) -> Result<(Vec<f32>, Vec<f32>), String>;
}

/// Why a split did not finish.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Error {
    /// The progress callback stopped it.
    Cancelled,
    /// The network or its data failed.
    Failed(String),
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancelled => write!(f, "cancelled"),
            Self::Failed(why) => write!(f, "{why}"),
        }
    }
}

/// Segments overlap by a quarter, as in Demucs.
const OVERLAP: f64 = 0.25;

/// Mean and (sample) standard deviation.
fn mean_std(x: &[f32]) -> (f32, f32) {
    let n = x.len().max(1) as f64;
    let mean = x.iter().map(|v| f64::from(*v)).sum::<f64>() / n;
    let var = x
        .iter()
        .map(|v| (f64::from(*v) - mean).powi(2))
        .sum::<f64>()
        / (n - 1.0).max(1.0);
    (mean as f32, var.sqrt() as f32)
}

/// Demucs's cross-fade weight: a triangle peaking mid-segment.
fn weights(length: usize) -> Vec<f32> {
    let half = length / 2;
    let peak = half.max(length - half) as f32;
    (1..=half)
        .chain((1..=length - half).rev())
        .map(|w| w as f32 / peak)
        .collect()
}

/// `segment` samples of `x` centred on `start..start + length`, zero
/// outside the sound (Demucs's `TensorChunk.padded`).
fn padded(x: &[f32], start: usize, length: usize, segment: usize) -> Vec<f32> {
    let from = start as isize - ((segment - length) / 2) as isize;
    (0..segment as isize)
        .map(|i| {
            usize::try_from(from + i)
                .ok()
                .and_then(|j| x.get(j))
                .copied()
                .unwrap_or(0.0)
        })
        .collect()
}

/// Runs the network on one segment and returns each source's two channels.
fn segment(
    network: &mut dyn Network,
    transform: &Transform,
    info: &Info,
    left: &[f32],
    right: &[f32],
) -> Result<Vec<[Vec<f32>; 2]>, String> {
    let length = info.segment;
    let (bins, frames) = (transform.bins(), transform.frames(length));
    let plane = bins * frames;
    let mut spec = Vec::with_capacity(4 * plane);
    for side in [left, right] {
        let z = transform.forward(side);
        spec.extend(z.iter().map(|c| c.0));
        spec.extend(z.iter().map(|c| c.1));
    }
    let mut mix = Vec::with_capacity(2 * length);
    mix.extend_from_slice(left);
    mix.extend_from_slice(right);
    let (freq, time) = network.run(mix, spec)?;
    let sources = info.sources.len();
    if freq.len() != sources * 4 * plane || time.len() != sources * 2 * length {
        return Err("the stem model gave outputs of the wrong size".to_owned());
    }
    Ok((0..sources)
        .map(|s| {
            std::array::from_fn(|c| {
                let base = (s * 4 + c * 2) * plane;
                let z: Vec<_> = (0..plane)
                    .map(|i| (freq[base + i], freq[base + plane + i]))
                    .collect();
                let mut wave = transform.inverse(&z, frames, length);
                let t = &time[(s * 2 + c) * length..(s * 2 + c + 1) * length];
                wave.iter_mut().zip(t).for_each(|(w, t)| *w += t);
                wave
            })
        })
        .collect())
}

/// Splits a stereo sound with `network`, as Demucs's `apply_model` does
/// (no random shifts). `progress` hears the fraction done after each
/// segment; returning `false` stops the split. The stems add up to the
/// sound: whatever Demucs leaves out of vocals, drums and bass is other.
///
/// # Errors
///
/// [`Error::Cancelled`] or why the network failed.
pub fn separate(
    network: &mut dyn Network,
    info: &Info,
    left: &[f32],
    right: &[f32],
    sample_rate: f64,
    progress: &mut dyn FnMut(f32) -> bool,
) -> Result<Stems, Error> {
    let len = left.len().min(right.len());
    let (left, right) = (&left[..len], &right[..len]);
    let model_rate = f64::from(info.sample_rate);
    let factor = model_rate / sample_rate.max(1.0);
    let resampled = (factor - 1.0).abs() > 1e-9;
    let to_model = |x: &[f32]| {
        if resampled {
            musiccraft_dsp::edit::resample(x, factor)
        } else {
            x.to_vec()
        }
    };
    let (l, r) = (to_model(left), to_model(right));
    let mono: Vec<f32> = l.iter().zip(&r).map(|(a, b)| (a + b) * 0.5).collect();
    let (mean, std) = mean_std(&mono);
    let std = std + 1e-8;
    let norm = |x: &[f32]| -> Vec<f32> { x.iter().map(|v| (v - mean) / std).collect() };
    let (l, r) = (norm(&l), norm(&r));

    let n = l.len();
    let seg = info.segment;
    let stride = (((1.0 - OVERLAP) * seg as f64) as usize).max(1);
    let weight = weights(seg);
    let transform = Transform::new(info.nfft);
    let sources = info.sources.len();
    let mut out: Vec<[Vec<f32>; 2]> = (0..sources).map(|_| [vec![0.0; n], vec![0.0; n]]).collect();
    let mut total = vec![0.0_f32; n];
    let offsets: Vec<usize> = (0..n).step_by(stride).collect();
    for (k, &offset) in offsets.iter().enumerate() {
        let length = seg.min(n - offset);
        let delta = (seg - length) / 2;
        let chunk_l = padded(&l, offset, length, seg);
        let chunk_r = padded(&r, offset, length, seg);
        let result =
            segment(network, &transform, info, &chunk_l, &chunk_r).map_err(Error::Failed)?;
        for (stem, chunk) in out.iter_mut().zip(&result) {
            for c in 0..2 {
                for i in 0..length {
                    stem[c][offset + i] += weight[i] * chunk[c][delta + i];
                }
            }
        }
        for i in 0..length {
            total[offset + i] += weight[i];
        }
        if !progress((k + 1) as f32 / offsets.len() as f32) {
            return Err(Error::Cancelled);
        }
    }
    let back = |x: &[f32]| -> Vec<f32> {
        let x: Vec<f32> = x
            .iter()
            .zip(&total)
            .map(|(v, w)| if *w > 0.0 { v / w * std + mean } else { mean })
            .collect();
        let mut x = if resampled {
            musiccraft_dsp::edit::resample(&x, 1.0 / factor)
        } else {
            x
        };
        x.resize(len, 0.0);
        x
    };
    let mut stems: [Audio; 4] = std::array::from_fn(|_| Audio::default());
    for (name, stem) in info.sources.iter().zip(&out) {
        let index = match name.as_str() {
            "vocals" => 0,
            "drums" => 1,
            "bass" => 2,
            _ => continue,
        };
        stems[index] = Audio {
            left: back(&stem[0]),
            right: back(&stem[1]),
        };
    }
    for stem in stems.iter_mut().take(3) {
        stem.left.resize(len, 0.0);
        stem.right.resize(len, 0.0);
    }
    let [vocals, drums, bass, other] = &mut stems;
    other.left = (0..len)
        .map(|i| left[i] - vocals.left[i] - drums.left[i] - bass.left[i])
        .collect();
    other.right = (0..len)
        .map(|i| right[i] - vocals.right[i] - drums.right[i] - bass.right[i])
        .collect();
    Ok(Stems::new(stems))
}

/// Where ONNX Runtime's shared library may be, in the order tried:
/// `MUSICCRAFT_ONNXRUNTIME` or `ORT_DYLIB_PATH` when set, then a system-wide
/// install (which may have CUDA or ROCm), then the one shipped with the
/// program (beside it, in `../lib` or in the app bundle's `Frameworks`).
#[must_use]
pub fn runtime_candidates() -> Vec<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["onnxruntime.dll"]
    } else if cfg!(target_os = "macos") {
        &["libonnxruntime.dylib"]
    } else {
        &["libonnxruntime.so", "libonnxruntime.so.1"]
    };
    let from_env = ["MUSICCRAFT_ONNXRUNTIME", "ORT_DYLIB_PATH"]
        .iter()
        .filter_map(std::env::var_os)
        .filter(|p| !p.is_empty())
        .map(PathBuf::from);
    let system: &[&str] = if cfg!(target_os = "macos") {
        &["/opt/homebrew/lib", "/usr/local/lib"]
    } else if cfg!(windows) {
        &[]
    } else {
        &[
            "/usr/lib",
            "/usr/lib64",
            "/usr/local/lib",
            "/usr/lib/x86_64-linux-gnu",
        ]
    };
    let exe_dir = std::env::current_exe()
        .ok()
        .and_then(|exe| exe.parent().map(Path::to_path_buf));
    let shipped = exe_dir.into_iter().flat_map(|dir| {
        let lib = dir.join("..").join("lib");
        let frameworks = dir.join("..").join("Frameworks");
        [dir, lib, frameworks]
    });
    let dirs: Vec<PathBuf> = system.iter().map(PathBuf::from).chain(shipped).collect();
    from_env
        .chain(
            dirs.iter()
                .flat_map(|dir| names.iter().map(move |name| dir.join(name))),
        )
        .filter(|path| path.is_file())
        .collect()
}

/// The ONNX Runtime the process has loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Runtime {
    /// The shared library.
    pub path: PathBuf,
    /// Its version, such as `1.22.0`, when its build says.
    pub version: Option<String>,
    /// The GPU execution providers it was built with, best first.
    pub gpu: Vec<&'static str>,
}

struct Loaded {
    environment: ort::environment::Environment,
    path: PathBuf,
}

static LOADED: OnceLock<Loaded> = OnceLock::new();
static LOADING: Mutex<()> = Mutex::new(());

/// Loads ONNX Runtime from the first of [`runtime_candidates`] that works,
/// once per process.
fn loaded() -> Result<&'static Loaded, String> {
    let _guard = LOADING
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    if let Some(loaded) = LOADED.get() {
        return Ok(loaded);
    }
    let candidates = runtime_candidates();
    if candidates.is_empty() {
        return Err("ONNX Runtime is not installed (see docs/spec/stems.md)".to_owned());
    }
    let mut errors = Vec::new();
    for path in candidates {
        let built = ort::init_from(&path)
            .map_err(|e| e.to_string())
            .and_then(|b| b.with_name("musiccraft").build().map_err(|e| e.to_string()));
        match built {
            Ok(environment) => return Ok(LOADED.get_or_init(|| Loaded { environment, path })),
            Err(why) => errors.push(format!("{}: {why}", path.display())),
        }
    }
    Err(format!("cannot load ONNX Runtime ({})", errors.join("; ")))
}

/// Loads ONNX Runtime (see [`runtime_candidates`]) and says what it offers.
///
/// # Errors
///
/// Why no runtime could be loaded.
pub fn runtime() -> Result<Runtime, String> {
    let loaded = loaded()?;
    // Release builds name their branch `rel-<version>`.
    let version = ort::info()
        .split("rel-")
        .nth(1)
        .and_then(|rest| rest.split([',', ' ']).next())
        .map(str::to_owned);
    let gpu = gpu_providers()
        .into_iter()
        .filter(|(_, available, _)| *available)
        .map(|(name, _, _)| name)
        .collect();
    Ok(Runtime {
        path: loaded.path.clone(),
        version,
        gpu,
    })
}

/// The GPU execution providers to try, best first, with their names.
fn gpu_providers() -> Vec<(&'static str, bool, ExecutionProviderDispatch)> {
    let mut list = Vec::new();
    if cfg!(target_os = "macos") {
        list.push((
            "CoreML",
            ep::CoreML::is_available().unwrap_or(false),
            ep::CoreML::default().build(),
        ));
    } else {
        list.push((
            "CUDA",
            ep::CUDA::is_available().unwrap_or(false),
            ep::CUDA::default().build(),
        ));
        list.push((
            "MIGraphX",
            ep::MIGraphX::is_available().unwrap_or(false),
            ep::MIGraphX::default().build(),
        ));
        list.push((
            "ROCm",
            ep::ROCm::is_available().unwrap_or(false),
            ep::ROCm::default().build(),
        ));
        if cfg!(windows) {
            list.push((
                "DirectML",
                ep::DirectML::is_available().unwrap_or(false),
                ep::DirectML::default().build(),
            ));
        }
    }
    list.push((
        "WebGPU",
        ep::WebGPU::is_available().unwrap_or(false),
        ep::WebGPU::default().build(),
    ));
    list
}

/// Demucs in an ONNX Runtime session.
pub struct Demucs {
    session: Session,
    info: Info,
    backend: &'static str,
}

impl Network for Demucs {
    fn run(&mut self, mix: Vec<f32>, spec: Vec<f32>) -> Result<(Vec<f32>, Vec<f32>), String> {
        let length = self.info.segment;
        let bins = self.info.nfft / 2;
        let frames = length.div_ceil(self.info.hop);
        let fail = |e: ort::Error| e.to_string();
        let mix = Tensor::from_array(([1_usize, 2, length], mix)).map_err(fail)?;
        let spec = Tensor::from_array(([1_usize, 4, bins, frames], spec)).map_err(fail)?;
        let outputs = self
            .session
            .run(ort::inputs!["mix" => mix, "spec" => spec])
            .map_err(fail)?;
        let take = |name: &str| -> Result<Vec<f32>, String> {
            let value = outputs
                .get(name)
                .ok_or_else(|| format!("the stem model has no output `{name}`"))?;
            let (_, data) = value.try_extract_tensor::<f32>().map_err(fail)?;
            Ok(data.to_vec())
        };
        Ok((take("freq")?, take("time")?))
    }
}

impl Demucs {
    /// Opens `model` with ONNX Runtime (see [`runtime_candidates`]), on the
    /// first GPU execution provider that takes it, else on the CPU.
    ///
    /// # Errors
    ///
    /// Why the runtime or the model could not be loaded.
    pub fn load(model: &Path) -> Result<Self, String> {
        let env = &loaded()?.environment;
        let open = |providers: Vec<ExecutionProviderDispatch>| -> Result<Session, String> {
            Session::builder(env)
                .map_err(|e| e.to_string())?
                .with_execution_providers(providers)
                .map_err(|e| e.to_string())?
                .commit_from_file(model)
                .map_err(|e| format!("cannot open the stem model {}: {e}", model.display()))
        };
        let gpu = gpu_providers()
            .into_iter()
            .filter(|(_, available, _)| *available)
            .find_map(|(name, _, provider)| {
                open(vec![provider.error_on_failure()])
                    .ok()
                    .map(|s| (name, s))
            });
        let (backend, session) = match gpu {
            Some(found) => found,
            None => ("CPU", open(Vec::new())?),
        };
        let info = session
            .metadata()
            .ok()
            .and_then(|m| m.custom("musiccraft"))
            .ok_or_else(|| "the stem model has no MusicCraft metadata".to_owned())?;
        let info: Info = serde_json::from_str(&info).map_err(|e| e.to_string())?;
        if info.nfft != 4 * info.hop || !info.nfft.is_power_of_two() || info.segment == 0 {
            return Err("the stem model's transform settings are not supported".to_owned());
        }
        Ok(Self {
            session,
            info,
            backend,
        })
    }

    /// Where the network runs: `CUDA`, `MIGraphX`, `ROCm`, `DirectML`,
    /// `CoreML`, `WebGPU` or `CPU`.
    #[must_use]
    pub fn backend(&self) -> &'static str {
        self.backend
    }

    /// The model's settings.
    #[must_use]
    pub fn info(&self) -> &Info {
        &self.info
    }

    /// Splits a stereo sound; see [`separate`].
    ///
    /// # Errors
    ///
    /// [`Error::Cancelled`] or why the network failed.
    pub fn split(
        &mut self,
        left: &[f32],
        right: &[f32],
        sample_rate: f64,
        progress: &mut dyn FnMut(f32) -> bool,
    ) -> Result<Stems, Error> {
        let info = self.info.clone();
        separate(self, &info, left, right, sample_rate, progress)
    }
}

#[cfg(test)]
mod tests;
