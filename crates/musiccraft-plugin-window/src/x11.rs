//! The X11 window.

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
