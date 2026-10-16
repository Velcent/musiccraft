//! DSP building blocks. Everything that processes audio runs on the audio thread:
//! no allocation, no locks, no I/O, no panics. Constructors (`new`) may allocate
//! and must be called on the main thread.

pub mod beat;
pub mod blocks;
pub mod character;
pub mod convolution;
pub mod creative;
mod dynamics;
pub mod edit;
mod effect;
mod envelope;
pub mod fft;
mod filter;
pub mod guitar;
pub mod hall;
pub mod mastering;
pub mod modulation;
mod osc;
pub mod pitch;
pub mod rack;
pub mod response;
pub mod resynth;
mod reverb;
mod sample;
pub mod scratch;
mod time;
pub mod toolbox;
mod util;

pub use dynamics::{Compressor, Distortion, Limiter};
pub use effect::{Effect, FilterEffect, ProcessContext, ThreeBandEq};
pub use envelope::{Adsr, AdsrParams};
pub use filter::{Biquad, Svf, SvfMode};
pub use osc::{Oscillator, Sine, Waveform};
pub use reverb::Reverb;
pub use sample::SampleBuffer;
pub use time::{Chorus, Delay};
pub use util::{PanLaw, db_to_gain, flush_denormal, gain_to_db, key_to_hz, pan_gains};
