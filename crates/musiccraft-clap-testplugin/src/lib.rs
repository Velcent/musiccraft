//! Two tiny CLAP plugins used to test the MusicCraft host against the real C
//! ABI. Built as a `cdylib` it is a normal `.clap` file; linked as an `rlib`,
//! tests can use [`ENTRY`] without loading a file.
//!
//! - `org.musiccraft.test.gain` — stereo effect, parameter 1 `Gain` (0–2).
//! - `org.musiccraft.test.tone` — instrument: a sine per held note, parameter
//!   1 `Level` (0–1). A note on key 127 makes it report `Level = 0.25` as if
//!   its own window had moved the parameter.
//!
//! Both save their parameter as eight little-endian bytes of state.
//!
//! Like `musiccraft-clap`, this crate is exempt from the workspace's `unsafe`
//! ban: a CLAP plugin is a set of C callbacks.
#![allow(unsafe_code)]
#![deny(unsafe_op_in_unsafe_fn)]

use std::ffi::{CStr, c_char, c_void};
use std::ptr;

use clap_sys::audio_buffer::clap_audio_buffer;
use clap_sys::entry::clap_plugin_entry;
use clap_sys::events::{
    CLAP_CORE_EVENT_SPACE_ID, CLAP_EVENT_NOTE_OFF, CLAP_EVENT_NOTE_ON, CLAP_EVENT_PARAM_VALUE,
    clap_event_header, clap_event_note, clap_event_param_value, clap_input_events,
    clap_output_events,
};
use clap_sys::ext::audio_ports::{
    CLAP_AUDIO_PORT_IS_MAIN, CLAP_EXT_AUDIO_PORTS, CLAP_PORT_STEREO, clap_audio_port_info,
    clap_plugin_audio_ports,
};
use clap_sys::ext::note_ports::{
    CLAP_EXT_NOTE_PORTS, CLAP_NOTE_DIALECT_CLAP, clap_note_port_info, clap_plugin_note_ports,
};
use clap_sys::ext::params::{
    CLAP_EXT_PARAMS, CLAP_PARAM_IS_AUTOMATABLE, clap_param_info, clap_plugin_params,
};
use clap_sys::ext::state::{CLAP_EXT_STATE, clap_plugin_state};
use clap_sys::factory::plugin_factory::{CLAP_PLUGIN_FACTORY_ID, clap_plugin_factory};
use clap_sys::host::clap_host;
use clap_sys::id::clap_id;
use clap_sys::plugin::{clap_plugin, clap_plugin_descriptor};
use clap_sys::process::{CLAP_PROCESS_CONTINUE, clap_process, clap_process_status};
use clap_sys::stream::{clap_istream, clap_ostream};
use clap_sys::version::CLAP_VERSION;

/// Id of the gain effect.
pub const GAIN_ID: &str = "org.musiccraft.test.gain";
/// Id of the tone instrument.
pub const TONE_ID: &str = "org.musiccraft.test.tone";
/// The single parameter's id in both plugins.
pub const PARAM: clap_id = 1;
/// Peak level of one tone voice at `Level` 1.
pub const TONE_PEAK: f32 = 0.5;

/// A pointer list that may live in a `static`.
struct Features([*const c_char; 3]);
// SAFETY: the pointers refer to string literals, which are immutable and live
// for the whole program.
unsafe impl Sync for Features {}

static GAIN_FEATURES: Features =
    Features([c"audio-effect".as_ptr(), c"utility".as_ptr(), ptr::null()]);
static TONE_FEATURES: Features =
    Features([c"instrument".as_ptr(), c"synthesizer".as_ptr(), ptr::null()]);

fn descriptor(
    id: &'static CStr,
    name: &'static CStr,
    features: &'static Features,
) -> clap_plugin_descriptor {
    clap_plugin_descriptor {
        clap_version: CLAP_VERSION,
        id: id.as_ptr(),
        name: name.as_ptr(),
        vendor: c"MusicCraft tests".as_ptr(),
        url: c"".as_ptr(),
        manual_url: c"".as_ptr(),
        support_url: c"".as_ptr(),
        version: c"1.0.0".as_ptr(),
        description: c"A plugin for testing the MusicCraft host".as_ptr(),
        features: features.0.as_ptr(),
    }
}

/// Descriptors for both plugins, in a `static`.
struct Descriptors([clap_plugin_descriptor; 2]);
// SAFETY: as for `Features`; descriptors only point at static data.
unsafe impl Sync for Descriptors {}

static DESCRIPTORS: std::sync::LazyLock<Descriptors> = std::sync::LazyLock::new(|| {
    Descriptors([
        descriptor(c"org.musiccraft.test.gain", c"Test Gain", &GAIN_FEATURES),
        descriptor(c"org.musiccraft.test.tone", c"Test Tone", &TONE_FEATURES),
    ])
});

/// One sounding sine.
#[derive(Clone, Copy, Default)]
struct Voice {
    key: i16,
    on: bool,
    phase: f32,
    velocity: f32,
}

/// Per-instance data behind `clap_plugin::plugin_data`.
struct Instance {
    plugin: clap_plugin,
    instrument: bool,
    value: f64,
    sample_rate: f32,
    voices: [Voice; 16],
    /// Set by a note on key 127; reported through the output events.
    report: bool,
}

fn instance<'a>(plugin: *const clap_plugin) -> Option<&'a mut Instance> {
    // SAFETY: `plugin_data` is the `Instance` that owns this `clap_plugin`; the
    // host calls one function at a time per thread rule, and these test
    // plugins keep audio and main-thread state in the same struct only for
    // fields each side touches.
    unsafe { plugin.as_ref()?.plugin_data.cast::<Instance>().as_mut() }
}

unsafe extern "C" fn init(_plugin: *const clap_plugin) -> bool {
    true
}

unsafe extern "C" fn destroy(plugin: *const clap_plugin) {
    if let Some(data) = instance(plugin) {
        // SAFETY: created by `Box::into_raw` in `create_plugin`; the host
        // calls `destroy` exactly once and never uses the plugin afterwards.
        drop(unsafe { Box::from_raw(ptr::from_mut(data)) });
    }
}

unsafe extern "C" fn activate(plugin: *const clap_plugin, rate: f64, _min: u32, _max: u32) -> bool {
    if let Some(data) = instance(plugin) {
        data.sample_rate = rate as f32;
    }
    true
}

unsafe extern "C" fn deactivate(_plugin: *const clap_plugin) {}

unsafe extern "C" fn start_processing(_plugin: *const clap_plugin) -> bool {
    true
}

unsafe extern "C" fn stop_processing(_plugin: *const clap_plugin) {}

unsafe extern "C" fn reset(plugin: *const clap_plugin) {
    if let Some(data) = instance(plugin) {
        data.voices = [Voice::default(); 16];
    }
}

fn handle_event(data: &mut Instance, header: &clap_event_header) {
    if header.space_id != CLAP_CORE_EVENT_SPACE_ID {
        return;
    }
    let header_ptr = ptr::from_ref(header);
    match header.type_ {
        CLAP_EVENT_PARAM_VALUE => {
            // SAFETY: the type says the header starts a param value event.
            let event = unsafe { &*header_ptr.cast::<clap_event_param_value>() };
            if event.param_id == PARAM {
                data.value = event
                    .value
                    .clamp(0.0, if data.instrument { 1.0 } else { 2.0 });
            }
        }
        CLAP_EVENT_NOTE_ON if data.instrument => {
            // SAFETY: the type says the header starts a note event.
            let event = unsafe { &*header_ptr.cast::<clap_event_note>() };
            if event.key == 127 {
                data.report = true;
                return;
            }
            if let Some(voice) = data.voices.iter_mut().find(|v| !v.on) {
                *voice = Voice {
                    key: event.key,
                    on: true,
                    phase: 0.0,
                    velocity: event.velocity as f32,
                };
            }
        }
        CLAP_EVENT_NOTE_OFF if data.instrument => {
            // SAFETY: the type says the header starts a note event.
            let event = unsafe { &*header_ptr.cast::<clap_event_note>() };
            for voice in data.voices.iter_mut().filter(|v| v.on) {
                if event.key < 0 || voice.key == event.key {
                    voice.on = false;
                }
            }
        }
        _ => {}
    }
}

/// Channel `c` of the first buffer in `buffers`, if present.
///
/// # Safety
///
/// `buffers` must point at `count` valid buffers whose `data32` arrays hold
/// `channel_count` pointers to at least `frames` samples each.
unsafe fn channel<'a>(
    buffers: *const clap_audio_buffer,
    count: u32,
    c: u32,
    frames: usize,
) -> Option<&'a mut [f32]> {
    if count == 0 || buffers.is_null() {
        return None;
    }
    // SAFETY: guaranteed by the caller.
    let buffer = unsafe { &*buffers };
    if c >= buffer.channel_count || buffer.data32.is_null() {
        return None;
    }
    // SAFETY: guaranteed by the caller.
    let data = unsafe { *buffer.data32.add(c as usize) };
    // SAFETY: guaranteed by the caller.
    (!data.is_null()).then(|| unsafe { std::slice::from_raw_parts_mut(data, frames) })
}

unsafe extern "C" fn process(
    plugin: *const clap_plugin,
    process: *const clap_process,
) -> clap_process_status {
    let Some(data) = instance(plugin) else {
        return 0;
    };
    // SAFETY: the host passes a valid process structure for this call.
    let Some(process) = (unsafe { process.as_ref() }) else {
        return 0;
    };
    let frames = process.frames_count as usize;
    // SAFETY: the host provides a valid event list for this call.
    let events = unsafe { process.in_events.as_ref() };
    // SAFETY: the list's own size function.
    let size = events
        .and_then(|e| e.size.map(|f| unsafe { f(e) }))
        .unwrap_or(0);
    let event_at = |i: u32| -> Option<&clap_event_header> {
        let list = events?;
        // SAFETY: callers pass an index below `size`; the host owns the event.
        unsafe { list.get?(list, i).as_ref() }
    };
    let mut next = 0;
    if !data.instrument {
        while next < size {
            if let Some(header) = event_at(next) {
                handle_event(data, header);
            }
            next += 1;
        }
    }
    // SAFETY: the host lays out buffers matching our audio ports.
    let (out_l, out_r) = unsafe {
        (
            channel(
                process.audio_outputs,
                process.audio_outputs_count,
                0,
                frames,
            ),
            channel(
                process.audio_outputs,
                process.audio_outputs_count,
                1,
                frames,
            ),
        )
    };
    let (Some(out_l), Some(out_r)) = (out_l, out_r) else {
        return CLAP_PROCESS_CONTINUE;
    };
    if data.instrument {
        let rate = data.sample_rate.max(1.0);
        for frame in 0..frames {
            // Sample-accurate: apply the events due at this frame.
            while let Some(header) =
                event_at(next).filter(|h| next < size && h.time as usize <= frame)
            {
                handle_event(data, header);
                next += 1;
            }
            let level = data.value as f32 * TONE_PEAK;
            let mut sum = 0.0;
            for voice in data.voices.iter_mut().filter(|v| v.on) {
                let hz = 440.0 * 2f32.powf((f32::from(voice.key) - 69.0) / 12.0);
                sum += (voice.phase * std::f32::consts::TAU).sin() * level * voice.velocity;
                voice.phase = (voice.phase + hz / rate).fract();
            }
            out_l[frame] = sum;
            out_r[frame] = sum;
        }
    } else {
        // SAFETY: as above, for the input buffers.
        let (in_l, in_r) = unsafe {
            (
                channel(process.audio_inputs, process.audio_inputs_count, 0, frames),
                channel(process.audio_inputs, process.audio_inputs_count, 1, frames),
            )
        };
        let gain = data.value as f32;
        if let (Some(in_l), Some(in_r)) = (in_l, in_r) {
            for i in 0..frames {
                out_l[i] = in_l[i] * gain;
                out_r[i] = in_r[i] * gain;
            }
        }
    }
    if data.report {
        data.report = false;
        // SAFETY: the host provides a valid output list for this call.
        if let Some(out) = unsafe { process.out_events.as_ref() } {
            let event = clap_event_param_value {
                header: clap_event_header {
                    size: size_of::<clap_event_param_value>() as u32,
                    time: 0,
                    space_id: CLAP_CORE_EVENT_SPACE_ID,
                    type_: CLAP_EVENT_PARAM_VALUE,
                    flags: 0,
                },
                param_id: PARAM,
                cookie: ptr::null_mut(),
                note_id: -1,
                port_index: -1,
                channel: -1,
                key: -1,
                value: 0.25,
            };
            data.value = 0.25;
            if let Some(push) = out.try_push {
                // SAFETY: the event outlives the call; the host copies it.
                unsafe { push(out, &event.header) };
            }
        }
    }
    CLAP_PROCESS_CONTINUE
}

// --- params -------------------------------------------------------------

unsafe extern "C" fn params_count(_plugin: *const clap_plugin) -> u32 {
    1
}

fn write_name(dest: &mut [c_char], text: &str) {
    for (d, b) in dest.iter_mut().zip(text.bytes().chain(std::iter::once(0))) {
        *d = b as c_char;
    }
}

unsafe extern "C" fn params_info(
    plugin: *const clap_plugin,
    index: u32,
    info: *mut clap_param_info,
) -> bool {
    let (Some(data), 0) = (instance(plugin), index) else {
        return false;
    };
    // SAFETY: the host passes a writable info struct.
    let Some(info) = (unsafe { info.as_mut() }) else {
        return false;
    };
    info.id = PARAM;
    info.flags = CLAP_PARAM_IS_AUTOMATABLE;
    info.cookie = ptr::null_mut();
    write_name(
        &mut info.name,
        if data.instrument { "Level" } else { "Gain" },
    );
    write_name(&mut info.module, "");
    info.min_value = 0.0;
    info.max_value = if data.instrument { 1.0 } else { 2.0 };
    info.default_value = if data.instrument { 0.5 } else { 1.0 };
    true
}

unsafe extern "C" fn params_value(plugin: *const clap_plugin, id: clap_id, out: *mut f64) -> bool {
    match (instance(plugin), id) {
        (Some(data), PARAM) if !out.is_null() => {
            // SAFETY: checked non-null; the host passes a writable f64.
            unsafe { *out = data.value };
            true
        }
        _ => false,
    }
}

unsafe extern "C" fn params_to_text(
    plugin: *const clap_plugin,
    id: clap_id,
    value: f64,
    out: *mut c_char,
    capacity: u32,
) -> bool {
    if instance(plugin).is_none() || id != PARAM || out.is_null() || capacity == 0 {
        return false;
    }
    let text = format!("{value:.2}x");
    // SAFETY: the host gives `capacity` writable bytes.
    let dest = unsafe { std::slice::from_raw_parts_mut(out, capacity as usize) };
    let n = text.len().min(dest.len() - 1);
    write_name(&mut dest[..=n], &text[..n]);
    true
}

unsafe extern "C" fn params_from_text(
    _p: *const clap_plugin,
    _id: clap_id,
    _t: *const c_char,
    _o: *mut f64,
) -> bool {
    false
}

unsafe extern "C" fn params_flush(
    plugin: *const clap_plugin,
    input: *const clap_input_events,
    _out: *const clap_output_events,
) {
    let Some(data) = instance(plugin) else { return };
    // SAFETY: the host passes a valid list for this call.
    let Some(events) = (unsafe { input.as_ref() }) else {
        return;
    };
    // SAFETY: the list's own size function, called with the list.
    let size = events.size.map_or(0, |f| unsafe { f(events) });
    for i in 0..size {
        let Some(get) = events.get else { break };
        // SAFETY: index below `size`.
        if let Some(header) = unsafe { get(events, i).as_ref() } {
            handle_event(data, header);
        }
    }
}

static PARAMS: clap_plugin_params = clap_plugin_params {
    count: Some(params_count),
    get_info: Some(params_info),
    get_value: Some(params_value),
    value_to_text: Some(params_to_text),
    text_to_value: Some(params_from_text),
    flush: Some(params_flush),
};

// --- state --------------------------------------------------------------

unsafe extern "C" fn state_save(plugin: *const clap_plugin, stream: *const clap_ostream) -> bool {
    let Some(data) = instance(plugin) else {
        return false;
    };
    // SAFETY: the host passes a valid stream for this call.
    let Some(stream) = (unsafe { stream.as_ref() }) else {
        return false;
    };
    let bytes = data.value.to_le_bytes();
    let Some(write) = stream.write else {
        return false;
    };
    // SAFETY: `bytes` is valid for its length during the call.
    unsafe { write(stream, bytes.as_ptr().cast(), bytes.len() as u64) == bytes.len() as i64 }
}

unsafe extern "C" fn state_load(plugin: *const clap_plugin, stream: *const clap_istream) -> bool {
    let Some(data) = instance(plugin) else {
        return false;
    };
    // SAFETY: the host passes a valid stream for this call.
    let Some(stream) = (unsafe { stream.as_ref() }) else {
        return false;
    };
    let Some(read) = stream.read else {
        return false;
    };
    let mut bytes = [0u8; 8];
    // SAFETY: `bytes` is writable for its length during the call.
    let n = unsafe { read(stream, bytes.as_mut_ptr().cast(), 8) };
    if n != 8 {
        return false;
    }
    data.value = f64::from_le_bytes(bytes);
    true
}

static STATE: clap_plugin_state = clap_plugin_state {
    save: Some(state_save),
    load: Some(state_load),
};

// --- ports --------------------------------------------------------------

unsafe extern "C" fn audio_ports_count(plugin: *const clap_plugin, is_input: bool) -> u32 {
    match instance(plugin) {
        Some(data) if is_input && data.instrument => 0,
        Some(_) => 1,
        None => 0,
    }
}

unsafe extern "C" fn audio_ports_get(
    plugin: *const clap_plugin,
    index: u32,
    is_input: bool,
    info: *mut clap_audio_port_info,
) -> bool {
    // SAFETY: called with the plugin's own pointer.
    if index != 0 || unsafe { audio_ports_count(plugin, is_input) } == 0 {
        return false;
    }
    // SAFETY: the host passes a writable info struct.
    let Some(info) = (unsafe { info.as_mut() }) else {
        return false;
    };
    info.id = 0;
    write_name(&mut info.name, if is_input { "In" } else { "Out" });
    info.flags = CLAP_AUDIO_PORT_IS_MAIN;
    info.channel_count = 2;
    info.port_type = CLAP_PORT_STEREO.as_ptr();
    info.in_place_pair = clap_sys::id::CLAP_INVALID_ID;
    true
}

static AUDIO_PORTS: clap_plugin_audio_ports = clap_plugin_audio_ports {
    count: Some(audio_ports_count),
    get: Some(audio_ports_get),
};

unsafe extern "C" fn note_ports_count(plugin: *const clap_plugin, is_input: bool) -> u32 {
    u32::from(is_input && instance(plugin).is_some_and(|d| d.instrument))
}

unsafe extern "C" fn note_ports_get(
    plugin: *const clap_plugin,
    index: u32,
    is_input: bool,
    info: *mut clap_note_port_info,
) -> bool {
    // SAFETY: called with the plugin's own pointer.
    if index != 0 || unsafe { note_ports_count(plugin, is_input) } == 0 {
        return false;
    }
    // SAFETY: the host passes a writable info struct.
    let Some(info) = (unsafe { info.as_mut() }) else {
        return false;
    };
    info.id = 0;
    info.supported_dialects = CLAP_NOTE_DIALECT_CLAP;
    info.preferred_dialect = CLAP_NOTE_DIALECT_CLAP;
    write_name(&mut info.name, "Notes");
    true
}

static NOTE_PORTS: clap_plugin_note_ports = clap_plugin_note_ports {
    count: Some(note_ports_count),
    get: Some(note_ports_get),
};

unsafe extern "C" fn get_extension(
    _plugin: *const clap_plugin,
    id: *const c_char,
) -> *const c_void {
    if id.is_null() {
        return ptr::null();
    }
    // SAFETY: the host passes a nul-terminated id.
    let id = unsafe { CStr::from_ptr(id) };
    if id == CLAP_EXT_PARAMS {
        ptr::from_ref(&PARAMS).cast()
    } else if id == CLAP_EXT_STATE {
        ptr::from_ref(&STATE).cast()
    } else if id == CLAP_EXT_AUDIO_PORTS {
        ptr::from_ref(&AUDIO_PORTS).cast()
    } else if id == CLAP_EXT_NOTE_PORTS {
        ptr::from_ref(&NOTE_PORTS).cast()
    } else {
        ptr::null()
    }
}

unsafe extern "C" fn on_main_thread(_plugin: *const clap_plugin) {}

// --- factory and entry --------------------------------------------------

unsafe extern "C" fn factory_count(_factory: *const clap_plugin_factory) -> u32 {
    2
}

unsafe extern "C" fn factory_descriptor(
    _factory: *const clap_plugin_factory,
    index: u32,
) -> *const clap_plugin_descriptor {
    DESCRIPTORS
        .0
        .get(index as usize)
        .map_or(ptr::null(), ptr::from_ref)
}

unsafe extern "C" fn create_plugin(
    _factory: *const clap_plugin_factory,
    host: *const clap_host,
    id: *const c_char,
) -> *const clap_plugin {
    if host.is_null() || id.is_null() {
        return ptr::null();
    }
    // SAFETY: the host passes a nul-terminated id.
    let id = unsafe { CStr::from_ptr(id) };
    let (index, instrument) = if id.to_bytes() == GAIN_ID.as_bytes() {
        (0, false)
    } else if id.to_bytes() == TONE_ID.as_bytes() {
        (1, true)
    } else {
        return ptr::null();
    };
    let data = Box::into_raw(Box::new(Instance {
        plugin: clap_plugin {
            desc: &DESCRIPTORS.0[index],
            plugin_data: ptr::null_mut(),
            init: Some(init),
            destroy: Some(destroy),
            activate: Some(activate),
            deactivate: Some(deactivate),
            start_processing: Some(start_processing),
            stop_processing: Some(stop_processing),
            reset: Some(reset),
            process: Some(process),
            get_extension: Some(get_extension),
            on_main_thread: Some(on_main_thread),
        },
        instrument,
        value: if instrument { 0.5 } else { 1.0 },
        sample_rate: 48_000.0,
        voices: [Voice::default(); 16],
        report: false,
    }));
    // SAFETY: `data` was just allocated and is uniquely owned here.
    unsafe {
        (*data).plugin.plugin_data = data.cast();
        &raw const (*data).plugin
    }
}

static FACTORY: clap_plugin_factory = clap_plugin_factory {
    get_plugin_count: Some(factory_count),
    get_plugin_descriptor: Some(factory_descriptor),
    create_plugin: Some(create_plugin),
};

unsafe extern "C" fn entry_init(_path: *const c_char) -> bool {
    true
}

unsafe extern "C" fn entry_deinit() {}

unsafe extern "C" fn entry_get_factory(id: *const c_char) -> *const c_void {
    if id.is_null() {
        return ptr::null();
    }
    // SAFETY: the host passes a nul-terminated id.
    if unsafe { CStr::from_ptr(id) } == CLAP_PLUGIN_FACTORY_ID {
        ptr::from_ref(&FACTORY).cast()
    } else {
        ptr::null()
    }
}

/// The CLAP entry point; exported as `clap_entry` from the `cdylib`.
#[unsafe(no_mangle)]
#[allow(non_upper_case_globals)]
pub static clap_entry: clap_plugin_entry = clap_plugin_entry {
    clap_version: CLAP_VERSION,
    init: Some(entry_init),
    deinit: Some(entry_deinit),
    get_factory: Some(entry_get_factory),
};

/// The entry, for linking the plugins into a test binary.
pub static ENTRY: &clap_plugin_entry = &clap_entry;
