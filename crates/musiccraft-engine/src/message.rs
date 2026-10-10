use crate::{ChannelParams, EffectParams, External, Graph, Preview, Sequence, TrackParams};

/// A message from the main thread to the engine.
///
/// All variants are fixed-size. The two payload variants carry boxes that were
/// allocated on the main thread; the values they replace come back as [`Garbage`].
#[derive(Debug)]
pub enum EngineMessage {
    /// Start advancing the playhead from its current position.
    Play,
    /// Halt, release sounding notes, and return the playhead to the start.
    Stop,
    /// Halt, release sounding notes, and keep the playhead where it is.
    Pause,
    /// Change the tempo in BPM. Non-finite or non-positive values are ignored.
    SetTempo(f64),
    /// Move the playhead to a position in ticks.
    Seek(f64),
    /// Update one channel's settings.
    Channel {
        /// Channel index.
        channel: u16,
        /// New settings.
        params: ChannelParams,
    },
    /// Update one mixer track's settings.
    Track {
        /// Track index (0 = master).
        track: u16,
        /// New settings.
        params: TrackParams,
    },
    /// Update one effect slot's settings.
    Effect {
        /// Track index (0 = master).
        track: u16,
        /// Slot index on the track.
        slot: u8,
        /// New settings.
        params: EffectParams,
    },
    /// Start a held note outside the sequence (preview, MIDI input).
    NoteOn {
        /// Channel index.
        channel: u16,
        /// MIDI key.
        key: u8,
        /// Velocity, 0..=1.
        velocity: f32,
    },
    /// Release a held note started by [`EngineMessage::NoteOn`].
    NoteOff {
        /// Channel index.
        channel: u16,
        /// MIDI key.
        key: u8,
    },
    /// Release every sounding voice.
    AllNotesOff,
    /// Turn the beat click on or off.
    SetMetronome(bool),
    /// Replace the channel/mixer graph. Voices are silenced.
    SetGraph(Box<Graph>),
    /// Replace the note sequence.
    SetSequence(Box<Sequence>),
    /// Audition a sample directly on the master output (browser preview).
    PlayPreview(Box<Preview>),
    /// Silence the browser preview.
    StopPreview,
    /// Install (or with `None`, remove) the external processor in a slot. A
    /// replaced processor is stopped and handed back as garbage.
    SetExternal {
        /// Slot index, below [`crate::MAX_EXTERNALS`].
        slot: u16,
        /// The new processor.
        external: Option<Box<dyn External>>,
    },
    /// Set a parameter of the external processor in a slot.
    ExternalParam {
        /// Slot index.
        slot: u16,
        /// Parameter id in the processor's numbering.
        id: u32,
        /// Value in the processor's own units.
        value: f64,
    },
}

impl EngineMessage {
    /// Whether applying this message produces a [`Garbage`] value.
    #[must_use]
    pub fn returns_garbage(&self) -> bool {
        matches!(
            self,
            Self::SetGraph(_)
                | Self::SetSequence(_)
                | Self::PlayPreview(_)
                | Self::SetExternal { .. }
        )
    }
}

/// A replaced value sent back to the main thread to be dropped there.
#[derive(Debug)]
pub enum Garbage {
    /// A previous graph.
    Graph(Box<Graph>),
    /// A previous sequence.
    Sequence(Box<Sequence>),
    /// A previous preview.
    Preview(Box<Preview>),
    /// A replaced external processor, already stopped.
    External(Box<dyn External>),
}

/// A note event from a live source such as a MIDI keyboard. Sent through its own
/// queue so that a MIDI thread can talk to the engine directly.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum LiveEvent {
    /// Start a held note.
    NoteOn {
        /// Channel index.
        channel: u16,
        /// MIDI key.
        key: u8,
        /// Velocity, 0..=1.
        velocity: f32,
    },
    /// Release a held note.
    NoteOff {
        /// Channel index.
        channel: u16,
        /// MIDI key.
        key: u8,
    },
}
