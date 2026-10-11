//! The audio-thread side of a plugin: turns engine notes and parameter
//! changes into CLAP events and calls `process`. Nothing here allocates after
//! construction.

use std::ffi::c_void;
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::Ordering;

use clap_sys::audio_buffer::clap_audio_buffer;
use clap_sys::events::{
    CLAP_CORE_EVENT_SPACE_ID, CLAP_EVENT_MIDI, CLAP_EVENT_NOTE_EXPRESSION, CLAP_EVENT_NOTE_OFF,
    CLAP_EVENT_NOTE_ON, CLAP_EVENT_PARAM_VALUE, CLAP_NOTE_EXPRESSION_PAN,
    CLAP_TRANSPORT_HAS_BEATS_TIMELINE, CLAP_TRANSPORT_HAS_SECONDS_TIMELINE,
    CLAP_TRANSPORT_HAS_TEMPO, CLAP_TRANSPORT_HAS_TIME_SIGNATURE, CLAP_TRANSPORT_IS_PLAYING,
    clap_event_header, clap_event_midi, clap_event_note, clap_event_note_expression,
    clap_event_param_value, clap_event_transport, clap_input_events, clap_output_events,
};
use clap_sys::fixedpoint::{CLAP_BEATTIME_FACTOR, CLAP_SECTIME_FACTOR};
use clap_sys::process::clap_process;
use musiccraft_engine::{External, ExternalTransport};
use rtrb::Producer;

use crate::host::set_audio_thread;
use crate::instance::Shared;

/// Events queued for one block before newer ones are dropped.
const MAX_EVENTS: usize = 512;
/// Notes the processor tracks for gates and all-notes-off.
const MAX_HELD: usize = 128;

/// How a plugin wants notes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum NoteDialect {
    Clap,
    Midi,
}

/// Storage for any event the processor sends.
#[repr(C)]
#[derive(Clone, Copy)]
union EventData {
    header: clap_event_header,
    note: clap_event_note,
    expression: clap_event_note_expression,
    param: clap_event_param_value,
    midi: clap_event_midi,
}

#[derive(Clone, Copy)]
struct Queued {
    time: u32,
    /// Parameter id for parameter events, to merge repeated changes.
    param: Option<u32>,
    data: EventData,
}

#[derive(Debug, Clone, Copy)]
struct Held {
    key: u8,
    /// Frames from the start of the next block until release, if gated.
    remaining: Option<u64>,
}

/// Preallocated buffers for a set of audio ports.
struct Ports {
    _samples: Vec<Vec<f32>>,
    /// Per port: a pointer to each channel's samples.
    _pointers: Vec<Vec<*mut f32>>,
    clap: Vec<clap_audio_buffer>,
    /// Pointers to the main port's first two channels (or one, twice).
    main: Option<(*mut f32, *mut f32)>,
}

impl Ports {
    fn new(channels: &[u32], frames: usize) -> Self {
        let mut samples: Vec<Vec<f32>> = Vec::new();
        let mut pointers: Vec<Vec<*mut f32>> = Vec::new();
        for count in channels {
            let mut port = Vec::new();
            for _ in 0..*count {
                let mut buffer = vec![0.0; frames.max(1)];
                port.push(buffer.as_mut_ptr());
                samples.push(buffer);
            }
            pointers.push(port);
        }
        // The heap blocks never move after this, so the pointers stay valid.
        let clap = pointers
            .iter_mut()
            .map(|port| clap_audio_buffer {
                data32: port.as_mut_ptr(),
                data64: ptr::null_mut(),
                channel_count: port.len() as u32,
                latency: 0,
                constant_mask: 0,
            })
            .collect();
        let main = pointers.first().and_then(|port| match port.as_slice() {
            [mono] => Some((*mono, *mono)),
            [l, r, ..] => Some((*l, *r)),
            [] => None,
        });
        Self {
            _samples: samples,
            _pointers: pointers,
            clap,
            main,
        }
    }
}

/// The event list handed to `process` as `ctx`.
struct EventList {
    events: *const Queued,
    len: u32,
}

unsafe extern "C" fn list_size(list: *const clap_input_events) -> u32 {
    // SAFETY: `ctx` is the `EventList` built in `process`, alive for the call.
    unsafe {
        list.as_ref()
            .and_then(|l| l.ctx.cast::<EventList>().as_ref())
    }
    .map_or(0, |l| l.len)
}

unsafe extern "C" fn list_get(
    list: *const clap_input_events,
    index: u32,
) -> *const clap_event_header {
    // SAFETY: as in `list_size`.
    let Some(events) = (unsafe {
        list.as_ref()
            .and_then(|l| l.ctx.cast::<EventList>().as_ref())
    }) else {
        return ptr::null();
    };
    if index >= events.len {
        return ptr::null();
    }
    // SAFETY: `index` is below `len`, the number of queued events.
    let queued = unsafe { &*events.events.add(index as usize) };
    // Every variant starts with the header.
    &raw const queued.data.header
}

unsafe extern "C" fn out_push(
    list: *const clap_output_events,
    event: *const clap_event_header,
) -> bool {
    // SAFETY: `ctx` is the processor's producer, borrowed for the call.
    let Some(producer) = (unsafe {
        list.as_ref()
            .and_then(|l| l.ctx.cast::<Producer<(u32, f64)>>().as_mut())
    }) else {
        return false;
    };
    // SAFETY: the plugin passes a valid event for the call.
    let Some(header) = (unsafe { event.as_ref() }) else {
        return false;
    };
    if header.space_id == CLAP_CORE_EVENT_SPACE_ID && header.type_ == CLAP_EVENT_PARAM_VALUE {
        // SAFETY: the type says this is a parameter value event.
        let param = unsafe { &*event.cast::<clap_event_param_value>() };
        let _ = producer.push((param.param_id, param.value));
    }
    true
}

fn header(size: usize, time: u32, type_: u16) -> clap_event_header {
    clap_event_header {
        size: size as u32,
        time,
        space_id: CLAP_CORE_EVENT_SPACE_ID,
        type_,
        flags: 0,
    }
}

/// The audio-thread half of a plugin; an engine [`External`].
pub(crate) struct ClapProcessor {
    shared: Arc<Shared>,
    dialect: Option<NoteDialect>,
    started: bool,
    events: Vec<Queued>,
    held: Vec<Held>,
    inputs: Ports,
    outputs: Ports,
    max_frames: usize,
    changes: Producer<(u32, f64)>,
    steady: i64,
}

// SAFETY: the raw pointers point into buffers this processor owns; the
// plugin is only used here for audio-thread functions (see `Shared`).
unsafe impl Send for ClapProcessor {}

impl std::fmt::Debug for ClapProcessor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClapProcessor").finish_non_exhaustive()
    }
}

impl ClapProcessor {
    pub fn new(
        shared: Arc<Shared>,
        inputs: &[u32],
        outputs: &[u32],
        dialect: Option<NoteDialect>,
        max_frames: usize,
        changes: Producer<(u32, f64)>,
    ) -> Self {
        Self {
            shared,
            dialect,
            started: false,
            events: Vec::with_capacity(MAX_EVENTS),
            held: Vec::with_capacity(MAX_HELD),
            inputs: Ports::new(inputs, max_frames),
            outputs: Ports::new(outputs, max_frames),
            max_frames,
            changes,
            steady: 0,
        }
    }

    fn queue(&mut self, time: u32, param: Option<u32>, data: EventData) -> bool {
        if self.events.len() >= MAX_EVENTS {
            return false;
        }
        self.events.push(Queued { time, param, data });
        true
    }

    fn queue_note(&mut self, time: u32, key: u8, velocity: f32, on: bool) -> bool {
        let data = match self.dialect {
            None => return false,
            Some(NoteDialect::Clap) => EventData {
                note: clap_event_note {
                    header: header(
                        size_of::<clap_event_note>(),
                        time,
                        if on {
                            CLAP_EVENT_NOTE_ON
                        } else {
                            CLAP_EVENT_NOTE_OFF
                        },
                    ),
                    note_id: -1,
                    port_index: 0,
                    channel: 0,
                    key: i16::from(key),
                    velocity: f64::from(velocity),
                },
            },
            Some(NoteDialect::Midi) => EventData {
                midi: clap_event_midi {
                    header: header(size_of::<clap_event_midi>(), time, CLAP_EVENT_MIDI),
                    port_index: 0,
                    data: [
                        if on { 0x90 } else { 0x80 },
                        key.min(127),
                        (velocity.clamp(0.0, 1.0) * 127.0)
                            .round()
                            .max(f32::from(on)) as u8,
                    ],
                },
            },
        };
        self.queue(time, None, data)
    }

    /// Sorts queued events by time, keeping arrival order for equal times
    /// (insertion sort: stable and allocation-free).
    fn sort_events(&mut self) {
        for i in 1..self.events.len() {
            let mut j = i;
            while j > 0 && self.events[j - 1].time > self.events[j].time {
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

    fn transport(transport: &ExternalTransport) -> clap_event_transport {
        let beats = transport.position_beats.max(0.0);
        let seconds = beats * 60.0 / transport.tempo_bpm.max(1.0);
        let mut flags = CLAP_TRANSPORT_HAS_TEMPO
            | CLAP_TRANSPORT_HAS_BEATS_TIMELINE
            | CLAP_TRANSPORT_HAS_SECONDS_TIMELINE
            | CLAP_TRANSPORT_HAS_TIME_SIGNATURE;
        if transport.playing {
            flags |= CLAP_TRANSPORT_IS_PLAYING;
        }
        let bar = (beats / 4.0).floor();
        clap_event_transport {
            header: header(
                size_of::<clap_event_transport>(),
                0,
                clap_sys::events::CLAP_EVENT_TRANSPORT,
            ),
            flags,
            song_pos_beats: (beats * CLAP_BEATTIME_FACTOR as f64) as i64,
            song_pos_seconds: (seconds * CLAP_SECTIME_FACTOR as f64) as i64,
            tempo: transport.tempo_bpm,
            tempo_inc: 0.0,
            loop_start_beats: 0,
            loop_end_beats: 0,
            loop_start_seconds: 0,
            loop_end_seconds: 0,
            bar_start: (bar * 4.0 * CLAP_BEATTIME_FACTOR as f64) as i64,
            bar_number: bar as i32,
            tsig_num: 4,
            tsig_denom: 4,
        }
    }
}

impl External for ClapProcessor {
    fn note_on(&mut self, delay: u32, key: u8, velocity: f32, pan: f32, gate: Option<u64>) {
        if self.held.len() >= MAX_HELD || !self.queue_note(delay, key, velocity, true) {
            return;
        }
        if pan != 0.0 && self.dialect == Some(NoteDialect::Clap) {
            let expression = EventData {
                expression: clap_event_note_expression {
                    header: header(
                        size_of::<clap_event_note_expression>(),
                        delay,
                        CLAP_EVENT_NOTE_EXPRESSION,
                    ),
                    expression_id: CLAP_NOTE_EXPRESSION_PAN,
                    note_id: -1,
                    port_index: 0,
                    channel: 0,
                    key: i16::from(key),
                    value: f64::from((pan + 1.0) / 2.0),
                },
            };
            self.queue(delay, None, expression);
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
        self.events.retain(|e| e.param.is_some());
        while let Some(held) = self.held.pop() {
            self.queue_note(0, held.key, 0.0, false);
        }
    }

    fn set_param(&mut self, id: u32, value: f64) {
        if let Some(queued) = self.events.iter_mut().find(|e| e.param == Some(id)) {
            // Entries with `param` set hold parameter events.
            queued.data.param.value = value;
            return;
        }
        let data = EventData {
            param: clap_event_param_value {
                header: header(
                    size_of::<clap_event_param_value>(),
                    0,
                    CLAP_EVENT_PARAM_VALUE,
                ),
                param_id: id,
                cookie: ptr::null_mut(),
                note_id: -1,
                port_index: -1,
                channel: -1,
                key: -1,
                value,
            },
        };
        self.queue(0, Some(id), data);
    }

    fn process(&mut self, left: &mut [f32], right: &mut [f32], transport: &ExternalTransport) {
        let frames = left.len().min(right.len()).min(self.max_frames);
        if frames == 0 {
            return;
        }
        set_audio_thread(true);
        let plugin = self.shared.ptr();
        let table = *self.shared.plugin();
        if !self.started {
            // SAFETY: the plugin is active; audio thread.
            self.started = table
                .start_processing
                .is_some_and(|start| unsafe { start(plugin) });
            self.shared
                .processing
                .store(self.started, Ordering::Release);
        }
        let Some(process) = table.process.filter(|_| self.started) else {
            set_audio_thread(false);
            return;
        };
        self.release_due(frames as u64);
        self.sort_events();
        let last = frames as u32 - 1;
        for event in &mut self.events {
            event.time = event.time.min(last);
            // Every variant starts with the header.
            event.data.header.time = event.time;
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

        let list = EventList {
            events: self.events.as_ptr(),
            len: self.events.len() as u32,
        };
        let in_events = clap_input_events {
            ctx: ptr::from_ref(&list).cast_mut().cast::<c_void>(),
            size: Some(list_size),
            get: Some(list_get),
        };
        let out_events = clap_output_events {
            ctx: ptr::from_mut(&mut self.changes).cast(),
            try_push: Some(out_push),
        };
        let transport_event = Self::transport(transport);
        let call = clap_process {
            steady_time: self.steady,
            frames_count: frames as u32,
            transport: &transport_event,
            audio_inputs: self.inputs.clap.as_ptr(),
            audio_outputs: self.outputs.clap.as_mut_ptr(),
            audio_inputs_count: self.inputs.clap.len() as u32,
            audio_outputs_count: self.outputs.clap.len() as u32,
            in_events: &in_events,
            out_events: &out_events,
        };
        // SAFETY: audio thread, processing started; every pointer in `call`
        // refers to buffers and lists that outlive the call.
        let status = unsafe { process(plugin, &call) };
        self.events.clear();
        self.steady += frames as i64;
        set_audio_thread(false);
        if status == clap_sys::process::CLAP_PROCESS_ERROR {
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
            set_audio_thread(true);
            if let Some(stop) = self.shared.plugin().stop_processing {
                // SAFETY: audio thread; processing was started here.
                unsafe { stop(self.shared.ptr()) };
            }
            set_audio_thread(false);
            self.started = false;
            self.shared.processing.store(false, Ordering::Release);
        }
    }
}
