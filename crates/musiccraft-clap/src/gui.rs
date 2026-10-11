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
    window: Option<x11::Window>,
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
            let window = match x11::Window::new(title, width.max(50), height.max(50), resizable) {
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
                x11::Polled::Closed => closed = true,
                x11::Polled::Resized(w, h) => {
                    let (w, h) = fit(shared, w, h);
                    window.resize(w, h);
                }
                x11::Polled::Nothing => {}
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

#[cfg(target_os = "linux")]
mod x11 {
    //! A plain top-level X11 window to embed a plugin in.

    use x11rb::connection::Connection;
    use x11rb::properties::{WmSizeHints, WmSizeHintsSpecification};
    use x11rb::protocol::Event;
    use x11rb::protocol::xproto::{
        AtomEnum, ConfigureWindowAux, ConnectionExt as _, CreateWindowAux, EventMask, PropMode,
        WindowClass,
    };
    use x11rb::rust_connection::RustConnection;
    use x11rb::wrapper::ConnectionExt as _;

    pub enum Polled {
        Nothing,
        Closed,
        Resized(u32, u32),
    }

    pub struct Window {
        conn: RustConnection,
        id: u32,
        delete: u32,
        resizable: bool,
        size: std::cell::Cell<(u32, u32)>,
    }

    fn atom(conn: &RustConnection, name: &[u8]) -> Result<u32, String> {
        Ok(conn
            .intern_atom(false, name)
            .map_err(|e| e.to_string())?
            .reply()
            .map_err(|e| e.to_string())?
            .atom)
    }

    impl Window {
        pub fn new(title: &str, width: u32, height: u32, resizable: bool) -> Result<Self, String> {
            let (conn, screen) = RustConnection::connect(None).map_err(|e| e.to_string())?;
            let root = conn.setup().roots[screen].root;
            let black = conn.setup().roots[screen].black_pixel;
            let id = conn.generate_id().map_err(|e| e.to_string())?;
            let aux = CreateWindowAux::new()
                .background_pixel(black)
                .event_mask(EventMask::STRUCTURE_NOTIFY);
            conn.create_window(
                x11rb::COPY_DEPTH_FROM_PARENT,
                id,
                root,
                0,
                0,
                u16::try_from(width).unwrap_or(u16::MAX),
                u16::try_from(height).unwrap_or(u16::MAX),
                0,
                WindowClass::INPUT_OUTPUT,
                0,
                &aux,
            )
            .map_err(|e| e.to_string())?;
            let protocols = atom(&conn, b"WM_PROTOCOLS")?;
            let delete = atom(&conn, b"WM_DELETE_WINDOW")?;
            let net_name = atom(&conn, b"_NET_WM_NAME")?;
            let utf8 = atom(&conn, b"UTF8_STRING")?;
            // A dialog-type window floats in tiling window managers.
            let window_type = atom(&conn, b"_NET_WM_WINDOW_TYPE")?;
            let dialog = atom(&conn, b"_NET_WM_WINDOW_TYPE_DIALOG")?;
            let ok = conn
                .change_property32(PropMode::REPLACE, id, protocols, AtomEnum::ATOM, &[delete])
                .and_then(|_| {
                    conn.change_property8(
                        PropMode::REPLACE,
                        id,
                        AtomEnum::WM_NAME,
                        AtomEnum::STRING,
                        title.as_bytes(),
                    )
                })
                .and_then(|_| {
                    conn.change_property8(PropMode::REPLACE, id, net_name, utf8, title.as_bytes())
                })
                .and_then(|_| {
                    conn.change_property32(
                        PropMode::REPLACE,
                        id,
                        window_type,
                        AtomEnum::ATOM,
                        &[dialog],
                    )
                })
                .and_then(|_| {
                    conn.change_property8(
                        PropMode::REPLACE,
                        id,
                        AtomEnum::WM_CLASS,
                        AtomEnum::STRING,
                        b"musiccraft-plugin\0MusicCraft\0",
                    )
                });
            ok.map_err(|e| e.to_string())?;
            let window = Self {
                conn,
                id,
                delete,
                resizable,
                size: std::cell::Cell::new((width, height)),
            };
            window.set_hints(width, height);
            window.conn.flush().map_err(|e| e.to_string())?;
            Ok(window)
        }

        /// Tells the window manager the size, and that it is fixed unless
        /// the plugin can resize.
        fn set_hints(&self, width: u32, height: u32) {
            let (w, h) = (
                i32::try_from(width).unwrap_or(i32::MAX),
                i32::try_from(height).unwrap_or(i32::MAX),
            );
            let mut hints = WmSizeHints::new();
            hints.size = Some((WmSizeHintsSpecification::ProgramSpecified, w, h));
            if !self.resizable {
                hints.min_size = Some((w, h));
                hints.max_size = Some((w, h));
            }
            let _ = hints.set_normal_hints(&self.conn, self.id);
        }

        pub fn id(&self) -> u32 {
            self.id
        }

        pub fn map(&self) {
            let _ = self.conn.map_window(self.id);
            let _ = self.conn.flush();
        }

        pub fn resize(&self, width: u32, height: u32) {
            if self.size.get() == (width, height) {
                return;
            }
            self.size.set((width, height));
            self.set_hints(width, height);
            let aux = ConfigureWindowAux::new().width(width).height(height);
            let _ = self.conn.configure_window(self.id, &aux);
            let _ = self.conn.flush();
        }

        pub fn poll(&self) -> Polled {
            let mut result = Polled::Nothing;
            while let Ok(Some(event)) = self.conn.poll_for_event() {
                match event {
                    Event::ClientMessage(message)
                        if message.data.as_data32().first() == Some(&self.delete) =>
                    {
                        return Polled::Closed;
                    }
                    Event::ConfigureNotify(configure) if configure.window == self.id => {
                        let size = (u32::from(configure.width), u32::from(configure.height));
                        if size != self.size.get() {
                            self.size.set(size);
                            result = Polled::Resized(size.0, size.1);
                        }
                    }
                    Event::DestroyNotify(_) => return Polled::Closed,
                    _ => {}
                }
            }
            result
        }
    }

    impl Drop for Window {
        fn drop(&mut self) {
            let _ = self.conn.destroy_window(self.id);
            let _ = self.conn.flush();
        }
    }
}
