//! The plugins' real-time note functions (`musiccraft_instruments::live`)
//! against the app's, which apply when a sequence is built: the same held
//! notes give the same notes for the voices.

use musiccraft_commands::{Registry, Session};
use musiccraft_engine::{NoteEvent, NoteShape};
use musiccraft_instruments::live::{Note, NoteFunctions, Timing};
use musiccraft_params::PPQ;
use musiccraft_project::Project;

/// Frames per tick in the real-time run: 120 BPM at 4800 frames a second.
const TICK: u64 = 25;
const BLOCK: u32 = 64;

fn project(settings: &[&str]) -> Project {
    let mut s = Session::new(Project::default(), Registry::with_builtin());
    s.run_line("channel.add_synth").unwrap();
    for line in settings {
        s.run_line(&format!("channel.set_param 1 {line}"))
            .unwrap_or_else(|e| panic!("{line}: {e}"));
    }
    s.project().clone()
}

fn note(start: u64, length: u64, key: u8) -> NoteEvent {
    NoteEvent {
        shape: NoteShape::default(),
        start,
        length,
        channel: 0,
        key,
        velocity: 0.8,
        pan: 0.0,
        slide: false,
    }
}

/// `(tick, key, on)` for every start and end, sorted.
fn app(project: &Project, notes: &[NoteEvent]) -> Vec<(u64, u8, bool)> {
    let events = crate::delivery::apply(project, crate::arp::apply(project, notes.to_vec()));
    let mut out: Vec<_> = events
        .iter()
        .flat_map(|e| [(e.start, e.key, true), (e.start + e.length, e.key, false)])
        .collect();
    out.sort_unstable();
    out
}

fn live(project: &Project, notes: &[NoteEvent]) -> Vec<(u64, u8, bool)> {
    let params = project.channels[0].params.0;
    let mut inputs: Vec<(u64, Note)> = notes
        .iter()
        .flat_map(|n| {
            [
                (
                    n.start * TICK,
                    Note::On {
                        key: n.key,
                        velocity: n.velocity,
                    },
                ),
                ((n.start + n.length) * TICK, Note::Off { key: n.key }),
            ]
        })
        .collect();
    inputs.sort_by_key(|(t, n)| (*t, matches!(n, Note::On { .. })));
    let end = inputs.last().map_or(0, |(t, _)| *t) + 2_000 * TICK;
    let mut f = NoteFunctions::new();
    let mut out = Vec::new();
    let mut start = 0;
    while start < end {
        let block: Vec<(u32, Note)> = inputs
            .iter()
            .filter(|(t, _)| (start..start + u64::from(BLOCK)).contains(t))
            .map(|(t, n)| ((t - start) as u32, *n))
            .collect();
        let timing = Timing {
            tempo: 120.0,
            sample_rate: 4800.0,
            beats: Some(start as f64 / TICK as f64 / f64::from(PPQ)),
        };
        f.run(&params, 0.0, timing, BLOCK, &block, &mut |frame, note| {
            let tick = (start + u64::from(frame)) / TICK;
            out.push(match note {
                Note::On { key, .. } => (tick, key, true),
                Note::Off { key } => (tick, key, false),
            });
        });
        start += u64::from(BLOCK);
    }
    out.sort_unstable();
    out
}

#[test]
fn real_time_note_functions_match_the_app() {
    let chord = [note(0, 96, 60), note(0, 96, 64), note(0, 96, 67)];
    let melody = [note(0, 20, 60), note(48, 30, 62), note(96, 12, 65)];
    for (settings, notes) in [
        (vec!["arp_mode 1", "arp_rate 1", "arp_gate 0.5"], &chord[..]),
        (vec!["arp_mode 2", "arp_range 2", "arp_rate 2"], &chord[..]),
        (vec!["arp_mode 3", "arp_rate 0", "arp_gate 1"], &chord[..]),
        (vec!["time_shift 6"], &melody[..]),
        (vec!["gate 10"], &melody[..]),
        (
            vec![
                "echo_count 3",
                "echo_time 36",
                "echo_feed 0.6",
                "echo_pitch 5",
            ],
            &melody[..],
        ),
        (
            vec![
                "arp_mode 1",
                "arp_rate 1",
                "echo_count 1",
                "echo_time 12",
                "gate 6",
            ],
            &chord[..],
        ),
    ] {
        let p = project(&settings);
        assert_eq!(live(&p, notes), app(&p, notes), "{settings:?}");
    }
}
