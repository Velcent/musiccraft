//! The per-channel arpeggiator, applied when sequences are built.

use musiccraft_engine::NoteEvent;
use musiccraft_params::{ARP_RATES, inst};
use musiccraft_project::Project;

/// Replaces the notes of arpeggiated channels with their arpeggios.
pub(crate) fn apply(project: &Project, events: Vec<NoteEvent>) -> Vec<NoteEvent> {
    let arpeggiated: Vec<(u16, Settings)> = project
        .channels
        .iter()
        .enumerate()
        .filter_map(|(i, c)| Some((u16::try_from(i).ok()?, Settings::of(c)?)))
        .collect();
    if arpeggiated.is_empty() {
        return events;
    }
    let (mut out, mut held): (Vec<NoteEvent>, Vec<NoteEvent>) = events
        .into_iter()
        .partition(|e| !arpeggiated.iter().any(|(c, _)| *c == e.channel));
    held.sort_by_key(|e| (e.channel, e.start));
    for (channel, settings) in &arpeggiated {
        let notes: Vec<NoteEvent> = held
            .iter()
            .filter(|e| e.channel == *channel)
            .copied()
            .collect();
        out.extend(arpeggio(&notes, *settings));
    }
    out
}

#[derive(Debug, Clone, Copy)]
struct Settings {
    mode: u8,
    rate: u64,
    range: u8,
    gate: f32,
}

impl Settings {
    fn of(channel: &musiccraft_project::Channel) -> Option<Self> {
        let p = &channel.params;
        let mode = p.get(inst::ARP_MODE).round().clamp(0.0, 4.0) as u8;
        if mode == 0 {
            return None;
        }
        let rate_index = p.get(inst::ARP_RATE).round().clamp(0.0, 4.0) as usize;
        Some(Self {
            mode,
            rate: u64::from(ARP_RATES.get(rate_index).map_or(24, |(_, t)| *t)),
            range: p.get(inst::ARP_RANGE).round().clamp(1.0, 4.0) as u8,
            gate: p.get(inst::ARP_GATE).clamp(0.05, 1.0),
        })
    }
}

/// A stable pseudo-random number for a step, so random order repeats.
fn hash(step: u64) -> u64 {
    let mut x = step.wrapping_add(0x9E37_79B9_7F4A_7C15);
    x = (x ^ (x >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
    x = (x ^ (x >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
    x ^ (x >> 31)
}

/// The keys to cycle through, in play order.
fn order(mut keys: Vec<u8>, settings: Settings) -> Vec<u8> {
    keys.sort_unstable();
    keys.dedup();
    let mut spread: Vec<u8> = (0..settings.range)
        .flat_map(|octave| keys.iter().filter_map(move |k| k.checked_add(12 * octave)))
        .filter(|k| *k <= 127)
        .collect();
    spread.sort_unstable();
    spread.dedup();
    match settings.mode {
        2 => spread.reverse(),
        3 if spread.len() > 2 => {
            let back: Vec<u8> = spread[1..spread.len() - 1].iter().rev().copied().collect();
            spread.extend(back);
        }
        _ => {}
    }
    spread
}

fn arpeggio(notes: &[NoteEvent], settings: Settings) -> Vec<NoteEvent> {
    let Some(first) = notes.iter().map(|n| n.start).min() else {
        return Vec::new();
    };
    let end = notes
        .iter()
        .map(|n| n.start + n.length)
        .max()
        .unwrap_or(first);
    let rate = settings.rate.max(1);
    let length = ((rate as f32 * settings.gate).round() as u64).max(1);
    let mut out = Vec::new();
    let mut counter = 0usize;
    let mut t = first / rate * rate;
    while t < end {
        // Keys held at any time during this step.
        let held: Vec<&NoteEvent> = notes
            .iter()
            .filter(|n| n.start < t + rate && n.start + n.length > t)
            .collect();
        if held.is_empty() {
            counter = 0;
            t += rate;
            continue;
        }
        let keys = order(held.iter().map(|n| n.key).collect(), settings);
        let key = if settings.mode == 4 {
            keys[(hash(t) % keys.len() as u64) as usize]
        } else {
            keys[counter % keys.len()]
        };
        let loudest = held.iter().map(|n| n.velocity).fold(0.0, f32::max);
        // A step that starts before the first note waits for it.
        let start = t.max(held.iter().map(|n| n.start).min().unwrap_or(t));
        out.push(NoteEvent {
            start,
            length: length.min(t + rate - start).max(1),
            key,
            velocity: loudest,
            slide: false,
            ..*held[0]
        });
        counter += 1;
        t += rate;
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn note(start: u64, length: u64, key: u8) -> NoteEvent {
        NoteEvent {
            start,
            length,
            channel: 0,
            key,
            velocity: 0.8,
            pan: 0.0,
            slide: false,
        }
    }

    fn settings(mode: u8, range: u8) -> Settings {
        Settings {
            mode,
            rate: 24,
            range,
            gate: 0.5,
        }
    }

    #[test]
    fn chords_become_steps_in_the_chosen_order() {
        let chord = [note(0, 96, 60), note(0, 96, 64), note(0, 96, 67)];
        let keys = |s| {
            arpeggio(&chord, s)
                .iter()
                .map(|e| e.key)
                .collect::<Vec<_>>()
        };
        assert_eq!(keys(settings(1, 1)), [60, 64, 67, 60]);
        assert_eq!(keys(settings(2, 1)), [67, 64, 60, 67]);
        assert_eq!(keys(settings(3, 1)), [60, 64, 67, 64]);
        assert_eq!(keys(settings(1, 2)), [60, 64, 67, 72]);
        let random = keys(settings(4, 1));
        assert_eq!(random, keys(settings(4, 1)), "random is repeatable");
        let steps = arpeggio(&chord, settings(1, 1));
        assert!(steps.iter().all(|e| e.length == 12));
        assert_eq!(
            steps.iter().map(|e| e.start).collect::<Vec<_>>(),
            [0, 24, 48, 72]
        );
    }

    #[test]
    fn order_restarts_after_a_gap() {
        let notes = [
            note(0, 24, 60),
            note(0, 24, 64),
            note(96, 24, 60),
            note(96, 24, 64),
        ];
        let keys: Vec<u8> = arpeggio(&notes, settings(1, 1))
            .iter()
            .map(|e| e.key)
            .collect();
        assert_eq!(keys, [60, 60]);
    }
}
