//! MIDI keyboard input: notes go straight to the engine's live queue and play on
//! the selected channel.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::Sender;
use std::sync::{Arc, Mutex};

use eframe::egui;
use midir::{MidiInput, MidiInputConnection};
use musiccraft_engine::{LiveEvent, LiveSender, Meters};

/// A note event forwarded to the main thread for recording: (key, velocity or 0
/// for note off, engine position in ticks when it arrived).
pub type Played = (u8, u8, f64);

/// A live MIDI input connection. Dropping it disconnects.
pub struct MidiIn {
    _connection: MidiInputConnection<()>,
    /// Port name.
    pub name: String,
}

/// Decodes a MIDI message into a live event for `channel`.
fn decode(message: &[u8], channel: u16) -> Option<LiveEvent> {
    match *message {
        [status, key, velocity, ..] if status & 0xf0 == 0x90 && velocity > 0 => {
            Some(LiveEvent::NoteOn {
                channel,
                key: key & 0x7f,
                velocity: f32::from(velocity) / 127.0,
            })
        }
        [status, key, ..] if status & 0xf0 == 0x80 || status & 0xf0 == 0x90 => {
            Some(LiveEvent::NoteOff {
                channel,
                key: key & 0x7f,
            })
        }
        _ => None,
    }
}

/// Names of the MIDI input ports.
pub fn port_names() -> Vec<String> {
    let Ok(input) = MidiInput::new("MusicCraft") else {
        return Vec::new();
    };
    input
        .ports()
        .iter()
        .filter_map(|p| input.port_name(p).ok())
        .collect()
}

/// Connects to the input port named `port`, or with `None` to the first real
/// keyboard.
///
/// # Errors
///
/// Returns a description when there is no such port or connecting fails.
pub fn connect(
    sender: Arc<Mutex<LiveSender>>,
    selected: Arc<AtomicUsize>,
    meters: Arc<Meters>,
    played: Sender<Played>,
    ctx: egui::Context,
    port: Option<&str>,
) -> Result<MidiIn, String> {
    let input = MidiInput::new("MusicCraft").map_err(|e| e.to_string())?;
    let ports = input.ports();
    let port = match port {
        Some(wanted) => ports
            .iter()
            .find(|p| input.port_name(p).is_ok_and(|name| name == wanted))
            .ok_or_else(|| format!("MIDI port `{wanted}` not found"))?,
        // Prefer a real device over virtual loopback ports such as ALSA's
        // "Midi Through".
        None => ports
            .iter()
            .find(|p| {
                input
                    .port_name(p)
                    .is_ok_and(|name| !name.to_ascii_lowercase().contains("through"))
            })
            .ok_or("no MIDI keyboard found")?,
    };
    let name = input
        .port_name(port)
        .unwrap_or_else(|_| "MIDI input".to_owned());
    let connection = input
        .connect(
            port,
            "musiccraft-in",
            move |_stamp, message, ()| {
                let channel = u16::try_from(selected.load(Ordering::Relaxed)).unwrap_or(0);
                if let Some(event) = decode(message, channel) {
                    // Only MIDI threads take this lock, never the audio thread.
                    if let Ok(mut sender) = sender.lock() {
                        sender.send(event);
                    }
                    let position = meters.position_ticks();
                    let note = match event {
                        LiveEvent::NoteOn { key, velocity, .. } => {
                            (key, (velocity * 127.0).round().max(1.0) as u8, position)
                        }
                        LiveEvent::NoteOff { key, .. } => (key, 0, position),
                    };
                    let _ = played.send(note);
                    ctx.request_repaint();
                }
            },
            (),
        )
        .map_err(|e| e.to_string())?;
    Ok(MidiIn {
        _connection: connection,
        name,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_note_messages() {
        assert_eq!(
            decode(&[0x91, 60, 127], 3),
            Some(LiveEvent::NoteOn {
                channel: 3,
                key: 60,
                velocity: 1.0
            })
        );
        assert_eq!(
            decode(&[0x90, 60, 0], 0),
            Some(LiveEvent::NoteOff {
                channel: 0,
                key: 60
            })
        );
        assert_eq!(
            decode(&[0x80, 61, 40], 0),
            Some(LiveEvent::NoteOff {
                channel: 0,
                key: 61
            })
        );
        assert_eq!(decode(&[0xb0, 7, 100], 0), None);
        assert_eq!(decode(&[0x90], 0), None);
    }
}
