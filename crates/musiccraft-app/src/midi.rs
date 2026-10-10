//! MIDI keyboard input: notes go straight to the engine's live queue and play on
//! the selected channel.

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};

use midir::{MidiInput, MidiInputConnection};
use musiccraft_engine::{LiveEvent, LiveSender};

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

/// Connects to the first MIDI input port, if any.
///
/// # Errors
///
/// Returns a description when there is no port or connecting fails.
pub fn connect(mut sender: LiveSender, selected: Arc<AtomicUsize>) -> Result<MidiIn, String> {
    let input = MidiInput::new("MusicCraft").map_err(|e| e.to_string())?;
    let ports = input.ports();
    // Prefer a real device over virtual loopback ports such as ALSA's "Midi Through".
    let port = ports
        .iter()
        .find(|p| {
            input
                .port_name(p)
                .is_ok_and(|name| !name.to_ascii_lowercase().contains("through"))
        })
        .ok_or("no MIDI keyboard found")?;
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
                    sender.send(event);
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
