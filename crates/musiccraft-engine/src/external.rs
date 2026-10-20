use std::any::Any;
use std::fmt;

/// Most external processors (hosted plugins) an engine holds at once.
pub const MAX_EXTERNALS: usize = 128;

/// Transport state handed to an external processor with every block.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ExternalTransport {
    /// Whether the playhead is advancing.
    pub playing: bool,
    /// Tempo in beats per minute.
    pub tempo_bpm: f64,
    /// Playhead at the start of the block, in beats (quarter notes).
    pub position_beats: f64,
    /// The time signature in force at the playhead.
    pub time_signature: musiccraft_meter::TimeSig,
    /// Start of the bar holding the playhead, in beats (quarter notes).
    pub bar_start_beats: f64,
    /// That bar's number, from 0.
    pub bar_number: u32,
    /// Engine sample rate.
    pub sample_rate: f64,
}

/// A sound processor whose code lives outside the engine, such as a hosted
/// plugin. It is created on the main thread, moved into one of the engine's
/// external slots with [`crate::EngineMessage::SetExternal`], and handed back as
/// [`crate::Garbage::External`] when replaced.
///
/// Every method runs on the audio thread and must obey the real-time rules: no
/// allocation, locks, I/O or panics. Queues for notes and parameter changes
/// must be preallocated; when one is full, events are dropped.
pub trait External: Send + fmt::Debug {
    /// Starts a note `delay` frames into the next processed block. With a
    /// `gate`, the processor releases the note itself that many frames after it
    /// starts; with `None` it is held until [`External::note_off`].
    fn note_on(&mut self, delay: u32, key: u8, velocity: f32, pan: f32, gate: Option<u64>);

    /// Releases a held note `delay` frames into the next block.
    fn note_off(&mut self, delay: u32, key: u8);

    /// Releases every sounding note at the start of the next block.
    fn all_notes_off(&mut self);

    /// Sets a parameter, in the processor's own units, at the start of the next
    /// block.
    fn set_param(&mut self, id: u32, value: f64);

    /// Processes one block in place. An instrument receives silent buffers and
    /// adds its output; an effect receives its input and replaces it.
    fn process(&mut self, left: &mut [f32], right: &mut [f32], transport: &ExternalTransport);

    /// Called on the audio thread just before the processor is handed back to
    /// the main thread, so it can end processing where it began.
    fn stop(&mut self);

    /// Takes data sent with [`crate::EngineMessage::ExternalData`] and
    /// returns a box for the main thread to drop: the same box, holding
    /// whatever the data replaced. Processors that take no data return it
    /// unchanged.
    fn receive(&mut self, data: Box<dyn Any + Send>) -> Box<dyn Any + Send> {
        data
    }
}
