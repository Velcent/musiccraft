//! DSP building blocks. Everything that processes audio runs on the audio thread:
//! no allocation, no locks, no I/O, no panics. Constructors (`new`) may allocate
//! and must be called on the main thread.

mod dynamics;
mod effect;
mod envelope;
mod filter;
mod osc;
mod reverb;
mod sample;
mod time;
mod util;

pub use dynamics::{Compressor, Distortion, Limiter};
pub use effect::{Effect, FilterEffect, ProcessContext, ThreeBandEq};
pub use envelope::{Adsr, AdsrParams};
pub use filter::{Biquad, Svf, SvfMode};
pub use osc::{Oscillator, Sine, Waveform};
pub use reverb::Reverb;
pub use sample::SampleBuffer;
pub use time::{Chorus, Delay};
pub use util::{db_to_gain, gain_to_db, key_to_hz, pan_gains};
