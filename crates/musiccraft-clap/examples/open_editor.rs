//! Opens a plugin's own window for a few seconds (manual check of window
//! hosting): `cargo run -p musiccraft-clap --example open_editor <plugin-id> [seconds]`.

use std::time::{Duration, Instant};

use musiccraft_clap::ClapHost;
use musiccraft_render::{PluginFactory, PluginRequest};

fn main() {
    let mut args = std::env::args().skip(1);
    let Some(id) = args.next() else {
        eprintln!("usage: open_editor <plugin-id> [seconds]");
        std::process::exit(2);
    };
    let seconds: u64 = args.next().and_then(|s| s.parse().ok()).unwrap_or(4);
    let host = ClapHost::new();
    let request = PluginRequest {
        id: &id,
        sample_rate: 48_000,
        max_frames: 256,
        state: None,
    };
    let mut live = match host.instantiate(&request) {
        Ok(live) => live,
        Err(err) => {
            eprintln!("{id}: {err}");
            std::process::exit(1);
        }
    };
    println!("has editor: {}", live.instance.has_editor());
    if let Err(err) = live.instance.set_editor_open(true, &id) {
        eprintln!("cannot open: {err}");
        std::process::exit(1);
    }
    println!("open: {}", live.instance.editor_open());
    #[cfg(target_os = "linux")]
    if let Some(id) = find_window() {
        println!("window: 0x{id:x}");
    }
    let end = Instant::now() + Duration::from_secs(seconds);
    while Instant::now() < end {
        for event in live.instance.poll() {
            println!("event: {event:?}");
        }
        std::thread::sleep(Duration::from_millis(16));
    }
    live.instance.set_editor_open(false, &id).ok();
    println!("closed cleanly");
}

/// The X11 window MusicCraft made for the plugin, found by its class.
#[cfg(target_os = "linux")]
fn find_window() -> Option<u32> {
    use x11rb::connection::Connection;
    use x11rb::protocol::xproto::{AtomEnum, ConnectionExt};
    let (conn, screen) = x11rb::connect(None).ok()?;
    let root = conn.setup().roots[screen].root;
    let tree = conn.query_tree(root).ok()?.reply().ok()?;
    tree.children.into_iter().find(|w| {
        conn.get_property(false, *w, AtomEnum::WM_CLASS, AtomEnum::STRING, 0, 64)
            .ok()
            .and_then(|c| c.reply().ok())
            .is_some_and(|p| p.value.starts_with(b"musiccraft-plugin"))
    })
}
