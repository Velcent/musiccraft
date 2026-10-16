//! A plain top-level X11 window for plugin hosts to embed a plugin's view
//! in (through XWayland on Wayland desktops). Shared by the CLAP and VST3
//! hosts; safe code only. On other platforms the crate is empty.

#[cfg(target_os = "linux")]
mod x11;

#[cfg(target_os = "linux")]
pub use x11::{Polled, Window};
