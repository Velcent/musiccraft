//! Piano roll and step tools that rewrite a channel's notes in one step:
//! strum, humanize, legato, chop, glue, flip, snap to scale, arpeggiate, fill
//! and shift.

use std::collections::BTreeMap;

use musiccraft_project::music::{SCALES, parse_key};
use musiccraft_project::params::TICKS_PER_STEP;
use musiccraft_project::{Note, Project};

use super::notes::{Target, rewrite};
use crate::args::Args;
use crate::command::cmd;
use crate::{BATCH, Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "note.strum",
            "note.strum <channel> <spread> [up|down] [@pattern]",
            "Stagger notes that start together, like strumming.",
            strum,
        ),
        cmd(
            "note.humanize",
            "note.humanize <channel> <timing> <velocity> [seed] [@pattern]",
            "Randomly nudge note timing and velocity (repeatable with a seed).",
            humanize,
        ),
        cmd(
            "note.legato",
            "note.legato <channel> [@pattern]",
            "Make every note last until the next one starts.",
            legato,
        ),
        cmd(
            "note.chop",
            "note.chop <channel> <length> [@pattern]",
            "Split notes into pieces of a length.",
            chop,
        ),
        cmd(
            "note.glue",
            "note.glue <channel> [@pattern]",
            "Join touching notes of the same key.",
            glue,
        ),
        cmd(
            "note.flip",
            "note.flip <channel> <time|pitch> [@pattern]",
            "Mirror notes in time or upside down.",
            flip,
        ),
        cmd(
            "note.to_scale",
            "note.to_scale <channel> <root> <scale> [@pattern]",
            "Move notes to the nearest key of a scale.",
            to_scale,
        ),
        cmd(
            "note.arpeggiate",
            "note.arpeggiate <channel> <step> [up|down|updown|random] [seed] [@pattern]",
            "Turn chords into arpeggios.",
            arpeggiate,
        ),
        cmd(
            "channel.slice",
            "channel.slice <channel> <slices> [@pattern]",
            "Cut a sampler's sample into slices and lay them out as notes.",
            slice,
        ),
        cmd(
            "step.fill",
            "step.fill <channel> <every> [offset] [@pattern]",
            "Turn on every Nth step across the pattern.",
            fill,
        ),
        cmd(
            "step.shift",
            "step.shift <channel> <steps> [@pattern]",
            "Rotate a channel's notes by whole steps, wrapping around.",
            shift,
        ),
    ]
}

/// A small deterministic random generator (xorshift64*), so tools that use
/// randomness give the same result for the same seed.
struct Rng(u64);

impl Rng {
    fn new(seed: u64) -> Self {
        Self(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }

    fn next(&mut self) -> u64 {
        self.0 ^= self.0 >> 12;
        self.0 ^= self.0 << 25;
        self.0 ^= self.0 >> 27;
        self.0.wrapping_mul(0x2545_F491_4F6C_DD1D)
    }

    /// A whole number in `-range..=range`.
    fn spread(&mut self, range: u32) -> i64 {
        if range == 0 {
            return 0;
        }
        let span = u64::from(range) * 2 + 1;
        (self.next() % span) as i64 - i64::from(range)
    }
}

/// Notes grouped by start time, in time order.
fn by_start(notes: &[Note]) -> BTreeMap<u32, Vec<Note>> {
    let mut groups: BTreeMap<u32, Vec<Note>> = BTreeMap::new();
    for note in notes {
        groups.entry(note.start).or_default().push(*note);
    }
    for group in groups.values_mut() {
        group.sort_by_key(|n| n.key);
    }
    groups
}

/// Runs `f` over the target channel's notes and reports the result.
fn apply(
    project: &mut Project,
    args: &Args<'_>,
    what: &str,
    f: impl FnOnce(Vec<Note>) -> Vec<Note>,
) -> Result<Outcome, CommandError> {
    let target = Target::resolve(project, args)?;
    let (old, count) = rewrite(project, &target, f);
    Ok(Outcome::undoable(
        format!("{what}: {count} notes"),
        target.set_all_inverse(&old),
    ))
}

fn strum(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 3)?;
    let spread = args.duration(1, TICKS_PER_STEP)?;
    let down = match args.opt(2) {
        None | Some("up") => false,
        Some("down") => true,
        Some(other) => return Err(args.invalid(format!("`{other}` is not up or down"))),
    };
    apply(project, args, "strummed", |notes| {
        let mut out = Vec::new();
        for (_, mut group) in by_start(&notes) {
            if down {
                group.reverse();
            }
            for (i, n) in group.into_iter().enumerate() {
                let offset = spread * i as u32;
                let end = n.start + n.length;
                let start = n.start + offset;
                out.push(Note {
                    start,
                    length: end.saturating_sub(start).max(1),
                    ..n
                });
            }
        }
        out
    })
}

fn humanize(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 4)?;
    let timing = args.time(1, TICKS_PER_STEP)?;
    let velocity: u32 = args.parse(2, "velocity amount")?;
    let seed: u64 = match args.opt(3) {
        Some(_) => args.parse(3, "seed")?,
        None => 1,
    };
    let mut rng = Rng::new(seed);
    apply(project, args, "humanized", |mut notes| {
        notes.sort_by_key(|n| (n.start, n.key));
        notes
            .into_iter()
            .map(|n| {
                let start = (i64::from(n.start) + rng.spread(timing)).max(0);
                let vel = (i64::from(n.velocity) + rng.spread(velocity)).clamp(1, 127);
                Note {
                    start: u32::try_from(start).unwrap_or(n.start),
                    velocity: u8::try_from(vel).unwrap_or(n.velocity),
                    ..n
                }
            })
            .collect()
    })
}

fn legato(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    apply(project, args, "legato", |notes| {
        let starts: Vec<u32> = by_start(&notes).keys().copied().collect();
        notes
            .into_iter()
            .map(|n| match starts.iter().find(|s| **s > n.start) {
                Some(next) => Note {
                    length: next - n.start,
                    ..n
                },
                None => n,
            })
            .collect()
    })
}

fn chop(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let piece = args.duration(1, TICKS_PER_STEP)?.max(1);
    apply(project, args, "chopped", |notes| {
        let mut out = Vec::new();
        for n in notes {
            let mut start = n.start;
            let end = n.start + n.length;
            while start < end {
                out.push(Note {
                    start,
                    length: piece.min(end - start),
                    ..n
                });
                start += piece;
            }
        }
        out
    })
}

fn glue(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    apply(project, args, "glued", |mut notes| {
        notes.sort_by_key(|n| (n.key, n.start));
        let mut out: Vec<Note> = Vec::new();
        for n in notes {
            match out.last_mut() {
                Some(last) if last.key == n.key && last.start + last.length == n.start => {
                    last.length += n.length;
                }
                _ => out.push(n),
            }
        }
        out
    })
}

fn flip(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let target = Target::resolve(project, args)?;
    let length = project.patterns[target.pattern].length_ticks();
    match args.str(1)? {
        "time" => apply(project, args, "flipped in time", |notes| {
            notes
                .into_iter()
                .map(|n| Note {
                    start: length.saturating_sub(n.start + n.length),
                    ..n
                })
                .collect()
        }),
        "pitch" => apply(project, args, "flipped in pitch", |notes| {
            let low = notes.iter().map(|n| n.key).min().unwrap_or(60);
            let high = notes.iter().map(|n| n.key).max().unwrap_or(60);
            notes
                .into_iter()
                .map(|n| Note {
                    key: low + high - n.key,
                    ..n
                })
                .collect()
        }),
        other => Err(args.invalid(format!("`{other}` is not time or pitch"))),
    }
}

/// The scale named `name` (spaces or underscores).
fn scale(name: &str) -> Option<&'static [u8]> {
    let wanted = name.replace('_', " ");
    SCALES
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(&wanted))
        .map(|(_, d)| *d)
}

/// The pitch class of a root such as `C`, `F#` or `Bb` (or a key number).
fn pitch_class(text: &str) -> Option<u8> {
    text.parse::<u8>()
        .ok()
        .or_else(|| parse_key(&format!("{text}4")))
        .map(|k| k % 12)
}

/// The nearest key to `key` in the scale, ties going down.
fn nearest_in_scale(key: u8, root: u8, degrees: &[u8]) -> u8 {
    let fits = |k: i32| {
        (0..=127).contains(&k) && degrees.contains(&(((k - i32::from(root)).rem_euclid(12)) as u8))
    };
    let key = i32::from(key);
    for distance in 0..12 {
        if fits(key - distance) {
            return u8::try_from(key - distance).unwrap_or(0);
        }
        if fits(key + distance) {
            return u8::try_from(key + distance).unwrap_or(127);
        }
    }
    u8::try_from(key).unwrap_or(0)
}

fn to_scale(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let root = pitch_class(args.str(1)?).ok_or_else(|| {
        args.invalid(format!(
            "`{}` is not a root note",
            args.str(1).unwrap_or("")
        ))
    })?;
    let name = args.str(2)?;
    let degrees = scale(name).ok_or_else(|| {
        let names: Vec<_> = SCALES.iter().map(|(n, _)| n.replace(' ', "_")).collect();
        args.invalid(format!(
            "unknown scale `{name}`; choose from {}",
            names.join(", ")
        ))
    })?;
    apply(project, args, "moved to the scale", |notes| {
        notes
            .into_iter()
            .map(|n| Note {
                key: nearest_in_scale(n.key, root, degrees),
                ..n
            })
            .collect()
    })
}

fn arpeggiate(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 4)?;
    let step = args.duration(1, TICKS_PER_STEP)?.max(1);
    let mode = args.opt(2).unwrap_or("up");
    if !["up", "down", "updown", "random"].contains(&mode) {
        return Err(args.invalid(format!("`{mode}` is not up, down, updown or random")));
    }
    let seed: u64 = match args.opt(3) {
        Some(_) => args.parse(3, "seed")?,
        None => 1,
    };
    let mut rng = Rng::new(seed);
    apply(project, args, "arpeggiated", |notes| {
        let mut out = Vec::new();
        for (start, group) in by_start(&notes) {
            let span = group.iter().map(|n| n.length).max().unwrap_or(step);
            let mut order: Vec<Note> = group.clone();
            match mode {
                "down" => order.reverse(),
                "updown" if order.len() > 2 => {
                    let back: Vec<Note> = order[1..order.len() - 1].iter().rev().copied().collect();
                    order.extend(back);
                }
                _ => {}
            }
            let mut t = 0;
            let mut i = 0;
            while t < span {
                let n = if mode == "random" {
                    group[(rng.next() % group.len() as u64) as usize]
                } else {
                    order[i % order.len()]
                };
                out.push(Note {
                    start: start + t,
                    length: step.min(span - t),
                    ..n
                });
                t += step;
                i += 1;
            }
        }
        out
    })
}

fn slice(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    use musiccraft_project::params::inst;
    args.expect(2, 2)?;
    let slices: u32 = args.parse(1, "slice count")?;
    if !(1..=64).contains(&slices) {
        return Err(args.invalid("slice into 1 to 64 pieces"));
    }
    let target = Target::resolve(project, args)?;
    let channel = &mut project.channels[target.channel];
    if !matches!(
        channel.instrument,
        musiccraft_project::Instrument::Sampler { .. }
    ) {
        return Err(args.invalid(format!("channel {} is not a sampler", target.channel + 1)));
    }
    let previous = channel.params.get(inst::SLICES);
    channel.params.0[inst::SLICES] = slices as f32;
    let root = channel.params.get(inst::ROOT_KEY).round() as u32;
    let length = project.patterns[target.pattern].length_ticks();
    let piece = (length / slices).max(1);
    let (old, count) = rewrite(project, &target, |_| {
        (0..slices)
            .map(|i| Note {
                channel: musiccraft_project::ChannelId(0),
                start: i * piece,
                length: piece,
                key: u8::try_from((root + i).min(127)).unwrap_or(127),
                velocity: musiccraft_project::DEFAULT_VELOCITY,
                pan: 0,
                slide: false,
            })
            .collect()
    });
    let restore = Invocation::new(
        "channel.set_param",
        [
            (target.channel + 1).to_string(),
            "slices".to_owned(),
            format!("{previous}"),
        ],
    );
    let inverse = Invocation::new(
        BATCH,
        [
            restore.to_string(),
            target.set_all_inverse(&old).to_string(),
        ],
    );
    Ok(Outcome::undoable(
        format!("sliced into {count} pieces"),
        inverse,
    ))
}

fn fill(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 3)?;
    let every: u32 = args.parse(1, "step interval")?;
    if every == 0 {
        return Err(args.invalid("fill every 1 step or more"));
    }
    let offset: u32 = match args.opt(2) {
        Some(_) => args.parse(2, "offset")?,
        None => 0,
    };
    let target = Target::resolve(project, args)?;
    let steps = project.patterns[target.pattern].length_steps;
    apply(project, args, "filled", |mut notes| {
        let mut step = offset;
        while step < steps {
            let start = step * TICKS_PER_STEP;
            if !notes.iter().any(|n| n.start == start) {
                notes.push(Note {
                    slide: false,
                    channel: musiccraft_project::ChannelId(0),
                    start,
                    length: TICKS_PER_STEP,
                    key: musiccraft_project::STEP_KEY,
                    velocity: musiccraft_project::DEFAULT_VELOCITY,
                    pan: 0,
                });
            }
            step += every;
        }
        notes
    })
}

fn shift(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let steps: i64 = args.parse(1, "step count")?;
    let target = Target::resolve(project, args)?;
    let length = i64::from(project.patterns[target.pattern].length_steps.max(1) * TICKS_PER_STEP);
    let by = steps * i64::from(TICKS_PER_STEP);
    apply(project, args, "shifted", |notes| {
        notes
            .into_iter()
            .map(|n| Note {
                start: u32::try_from((i64::from(n.start) + by).rem_euclid(length)).unwrap_or(0),
                ..n
            })
            .collect()
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn nearest_scale_keys_and_roots() {
        let major = scale("major").unwrap();
        assert_eq!(nearest_in_scale(61, 0, major), 60, "C# goes down to C");
        assert_eq!(nearest_in_scale(66, 0, major), 65, "F# goes down to F");
        assert_eq!(nearest_in_scale(64, 0, major), 64);
        assert_eq!(pitch_class("F#"), Some(6));
        assert_eq!(pitch_class("62"), Some(2));
        assert!(scale("harmonic_minor").is_some());
    }

    #[test]
    fn rng_is_repeatable() {
        let a: Vec<i64> = {
            let mut r = Rng::new(7);
            (0..5).map(|_| r.spread(10)).collect()
        };
        let mut r = Rng::new(7);
        let b: Vec<i64> = (0..5).map(|_| r.spread(10)).collect();
        assert_eq!(a, b);
        assert!(a.iter().all(|v| (-10..=10).contains(v)));
    }
}
