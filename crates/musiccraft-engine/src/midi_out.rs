//! MIDI Out channels: instead of sounding, they queue MIDI note messages for
//! external gear. The audio thread only pushes fixed-size events into a
//! preallocated queue; the app sends them from its own thread.

use rtrb::{Consumer, Producer, RingBuffer};

use crate::MAX_BLOCK;

/// Capacity of the outgoing MIDI queue; when it is full, messages are dropped.
pub const MIDI_OUT_CAPACITY: usize = 1024;
/// Most notes MIDI Out channels keep track of at once.
const MAX_HELD: usize = 256;

/// A MIDI message for external gear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MidiOutEvent {
    /// Frames into the audio block in which the engine produced it.
    pub delay: u32,
    /// The message: note on or off and control change use all three bytes,
    /// program change two, clock and transport messages one (see `len`).
    pub bytes: [u8; 3],
    /// How many of `bytes` the message uses.
    pub len: u8,
}

impl MidiOutEvent {
    fn note_on(midi_channel: u8, key: u8, velocity: f32, delay: u32) -> Self {
        let velocity = (velocity.clamp(0.0, 1.0) * 127.0).round().clamp(1.0, 127.0) as u8;
        Self {
            delay,
            bytes: [0x90 | (midi_channel & 0x0f), key & 0x7f, velocity],
            len: 3,
        }
    }

    fn note_off(midi_channel: u8, key: u8, delay: u32) -> Self {
        Self {
            delay,
            bytes: [0x80 | (midi_channel & 0x0f), key & 0x7f, 0],
            len: 3,
        }
    }

    /// The message's bytes.
    #[must_use]
    pub fn message(&self) -> &[u8] {
        self.bytes
            .get(..usize::from(self.len))
            .unwrap_or(&self.bytes)
    }
}

/// MIDI clock (24 per quarter note) and transport bytes.
pub(crate) const CLOCK: u8 = 0xf8;
pub(crate) const START: u8 = 0xfa;
pub(crate) const CONTINUE: u8 = 0xfb;
pub(crate) const STOP: u8 = 0xfc;

/// Receives the messages MIDI Out channels produce (main side).
pub struct MidiOutReceiver {
    consumer: Consumer<MidiOutEvent>,
}

impl MidiOutReceiver {
    /// The next queued message, if any.
    pub fn pop(&mut self) -> Option<MidiOutEvent> {
        self.consumer.pop().ok()
    }
}

/// A note a MIDI Out channel has started and not yet ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Held {
    /// Engine channel that started it.
    channel: u16,
    midi_channel: u8,
    key: u8,
    /// Frames from the start of the current block until its note off;
    /// `None` until a live note off arrives.
    remaining: Option<u64>,
}

/// The audio-thread side: sends note messages and ends gated notes on time.
pub(crate) struct MidiOut {
    producer: Producer<MidiOutEvent>,
    held: [Option<Held>; MAX_HELD],
}

impl std::fmt::Debug for MidiOut {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MidiOut")
            .field("held", &self.held.iter().flatten().count())
            .finish_non_exhaustive()
    }
}

/// Creates the two ends of the outgoing MIDI queue. Main thread only.
pub(crate) fn midi_out_pair() -> (MidiOut, MidiOutReceiver) {
    let (producer, consumer) = RingBuffer::new(MIDI_OUT_CAPACITY);
    (
        MidiOut {
            producer,
            held: [None; MAX_HELD],
        },
        MidiOutReceiver { consumer },
    )
}

impl MidiOut {
    fn send(&mut self, event: MidiOutEvent) {
        // A full queue (nobody listening, or a stalled sender) drops messages.
        let _ = self.producer.push(event);
    }

    /// Sends a control change.
    pub(crate) fn control(&mut self, midi_channel: u8, controller: u8, value: u8) {
        self.send(MidiOutEvent {
            delay: 0,
            bytes: [
                0xb0 | (midi_channel & 0x0f),
                controller & 0x7f,
                value & 0x7f,
            ],
            len: 3,
        });
    }

    /// Sends a program change (`program` 0..=127).
    pub(crate) fn program(&mut self, midi_channel: u8, program: u8) {
        self.send(MidiOutEvent {
            delay: 0,
            bytes: [0xc0 | (midi_channel & 0x0f), program & 0x7f, 0],
            len: 2,
        });
    }

    /// Sends a one-byte system real-time message (clock, start, stop).
    pub(crate) fn realtime(&mut self, byte: u8, delay: u32) {
        self.send(MidiOutEvent {
            delay,
            bytes: [byte, 0, 0],
            len: 1,
        });
    }

    /// Starts a note `delay` frames into the current block; with a `gate` it
    /// ends that many frames after it starts.
    pub(crate) fn note_on(
        &mut self,
        channel: u16,
        midi_channel: u8,
        key: u8,
        velocity: f32,
        delay: u32,
        gate: Option<u64>,
    ) {
        // Gear plays one note per key and channel: end a sounding one first.
        for entry in &mut self.held {
            if entry.is_some_and(|h| h.midi_channel == midi_channel && h.key == key) {
                *entry = None;
                let _ = self
                    .producer
                    .push(MidiOutEvent::note_off(midi_channel, key, delay));
            }
        }
        let Some(free) = self.held.iter_mut().find(|h| h.is_none()) else {
            // Without room to remember it, the note could never end.
            return;
        };
        *free = Some(Held {
            channel,
            midi_channel,
            key,
            remaining: gate.map(|g| u64::from(delay) + g.max(1)),
        });
        self.send(MidiOutEvent::note_on(midi_channel, key, velocity, delay));
    }

    /// Ends the live (ungated) note `key` that `channel` started.
    pub(crate) fn note_off(&mut self, channel: u16, key: u8) {
        for i in 0..MAX_HELD {
            let Some(held) = self.held.get(i).copied().flatten() else {
                continue;
            };
            if held.channel == channel && held.key == key && held.remaining.is_none() {
                if let Some(entry) = self.held.get_mut(i) {
                    *entry = None;
                }
                self.send(MidiOutEvent::note_off(held.midi_channel, key, 0));
            }
        }
    }

    /// Ends every note whose `keep` test fails, at the start of the block.
    pub(crate) fn release_unless(&mut self, keep: impl Fn(u16, u8) -> bool) {
        for i in 0..MAX_HELD {
            let Some(held) = self.held.get(i).copied().flatten() else {
                continue;
            };
            if !keep(held.channel, held.midi_channel) {
                if let Some(entry) = self.held.get_mut(i) {
                    *entry = None;
                }
                self.send(MidiOutEvent::note_off(held.midi_channel, held.key, 0));
            }
        }
    }

    /// Ends every held note.
    pub(crate) fn all_off(&mut self) {
        self.release_unless(|_, _| false);
    }

    /// Sends the note offs that fall inside the next `frames` frames and
    /// counts the rest down.
    pub(crate) fn advance(&mut self, frames: usize) {
        let frames = frames.min(MAX_BLOCK) as u64;
        for i in 0..MAX_HELD {
            let Some(held) = self.held.get(i).copied().flatten() else {
                continue;
            };
            let Some(remaining) = held.remaining else {
                continue;
            };
            let entry = self.held.get_mut(i);
            if remaining < frames {
                if let Some(entry) = entry {
                    *entry = None;
                }
                self.send(MidiOutEvent::note_off(
                    held.midi_channel,
                    held.key,
                    remaining as u32,
                ));
            } else if let Some(Some(entry)) = entry {
                entry.remaining = Some(remaining - frames);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn drain(rx: &mut MidiOutReceiver) -> Vec<MidiOutEvent> {
        std::iter::from_fn(|| rx.pop()).collect()
    }

    #[test]
    fn gated_notes_end_on_time() {
        let (mut out, mut rx) = midi_out_pair();
        out.note_on(0, 2, 60, 1.0, 10, Some(100));
        out.advance(64);
        assert_eq!(
            drain(&mut rx),
            vec![MidiOutEvent {
                delay: 10,
                bytes: [0x92, 60, 127],
                len: 3,
            }]
        );
        out.advance(64);
        // 10 + 100 = 110 frames after the first block start: 46 into the second.
        assert_eq!(
            drain(&mut rx),
            vec![MidiOutEvent {
                delay: 46,
                bytes: [0x82, 60, 0],
                len: 3,
            }]
        );
        out.advance(64);
        assert!(drain(&mut rx).is_empty());
    }

    #[test]
    fn live_notes_wait_for_their_note_off() {
        let (mut out, mut rx) = midi_out_pair();
        out.note_on(3, 0, 64, 0.5, 0, None);
        out.advance(10_000);
        out.note_off(4, 64);
        assert_eq!(drain(&mut rx).len(), 1, "other channel's off ignored");
        out.note_off(3, 64);
        assert_eq!(drain(&mut rx).last().map(|e| e.bytes), Some([0x80, 64, 0]));
    }

    #[test]
    fn retriggering_a_key_ends_the_old_note_first() {
        let (mut out, mut rx) = midi_out_pair();
        out.note_on(0, 0, 60, 1.0, 0, None);
        out.note_on(0, 0, 60, 0.0, 5, Some(10));
        let bytes: Vec<[u8; 3]> = drain(&mut rx).iter().map(|e| e.bytes).collect();
        assert_eq!(bytes, vec![[0x90, 60, 127], [0x80, 60, 0], [0x90, 60, 1]]);
    }

    #[test]
    fn all_off_and_selective_release() {
        let (mut out, mut rx) = midi_out_pair();
        out.note_on(0, 0, 60, 1.0, 0, None);
        out.note_on(1, 1, 62, 1.0, 0, Some(1_000));
        drain(&mut rx);
        out.release_unless(|channel, _| channel == 1);
        assert_eq!(
            drain(&mut rx).iter().map(|e| e.bytes).collect::<Vec<_>>(),
            vec![[0x80, 60, 0]]
        );
        out.all_off();
        assert_eq!(
            drain(&mut rx).iter().map(|e| e.bytes).collect::<Vec<_>>(),
            vec![[0x81, 62, 0]]
        );
        out.advance(2_000);
        assert!(drain(&mut rx).is_empty(), "nothing left to end");
    }

    #[test]
    fn controls_programs_and_clock_have_their_lengths() {
        let (mut out, mut rx) = midi_out_pair();
        out.control(2, 74, 100);
        out.program(2, 5);
        out.realtime(CLOCK, 7);
        let got: Vec<(Vec<u8>, u32)> = drain(&mut rx)
            .iter()
            .map(|e| (e.message().to_vec(), e.delay))
            .collect();
        assert_eq!(
            got,
            vec![
                (vec![0xb2, 74, 100], 0),
                (vec![0xc2, 5], 0),
                (vec![0xf8], 7)
            ]
        );
    }

    #[test]
    fn a_full_queue_drops_instead_of_blocking() {
        let (mut out, _rx) = midi_out_pair();
        for i in 0..(MIDI_OUT_CAPACITY * 2) {
            out.note_on(0, 0, (i % 128) as u8, 1.0, 0, Some(1));
            out.advance(4);
        }
    }
}
