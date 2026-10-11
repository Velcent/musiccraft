//! The `clap_host` MusicCraft hands to each plugin instance, and the
//! callbacks behind it. Callbacks may arrive on any thread, so everything they
//! touch is atomic or behind a mutex the audio thread never takes.

use std::cell::Cell;
use std::ffi::{CStr, c_char, c_void};
use std::ptr;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::thread::ThreadId;
use std::time::{Duration, Instant};

use clap_sys::ext::audio_ports::{CLAP_EXT_AUDIO_PORTS, clap_host_audio_ports};
use clap_sys::ext::gui::{CLAP_EXT_GUI, clap_host_gui};
use clap_sys::ext::latency::{CLAP_EXT_LATENCY, clap_host_latency};
use clap_sys::ext::log::{
    CLAP_EXT_LOG, CLAP_LOG_ERROR, CLAP_LOG_FATAL, CLAP_LOG_HOST_MISBEHAVING,
    CLAP_LOG_PLUGIN_MISBEHAVING, CLAP_LOG_WARNING, clap_host_log, clap_log_severity,
};
use clap_sys::ext::note_ports::{
    CLAP_EXT_NOTE_PORTS, CLAP_NOTE_DIALECT_CLAP, CLAP_NOTE_DIALECT_MIDI, clap_host_note_ports,
    clap_note_dialect,
};
use clap_sys::ext::params::{
    CLAP_EXT_PARAMS, clap_host_params, clap_param_clear_flags, clap_param_rescan_flags,
};
use clap_sys::ext::posix_fd_support::{
    CLAP_EXT_POSIX_FD_SUPPORT, clap_host_posix_fd_support, clap_posix_fd_flags,
};
use clap_sys::ext::state::{CLAP_EXT_STATE, clap_host_state};
use clap_sys::ext::thread_check::{CLAP_EXT_THREAD_CHECK, clap_host_thread_check};
use clap_sys::ext::timer_support::{CLAP_EXT_TIMER_SUPPORT, clap_host_timer_support};
use clap_sys::host::clap_host;
use clap_sys::id::clap_id;
use clap_sys::version::CLAP_VERSION;

thread_local! {
    /// Set while this thread is inside a plugin's `process`.
    static AUDIO_THREAD: Cell<bool> = const { Cell::new(false) };
}

/// Marks the current thread as (not) processing audio, for `thread-check`.
pub(crate) fn set_audio_thread(on: bool) {
    AUDIO_THREAD.with(|flag| flag.set(on));
}

fn is_audio_thread() -> bool {
    AUDIO_THREAD.with(Cell::get)
}

/// Requests a plugin made that the main thread acts on.
pub(crate) mod flag {
    pub const CALLBACK: u32 = 1 << 0;
    pub const RESTART: u32 = 1 << 1;
    pub const STATE_DIRTY: u32 = 1 << 2;
    pub const PARAMS_RESCAN: u32 = 1 << 3;
    pub const GUI_CLOSED: u32 = 1 << 4;
    pub const GUI_RESIZE: u32 = 1 << 5;
    pub const GUI_HIDE: u32 = 1 << 6;
}

/// A timer a plugin registered.
#[derive(Debug, Clone, Copy)]
pub(crate) struct Timer {
    pub id: clap_id,
    pub period: Duration,
    pub due: Instant,
}

/// Everything behind one plugin's `clap_host::host_data`. Boxed, so its
/// address is stable for the plugin's lifetime.
pub(crate) struct HostData {
    pub clap: clap_host,
    main_thread: ThreadId,
    flags: AtomicU32,
    /// Size the plugin asked its window to be, packed as `w << 32 | h`.
    resize: AtomicU64,
    pub timers: Mutex<Vec<Timer>>,
    next_timer: AtomicU32,
    /// Registered file descriptors and the events wanted.
    pub fds: Mutex<Vec<(i32, clap_posix_fd_flags)>>,
}

impl HostData {
    /// A host for one instance, created on the main thread.
    pub fn new() -> Box<Self> {
        let mut data = Box::new(Self {
            clap: clap_host {
                clap_version: CLAP_VERSION,
                host_data: ptr::null_mut(),
                name: c"MusicCraft".as_ptr(),
                vendor: c"MusicCraft".as_ptr(),
                url: c"https://github.com/bhimio1/musiccraft".as_ptr(),
                version: c"0.1.0".as_ptr(),
                get_extension: Some(get_extension),
                request_restart: Some(request_restart),
                request_process: Some(request_process),
                request_callback: Some(request_callback),
            },
            main_thread: std::thread::current().id(),
            flags: AtomicU32::new(0),
            resize: AtomicU64::new(0),
            timers: Mutex::new(Vec::new()),
            next_timer: AtomicU32::new(1),
            fds: Mutex::new(Vec::new()),
        });
        data.clap.host_data = ptr::from_mut(data.as_mut()).cast();
        data
    }

    fn raise(&self, bits: u32) {
        self.flags.fetch_or(bits, Ordering::AcqRel);
    }

    /// Takes the pending request flags.
    pub fn take_flags(&self) -> u32 {
        self.flags.swap(0, Ordering::AcqRel)
    }

    /// Takes a pending resize request.
    pub fn take_resize(&self) -> Option<(u32, u32)> {
        let packed = self.resize.swap(0, Ordering::AcqRel);
        (packed != 0).then_some(((packed >> 32) as u32, packed as u32))
    }
}

/// The `HostData` behind a host pointer.
///
/// # Safety
///
/// `host` must be null or a host created by [`HostData::new`] that is still
/// alive (true for every callback: plugins only call their own host).
unsafe fn data<'a>(host: *const clap_host) -> Option<&'a HostData> {
    // SAFETY: guaranteed by the caller.
    unsafe { host.as_ref()?.host_data.cast::<HostData>().as_ref() }
}

unsafe extern "C" fn get_extension(_host: *const clap_host, id: *const c_char) -> *const c_void {
    if id.is_null() {
        return ptr::null();
    }
    // SAFETY: the plugin passes a nul-terminated extension id.
    let id = unsafe { CStr::from_ptr(id) };
    let table: Option<*const c_void> = if id == CLAP_EXT_LOG {
        Some(ptr::from_ref(&LOG).cast())
    } else if id == CLAP_EXT_THREAD_CHECK {
        Some(ptr::from_ref(&THREAD_CHECK).cast())
    } else if id == CLAP_EXT_PARAMS {
        Some(ptr::from_ref(&PARAMS).cast())
    } else if id == CLAP_EXT_STATE {
        Some(ptr::from_ref(&STATE).cast())
    } else if id == CLAP_EXT_GUI {
        Some(ptr::from_ref(&GUI).cast())
    } else if id == CLAP_EXT_TIMER_SUPPORT {
        Some(ptr::from_ref(&TIMERS).cast())
    } else if id == CLAP_EXT_POSIX_FD_SUPPORT && cfg!(unix) {
        Some(ptr::from_ref(&FDS).cast())
    } else if id == CLAP_EXT_LATENCY {
        Some(ptr::from_ref(&LATENCY).cast())
    } else if id == CLAP_EXT_AUDIO_PORTS {
        Some(ptr::from_ref(&AUDIO_PORTS).cast())
    } else if id == CLAP_EXT_NOTE_PORTS {
        Some(ptr::from_ref(&NOTE_PORTS).cast())
    } else {
        None
    };
    table.unwrap_or(ptr::null())
}

unsafe extern "C" fn request_restart(host: *const clap_host) {
    // SAFETY: plugins call back with their own host.
    if let Some(data) = unsafe { data(host) } {
        data.raise(flag::RESTART);
    }
}

unsafe extern "C" fn request_process(_host: *const clap_host) {
    // The engine calls `process` on every block anyway.
}

unsafe extern "C" fn request_callback(host: *const clap_host) {
    // SAFETY: plugins call back with their own host.
    if let Some(data) = unsafe { data(host) } {
        data.raise(flag::CALLBACK);
    }
}

// --- log ----------------------------------------------------------------

unsafe extern "C" fn log(_host: *const clap_host, severity: clap_log_severity, msg: *const c_char) {
    // Never print from the audio thread; keep warnings and errors otherwise.
    if is_audio_thread() || msg.is_null() {
        return;
    }
    let level = match severity {
        CLAP_LOG_WARNING => "warning",
        CLAP_LOG_ERROR | CLAP_LOG_FATAL => "error",
        CLAP_LOG_HOST_MISBEHAVING => "host misbehaving",
        CLAP_LOG_PLUGIN_MISBEHAVING => "plugin misbehaving",
        _ => return,
    };
    // SAFETY: the plugin passes a nul-terminated message.
    let text = unsafe { CStr::from_ptr(msg) }.to_string_lossy();
    eprintln!("plugin {level}: {text}");
}

static LOG: clap_host_log = clap_host_log { log: Some(log) };

// --- thread check -------------------------------------------------------

unsafe extern "C" fn is_main_thread(host: *const clap_host) -> bool {
    // SAFETY: plugins call back with their own host.
    unsafe { data(host) }.is_some_and(|d| d.main_thread == std::thread::current().id())
}

unsafe extern "C" fn is_audio_thread_cb(_host: *const clap_host) -> bool {
    is_audio_thread()
}

static THREAD_CHECK: clap_host_thread_check = clap_host_thread_check {
    is_main_thread: Some(is_main_thread),
    is_audio_thread: Some(is_audio_thread_cb),
};

// --- params -------------------------------------------------------------

unsafe extern "C" fn params_rescan(host: *const clap_host, _flags: clap_param_rescan_flags) {
    // SAFETY: plugins call back with their own host.
    if let Some(data) = unsafe { data(host) } {
        data.raise(flag::PARAMS_RESCAN);
    }
}

unsafe extern "C" fn params_clear(
    _host: *const clap_host,
    _id: clap_id,
    _flags: clap_param_clear_flags,
) {
}

unsafe extern "C" fn params_request_flush(_host: *const clap_host) {
    // `process` runs on every block, which flushes parameter changes.
}

static PARAMS: clap_host_params = clap_host_params {
    rescan: Some(params_rescan),
    clear: Some(params_clear),
    request_flush: Some(params_request_flush),
};

// --- state --------------------------------------------------------------

unsafe extern "C" fn mark_dirty(host: *const clap_host) {
    // SAFETY: plugins call back with their own host.
    if let Some(data) = unsafe { data(host) } {
        data.raise(flag::STATE_DIRTY);
    }
}

static STATE: clap_host_state = clap_host_state {
    mark_dirty: Some(mark_dirty),
};

// --- gui ----------------------------------------------------------------

unsafe extern "C" fn gui_hints_changed(_host: *const clap_host) {}

unsafe extern "C" fn gui_request_resize(host: *const clap_host, width: u32, height: u32) -> bool {
    // SAFETY: plugins call back with their own host.
    let Some(data) = (unsafe { data(host) }) else {
        return false;
    };
    data.resize.store(
        (u64::from(width) << 32) | u64::from(height.max(1)),
        Ordering::Release,
    );
    data.raise(flag::GUI_RESIZE);
    true
}

unsafe extern "C" fn gui_request_show(_host: *const clap_host) -> bool {
    false
}

unsafe extern "C" fn gui_request_hide(host: *const clap_host) -> bool {
    // SAFETY: plugins call back with their own host.
    if let Some(data) = unsafe { data(host) } {
        data.raise(flag::GUI_HIDE);
    }
    true
}

unsafe extern "C" fn gui_closed(host: *const clap_host, _was_destroyed: bool) {
    // SAFETY: plugins call back with their own host.
    if let Some(data) = unsafe { data(host) } {
        data.raise(flag::GUI_CLOSED);
    }
}

static GUI: clap_host_gui = clap_host_gui {
    resize_hints_changed: Some(gui_hints_changed),
    request_resize: Some(gui_request_resize),
    request_show: Some(gui_request_show),
    request_hide: Some(gui_request_hide),
    closed: Some(gui_closed),
};

// --- timers -------------------------------------------------------------

unsafe extern "C" fn register_timer(
    host: *const clap_host,
    period_ms: u32,
    id: *mut clap_id,
) -> bool {
    // SAFETY: plugins call back with their own host.
    let Some(data) = (unsafe { data(host) }) else {
        return false;
    };
    if id.is_null() {
        return false;
    }
    let timer_id = data.next_timer.fetch_add(1, Ordering::Relaxed);
    let period = Duration::from_millis(u64::from(period_ms.max(10)));
    let Ok(mut timers) = data.timers.lock() else {
        return false;
    };
    timers.push(Timer {
        id: timer_id,
        period,
        due: Instant::now() + period,
    });
    // SAFETY: checked non-null; the plugin passes a writable id.
    unsafe { *id = timer_id };
    true
}

unsafe extern "C" fn unregister_timer(host: *const clap_host, id: clap_id) -> bool {
    // SAFETY: plugins call back with their own host.
    let Some(data) = (unsafe { data(host) }) else {
        return false;
    };
    let Ok(mut timers) = data.timers.lock() else {
        return false;
    };
    let before = timers.len();
    timers.retain(|t| t.id != id);
    timers.len() != before
}

static TIMERS: clap_host_timer_support = clap_host_timer_support {
    register_timer: Some(register_timer),
    unregister_timer: Some(unregister_timer),
};

// --- file descriptors ---------------------------------------------------

unsafe extern "C" fn register_fd(
    host: *const clap_host,
    fd: i32,
    flags: clap_posix_fd_flags,
) -> bool {
    // SAFETY: plugins call back with their own host.
    let Some(data) = (unsafe { data(host) }) else {
        return false;
    };
    let Ok(mut fds) = data.fds.lock() else {
        return false;
    };
    fds.retain(|(f, _)| *f != fd);
    fds.push((fd, flags));
    true
}

unsafe extern "C" fn modify_fd(
    host: *const clap_host,
    fd: i32,
    flags: clap_posix_fd_flags,
) -> bool {
    // SAFETY: forwarded with the same host.
    unsafe { register_fd(host, fd, flags) }
}

unsafe extern "C" fn unregister_fd(host: *const clap_host, fd: i32) -> bool {
    // SAFETY: plugins call back with their own host.
    let Some(data) = (unsafe { data(host) }) else {
        return false;
    };
    let Ok(mut fds) = data.fds.lock() else {
        return false;
    };
    let before = fds.len();
    fds.retain(|(f, _)| *f != fd);
    fds.len() != before
}

static FDS: clap_host_posix_fd_support = clap_host_posix_fd_support {
    register_fd: Some(register_fd),
    modify_fd: Some(modify_fd),
    unregister_fd: Some(unregister_fd),
};

// --- ports and latency (changes are not supported while running) --------

unsafe extern "C" fn latency_changed(_host: *const clap_host) {}

static LATENCY: clap_host_latency = clap_host_latency {
    changed: Some(latency_changed),
};

unsafe extern "C" fn audio_ports_flag_supported(_host: *const clap_host, _flag: u32) -> bool {
    false
}

unsafe extern "C" fn ports_rescan(_host: *const clap_host, _flags: u32) {}

static AUDIO_PORTS: clap_host_audio_ports = clap_host_audio_ports {
    is_rescan_flag_supported: Some(audio_ports_flag_supported),
    rescan: Some(ports_rescan),
};

unsafe extern "C" fn note_dialects(_host: *const clap_host) -> clap_note_dialect {
    CLAP_NOTE_DIALECT_CLAP | CLAP_NOTE_DIALECT_MIDI
}

static NOTE_PORTS: clap_host_note_ports = clap_host_note_ports {
    supported_dialects: Some(note_dialects),
    rescan: Some(ports_rescan),
};
