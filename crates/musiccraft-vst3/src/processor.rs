//! The audio-thread side of a plugin: turns engine notes and parameter
//! changes into VST3 events and parameter queues and calls `process`. Nothing
//! here allocates or locks after construction.

use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicI32, AtomicPtr, AtomicU32, AtomicU64, AtomicUsize, Ordering};

use musiccraft_engine::{External, ExternalTransport};
use rtrb::{Consumer, Producer};
use vst3::Steinberg::Vst::{
    AudioBusBuffers, AudioBusBuffers__type0, Event, Event_, IAudioProcessorTrait, IEventList,
    IEventListTrait, IParamValueQueue, IParamValueQueueTrait, IParameterChanges,
    IParameterChangesTrait, NoteOffEvent, NoteOnEvent, ProcessContext, ProcessContext_,
    ProcessData, ProcessModes_, SymbolicSampleSizes_,
};
use vst3::Steinberg::{kInvalidArgument, kResultFalse, kResultOk, tresult};
use vst3::{Class, ComWrapper};

use crate::instance::Shared;

/// Events queued for one block before newer ones are dropped.
const MAX_EVENTS: usize = 512;
/// Notes the processor tracks for gates and all-notes-off.
const MAX_HELD: usize = 128;
/// Parameters that can change in one block.
const MAX_QUEUES: usize = 64;
/// Points one parameter can take in one block; later ones replace the last.
const MAX_POINTS: usize = 16;

// --- parameter queues -----------------------------------------------------------

/// The changes of one parameter within a block.
pub(crate) struct Queue {
    id: AtomicU32,
    len: AtomicUsize,
    offsets: [AtomicI32; MAX_POINTS],
    values: [AtomicU64; MAX_POINTS],
}

impl Class for Queue {
    type Interfaces = (IParamValueQueue,);
}

impl Queue {
    fn new() -> Self {
        Self {
            id: AtomicU32::new(0),
            len: AtomicUsize::new(0),
            offsets: std::array::from_fn(|_| AtomicI32::new(0)),
            values: std::array::from_fn(|_| AtomicU64::new(0)),
        }
    }

    /// Appends a point, or replaces the last one when full. Returns its index.
    fn push(&self, offset: i32, value: f64) -> usize {
        let len = self.len.load(Ordering::Relaxed);
        let index = len.min(MAX_POINTS - 1);
        if let (Some(o), Some(v)) = (self.offsets.get(index), self.values.get(index)) {
            o.store(offset, Ordering::Relaxed);
            v.store(value.to_bits(), Ordering::Relaxed);
        }
        self.len.store(index + 1, Ordering::Relaxed);
        index
    }

    fn last(&self) -> Option<f64> {
        let len = self.len.load(Ordering::Relaxed);
        let value = self.values.get(len.checked_sub(1)?)?;
        Some(f64::from_bits(value.load(Ordering::Relaxed)))
    }
}

impl IParamValueQueueTrait for Queue {
    unsafe fn getParameterId(&self) -> u32 {
        self.id.load(Ordering::Relaxed)
    }

    unsafe fn getPointCount(&self) -> i32 {
        self.len.load(Ordering::Relaxed) as i32
    }

    unsafe fn getPoint(&self, index: i32, offset: *mut i32, value: *mut f64) -> tresult {
        let Ok(index) = usize::try_from(index) else {
            return kInvalidArgument;
        };
        if index >= self.len.load(Ordering::Relaxed) {
            return kInvalidArgument;
        }
        // SAFETY: the plugin passes writable outputs.
        let (Some(offset), Some(value)) = (unsafe { offset.as_mut() }, unsafe { value.as_mut() })
        else {
            return kInvalidArgument;
        };
        if let (Some(o), Some(v)) = (self.offsets.get(index), self.values.get(index)) {
            *offset = o.load(Ordering::Relaxed);
            *value = f64::from_bits(v.load(Ordering::Relaxed));
        }
        kResultOk
    }

    unsafe fn addPoint(&self, offset: i32, value: f64, index: *mut i32) -> tresult {
        let at = self.push(offset, value);
        // SAFETY: the plugin passes a writable index (or null).
        if let Some(index) = unsafe { index.as_mut() } {
            *index = at as i32;
        }
        kResultOk
    }
}

/// The parameter changes of one block: up to [`MAX_QUEUES`] parameters.
pub(crate) struct ParamChanges {
    queues: Vec<ComWrapper<Queue>>,
    used: AtomicUsize,
}

impl Class for ParamChanges {
    type Interfaces = (IParameterChanges,);
}

impl ParamChanges {
    /// Allocates every queue (main thread).
    pub fn new() -> Self {
        Self {
            queues: (0..MAX_QUEUES)
                .map(|_| ComWrapper::new(Queue::new()))
                .collect(),
            used: AtomicUsize::new(0),
        }
    }

    fn used(&self) -> &[ComWrapper<Queue>] {
        let used = self.used.load(Ordering::Relaxed);
        self.queues.get(..used).unwrap_or_default()
    }

    /// The queue for `id`, taking a free one if needed.
    fn queue(&self, id: u32) -> Option<(usize, &ComWrapper<Queue>)> {
        if let Some(found) = self
            .used()
            .iter()
            .enumerate()
            .find(|(_, q)| q.id.load(Ordering::Relaxed) == id)
        {
            return Some(found);
        }
        let used = self.used.load(Ordering::Relaxed);
        let queue = self.queues.get(used)?;
        queue.id.store(id, Ordering::Relaxed);
        queue.len.store(0, Ordering::Relaxed);
        self.used.store(used + 1, Ordering::Relaxed);
        Some((used, queue))
    }

    /// Adds a change (host side).
    pub fn add(&self, id: u32, offset: i32, value: f64) {
        if let Some((_, queue)) = self.queue(id) {
            queue.push(offset, value);
        }
    }

    /// Each parameter's last value.
    pub fn last_values(&self) -> impl Iterator<Item = (u32, f64)> + '_ {
        self.used()
            .iter()
            .filter_map(|q| Some((q.id.load(Ordering::Relaxed), q.last()?)))
    }

    pub fn clear(&self) {
        self.used.store(0, Ordering::Relaxed);
    }
}

impl IParameterChangesTrait for ParamChanges {
    unsafe fn getParameterCount(&self) -> i32 {
        self.used.load(Ordering::Relaxed) as i32
    }

    unsafe fn getParameterData(&self, index: i32) -> *mut IParamValueQueue {
        usize::try_from(index)
            .ok()
            .and_then(|i| self.used().get(i))
            .and_then(|q| q.as_com_ref::<IParamValueQueue>())
            .map_or(ptr::null_mut(), |r| r.as_ptr())
    }

    unsafe fn addParameterData(&self, id: *const u32, index: *mut i32) -> *mut IParamValueQueue {
        // SAFETY: the plugin passes a readable id.
        let Some(id) = (unsafe { id.as_ref() }) else {
            return ptr::null_mut();
        };
        let Some((at, queue)) = self.queue(*id) else {
            return ptr::null_mut();
        };
        // SAFETY: the plugin passes a writable index (or null).
        if let Some(index) = unsafe { index.as_mut() } {
            *index = at as i32;
        }
        queue
            .as_com_ref::<IParamValueQueue>()
            .map_or(ptr::null_mut(), |r| r.as_ptr())
    }
}

// --- events ---------------------------------------------------------------------

/// The input event list handed to `process`: a view of the processor's
/// queued events, set just before the call.
pub(crate) struct EventList {
    events: AtomicPtr<Event>,
    len: AtomicUsize,
}

impl Class for EventList {
    type Interfaces = (IEventList,);
}

impl IEventListTrait for EventList {
    unsafe fn getEventCount(&self) -> i32 {
        self.len.load(Ordering::Relaxed) as i32
    }

    unsafe fn getEvent(&self, index: i32, event: *mut Event) -> tresult {
        let Ok(index) = usize::try_from(index) else {
            return kInvalidArgument;
        };
        let events = self.events.load(Ordering::Relaxed);
        if index >= self.len.load(Ordering::Relaxed) || events.is_null() || event.is_null() {
            return kInvalidArgument;
        }
        // SAFETY: during `process` the pointer and length describe the
        // processor's event buffer; `index` is below the length; the plugin
        // passes a writable event.
        unsafe { *event = *events.add(index) };
        kResultOk
    }

    unsafe fn addEvent(&self, _event: *mut Event) -> tresult {
        kResultFalse
    }
}

/// The raw pointer of an interface reference, or null.
fn pointer<I: vst3::Interface>(reference: Option<vst3::ComRef<'_, I>>) -> *mut I {
    reference.map_or(ptr::null_mut(), |r| r.as_ptr())
}

// --- buffers --------------------------------------------------------------------

/// Preallocated buffers for a set of audio buses.
struct Ports {
    _samples: Vec<Vec<f32>>,
    /// Per bus: a pointer to each channel's samples.
    _pointers: Vec<Vec<*mut f32>>,
    buses: Vec<AudioBusBuffers>,
    /// Pointers to the main bus's first two channels (or one, twice).
    main: Option<(*mut f32, *mut f32)>,
    /// The same for the first auxiliary (sidechain) bus.
    aux: Option<(*mut f32, *mut f32)>,
    /// Samples in each buffer.
    frames: usize,
}

impl Ports {
    fn new(channels: &[u32], frames: usize) -> Self {
        let mut samples: Vec<Vec<f32>> = Vec::new();
        let mut pointers: Vec<Vec<*mut f32>> = Vec::new();
        for count in channels {
            let mut bus = Vec::new();
            for _ in 0..*count {
                let mut buffer = vec![0.0; frames.max(1)];
                bus.push(buffer.as_mut_ptr());
                samples.push(buffer);
            }
            pointers.push(bus);
        }
        // The heap blocks never move after this, so the pointers stay valid.
        let buses = pointers
            .iter_mut()
            .map(|bus| AudioBusBuffers {
                numChannels: bus.len() as i32,
                silenceFlags: 0,
                __field0: AudioBusBuffers__type0 {
                    channelBuffers32: bus.as_mut_ptr(),
                },
            })
            .collect();
        let pair = |bus: &Vec<*mut f32>| match bus.as_slice() {
            [mono] => Some((*mono, *mono)),
            [l, r, ..] => Some((*l, *r)),
            [] => None,
        };
        let main = pointers.first().and_then(pair);
        let aux = pointers.get(1).and_then(pair);
        Self {
            _samples: samples,
            _pointers: pointers,
            buses,
            main,
            aux,
            frames: frames.max(1),
        }
    }

    /// Copies `left` and `right` into the first auxiliary bus, silent past
    /// their end (both empty: silence). Allocation-free.
    fn fill_aux(&mut self, left: &[f32], right: &[f32]) {
        let Some((l, r)) = self.aux else {
            return;
        };
        // SAFETY: the aux buffers hold `frames` samples, are owned by these
        // buses and only touched on the audio thread, outside `process`.
        let al = unsafe { std::slice::from_raw_parts_mut(l, self.frames) };
        for (i, s) in al.iter_mut().enumerate() {
            let (x, y) = (left.get(i).copied(), right.get(i).copied());
            *s = match (l == r, x, y) {
                (true, Some(x), Some(y)) => (x + y) * 0.5,
                (_, x, _) => x.unwrap_or(0.0),
            };
        }
        if l != r {
            // SAFETY: as above; a different buffer from `al`.
            let ar = unsafe { std::slice::from_raw_parts_mut(r, self.frames) };
            for (i, s) in ar.iter_mut().enumerate() {
                *s = right.get(i).copied().unwrap_or(0.0);
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct Held {
    key: u8,
    /// Frames from the start of the next block until release, if gated.
    remaining: Option<u64>,
}

/// Queues between the halves of a plugin.
pub(crate) struct Links {
    /// Parameter changes the processor reported (normalized).
    pub reported: Producer<(u32, f64)>,
    /// Values the host set, for the controller to hear (normalized).
    pub host_set: Producer<(u32, f64)>,
    /// Edits made in the plugin's window, to pass to the processor.
    pub edits: Consumer<(u32, f64)>,
}

/// The audio-thread half of a plugin; an engine [`External`].
pub(crate) struct Vst3Processor {
    shared: Arc<Shared>,
    notes: bool,
    started: bool,
    events: Vec<Event>,
    held: Vec<Held>,
    inputs: Ports,
    outputs: Ports,
    max_frames: usize,
    /// Whether a key signal was given for the next block, and whether the
    /// aux input still holds one from an earlier block.
    keyed: bool,
    aux_dirty: bool,
    in_changes: ComWrapper<ParamChanges>,
    out_changes: ComWrapper<ParamChanges>,
    event_list: ComWrapper<EventList>,
    links: Links,
    steady: i64,
    sample_rate: f64,
}

// SAFETY: the raw pointers point into buffers this processor owns; the
// plugin is only used here for audio-thread functions (see `Shared`).
unsafe impl Send for Vst3Processor {}

impl std::fmt::Debug for Vst3Processor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Vst3Processor").finish_non_exhaustive()
    }
}

impl Vst3Processor {
    pub fn new(
        shared: Arc<Shared>,
        inputs: &[u32],
        outputs: &[u32],
        notes: bool,
        max_frames: usize,
        sample_rate: f64,
        links: Links,
    ) -> Self {
        Self {
            shared,
            notes,
            started: false,
            events: Vec::with_capacity(MAX_EVENTS),
            held: Vec::with_capacity(MAX_HELD),
            inputs: Ports::new(inputs, max_frames),
            outputs: Ports::new(outputs, max_frames),
            max_frames,
            keyed: false,
            aux_dirty: false,
            in_changes: ComWrapper::new(ParamChanges::new()),
            out_changes: ComWrapper::new(ParamChanges::new()),
            event_list: ComWrapper::new(EventList {
                events: AtomicPtr::new(ptr::null_mut()),
                len: AtomicUsize::new(0),
            }),
            links,
            steady: 0,
            sample_rate,
        }
    }

    fn queue_note(&mut self, time: u32, key: u8, velocity: f32, on: bool) -> bool {
        if !self.notes || self.events.len() >= MAX_EVENTS {
            return false;
        }
        // SAFETY: an all-zero event is a valid value of this plain struct.
        let mut event: Event = unsafe { std::mem::zeroed() };
        event.busIndex = 0;
        event.sampleOffset = time as i32;
        event.flags = Event_::EventFlags_::kIsLive as u16;
        if on {
            event.r#type = Event_::EventTypes_::kNoteOnEvent as u16;
            event.__field0.noteOn = NoteOnEvent {
                channel: 0,
                pitch: i16::from(key),
                tuning: 0.0,
                velocity: velocity.clamp(0.0, 1.0),
                length: 0,
                noteId: -1,
            };
        } else {
            event.r#type = Event_::EventTypes_::kNoteOffEvent as u16;
            event.__field0.noteOff = NoteOffEvent {
                channel: 0,
                pitch: i16::from(key),
                velocity: 0.0,
                noteId: -1,
                tuning: 0.0,
            };
        }
        self.events.push(event);
        true
    }

    /// Sorts queued events by time, keeping arrival order for equal times
    /// (insertion sort: stable and allocation-free).
    fn sort_events(&mut self) {
        for i in 1..self.events.len() {
            let mut j = i;
            while j > 0 && self.events[j - 1].sampleOffset > self.events[j].sampleOffset {
                self.events.swap(j - 1, j);
                j -= 1;
            }
        }
    }

    /// Queues releases for gated notes that end within `frames`.
    fn release_due(&mut self, frames: u64) {
        let mut i = 0;
        while i < self.held.len() {
            let held = self.held[i];
            match held.remaining {
                Some(rest) if rest < frames => {
                    self.queue_note(rest as u32, held.key, 0.0, false);
                    self.held.swap_remove(i);
                }
                Some(rest) => {
                    self.held[i].remaining = Some(rest - frames);
                    i += 1;
                }
                None => i += 1,
            }
        }
    }

    fn context(&self, transport: &ExternalTransport) -> ProcessContext {
        use ProcessContext_::StatesAndFlags_ as F;
        let beats = transport.position_beats.max(0.0);
        let tempo = transport.tempo_bpm.max(1.0);
        let mut state = F::kTempoValid
            | F::kProjectTimeMusicValid
            | F::kTimeSigValid
            | F::kBarPositionValid
            | F::kContTimeValid;
        if transport.playing {
            state |= F::kPlaying;
        }
        // SAFETY: an all-zero context is a valid value of this plain struct.
        let mut context: ProcessContext = unsafe { std::mem::zeroed() };
        context.state = state as u32;
        context.sampleRate = self.sample_rate;
        context.projectTimeSamples = (beats * 60.0 / tempo * self.sample_rate) as i64;
        context.continousTimeSamples = self.steady;
        context.projectTimeMusic = beats;
        context.barPositionMusic = transport.bar_start_beats.max(0.0);
        context.tempo = tempo;
        let sig = transport.time_signature;
        context.timeSigNumerator = i32::try_from(sig.num()).unwrap_or(4);
        context.timeSigDenominator = i32::try_from(sig.den()).unwrap_or(4);
        context
    }

    /// The normalized value for a host value of parameter `id`.
    fn normalized(&self, id: u32, value: f64) -> f64 {
        let steps = self
            .shared
            .steps
            .iter()
            .find(|(p, _)| *p == id)
            .map_or(0, |(_, s)| *s);
        let normalized = if steps > 0 {
            value / f64::from(steps)
        } else {
            value
        };
        normalized.clamp(0.0, 1.0)
    }
}

impl External for Vst3Processor {
    fn sidechain(&mut self, left: &[f32], right: &[f32]) {
        self.inputs.fill_aux(left, right);
        self.keyed = true;
    }

    fn note_on(&mut self, delay: u32, key: u8, velocity: f32, _pan: f32, gate: Option<u64>) {
        if self.held.len() >= MAX_HELD || !self.queue_note(delay, key, velocity, true) {
            return;
        }
        self.held.push(Held {
            key,
            remaining: gate.map(|g| u64::from(delay) + g.max(1)),
        });
    }

    fn note_off(&mut self, delay: u32, key: u8) {
        if let Some(i) = self
            .held
            .iter()
            .position(|h| h.key == key && h.remaining.is_none())
        {
            self.held.swap_remove(i);
            self.queue_note(delay, key, 0.0, false);
        }
    }

    fn all_notes_off(&mut self) {
        // Pending note starts would otherwise sound after the release.
        self.events.clear();
        while let Some(held) = self.held.pop() {
            self.queue_note(0, held.key, 0.0, false);
        }
    }

    fn set_param(&mut self, id: u32, value: f64) {
        let normalized = self.normalized(id, value);
        self.in_changes.add(id, 0, normalized);
        let _ = self.links.host_set.push((id, normalized));
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32], transport: &ExternalTransport) {
        let frames = left.len().min(right.len()).min(self.max_frames);
        if frames == 0 {
            return;
        }
        while let Ok((id, value)) = self.links.edits.pop() {
            self.in_changes.add(id, 0, value);
        }
        if !self.started {
            // SAFETY: the component is active; audio thread. Plugins that do
            // not need this call report "not implemented", which is fine.
            unsafe { self.shared.processor.setProcessing(1) };
            self.started = true;
            self.shared.processing.store(true, Ordering::Release);
        }
        self.release_due(frames as u64);
        self.sort_events();
        let last = frames as i32 - 1;
        for event in &mut self.events {
            event.sampleOffset = event.sampleOffset.min(last);
        }

        if let Some((l, r)) = self.inputs.main {
            // SAFETY: the main input buffers hold `max_frames >= frames`
            // samples and are only touched here.
            let (il, ir) = unsafe {
                (
                    std::slice::from_raw_parts_mut(l, frames),
                    std::slice::from_raw_parts_mut(r, frames),
                )
            };
            il.copy_from_slice(&left[..frames]);
            if l != r {
                ir.copy_from_slice(&right[..frames]);
            } else {
                for (m, s) in il.iter_mut().zip(&right[..frames]) {
                    *m = (*m + s) * 0.5;
                }
            }
        }

        if !self.keyed && self.aux_dirty {
            self.inputs.fill_aux(&[], &[]);
        }
        self.aux_dirty = self.keyed;
        self.keyed = false;

        self.event_list
            .events
            .store(self.events.as_mut_ptr(), Ordering::Relaxed);
        self.event_list
            .len
            .store(self.events.len(), Ordering::Relaxed);
        let mut context = self.context(transport);
        let mut data = ProcessData {
            processMode: ProcessModes_::kRealtime as i32,
            symbolicSampleSize: SymbolicSampleSizes_::kSample32 as i32,
            numSamples: frames as i32,
            numInputs: self.inputs.buses.len() as i32,
            numOutputs: self.outputs.buses.len() as i32,
            inputs: if self.inputs.buses.is_empty() {
                ptr::null_mut()
            } else {
                self.inputs.buses.as_mut_ptr()
            },
            outputs: if self.outputs.buses.is_empty() {
                ptr::null_mut()
            } else {
                self.outputs.buses.as_mut_ptr()
            },
            inputParameterChanges: pointer(self.in_changes.as_com_ref::<IParameterChanges>()),
            outputParameterChanges: pointer(self.out_changes.as_com_ref::<IParameterChanges>()),
            inputEvents: if self.notes {
                pointer(self.event_list.as_com_ref::<IEventList>())
            } else {
                ptr::null_mut()
            },
            outputEvents: ptr::null_mut(),
            processContext: &mut context,
        };
        // SAFETY: audio thread, processing started; every pointer in `data`
        // refers to buffers, lists and the context, which outlive the call.
        let status = unsafe { self.shared.processor.process(&mut data) };
        self.events.clear();
        self.event_list.len.store(0, Ordering::Relaxed);
        self.in_changes.clear();
        for (id, value) in self.out_changes.last_values() {
            let _ = self.links.reported.push((id, value));
        }
        self.out_changes.clear();
        self.steady += frames as i64;
        if status != kResultOk {
            return;
        }
        let Some((l, r)) = self.outputs.main else {
            return;
        };
        // SAFETY: as for the inputs; the plugin is done writing.
        let (ol, or) = unsafe {
            (
                std::slice::from_raw_parts(l, frames),
                std::slice::from_raw_parts(r, frames),
            )
        };
        // Instruments get silent buffers, so replacing is the same as adding.
        left[..frames].copy_from_slice(ol);
        right[..frames].copy_from_slice(or);
    }

    fn stop(&mut self) {
        if self.started {
            // SAFETY: audio thread; processing was started here.
            unsafe { self.shared.processor.setProcessing(0) };
            self.started = false;
            self.shared.processing.store(false, Ordering::Release);
        }
    }
}
