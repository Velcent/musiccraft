//! A plugin's own window. Plugins that can open a floating window of their
//! own get one; on Linux, the rest are embedded in an X11 window MusicCraft
//! creates (through XWayland on Wayland desktops).

use std::ffi::{CStr, CString};

use clap_sys::ext::gui::{
    CLAP_WINDOW_API_COCOA, CLAP_WINDOW_API_WIN32, CLAP_WINDOW_API_X11, clap_plugin_gui,
};

use crate::instance::Shared;

/// The window API for floating windows on this platform.
fn floating_api() -> &'static CStr {
    if cfg!(target_os = "macos") {
        CLAP_WINDOW_API_COCOA
    } else if cfg!(windows) {
        CLAP_WINDOW_API_WIN32
    } else {
        CLAP_WINDOW_API_X11
    }
}

fn table(shared: &Shared) -> Option<&clap_plugin_gui> {
    // SAFETY: a non-null table is valid while the library is loaded.
    unsafe { shared.ext.gui.as_ref() }
}

fn api_supported(shared: &Shared, api: &CStr, floating: bool) -> bool {
    let Some(supported) = table(shared).and_then(|g| g.is_api_supported) else {
        return false;
    };
    // SAFETY: main thread; the api string is static.
    unsafe { supported(shared.ptr(), api.as_ptr(), floating) }
}

#[derive(Default)]
pub(crate) struct Gui {
    open: Option<Open>,
}

struct Open {
    #[cfg(target_os = "linux")]
    window: Option<musiccraft_plugin_window::Window>,
}

impl Gui {
    /// Whether the plugin has a window MusicCraft can show.
    pub fn supported(shared: &Shared) -> bool {
        api_supported(shared, floating_api(), true)
            || (cfg!(target_os = "linux") && api_supported(shared, CLAP_WINDOW_API_X11, false))
    }

    pub fn is_open(&self) -> bool {
        self.open.is_some()
    }

    pub fn open(&mut self, shared: &Shared, title: &str) -> Result<(), String> {
        if self.open.is_some() {
            return Ok(());
        }
        let gui = table(shared).ok_or("the plugin has no window")?;
        let plugin = shared.ptr();
        let (create, show) = (
            gui.create.ok_or("the plugin cannot create a window")?,
            gui.show.ok_or("the plugin cannot show a window")?,
        );
        if api_supported(shared, floating_api(), true) {
            // SAFETY: main thread; api string is static.
            if !unsafe { create(plugin, floating_api().as_ptr(), true) } {
                return Err("the plugin failed to create its window".to_owned());
            }
            if let (Some(suggest), Ok(title)) = (gui.suggest_title, CString::new(title)) {
                // SAFETY: main thread; the title outlives the call.
                unsafe { suggest(plugin, title.as_ptr()) };
            }
            // SAFETY: main thread; the window was created.
            unsafe { show(plugin) };
            self.open = Some(Open {
                #[cfg(target_os = "linux")]
                window: None,
            });
            return Ok(());
        }
        #[cfg(target_os = "linux")]
        {
            if !api_supported(shared, CLAP_WINDOW_API_X11, false) {
                return Err("the plugin's window type is not supported".to_owned());
            }
            let set_parent = gui.set_parent.ok_or("the plugin cannot be embedded")?;
            // SAFETY: main thread; api string is static.
            if !unsafe { create(plugin, CLAP_WINDOW_API_X11.as_ptr(), false) } {
                return Err("the plugin failed to create its window".to_owned());
            }
            let (mut width, mut height) = (640, 480);
            if let Some(get_size) = gui.get_size {
                // SAFETY: main thread; outputs are writable.
                unsafe { get_size(plugin, &mut width, &mut height) };
            }
            // SAFETY: main thread; the window was created.
            let resizable = gui.can_resize.is_some_and(|f| unsafe { f(plugin) });
            let window = match musiccraft_plugin_window::Window::new(
                title,
                width.max(50),
                height.max(50),
                resizable,
            ) {
                Ok(window) => window,
                Err(err) => {
                    self.destroy(shared);
                    return Err(format!("cannot open a window: {err}"));
                }
            };
            let parent = clap_sys::ext::gui::clap_window {
                api: CLAP_WINDOW_API_X11.as_ptr(),
                specific: clap_sys::ext::gui::clap_window_handle {
                    x11: std::ffi::c_ulong::from(window.id()),
                },
            };
            // SAFETY: main thread; `parent` and the X window outlive the call.
            if !unsafe { set_parent(plugin, &parent) } {
                self.destroy(shared);
                return Err("the plugin refused its window".to_owned());
            }
            // SAFETY: main thread; the window was created and parented.
            unsafe { show(plugin) };
            window.map();
            self.open = Some(Open {
                window: Some(window),
            });
            Ok(())
        }
        #[cfg(not(target_os = "linux"))]
        Err(
            "this plugin can only be embedded, which is not supported on this platform yet"
                .to_owned(),
        )
    }

    fn destroy(&mut self, shared: &Shared) {
        if let Some(destroy) = table(shared).and_then(|g| g.destroy) {
            // SAFETY: main thread; the window was created.
            unsafe { destroy(shared.ptr()) };
        }
    }

    pub fn close(&mut self, shared: &Shared) {
        let Some(_open) = self.open.take() else {
            return;
        };
        if let Some(hide) = table(shared).and_then(|g| g.hide) {
            // SAFETY: main thread; the window is shown.
            unsafe { hide(shared.ptr()) };
        }
        self.destroy(shared);
        // Our X window (if any) is destroyed when `_open` drops, after the
        // plugin has let go of it.
    }

    /// Handles window events and requests. Returns `true` if the window
    /// closed.
    pub fn poll(&mut self, shared: &Shared, resize: Option<(u32, u32)>, hide: bool) -> bool {
        if self.open.is_none() {
            return false;
        }
        let mut closed = hide;
        #[cfg(target_os = "linux")]
        if let Some(window) = self.open.as_ref().and_then(|o| o.window.as_ref()) {
            if let Some((w, h)) = resize {
                window.resize(w, h);
            }
            match window.poll() {
                musiccraft_plugin_window::Polled::Closed => closed = true,
                musiccraft_plugin_window::Polled::Resized(w, h) => {
                    let (w, h) = fit(shared, w, h);
                    window.resize(w, h);
                }
                musiccraft_plugin_window::Polled::Nothing => {}
            }
        }
        #[cfg(not(target_os = "linux"))]
        let _ = resize;
        if closed {
            self.close(shared);
        }
        closed
    }
}

/// The size the plugin accepts closest to `width` x `height`, applied to it:
/// resizable plugins adjust and take the size; fixed ones keep theirs.
#[cfg(target_os = "linux")]
fn fit(shared: &Shared, mut width: u32, mut height: u32) -> (u32, u32) {
    let Some(gui) = table(shared) else {
        return (width, height);
    };
    let plugin = shared.ptr();
    // SAFETY: main thread; the window is shown.
    let resizable = gui.can_resize.is_some_and(|f| unsafe { f(plugin) });
    if resizable {
        if let Some(adjust) = gui.adjust_size {
            // SAFETY: main thread; outputs are writable.
            unsafe { adjust(plugin, &mut width, &mut height) };
        }
        if let Some(set_size) = gui.set_size {
            // SAFETY: main thread; the size was adjusted by the plugin.
            unsafe { set_size(plugin, width, height) };
        }
    } else if let Some(get_size) = gui.get_size {
        // SAFETY: main thread; outputs are writable.
        unsafe { get_size(plugin, &mut width, &mut height) };
    }
    (width, height)
}
