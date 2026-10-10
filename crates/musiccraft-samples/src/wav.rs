use std::path::Path;

use musiccraft_dsp::SampleBuffer;

use crate::SampleError;

/// Sample encoding for written WAV files.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WavFormat {
    /// 16-bit integer PCM with triangular dither.
    #[default]
    Int16,
    /// 24-bit integer PCM.
    Int24,
    /// 32-bit IEEE float.
    Float32,
}

impl WavFormat {
    /// Parses `16`, `24` or `32f`/`float`.
    #[must_use]
    pub fn parse(text: &str) -> Option<Self> {
        match text {
            "16" => Some(Self::Int16),
            "24" => Some(Self::Int24),
            "32" | "32f" | "float" => Some(Self::Float32),
            _ => None,
        }
    }
}

fn format_error(err: hound::Error) -> SampleError {
    match err {
        hound::Error::IoError(e) => SampleError::Io(e.to_string()),
        other => SampleError::Format(other.to_string()),
    }
}

/// Decodes a WAV file. Mono files become identical left and right channels;
/// files with more than two channels keep the first two.
///
/// # Errors
///
/// Returns [`SampleError`] if the file cannot be read or decoded.
pub fn load_wav(path: &Path) -> Result<SampleBuffer, SampleError> {
    let reader = hound::WavReader::open(path)
        .map_err(|e| SampleError::Io(format!("cannot open {}: {e}", path.display())))?;
    let spec = reader.spec();
    let channels = usize::from(spec.channels.max(1));
    let interleaved: Vec<f32> = match spec.sample_format {
        hound::SampleFormat::Float => reader
            .into_samples::<f32>()
            .collect::<Result<_, _>>()
            .map_err(format_error)?,
        hound::SampleFormat::Int => {
            let scale = 1.0 / (1_i64 << (spec.bits_per_sample.clamp(1, 32) - 1)) as f32;
            reader
                .into_samples::<i32>()
                .map(|s| s.map(|v| v as f32 * scale))
                .collect::<Result<_, _>>()
                .map_err(format_error)?
        }
    };
    let frames = interleaved.len() / channels;
    let mut left = Vec::with_capacity(frames);
    let mut right = Vec::with_capacity(frames);
    for frame in interleaved.chunks_exact(channels) {
        let l = frame.first().copied().unwrap_or(0.0);
        left.push(l);
        right.push(frame.get(1).copied().unwrap_or(l));
    }
    Ok(SampleBuffer::stereo(
        left,
        right,
        f64::from(spec.sample_rate),
    ))
}

/// Writes stereo audio to a WAV file.
///
/// # Errors
///
/// Returns [`SampleError`] if the file cannot be written.
pub fn write_wav(
    path: &Path,
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
    format: WavFormat,
) -> Result<(), SampleError> {
    let (bits, sample_format) = match format {
        WavFormat::Int16 => (16, hound::SampleFormat::Int),
        WavFormat::Int24 => (24, hound::SampleFormat::Int),
        WavFormat::Float32 => (32, hound::SampleFormat::Float),
    };
    let spec = hound::WavSpec {
        channels: 2,
        sample_rate,
        bits_per_sample: bits,
        sample_format,
    };
    let mut writer = hound::WavWriter::create(path, spec)
        .map_err(|e| SampleError::Io(format!("cannot create {}: {e}", path.display())))?;
    let mut dither = Dither::default();
    for (l, r) in left.iter().zip(right) {
        for sample in [*l, *r] {
            let sample = if sample.is_finite() { sample } else { 0.0 };
            match format {
                WavFormat::Float32 => writer.write_sample(sample),
                WavFormat::Int16 => {
                    let value = (sample * 32_767.0 + dither.next()).round();
                    writer.write_sample(value.clamp(-32_768.0, 32_767.0) as i16)
                }
                WavFormat::Int24 => {
                    let value = (sample * 8_388_607.0).round();
                    writer.write_sample(value.clamp(-8_388_608.0, 8_388_607.0) as i32)
                }
            }
            .map_err(format_error)?;
        }
    }
    writer.finalize().map_err(format_error)
}

/// Triangular (TPDF) dither of ±1 LSB, deterministic so exports are repeatable.
#[derive(Debug)]
struct Dither {
    state: u32,
}

impl Default for Dither {
    fn default() -> Self {
        Self { state: 0x2545_f491 }
    }
}

impl Dither {
    fn uniform(&mut self) -> f32 {
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x as f32 / u32::MAX as f32
    }

    fn next(&mut self) -> f32 {
        self.uniform() - self.uniform()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("musiccraft-{}-{name}", std::process::id()))
    }

    #[test]
    fn float_wav_round_trips_exactly() {
        let path = temp("float.wav");
        let left = vec![0.0, 0.5, -0.25, 1.0];
        let right = vec![0.1, -0.5, 0.25, -1.0];
        write_wav(&path, &left, &right, 48_000, WavFormat::Float32).unwrap();
        let buffer = load_wav(&path).unwrap();
        assert_eq!(buffer.left(), left.as_slice());
        assert_eq!(buffer.right(), right.as_slice());
        assert!((buffer.sample_rate() - 48_000.0).abs() < f64::EPSILON);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn int16_wav_is_close() {
        let path = temp("int16.wav");
        let left = vec![0.0, 0.5, -0.5, 2.0];
        write_wav(&path, &left, &left, 44_100, WavFormat::Int16).unwrap();
        let buffer = load_wav(&path).unwrap();
        for (got, want) in buffer.left().iter().zip([0.0, 0.5, -0.5, 1.0]) {
            assert!((got - want).abs() < 2.0 / 32_768.0, "{got} vs {want}");
        }
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn mono_files_fill_both_channels() {
        let path = temp("mono.wav");
        let spec = hound::WavSpec {
            channels: 1,
            sample_rate: 22_050,
            bits_per_sample: 24,
            sample_format: hound::SampleFormat::Int,
        };
        let mut writer = hound::WavWriter::create(&path, spec).unwrap();
        writer.write_sample(4_194_304_i32).unwrap();
        writer.finalize().unwrap();
        let buffer = load_wav(&path).unwrap();
        assert_eq!(buffer.len(), 1);
        assert!((buffer.left()[0] - 0.5).abs() < 1e-6);
        assert_eq!(buffer.left(), buffer.right());
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn missing_and_bogus_files_are_errors() {
        assert!(matches!(
            load_wav(Path::new("/no/such/file.wav")),
            Err(SampleError::Io(_))
        ));
        let path = temp("bogus.wav");
        std::fs::write(&path, b"not a wav file at all").unwrap();
        assert!(load_wav(&path).is_err());
        std::fs::remove_file(path).unwrap();
        assert_eq!(WavFormat::parse("24"), Some(WavFormat::Int24));
        assert_eq!(WavFormat::parse("8"), None);
    }
}
