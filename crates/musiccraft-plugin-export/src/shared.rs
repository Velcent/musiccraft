//! State an effect plugin shares between the audio thread, the editor and
//! state loading: the detected pitch for the tuner display, the sample rate,
//! and the convolution reverb's impulse (a generated room or a WAV file).
//!
//! A new room needs a new convolver, which allocates. It is built off the
//! audio thread and handed over through a lock-free queue; the audio thread
//! hands the old effect back through a second queue, and it is dropped here,
//! off the audio thread. The lock below is only ever taken off the audio
//! thread.

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};

use musiccraft_dsp::convolution::Convolver;
use musiccraft_dsp::{Effect, SampleBuffer};
use musiccraft_params::IMPULSE_RESPONSES;

/// The room a new convolution reverb starts with.
pub const DEFAULT_ROOM: &str = "hall";

/// Effects waiting for, or coming back from, the audio thread.
const QUEUE: usize = 4;

/// The non-audio ends of the hand-over queues.
struct Handoff {
    to_audio: rtrb::Producer<Box<Effect>>,
    garbage: rtrb::Consumer<Box<Effect>>,
}

/// The audio thread's ends of the hand-over queues.
pub struct AudioSide {
    incoming: rtrb::Consumer<Box<Effect>>,
    garbage: rtrb::Producer<Box<Effect>>,
}

impl AudioSide {
    /// Swaps in an effect built off the audio thread, if one is waiting and
    /// the old one can be handed back. Allocation-free.
    pub fn take_new(&mut self, effect: &mut Box<Effect>) {
        if self.garbage.slots() == 0 {
            return;
        }
        if let Ok(new) = self.incoming.pop() {
            let old = std::mem::replace(effect, new);
            // Room was checked above; on failure the box would be dropped
            // here, which cannot happen.
            let _ = self.garbage.push(old);
        }
    }
}

/// The convolution reverb's impulse: a generated room, or a file and its
/// loaded audio.
#[derive(Clone)]
struct Impulse {
    name: String,
    file: Option<Arc<SampleBuffer>>,
}

/// See the module documentation.
pub struct Shared {
    sample_rate: AtomicU32,
    impulse: Mutex<Impulse>,
    /// Why the last impulse file could not be loaded.
    error: Mutex<Option<String>>,
    detected: AtomicU32,
    handoff: Mutex<Handoff>,
}

impl Shared {
    /// Shared state and the audio thread's side of it.
    pub fn new() -> (Self, AudioSide) {
        let (to_audio, incoming) = rtrb::RingBuffer::new(QUEUE);
        let (garbage_in, garbage) = rtrb::RingBuffer::new(QUEUE);
        (
            Self {
                sample_rate: AtomicU32::new(0),
                impulse: Mutex::new(Impulse {
                    name: DEFAULT_ROOM.to_owned(),
                    file: None,
                }),
                error: Mutex::new(None),
                detected: AtomicU32::new(f32::NAN.to_bits()),
                handoff: Mutex::new(Handoff { to_audio, garbage }),
            },
            AudioSide {
                incoming,
                garbage: garbage_in,
            },
        )
    }

    /// Records the sample rate the plugin was set up with.
    pub fn set_sample_rate(&self, rate: f32) {
        self.sample_rate.store(rate.to_bits(), Ordering::Release);
    }

    /// The sample rate, once the plugin is set up.
    pub fn sample_rate(&self) -> Option<f32> {
        let rate = f32::from_bits(self.sample_rate.load(Ordering::Acquire));
        (rate > 0.0).then_some(rate)
    }

    /// The pitch a tuner or pitch corrector last heard (audio thread).
    pub fn set_detected(&self, hz: Option<f32>) {
        let bits = hz.unwrap_or(f32::NAN).to_bits();
        self.detected.store(bits, Ordering::Relaxed);
    }

    /// The pitch last heard.
    pub fn detected(&self) -> Option<f32> {
        let hz = f32::from_bits(self.detected.load(Ordering::Relaxed));
        hz.is_finite().then_some(hz)
    }

    /// The convolution reverb's impulse: a room name or a file path.
    pub fn impulse_name(&self) -> String {
        self.impulse
            .lock()
            .map_or_else(|_| DEFAULT_ROOM.to_owned(), |i| i.name.clone())
    }

    /// Why the last impulse file could not be loaded, if it could not.
    pub fn impulse_error(&self) -> Option<String> {
        self.error.lock().ok().and_then(|e| e.clone())
    }

    /// A convolution reverb with the current impulse at `rate`.
    pub fn convolver(&self, rate: f32) -> Effect {
        let impulse = self.impulse.lock().map_or_else(
            |_| Impulse {
                name: DEFAULT_ROOM.to_owned(),
                file: None,
            },
            |i| i.clone(),
        );
        Effect::Convolution(match impulse.file {
            Some(buffer) => Convolver::from_buffer(&buffer, rate),
            None => Convolver::builtin(&impulse.name, rate),
        })
    }

    /// Chooses the convolution reverb's impulse (main or GUI thread): one of
    /// the generated rooms, or the path of a WAV file. A file that cannot be
    /// read keeps the previous impulse and records why. When the plugin is
    /// running, the new reverb is built here and handed to the audio thread.
    pub fn set_impulse(&self, impulse: &str) {
        let file = if IMPULSE_RESPONSES.contains(&impulse) {
            None
        } else {
            match musiccraft_antiserum_presets::load_sample(impulse, 48_000.0) {
                Some(buffer) if !buffer.is_empty() => Some(buffer),
                _ => {
                    if let Ok(mut error) = self.error.lock() {
                        *error = Some(format!("cannot read the WAV file {impulse}"));
                    }
                    return;
                }
            }
        };
        if let Ok(mut error) = self.error.lock() {
            *error = None;
        }
        {
            let Ok(mut current) = self.impulse.lock() else {
                return;
            };
            // The same room changes nothing; the same file is read again (it
            // may have been edited).
            if current.name == impulse && file.is_none() {
                return;
            }
            *current = Impulse {
                name: impulse.to_owned(),
                file,
            };
        }
        let Some(rate) = self.sample_rate() else {
            return;
        };
        let Ok(mut handoff) = self.handoff.lock() else {
            return;
        };
        while handoff.garbage.pop().is_ok() {}
        let _ = handoff.to_audio.push(Box::new(self.convolver(rate)));
    }

    /// Drops effects the audio thread handed back.
    pub fn collect_garbage(&self) {
        if let Ok(mut handoff) = self.handoff.lock() {
            while handoff.garbage.pop().is_ok() {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use musiccraft_params::EffectKind;

    #[test]
    fn a_new_room_reaches_the_audio_thread_and_the_old_one_comes_back() {
        let (shared, mut audio) = Shared::new();
        assert_eq!(shared.impulse_name(), DEFAULT_ROOM);
        shared.set_sample_rate(48_000.0);
        let mut effect = Box::new(shared.convolver(48_000.0));
        shared.set_impulse("plate");
        assert_eq!(shared.impulse_name(), "plate");
        audio.take_new(&mut effect);
        assert_eq!(effect.kind(), EffectKind::Convolution);
        assert_eq!(audio.incoming.slots(), 0, "taken");
        assert_eq!(audio.garbage.slots(), QUEUE - 1, "old one handed back");
        shared.collect_garbage();
        // Unreadable files and the same room change nothing.
        shared.set_impulse("nowhere");
        assert!(
            shared
                .impulse_error()
                .is_some_and(|e| e.contains("nowhere"))
        );
        shared.set_impulse("plate");
        assert_eq!(shared.impulse_name(), "plate");
        assert_eq!(shared.impulse_error(), None);
        assert_eq!(audio.incoming.slots(), 0);
    }

    #[test]
    fn an_impulse_file_is_loaded_and_convolves_with_its_own_shape() {
        let dir = std::env::temp_dir().join(format!("mc-ir-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        // Clicks at 0 and 100 frames at 24 kHz: through the reverb at 48 kHz
        // their peaks land 200 frames apart (resampled).
        let peak_of = |at: usize| {
            let path = dir.join(format!("click-{at}.wav"));
            let mut click = vec![0.0; 400];
            click[at] = 1.0;
            musiccraft_samples::write_wav(
                &path,
                &click,
                &click,
                24_000,
                musiccraft_samples::WavFormat::Float32,
            )
            .unwrap();
            let (shared, mut audio) = Shared::new();
            shared.set_sample_rate(48_000.0);
            let mut effect = Box::new(shared.convolver(48_000.0));
            let name = path.to_string_lossy().into_owned();
            shared.set_impulse(&name);
            assert_eq!(shared.impulse_name(), name);
            assert_eq!(shared.impulse_error(), None);
            audio.take_new(&mut effect);
            assert_eq!(audio.incoming.slots(), 0, "taken");
            let mut l = vec![0.0f32; 4096];
            let mut r = vec![0.0f32; 4096];
            l[0] = 1.0;
            r[0] = 1.0;
            let mut values = EffectKind::Convolution.defaults();
            values[0] = 1.0; // all wet
            let ctx = musiccraft_dsp::ProcessContext {
                sample_rate: 48_000.0,
                tempo_bpm: 120.0,
                beats: 0.0,
            };
            effect.process(&mut l, &mut r, &values, ctx);
            l.iter()
                .enumerate()
                .max_by(|a, b| a.1.abs().total_cmp(&b.1.abs()))
                .map(|(i, _)| i)
                .unwrap()
        };
        // Within a frame: the resampler interpolates the clicks.
        let apart = peak_of(100) - peak_of(0);
        assert!(
            (199..=201).contains(&apart),
            "the file's own shape: {apart}"
        );
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn before_setup_only_the_name_changes() {
        let (shared, audio) = Shared::new();
        shared.set_impulse("room");
        assert_eq!(shared.impulse_name(), "room");
        assert_eq!(audio.incoming.slots(), 0);
        shared.set_detected(Some(440.0));
        assert_eq!(shared.detected(), Some(440.0));
        shared.set_detected(None);
        assert_eq!(shared.detected(), None);
    }
}
