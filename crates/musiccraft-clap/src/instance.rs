//! A live plugin instance: creation, activation and the main-thread side.

use std::ffi::{CString, c_char, c_void};
use std::ptr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Instant;

use clap_sys::ext::audio_ports::{
    CLAP_EXT_AUDIO_PORTS, clap_audio_port_info, clap_plugin_audio_ports,
};
use clap_sys::ext::gui::{CLAP_EXT_GUI, clap_plugin_gui};
use clap_sys::ext::note_ports::{
    CLAP_EXT_NOTE_PORTS, CLAP_NOTE_DIALECT_CLAP, CLAP_NOTE_DIALECT_MIDI, clap_note_port_info,
    clap_plugin_note_ports,
};
use clap_sys::ext::params::{
    CLAP_EXT_PARAMS, CLAP_PARAM_IS_HIDDEN, CLAP_PARAM_IS_READONLY, CLAP_PARAM_IS_STEPPED,
    clap_param_info, clap_plugin_params,
};
use clap_sys::ext::posix_fd_support::{CLAP_EXT_POSIX_FD_SUPPORT, clap_plugin_posix_fd_support};
use clap_sys::ext::state::{CLAP_EXT_STATE, clap_plugin_state};
use clap_sys::ext::timer_support::{CLAP_EXT_TIMER_SUPPORT, clap_plugin_timer_support};
use clap_sys::plugin::clap_plugin;
use clap_sys::stream::{clap_istream, clap_ostream};
use musiccraft_render::{LivePlugin, PluginEvent, PluginInstance, PluginParam};
use rtrb::{Consumer, RingBuffer};

use crate::gui::Gui;
use crate::host::{HostData, flag};
use crate::library::{Library, field};
use crate::processor::{ClapProcessor, NoteDialect};

/// Parameter changes a plugin can report per poll before older ones drop.
const OUT_EVENT_CAPACITY: usize = 1024;

/// Extension tables a plugin provides (null when missing). They point into
/// the plugin library and stay valid while it is loaded.
#[derive(Clone, Copy)]
pub(crate) struct Extensions {
    pub params: *const clap_plugin_params,
    pub state: *const clap_plugin_state,
    pub gui: *const clap_plugin_gui,
    pub timer: *const clap_plugin_timer_support,
    pub fd: *const clap_plugin_posix_fd_support,
}

/// What both halves of a plugin share. The plugin is destroyed when the last
/// `Arc` goes, which is always on the main thread: the audio half comes back
/// through the engine's garbage queue.
pub(crate) struct Shared {
    plugin: *const clap_plugin,
    pub ext: Extensions,
    /// Set by the audio half between `start_processing` and `stop_processing`.
    pub processing: AtomicBool,
    // Dropped after the plugin is destroyed (field order): the host
    // callbacks and the library must outlive it.
    _host: Box<HostData>,
    _library: Arc<Library>,
}

// SAFETY: CLAP plugins are designed to be called from a main thread and an
// audio thread concurrently, each limited to its own functions; MusicCraft
// keeps to that split (main-thread calls in `ClapInstance` and `Drop`,
// audio-thread calls in `ClapProcessor`).
unsafe impl Send for Shared {}
// SAFETY: as above.
unsafe impl Sync for Shared {}

impl Shared {
    /// The plugin's function table.
    pub fn plugin(&self) -> &clap_plugin {
        // SAFETY: `plugin` was checked non-null at creation and stays valid
        // until `destroy` in `Drop`.
        unsafe { &*self.plugin }
    }

    pub fn ptr(&self) -> *const clap_plugin {
        self.plugin
    }

    pub fn host(&self) -> &HostData {
        &self._host
    }
}

impl Drop for Shared {
    fn drop(&mut self) {
        let plugin = self.plugin();
        if self.processing.swap(false, Ordering::AcqRel) {
            // The audio half was dropped without stopping (the engine itself
            // went away), so no audio thread is using the plugin any more and
            // this thread stands in for it.
            if let Some(stop) = plugin.stop_processing {
                crate::host::set_audio_thread(true);
                // SAFETY: the plugin is active and processing, and no other
                // thread is processing it.
                unsafe { stop(self.plugin) };
                crate::host::set_audio_thread(false);
            }
        }
        if let Some(deactivate) = plugin.deactivate {
            // SAFETY: activated in `create`; on the main thread.
            unsafe { deactivate(self.plugin) };
        }
        if let Some(destroy) = plugin.destroy {
            // SAFETY: last use of the plugin; on the main thread.
            unsafe { destroy(self.plugin) };
        }
    }
}

/// Looks up an extension table.
///
/// # Safety
///
/// `plugin` must be a valid, initialized plugin and `id` the extension's id;
/// `T` must be that extension's table type.
unsafe fn extension<T>(plugin: *const clap_plugin, id: &std::ffi::CStr) -> *const T {
    // SAFETY: guaranteed by the caller.
    let Some(get) = (unsafe { &*plugin }).get_extension else {
        return ptr::null();
    };
    // SAFETY: guaranteed by the caller.
    unsafe { get(plugin, id.as_ptr()) }.cast()
}

/// Channel counts of a plugin's audio ports, main port first.
fn audio_ports(plugin: *const clap_plugin, input: bool) -> Vec<u32> {
    // SAFETY: called on a valid, initialized plugin with the right type.
    let table: *const clap_plugin_audio_ports = unsafe { extension(plugin, CLAP_EXT_AUDIO_PORTS) };
    // SAFETY: a non-null table is valid while the library is loaded.
    let Some(table) = (unsafe { table.as_ref() }) else {
        // No audio-ports extension: assume stereo out (and in for effects is
        // unknowable, so none).
        return if input { Vec::new() } else { vec![2] };
    };
    let (Some(count), Some(get)) = (table.count, table.get) else {
        return Vec::new();
    };
    // SAFETY: plugin functions take the plugin pointer.
    let n = unsafe { count(plugin, input) };
    (0..n)
        .filter_map(|i| {
            // SAFETY: an all-zero info is a valid value of this plain struct.
            let mut info: clap_audio_port_info = unsafe { std::mem::zeroed() };
            // SAFETY: `i` is below the count and `info` is writable.
            unsafe { get(plugin, i, input, &mut info) }.then_some(info.channel_count.min(32))
        })
        .collect()
}

/// How notes must be sent, or `None` for plugins without note input.
fn note_dialect(plugin: *const clap_plugin) -> Option<NoteDialect> {
    // SAFETY: called on a valid, initialized plugin with the right type.
    let table: *const clap_plugin_note_ports = unsafe { extension(plugin, CLAP_EXT_NOTE_PORTS) };
    // SAFETY: a non-null table is valid while the library is loaded.
    let table = unsafe { table.as_ref() }?;
    let (count, get) = (table.count?, table.get?);
    // SAFETY: plugin functions take the plugin pointer.
    if unsafe { count(plugin, true) } == 0 {
        return None;
    }
    // SAFETY: an all-zero info is a valid value of this plain struct.
    let mut info: clap_note_port_info = unsafe { std::mem::zeroed() };
    // SAFETY: index 0 exists; `info` is writable.
    if !unsafe { get(plugin, 0, true, &mut info) } {
        return Some(NoteDialect::Clap);
    }
    if info.supported_dialects & CLAP_NOTE_DIALECT_CLAP != 0 {
        Some(NoteDialect::Clap)
    } else if info.supported_dialects & CLAP_NOTE_DIALECT_MIDI != 0 {
        Some(NoteDialect::Midi)
    } else {
        None
    }
}

/// Reads saved state from a byte slice.
struct ReadCursor<'a> {
    data: &'a [u8],
    at: usize,
}

unsafe extern "C" fn stream_read(
    stream: *const clap_istream,
    buffer: *mut c_void,
    size: u64,
) -> i64 {
    // SAFETY: `ctx` is the `ReadCursor` set up in `load_state`, alive for the
    // call to `load`.
    let Some(cursor) = (unsafe { stream.as_ref() })
        .and_then(|s| unsafe { s.ctx.cast::<ReadCursor<'_>>().as_mut() })
    else {
        return -1;
    };
    let rest = cursor.data.get(cursor.at..).unwrap_or_default();
    let n = rest.len().min(usize::try_from(size).unwrap_or(usize::MAX));
    if n > 0 && !buffer.is_null() {
        // SAFETY: the plugin gives a writable buffer of `size` bytes.
        unsafe { ptr::copy_nonoverlapping(rest.as_ptr(), buffer.cast::<u8>(), n) };
    }
    cursor.at += n;
    n as i64
}

unsafe extern "C" fn stream_write(
    stream: *const clap_ostream,
    buffer: *const c_void,
    size: u64,
) -> i64 {
    // SAFETY: `ctx` is the `Vec` set up in `save_state`, alive for the call.
    let Some(out) =
        (unsafe { stream.as_ref() }).and_then(|s| unsafe { s.ctx.cast::<Vec<u8>>().as_mut() })
    else {
        return -1;
    };
    let n = usize::try_from(size).unwrap_or(0);
    if n > 0 && !buffer.is_null() {
        // SAFETY: the plugin gives `size` readable bytes.
        out.extend_from_slice(unsafe { std::slice::from_raw_parts(buffer.cast::<u8>(), n) });
    }
    n as i64
}

fn load_state(plugin: *const clap_plugin, state: *const clap_plugin_state, data: &[u8]) -> bool {
    // SAFETY: a non-null table is valid while the library is loaded.
    let Some(load) = (unsafe { state.as_ref() }).and_then(|s| s.load) else {
        return false;
    };
    let mut cursor = ReadCursor { data, at: 0 };
    let stream = clap_istream {
        ctx: ptr::from_mut(&mut cursor).cast(),
        read: Some(stream_read),
    };
    // SAFETY: main thread; the stream and cursor outlive the call.
    unsafe { load(plugin, &stream) }
}

/// Creates, initializes, restores and activates plugin `id` from `library`.
pub(crate) fn create(
    library: Arc<Library>,
    id: &str,
    sample_rate: u32,
    max_frames: u32,
    state: Option<&[u8]>,
) -> Result<LivePlugin, String> {
    let factory = library
        .factory()
        .ok_or("the library has no plugin factory")?;
    let create = factory
        .create_plugin
        .ok_or("the factory cannot create plugins")?;
    let c_id = CString::new(id).map_err(|_| "invalid plugin id")?;
    let host = HostData::new();
    // SAFETY: factory functions take the factory pointer; the host lives in
    // `Shared` (below) for as long as the plugin.
    let plugin = unsafe { create(factory, &host.clap, c_id.as_ptr()) };
    if plugin.is_null() {
        return Err("the plugin refused to start".to_owned());
    }
    // SAFETY: just created and non-null.
    let table = unsafe { &*plugin };
    // SAFETY: CLAP requires `init` right after creation, on the main thread.
    let ok = table.init.is_some_and(|init| unsafe { init(plugin) });
    let ext = if ok {
        // SAFETY: initialized; the ids match the table types.
        unsafe {
            Extensions {
                params: extension(plugin, CLAP_EXT_PARAMS),
                state: extension(plugin, CLAP_EXT_STATE),
                gui: extension(plugin, CLAP_EXT_GUI),
                timer: extension(plugin, CLAP_EXT_TIMER_SUPPORT),
                fd: extension(plugin, CLAP_EXT_POSIX_FD_SUPPORT),
            }
        }
    } else {
        if let Some(destroy) = table.destroy {
            // SAFETY: created but unusable; destroy it before the host goes.
            unsafe { destroy(plugin) };
        }
        return Err("the plugin failed to initialize".to_owned());
    };
    let mut warning = None;
    if let Some(state) = state {
        if !load_state(plugin, ext.state, state) {
            warning = Some("its saved state could not be restored");
        }
    }
    let inputs = audio_ports(plugin, true);
    let outputs = audio_ports(plugin, false);
    let dialect = note_dialect(plugin);
    // SAFETY: initialized, inactive; main thread.
    let active = table
        .activate
        .is_some_and(|activate| unsafe { activate(plugin, f64::from(sample_rate), 1, max_frames) });
    if !active {
        if let Some(destroy) = table.destroy {
            // SAFETY: initialized but never activated.
            unsafe { destroy(plugin) };
        }
        return Err("the plugin failed to activate".to_owned());
    }
    if let Some(warning) = warning {
        eprintln!("plugin {id}: {warning}");
    }
    let shared = Arc::new(Shared {
        plugin,
        ext,
        processing: AtomicBool::new(false),
        _host: host,
        _library: library,
    });
    let (producer, consumer) = RingBuffer::new(OUT_EVENT_CAPACITY);
    let processor = ClapProcessor::new(
        Arc::clone(&shared),
        &inputs,
        &outputs,
        dialect,
        max_frames as usize,
        producer,
    );
    let instance = ClapInstance {
        shared,
        changes: consumer,
        gui: Gui::default(),
    };
    Ok(LivePlugin {
        instance: Box::new(instance),
        processor: Box::new(processor),
    })
}

/// The main-thread side of a plugin.
pub(crate) struct ClapInstance {
    shared: Arc<Shared>,
    /// Parameter changes the plugin reported while processing.
    changes: Consumer<(u32, f64)>,
    gui: Gui,
}

impl std::fmt::Debug for ClapInstance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ClapInstance").finish_non_exhaustive()
    }
}

impl ClapInstance {
    fn params_table(&self) -> Option<&clap_plugin_params> {
        // SAFETY: a non-null table is valid while the library is loaded.
        unsafe { self.shared.ext.params.as_ref() }
    }

    /// Runs due timers and ready file descriptors.
    fn run_timers_and_fds(&mut self) {
        let plugin = self.shared.ptr();
        // SAFETY: a non-null table is valid while the library is loaded.
        if let Some(on_timer) = unsafe { self.shared.ext.timer.as_ref() }.and_then(|t| t.on_timer) {
            let now = Instant::now();
            let due: Vec<u32> = match self.shared.host().timers.lock() {
                Ok(mut timers) => timers
                    .iter_mut()
                    .filter(|t| t.due <= now)
                    .map(|t| {
                        t.due = now + t.period;
                        t.id
                    })
                    .collect(),
                Err(_) => Vec::new(),
            };
            for id in due {
                // SAFETY: main thread; the plugin registered this timer.
                unsafe { on_timer(plugin, id) };
            }
        }
        #[cfg(unix)]
        // SAFETY: a non-null table is valid while the library is loaded.
        if let Some(on_fd) = unsafe { self.shared.ext.fd.as_ref() }.and_then(|f| f.on_fd) {
            let fds: Vec<(i32, u32)> = self
                .shared
                .host()
                .fds
                .lock()
                .map(|f| f.clone())
                .unwrap_or_default();
            for (fd, flags) in crate::fd::ready(&fds) {
                // SAFETY: main thread; the plugin registered this descriptor.
                unsafe { on_fd(plugin, fd, flags) };
            }
        }
    }
}

impl PluginInstance for ClapInstance {
    fn params(&self) -> Vec<PluginParam> {
        let Some(table) = self.params_table() else {
            return Vec::new();
        };
        let (Some(count), Some(get_info)) = (table.count, table.get_info) else {
            return Vec::new();
        };
        let plugin = self.shared.ptr();
        // SAFETY: main thread; plugin functions take the plugin pointer.
        let n = unsafe { count(plugin) };
        (0..n)
            .filter_map(|i| {
                // SAFETY: an all-zero info is a valid value of this struct.
                let mut info: clap_param_info = unsafe { std::mem::zeroed() };
                // SAFETY: `i` is below the count; `info` is writable.
                if !unsafe { get_info(plugin, i, &mut info) } {
                    return None;
                }
                Some(PluginParam {
                    id: info.id,
                    name: field(&info.name),
                    module: field(&info.module),
                    min: info.min_value,
                    max: info.max_value,
                    default: info.default_value,
                    stepped: info.flags & CLAP_PARAM_IS_STEPPED != 0,
                    hidden: info.flags & CLAP_PARAM_IS_HIDDEN != 0,
                    read_only: info.flags & CLAP_PARAM_IS_READONLY != 0,
                })
            })
            .collect()
    }

    fn param_value(&self, id: u32) -> Option<f64> {
        let get = self.params_table()?.get_value?;
        let mut value = 0.0;
        // SAFETY: main thread; `value` is writable.
        unsafe { get(self.shared.ptr(), id, &mut value) }.then_some(value)
    }

    fn format_param(&self, id: u32, value: f64) -> Option<String> {
        let to_text = self.params_table()?.value_to_text?;
        let mut buffer = [0 as c_char; 128];
        // SAFETY: main thread; the buffer and its length are passed together.
        let ok = unsafe { to_text(self.shared.ptr(), id, value, buffer.as_mut_ptr(), 128) };
        ok.then(|| field(&buffer))
    }

    fn save_state(&mut self) -> Option<Vec<u8>> {
        // SAFETY: a non-null table is valid while the library is loaded.
        let save = unsafe { self.shared.ext.state.as_ref() }?.save?;
        let mut out: Vec<u8> = Vec::new();
        let stream = clap_ostream {
            ctx: ptr::from_mut(&mut out).cast(),
            write: Some(stream_write),
        };
        // SAFETY: main thread; the stream and buffer outlive the call.
        unsafe { save(self.shared.ptr(), &stream) }.then_some(out)
    }

    fn poll(&mut self) -> Vec<PluginEvent> {
        let mut events = Vec::new();
        let flags = self.shared.host().take_flags();
        if flags & flag::CALLBACK != 0 {
            if let Some(callback) = self.shared.plugin().on_main_thread {
                // SAFETY: the plugin asked for this call on the main thread.
                unsafe { callback(self.shared.ptr()) };
            }
        }
        self.run_timers_and_fds();
        if flags & flag::STATE_DIRTY != 0 {
            events.push(PluginEvent::StateChanged);
        }
        if flags & flag::PARAMS_RESCAN != 0 {
            events.push(PluginEvent::ParamsChanged);
        }
        if flags & flag::RESTART != 0 {
            // Restarting needs the audio thread to pause the plugin; until
            // that exists, keep running with the current setup.
            eprintln!("plugin asked for a restart; ignored");
        }
        let resize = if flags & flag::GUI_RESIZE != 0 {
            self.shared.host().take_resize()
        } else {
            None
        };
        let hide = flags & (flag::GUI_HIDE | flag::GUI_CLOSED) != 0;
        if self.gui.poll(&self.shared, resize, hide) {
            events.push(PluginEvent::EditorClosed);
        }
        while let Ok((id, value)) = self.changes.pop() {
            events.push(PluginEvent::Param { id, value });
        }
        events
    }

    fn has_editor(&self) -> bool {
        Gui::supported(&self.shared)
    }

    fn editor_open(&self) -> bool {
        self.gui.is_open()
    }

    fn set_editor_open(&mut self, open: bool, title: &str) -> Result<(), String> {
        if open {
            self.gui.open(&self.shared, title)
        } else {
            self.gui.close(&self.shared);
            Ok(())
        }
    }
}

impl Drop for ClapInstance {
    fn drop(&mut self) {
        self.gui.close(&self.shared);
    }
}
