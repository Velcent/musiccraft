//! State an effect plugin shares between the audio thread, the editor and
//! state loading: the detected pitch for the tuner display, the sample rate,
//! and the convolution reverb's room.
//!
//! A new room needs a new convolver, which allocates. It is built off the
//! audio thread and handed over through a lock-free queue; the audio thread
//! hands the old effect back through a second queue, and it is dropped here,
//! off the audio thread. The lock below is only ever taken off the audio
//! thread.

use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, Ordering};

use musiccraft_dsp::Effect;
use musiccraft_dsp::convolution::Convolver;
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

/// See the module documentation.
pub struct Shared {
    sample_rate: AtomicU32,
    impulse: AtomicU32,
    detected: AtomicU32,
    handoff: Mutex<Handoff>,
}

impl Shared {
    /// Shared state and the audio thread's side of it.
    pub fn new() -> (Self, AudioSide) {
        let (to_audio, incoming) = rtrb::RingBuffer::new(QUEUE);
        let (garbage_in, garbage) = rtrb::RingBuffer::new(QUEUE);
        let impulse = IMPULSE_RESPONSES
            .iter()
            .position(|r| *r == DEFAULT_ROOM)
            .unwrap_or(0);
        (
            Self {
                sample_rate: AtomicU32::new(0),
                impulse: AtomicU32::new(impulse as u32),
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

    /// The convolution reverb's room.
    pub fn impulse_name(&self) -> String {
        let index = self.impulse.load(Ordering::Acquire) as usize;
        IMPULSE_RESPONSES
            .get(index)
            .copied()
            .unwrap_or(DEFAULT_ROOM)
            .to_owned()
    }

    /// A convolution reverb with the current room at `rate`.
    pub fn convolver(&self, rate: f32) -> Effect {
        Effect::Convolution(Convolver::builtin(&self.impulse_name(), rate))
    }

    /// Chooses the convolution reverb's room (main or GUI thread). When the
    /// plugin is running, the new reverb is built here and handed to the
    /// audio thread. Unknown rooms are ignored.
    pub fn set_impulse(&self, room: &str) {
        let Some(index) = IMPULSE_RESPONSES.iter().position(|r| *r == room) else {
            return;
        };
        let previous = self.impulse.swap(index as u32, Ordering::AcqRel);
        if previous == index as u32 {
            return;
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
        // Unknown rooms and the same room change nothing.
        shared.set_impulse("nowhere");
        shared.set_impulse("plate");
        assert_eq!(shared.impulse_name(), "plate");
        assert_eq!(audio.incoming.slots(), 0);
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
