//! Per-channel note delivery applied when the sequence is built: time shift,
//! gate (a fixed note length) and note echo.

use musiccraft_engine::NoteEvent;
use musiccraft_params::inst;
use musiccraft_project::{Channel, Project};

/// A channel's delivery settings.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Delivery {
    shift: i64,
    gate: u64,
    echoes: u32,
    echo_time: u64,
    feed: f32,
    pitch: i32,
}

impl Delivery {
    fn of(channel: &Channel) -> Self {
        let p = &channel.params;
        let int = |i: usize| p.get(i).round() as i64;
        Self {
            shift: int(inst::TIME_SHIFT),
            gate: int(inst::GATE).max(0) as u64,
            echoes: int(inst::ECHO_COUNT).clamp(0, 8) as u32,
            echo_time: int(inst::ECHO_TIME).max(1) as u64,
            feed: p.get(inst::ECHO_FEED).clamp(0.0, 1.0),
            pitch: int(inst::ECHO_PITCH) as i32,
        }
    }

    fn is_plain(self) -> bool {
        self.shift == 0 && self.gate == 0 && self.echoes == 0
    }
}

/// Applies each channel's time shift, gate and echo to `events`.
pub(crate) fn apply(project: &Project, events: Vec<NoteEvent>) -> Vec<NoteEvent> {
    let settings: Vec<Delivery> = project.channels.iter().map(Delivery::of).collect();
    if settings.iter().all(|d| d.is_plain()) {
        return events;
    }
    let mut out = Vec::with_capacity(events.len());
    for event in events {
        let Some(d) = settings.get(usize::from(event.channel)).copied() else {
            out.push(event);
            continue;
        };
        let start = (event.start as i64 + d.shift).max(0) as u64;
        let length = if d.gate > 0 { d.gate } else { event.length };
        let first = NoteEvent {
            start,
            length,
            ..event
        };
        out.push(first);
        let mut velocity = event.velocity;
        for i in 1..=d.echoes {
            velocity *= d.feed;
            if velocity < 1.0 / 127.0 {
                break;
            }
            let key = i32::from(event.key) + d.pitch * i as i32;
            let Ok(key) = u8::try_from(key) else {
                break;
            };
            if key > 127 {
                break;
            }
            out.push(NoteEvent {
                start: start + d.echo_time * u64::from(i),
                velocity,
                key,
                // Echoes never slide; each is its own note.
                slide: false,
                ..first
            });
        }
    }
    out
}
