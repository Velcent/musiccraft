//! More piano roll tools: limit, LFO, scale levels, flam, randomize, chord
//! progressions, articulate and claw machine. Each rewrites a channel's notes
//! in one undo step.

use musiccraft_project::params::{TICKS_PER_BAR, TICKS_PER_STEP};
use musiccraft_project::{ChannelId, Note, NoteProps, Project};

use super::note_tools::{Rng, apply, by_start, nearest_in_scale, pitch_class, scale};
use crate::args::Args;
use crate::command::cmd;
use crate::{Command, CommandError, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "note.limit",
            "note.limit <channel> <low key> <high key> [@pattern]",
            "Fold notes outside a key range into it by octaves.",
            limit,
        ),
        cmd(
            "note.lfo",
            "note.lfo <channel> <velocity|pan|fine|mod_x|mod_y> <period> <depth 0-100> [sine|triangle|square|saw] [@pattern]",
            "Shape a note property over time with an LFO.",
            lfo,
        ),
        cmd(
            "note.scale_levels",
            "note.scale_levels <channel> <velocity|pan|fine|mod_x|mod_y> <percent> [offset] [@pattern]",
            "Scale a note property (around its centre) and add an offset.",
            scale_levels,
        ),
        cmd(
            "note.flam",
            "note.flam <channel> <spacing> [count] [velocity percent] [@pattern]",
            "Add quieter grace notes just before each note.",
            flam,
        ),
        cmd(
            "note.randomize",
            "note.randomize <channel> <low key> <high key> <density 0-100> [seed] [root scale] [@pattern]",
            "Replace the channel's notes with random notes on the step grid.",
            randomize,
        ),
        cmd(
            "note.chords",
            "note.chords <channel> <root> <major|minor> <degrees e.g. 1-5-6-4> [bars per chord] [@pattern]",
            "Lay down a chord progression from scale degrees.",
            chords,
        ),
        cmd(
            "note.articulate",
            "note.articulate <channel> <length percent> [accent] [@pattern]",
            "Set note lengths to a share of the gap to the next note; accent notes on beats.",
            articulate,
        ),
        cmd(
            "note.claw",
            "note.claw <channel> <every> [offset] [@pattern]",
            "Keep every Nth note (or chord) and drop the rest.",
            claw,
        ),
    ]
}

/// A per-note property tools can shape: its range and centre.
#[derive(Clone, Copy)]
enum Prop {
    Velocity,
    Pan,
    Fine,
    ModX,
    ModY,
}

impl Prop {
    fn parse(args: &Args<'_>, i: usize) -> Result<Self, CommandError> {
        Ok(match args.str(i)? {
            "velocity" => Self::Velocity,
            "pan" => Self::Pan,
            "fine" => Self::Fine,
            "mod_x" => Self::ModX,
            "mod_y" => Self::ModY,
            other => {
                return Err(args.invalid(format!(
                    "`{other}` is not velocity, pan, fine, mod_x or mod_y"
                )));
            }
        })
    }

    /// (lowest, centre, highest).
    fn range(self) -> (f32, f32, f32) {
        match self {
            Self::Velocity => (1.0, 64.0, 127.0),
            Self::Pan | Self::Fine => (-100.0, 0.0, 100.0),
            Self::ModX | Self::ModY => (0.0, 64.0, 127.0),
        }
    }

    fn get(self, n: &Note) -> f32 {
        match self {
            Self::Velocity => f32::from(n.velocity),
            Self::Pan => f32::from(n.pan),
            Self::Fine => f32::from(n.props.fine),
            Self::ModX => f32::from(n.props.mod_x),
            Self::ModY => f32::from(n.props.mod_y),
        }
    }

    fn set(self, n: &mut Note, value: f32) {
        let (low, _, high) = self.range();
        let v = value.round().clamp(low, high);
        match self {
            Self::Velocity => n.velocity = v as u8,
            Self::Pan => n.pan = v as i8,
            Self::Fine => n.props.fine = v as i8,
            Self::ModX => n.props.mod_x = v as u8,
            Self::ModY => n.props.mod_y = v as u8,
        }
    }
}

fn key_arg(args: &Args<'_>, i: usize) -> Result<u8, CommandError> {
    let raw = args.str(i)?;
    raw.parse::<u8>()
        .ok()
        .filter(|k| *k <= 127)
        .or_else(|| musiccraft_project::music::parse_key(raw))
        .ok_or_else(|| args.invalid(format!("`{raw}` is not a key")))
}

fn limit(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let (low, high) = (key_arg(args, 1)?, key_arg(args, 2)?);
    if high < low {
        return Err(args.invalid("the high key is below the low key"));
    }
    apply(project, args, "limited", |notes| {
        notes
            .into_iter()
            .map(|n| {
                let mut key = i32::from(n.key);
                while key > i32::from(high) && key - 12 >= i32::from(low) {
                    key -= 12;
                }
                while key < i32::from(low) && key + 12 <= i32::from(high) {
                    key += 12;
                }
                // A range under an octave may not hold the pitch class.
                let key = key.clamp(i32::from(low), i32::from(high));
                Note {
                    key: u8::try_from(key).unwrap_or(n.key),
                    ..n
                }
            })
            .collect()
    })
}

/// One LFO cycle, -1..1, at `phase` (cycles).
fn wave(shape: &str, phase: f64) -> f32 {
    let p = phase.rem_euclid(1.0);
    (match shape {
        "triangle" => 1.0 - 4.0 * (p - 0.5).abs(),
        "square" => {
            if p < 0.5 {
                1.0
            } else {
                -1.0
            }
        }
        "saw" => 2.0 * p - 1.0,
        _ => (p * std::f64::consts::TAU).sin(),
    }) as f32
}

fn lfo(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 5)?;
    let prop = Prop::parse(args, 1)?;
    let period = args.duration(2, TICKS_PER_STEP)?.max(1);
    let depth =
        musiccraft_project::check_range("depth", args.f32(3, "depth")?, 0.0, 100.0)? / 100.0;
    let shape = args.opt(4).unwrap_or("sine");
    if !["sine", "triangle", "square", "saw"].contains(&shape) {
        return Err(args.invalid(format!("`{shape}` is not sine, triangle, square or saw")));
    }
    let (low, centre, high) = prop.range();
    let swing = (high - centre).min(centre - low);
    apply(project, args, "shaped by the LFO", |notes| {
        notes
            .into_iter()
            .map(|mut n| {
                let phase = f64::from(n.start) / f64::from(period);
                prop.set(&mut n, centre + swing * depth * wave(shape, phase));
                n
            })
            .collect()
    })
}

fn scale_levels(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 4)?;
    let prop = Prop::parse(args, 1)?;
    let percent = musiccraft_project::check_range("percent", args.f32(2, "percent")?, 0.0, 400.0)?;
    let offset = match args.opt(3) {
        Some(_) => args.f32(3, "offset")?,
        None => 0.0,
    };
    let (_, centre, _) = prop.range();
    apply(project, args, "levels scaled", |notes| {
        notes
            .into_iter()
            .map(|mut n| {
                let value = centre + (prop.get(&n) - centre) * percent / 100.0 + offset;
                prop.set(&mut n, value);
                n
            })
            .collect()
    })
}

fn flam(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 4)?;
    let spacing = args.duration(1, TICKS_PER_STEP)?.max(1);
    let count: u32 = match args.opt(2) {
        Some(_) => args.parse(2, "count")?,
        None => 1,
    };
    if !(1..=8).contains(&count) {
        return Err(args.invalid("count must be 1 to 8"));
    }
    let percent = match args.opt(3) {
        Some(_) => {
            musiccraft_project::check_range("velocity", args.f32(3, "velocity")?, 1.0, 100.0)?
        }
        None => 60.0,
    };
    apply(project, args, "flammed", |notes| {
        let mut out = notes.clone();
        for n in &notes {
            for i in 1..=count {
                let Some(start) = n.start.checked_sub(spacing * i) else {
                    break;
                };
                let velocity = f32::from(n.velocity) * (percent / 100.0).powi(i as i32);
                out.push(Note {
                    start,
                    length: spacing.min(n.length),
                    velocity: velocity.round().clamp(1.0, 127.0) as u8,
                    ..*n
                });
            }
        }
        out
    })
}

fn randomize(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 7)?;
    let (low, high) = (key_arg(args, 1)?, key_arg(args, 2)?);
    if high < low {
        return Err(args.invalid("the high key is below the low key"));
    }
    let density = musiccraft_project::check_range("density", args.f32(3, "density")?, 0.0, 100.0)?;
    let seed: u64 = match args.opt(4) {
        Some(_) => args.parse(4, "seed")?,
        None => 1,
    };
    let in_scale = match (args.opt(5), args.opt(6)) {
        (Some(root), Some(name)) => Some((
            pitch_class(root).ok_or_else(|| args.invalid(format!("`{root}` is not a root")))?,
            scale(name).ok_or_else(|| args.invalid(format!("unknown scale `{name}`")))?,
        )),
        (None, None) => None,
        _ => return Err(args.invalid("give both a root and a scale, or neither")),
    };
    let target = super::notes::Target::resolve(project, args)?;
    let steps = project.patterns[target.pattern].length_ticks() / TICKS_PER_STEP;
    let mut rng = Rng::new(seed);
    apply(project, args, "randomized", |_| {
        let mut out = Vec::new();
        for step in 0..steps {
            if (rng.next() % 10_000) as f32 >= density * 100.0 {
                continue;
            }
            let span = u64::from(high - low) + 1;
            let mut key = low + (rng.next() % span) as u8;
            if let Some((root, degrees)) = in_scale {
                key = nearest_in_scale(key, root, degrees).clamp(low, high);
            }
            out.push(Note {
                channel: ChannelId(0),
                start: step * TICKS_PER_STEP,
                length: TICKS_PER_STEP,
                key,
                velocity: 70 + (rng.next() % 58) as u8,
                pan: 0,
                slide: false,
                props: NoteProps::default(),
            });
        }
        out
    })
}

fn chords(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 5)?;
    let root = key_arg(args, 1).or_else(|_| {
        pitch_class(args.str(1)?)
            .map(|pc| 48 + pc)
            .ok_or_else(|| args.invalid("not a root"))
    })?;
    let minor = match args.str(2)? {
        "major" => false,
        "minor" => true,
        other => return Err(args.invalid(format!("`{other}` is not major or minor"))),
    };
    let degrees: Vec<usize> = args
        .str(3)?
        .split('-')
        .map(|d| d.parse::<usize>().ok().filter(|d| (1..=7).contains(d)))
        .collect::<Option<_>>()
        .ok_or_else(|| args.invalid("degrees are 1 to 7, joined by '-'"))?;
    let bars = match args.opt(4) {
        Some(_) => args.duration(4, TICKS_PER_BAR)?.max(1),
        None => TICKS_PER_BAR,
    };
    let steps = scale(if minor { "minor" } else { "major" }).unwrap_or(&[0, 2, 4, 5, 7, 9, 11]);
    apply(project, args, "chords", |_| {
        let mut out = Vec::new();
        for (i, degree) in degrees.iter().enumerate() {
            let index = degree - 1;
            // A triad stacked in thirds on the scale degree.
            for third in [0, 2, 4] {
                let step = index + third;
                let octave = (step / steps.len()) as u8;
                let semitone = steps.get(step % steps.len()).copied().unwrap_or(0);
                let key = u16::from(root) + u16::from(semitone) + 12 * u16::from(octave);
                let Ok(key) = u8::try_from(key) else {
                    continue;
                };
                if key > 127 {
                    continue;
                }
                out.push(Note {
                    channel: ChannelId(0),
                    start: bars * i as u32,
                    length: bars,
                    key,
                    velocity: 100,
                    pan: 0,
                    slide: false,
                    props: NoteProps::default(),
                });
            }
        }
        out
    })
}

fn articulate(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 3)?;
    let percent = musiccraft_project::check_range("length", args.f32(1, "length")?, 5.0, 200.0)?;
    let accent: i32 = match args.opt(2) {
        Some(_) => args.parse(2, "accent")?,
        None => 0,
    };
    apply(project, args, "articulated", |notes| {
        let starts: Vec<u32> = by_start(&notes).keys().copied().collect();
        notes
            .into_iter()
            .map(|n| {
                let gap = starts
                    .iter()
                    .find(|s| **s > n.start)
                    .map_or(n.length, |next| next - n.start);
                let length = ((gap as f32 * percent / 100.0).round() as u32).max(1);
                let on_beat = n.start % musiccraft_project::params::PPQ == 0;
                let velocity = if on_beat {
                    (i32::from(n.velocity) + accent).clamp(1, 127) as u8
                } else {
                    n.velocity
                };
                Note {
                    length,
                    velocity,
                    ..n
                }
            })
            .collect()
    })
}

fn claw(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 3)?;
    let every: usize = args.parse(1, "every")?;
    if every == 0 {
        return Err(args.invalid("every must be at least 1"));
    }
    let offset: usize = match args.opt(2) {
        Some(_) => args.parse(2, "offset")?,
        None => 0,
    };
    apply(project, args, "clawed", |notes| {
        by_start(&notes)
            .into_values()
            .enumerate()
            .filter(|(i, _)| *i >= offset && (i - offset) % every == 0)
            .flat_map(|(_, group)| group)
            .collect()
    })
}
