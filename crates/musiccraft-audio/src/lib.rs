//! Audio device output. Opens the system's default output device with `cpal` and
//! runs an [`Engine`] inside the device callback.

use std::error::Error;
use std::fmt;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, StreamConfig};
use musiccraft_engine::Engine;

/// Frames of scratch space for devices that do not take `f32` directly.
const SCRATCH_FRAMES: usize = 4096;

/// Why audio output could not start.
#[derive(Debug)]
pub enum AudioError {
    /// The system reports no output device.
    NoDevice,
    /// The device or host returned an error.
    Device(String),
    /// The device only offers a sample format we do not convert to.
    UnsupportedFormat(String),
}

impl fmt::Display for AudioError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NoDevice => write!(f, "no audio output device found"),
            Self::Device(msg) => write!(f, "audio device error: {msg}"),
            Self::UnsupportedFormat(format) => write!(f, "unsupported sample format {format}"),
        }
    }
}

impl Error for AudioError {}

fn device_error(err: impl fmt::Display) -> AudioError {
    AudioError::Device(err.to_string())
}

/// An output device and the configuration it will be opened with.
pub struct OutputDevice {
    device: cpal::Device,
    config: StreamConfig,
    format: SampleFormat,
    name: String,
}

impl OutputDevice {
    /// The default output device of the default host.
    ///
    /// # Errors
    ///
    /// Returns an error if there is no device or it cannot report a configuration.
    pub fn default_device() -> Result<Self, AudioError> {
        let host = cpal::default_host();
        let device = host.default_output_device().ok_or(AudioError::NoDevice)?;
        let supported = device.default_output_config().map_err(device_error)?;
        let name = device
            .description()
            .map_or_else(|_| "unknown device".to_owned(), |d| d.name().to_owned());
        Ok(Self {
            format: supported.sample_format(),
            config: supported.config(),
            device,
            name,
        })
    }

    /// Sample rate the device will run at, in hertz. Create the engine with it.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate
    }

    /// Number of output channels.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.config.channels
    }

    /// Human-readable device name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Starts the stream, moving `engine` into the audio callback.
    ///
    /// # Errors
    ///
    /// Returns an error if the stream cannot be built or started.
    pub fn start(self, engine: Engine) -> Result<AudioOutput, AudioError> {
        let errors = Arc::new(AtomicU32::new(0));
        let stream = match self.format {
            SampleFormat::F32 => self.build_f32(engine, &errors)?,
            SampleFormat::I16 => self.build_converted::<i16>(engine, &errors)?,
            SampleFormat::I32 => self.build_converted::<i32>(engine, &errors)?,
            SampleFormat::U16 => self.build_converted::<u16>(engine, &errors)?,
            SampleFormat::F64 => self.build_converted::<f64>(engine, &errors)?,
            other => return Err(AudioError::UnsupportedFormat(other.to_string())),
        };
        stream.play().map_err(device_error)?;
        Ok(AudioOutput {
            _stream: stream,
            sample_rate: self.config.sample_rate,
            channels: self.config.channels,
            name: self.name,
            errors,
        })
    }

    fn error_callback(errors: &Arc<AtomicU32>) -> impl FnMut(cpal::Error) + Send + 'static {
        let errors = Arc::clone(errors);
        move |_err| {
            errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    fn build_f32(
        &self,
        mut engine: Engine,
        errors: &Arc<AtomicU32>,
    ) -> Result<cpal::Stream, AudioError> {
        let channels = usize::from(self.config.channels);
        self.device
            .build_output_stream(
                self.config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    engine.process(data, channels);
                },
                Self::error_callback(errors),
                None,
            )
            .map_err(device_error)
    }

    fn build_converted<T>(
        &self,
        mut engine: Engine,
        errors: &Arc<AtomicU32>,
    ) -> Result<cpal::Stream, AudioError>
    where
        T: SizedSample + FromSample<f32> + Send + 'static,
    {
        let channels = usize::from(self.config.channels);
        // Allocated here, on the main thread, and reused by the callback.
        let mut scratch = vec![0.0_f32; SCRATCH_FRAMES * channels.max(1)];
        self.device
            .build_output_stream(
                self.config,
                move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                    for chunk in data.chunks_mut(scratch.len().max(1)) {
                        let len = chunk.len().min(scratch.len());
                        let Some(rendered) = scratch.get_mut(..len) else {
                            continue;
                        };
                        engine.process(rendered, channels);
                        for (out, sample) in chunk.iter_mut().zip(rendered.iter()) {
                            *out = T::from_sample(*sample);
                        }
                    }
                },
                Self::error_callback(errors),
                None,
            )
            .map_err(device_error)
    }
}

/// A running output stream. Audio stops when this is dropped.
pub struct AudioOutput {
    _stream: cpal::Stream,
    sample_rate: u32,
    channels: u16,
    name: String,
    errors: Arc<AtomicU32>,
}

impl AudioOutput {
    /// Sample rate in hertz.
    #[must_use]
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Number of output channels.
    #[must_use]
    pub fn channels(&self) -> u16 {
        self.channels
    }

    /// Device name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Number of stream errors (for example underruns) reported so far.
    #[must_use]
    pub fn error_count(&self) -> u32 {
        self.errors.load(Ordering::Relaxed)
    }
}
