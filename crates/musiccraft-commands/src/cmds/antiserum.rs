//! Antiserum channels (`docs/spec/antiserum-app.md`): adding one, its
//! parameters, wavetables, drawn LFO shapes, sequencer steps and presets.

use std::path::Path;

use musiccraft_antiserum::params::{LFOS, OSCS};
use musiccraft_antiserum_presets::{factory, find, step_data};
use musiccraft_patch::{CATEGORIES, Patch, PointData, STEPS, StepData, ZoneRef};
use musiccraft_project::{Channel, ChannelId, Instrument, Project};

use super::channel::{channel_arg, check_sample, push_channel};
use crate::args::{Args, num};
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "channel.add_antiserum",
            "channel.add_antiserum [preset]",
            "Add an Antiserum wavetable synth channel, with a factory preset or the init sound.",
            add,
        ),
        cmd(
            "antiserum.set_table",
            "antiserum.set_table <channel> <osc 1-3> <factory:name|data:base64>",
            "Replace an Antiserum oscillator's wavetable.",
            set_table,
        ),
        cmd(
            "antiserum.set_lfo_shape",
            "antiserum.set_lfo_shape <channel> <lfo 1-8> <x:y:curve,...|->",
            "Replace an Antiserum LFO's drawn shape (- returns to the default).",
            set_lfo_shape,
        ),
        cmd(
            "antiserum.set_step",
            "antiserum.set_step <channel> <step 1-64> <on|off> <transpose> <velocity> <gate>",
            "Edit one step of an Antiserum sequencer.",
            set_step,
        ),
        cmd(
            "antiserum.load_preset",
            "antiserum.load_preset <channel> <preset>",
            "Load an Antiserum factory preset.",
            load_preset,
        ),
        cmd(
            "antiserum.set_patch",
            "antiserum.set_patch <channel> <json>",
            "Replace an Antiserum channel's whole sound (used by undo).",
            set_patch,
        ),
        cmd(
            "antiserum.save_patch",
            "antiserum.save_patch <channel> <path>",
            "Save an Antiserum channel's sound as a patch file (.json).",
            save_patch,
        ),
        cmd(
            "antiserum.open_patch",
            "antiserum.open_patch <channel> <path>",
            "Load a patch file into an Antiserum channel.",
            open_patch,
        ),
        cmd(
            "antiserum.set_sample",
            "antiserum.set_sample <channel> <osc 1-3> <sample|->",
            "Set (or with -, clear) the sample an Antiserum oscillator plays as a sample, granular or spectral oscillator.",
            set_sample,
        ),
        cmd(
            "antiserum.add_zone",
            "antiserum.add_zone <channel> <osc 1-3> <sample> [low] [high] [root]",
            "Add a multisample zone to an Antiserum oscillator (keys 0-127, root 60 by default).",
            add_zone,
        ),
        cmd(
            "antiserum.set_zone",
            "antiserum.set_zone <channel> <osc 1-3> <zone> <low> <high> <root>",
            "Change a multisample zone's key range and root key.",
            set_zone,
        ),
        cmd(
            "antiserum.remove_zone",
            "antiserum.remove_zone <channel> <osc 1-3> <zone>",
            "Remove a multisample zone.",
            remove_zone,
        ),
        cmd(
            "antiserum.insert_zone",
            "antiserum.insert_zone <channel> <osc 1-3> <zone> <json>",
            "Insert a multisample zone from its JSON form (used by undo).",
            insert_zone,
        ),
        cmd(
            "antiserum.presets",
            "antiserum.presets [category]",
            "List Antiserum's factory presets (name, category, tags).",
            presets,
        ),
    ]
}

/// The index of the Antiserum channel numbered by argument `i`.
fn antiserum_arg(project: &Project, args: &Args<'_>, i: usize) -> Result<usize, CommandError> {
    let index = channel_arg(project, args, i)?;
    if project.channels[index].instrument.patch().is_none() {
        return Err(args.invalid(format!("channel {} is not an Antiserum", index + 1)));
    }
    Ok(index)
}

fn patch_mut<'p>(
    project: &'p mut Project,
    args: &Args<'_>,
    index: usize,
) -> Result<&'p mut Patch, CommandError> {
    project
        .channels
        .get_mut(index)
        .and_then(|c| c.instrument.patch_mut())
        .ok_or_else(|| args.invalid(format!("channel {} is not an Antiserum", index + 1)))
}

/// A 1-based number argument from 1 to `max`, as a 0-based index.
fn bounded(args: &Args<'_>, i: usize, what: &str, max: usize) -> Result<usize, CommandError> {
    let n = args.number(i, what)?;
    if n > max {
        return Err(args.invalid(format!("{what} must be 1 to {max}")));
    }
    Ok(n - 1)
}

fn add(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    let patch = if args.len() > 0 {
        let name = args.rest(0)?;
        find(&name)
            .cloned()
            .ok_or_else(|| args.invalid(format!("no Antiserum preset `{name}`")))?
    } else {
        Patch::default()
    };
    let name = if args.len() > 0 {
        patch.name.clone()
    } else {
        format!("Antiserum {}", project.channels.len() + 1)
    };
    let id = ChannelId(project.alloc_id());
    let instrument = Instrument::Antiserum {
        patch: Box::new(patch),
    };
    Ok(push_channel(project, Channel::new(id, name, instrument)))
}

/// `channel.set_param` on an Antiserum channel.
pub(super) fn set_param(
    project: &mut Project,
    args: &Args<'_>,
    index: usize,
) -> Result<Outcome, CommandError> {
    let name = args.str(1)?;
    let param = musiccraft_antiserum::index_of(name)
        .ok_or_else(|| args.invalid(format!("Antiserum has no parameter `{name}`")))?;
    let spec = musiccraft_antiserum::params()[param];
    let value = args.f32(2, name)?;
    if !spec.accepts(value) {
        return Err(musiccraft_project::ProjectError::out_of_range(
            name,
            f64::from(value),
            f64::from(spec.min),
            f64::from(spec.max),
        )
        .into());
    }
    let value = spec.sanitize(value);
    let patch = patch_mut(project, args, index)?;
    let previous = patch.params.get(name).copied().unwrap_or(spec.default);
    if value == spec.default {
        patch.params.remove(name);
    } else {
        patch.params.insert(name.to_owned(), value);
    }
    let message = format!("channel {} {name} {}", index + 1, num(value));
    if previous == value {
        return Ok(Outcome::done(message));
    }
    Ok(Outcome::undoable(
        message,
        Invocation::new(
            args.name(),
            [(index + 1).to_string(), name.to_owned(), num(previous)],
        ),
    ))
}

fn set_table(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = antiserum_arg(project, args, 0)?;
    let osc = bounded(args, 1, "oscillator", OSCS)?;
    let table = args.str(2)?;
    musiccraft_antiserum_presets::wavetable(table).map_err(|e| args.invalid(e))?;
    let patch = patch_mut(project, args, index)?;
    let previous = patch.table(osc).to_owned();
    patch.set_table(osc, table.to_owned());
    let shown = table.strip_prefix("factory:").unwrap_or("edited table");
    Ok(Outcome::undoable(
        format!("channel {} oscillator {} plays {shown}", index + 1, osc + 1),
        Invocation::new(
            args.name(),
            [(index + 1).to_string(), (osc + 1).to_string(), previous],
        ),
    ))
}

/// Parses `x:y:curve,…` (or `-` for none).
fn parse_points(args: &Args<'_>, text: &str) -> Result<Vec<PointData>, CommandError> {
    if text == "-" {
        return Ok(Vec::new());
    }
    text.split(',')
        .map(|point| {
            let values: Vec<f32> = point
                .split(':')
                .map(str::parse::<f32>)
                .collect::<Result<_, _>>()
                .map_err(|_| args.invalid(format!("`{point}` is not x:y:curve")))?;
            match values.as_slice() {
                [x, y, curve] if values.iter().all(|v| v.is_finite()) => Ok([*x, *y, *curve]),
                [x, y] if values.iter().all(|v| v.is_finite()) => Ok([*x, *y, 0.0]),
                _ => Err(args.invalid(format!("`{point}` is not x:y:curve"))),
            }
        })
        .collect()
}

/// Points as `x:y:curve,…` (or `-`).
fn points_text(points: &[PointData]) -> String {
    if points.is_empty() {
        return "-".to_owned();
    }
    points
        .iter()
        .map(|p| format!("{}:{}:{}", num(p[0]), num(p[1]), num(p[2])))
        .collect::<Vec<_>>()
        .join(",")
}

fn set_lfo_shape(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = antiserum_arg(project, args, 0)?;
    let lfo = bounded(args, 1, "LFO", LFOS)?;
    let points = parse_points(args, args.str(2)?)?;
    if points.len() > musiccraft_antiserum::modulation::MAX_POINTS {
        return Err(args.invalid(format!(
            "a shape has at most {} points",
            musiccraft_antiserum::modulation::MAX_POINTS
        )));
    }
    let patch = patch_mut(project, args, index)?;
    let previous = points_text(patch.lfo_shape(lfo).unwrap_or(&[]));
    let count = points.len();
    patch.set_lfo_shape(lfo, points);
    Ok(Outcome::undoable(
        format!(
            "channel {} LFO {} shape: {count} points",
            index + 1,
            lfo + 1
        ),
        Invocation::new(
            args.name(),
            [(index + 1).to_string(), (lfo + 1).to_string(), previous],
        ),
    ))
}

fn default_step() -> StepData {
    step_data(musiccraft_antiserum::Step::default())
}

fn set_step(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(6, 6)?;
    let index = antiserum_arg(project, args, 0)?;
    let step = bounded(args, 1, "step", STEPS)?;
    let on = args.flag(2)?;
    let transpose = args.f32(3, "transpose")?;
    if !(-24.0..=24.0).contains(&transpose) {
        return Err(args.invalid("transpose must be -24 to 24 semitones"));
    }
    let velocity = args.f32(4, "velocity")?;
    let gate = args.f32(5, "gate")?;
    if !(0.0..=1.0).contains(&velocity) || !(0.05..=1.0).contains(&gate) {
        return Err(args.invalid("velocity is 0 to 1 and gate 0.05 to 1"));
    }
    let data = (on, transpose.round() as i8, velocity, gate);
    let patch = patch_mut(project, args, index)?;
    let previous = patch.step(step).unwrap_or_else(default_step);
    patch.set_step(step, data, default_step());
    let flag = |b: bool| if b { "on" } else { "off" }.to_owned();
    Ok(Outcome::undoable(
        format!("channel {} step {}", index + 1, step + 1),
        Invocation::new(
            args.name(),
            [
                (index + 1).to_string(),
                (step + 1).to_string(),
                flag(previous.0),
                previous.1.to_string(),
                num(previous.2),
                num(previous.3),
            ],
        ),
    ))
}

/// Replaces the patch of channel `index`; the undo restores the old one.
fn replace(
    project: &mut Project,
    args: &Args<'_>,
    index: usize,
    patch: Patch,
    message: String,
) -> Result<Outcome, CommandError> {
    let slot = patch_mut(project, args, index)?;
    let previous = std::mem::replace(slot, patch);
    let json = serde_json::to_string(&previous).map_err(|e| args.invalid(e.to_string()))?;
    Ok(Outcome::undoable(
        message,
        Invocation::new("antiserum.set_patch", [(index + 1).to_string(), json]),
    ))
}

fn load_preset(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, usize::MAX)?;
    let index = antiserum_arg(project, args, 0)?;
    let name = args.rest(1)?;
    let preset = find(&name)
        .cloned()
        .ok_or_else(|| args.invalid(format!("no Antiserum preset `{name}`")))?;
    let message = format!("channel {} loaded {}", index + 1, preset.name);
    replace(project, args, index, preset, message)
}

fn set_patch(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = antiserum_arg(project, args, 0)?;
    let patch = Patch::from_json(args.str(1)?).map_err(|e| args.invalid(e))?;
    let message = format!("channel {} plays {}", index + 1, patch.name);
    replace(project, args, index, patch, message)
}

fn save_patch(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = antiserum_arg(project, args, 0)?;
    let path = args.str(1)?;
    let patch = project.channels[index]
        .instrument
        .patch()
        .cloned()
        .unwrap_or_default();
    let json = patch.to_json().map_err(|e| args.invalid(e))?;
    std::fs::write(Path::new(path), json)
        .map_err(|e| args.invalid(format!("cannot write {path}: {e}")))?;
    Ok(Outcome::done(format!("saved {} to {path}", patch.name)))
}

fn open_patch(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = antiserum_arg(project, args, 0)?;
    let path = args.str(1)?;
    let text = std::fs::read_to_string(path)
        .map_err(|e| args.invalid(format!("cannot read {path}: {e}")))?;
    let mut patch = Patch::from_json(&text).map_err(|e| args.invalid(e))?;
    if patch.name.is_empty() {
        patch.name = Path::new(path)
            .file_stem()
            .map_or_else(String::new, |s| s.to_string_lossy().into_owned());
    }
    let message = format!("channel {} plays {}", index + 1, patch.name);
    replace(project, args, index, patch, message)
}

fn set_sample(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = antiserum_arg(project, args, 0)?;
    let osc = bounded(args, 1, "oscillator", OSCS)?;
    let sample = args.str(2)?;
    let sample = if sample == "-" {
        String::new()
    } else {
        check_sample(project, args, sample)?;
        sample.to_owned()
    };
    let patch = patch_mut(project, args, index)?;
    let mut source = patch.source(osc);
    let previous = std::mem::replace(&mut source.sample, sample.clone());
    patch.set_source(osc, source);
    let shown = if sample.is_empty() {
        "no sample"
    } else {
        &sample
    };
    let undo = if previous.is_empty() {
        "-".to_owned()
    } else {
        previous
    };
    Ok(Outcome::undoable(
        format!("channel {} oscillator {} plays {shown}", index + 1, osc + 1),
        Invocation::new(
            args.name(),
            [(index + 1).to_string(), (osc + 1).to_string(), undo],
        ),
    ))
}

/// A key argument (0–127, or a note name) at `i`, or `default` when absent.
fn key_or(args: &Args<'_>, i: usize, default: u8) -> Result<u8, CommandError> {
    match args.opt(i) {
        Some(_) => args.key(i),
        None => Ok(default),
    }
}

fn zone_arg(args: &Args<'_>, i: usize, zones: usize) -> Result<usize, CommandError> {
    let zone = args.number(i, "zone")?;
    if zone > zones {
        return Err(args.invalid(format!(
            "there is no zone {zone} (the oscillator has {zones})"
        )));
    }
    Ok(zone - 1)
}

fn add_zone(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 6)?;
    let index = antiserum_arg(project, args, 0)?;
    let osc = bounded(args, 1, "oscillator", OSCS)?;
    let sample = args.str(2)?;
    check_sample(project, args, sample)?;
    let (low, high, root) = (
        key_or(args, 3, 0)?,
        key_or(args, 4, 127)?,
        key_or(args, 5, 60)?,
    );
    if low > high {
        return Err(args.invalid("the low key is above the high key"));
    }
    let patch = patch_mut(project, args, index)?;
    let mut source = patch.source(osc);
    source
        .zones
        .push(ZoneRef::new(sample.to_owned(), low, high, root));
    let zone = source.zones.len();
    patch.set_source(osc, source);
    Ok(Outcome::undoable(
        format!(
            "channel {} oscillator {} zone {zone}: {sample}",
            index + 1,
            osc + 1
        ),
        Invocation::new(
            "antiserum.remove_zone",
            [
                (index + 1).to_string(),
                (osc + 1).to_string(),
                zone.to_string(),
            ],
        ),
    ))
}

fn set_zone(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(6, 6)?;
    let index = antiserum_arg(project, args, 0)?;
    let osc = bounded(args, 1, "oscillator", OSCS)?;
    let (low, high, root) = (args.key(3)?, args.key(4)?, args.key(5)?);
    if low > high {
        return Err(args.invalid("the low key is above the high key"));
    }
    let patch = patch_mut(project, args, index)?;
    let mut source = patch.source(osc);
    let zone = zone_arg(args, 2, source.zones.len())?;
    let Some(z) = source.zones.get_mut(zone) else {
        return Err(args.invalid("no such zone"));
    };
    let previous = (z.low, z.high, z.root);
    (z.low, z.high, z.root) = (low, high, root);
    patch.set_source(osc, source);
    Ok(Outcome::undoable(
        format!(
            "channel {} oscillator {} zone {}: {low}-{high}, root {root}",
            index + 1,
            osc + 1,
            zone + 1
        ),
        Invocation::new(
            args.name(),
            [
                (index + 1).to_string(),
                (osc + 1).to_string(),
                (zone + 1).to_string(),
                previous.0.to_string(),
                previous.1.to_string(),
                previous.2.to_string(),
            ],
        ),
    ))
}

fn remove_zone(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = antiserum_arg(project, args, 0)?;
    let osc = bounded(args, 1, "oscillator", OSCS)?;
    let patch = patch_mut(project, args, index)?;
    let mut source = patch.source(osc);
    let zone = zone_arg(args, 2, source.zones.len())?;
    let removed = source.zones.remove(zone);
    patch.set_source(osc, source);
    let json = serde_json::to_string(&removed).map_err(|e| args.invalid(e.to_string()))?;
    Ok(Outcome::undoable(
        format!(
            "channel {} oscillator {} removed zone {}",
            index + 1,
            osc + 1,
            zone + 1
        ),
        Invocation::new(
            "antiserum.insert_zone",
            [
                (index + 1).to_string(),
                (osc + 1).to_string(),
                (zone + 1).to_string(),
                json,
            ],
        ),
    ))
}

fn insert_zone(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 4)?;
    let index = antiserum_arg(project, args, 0)?;
    let osc = bounded(args, 1, "oscillator", OSCS)?;
    let zone: ZoneRef =
        serde_json::from_str(args.str(3)?).map_err(|e| args.invalid(e.to_string()))?;
    let patch = patch_mut(project, args, index)?;
    let mut source = patch.source(osc);
    let at = args.number(2, "zone")? - 1;
    if at > source.zones.len() {
        return Err(args.invalid(format!("zone {} is past the end", at + 1)));
    }
    source.zones.insert(at, zone);
    patch.set_source(osc, source);
    Ok(Outcome::undoable(
        format!(
            "channel {} oscillator {} zone {} restored",
            index + 1,
            osc + 1,
            at + 1
        ),
        Invocation::new(
            "antiserum.remove_zone",
            [
                (index + 1).to_string(),
                (osc + 1).to_string(),
                (at + 1).to_string(),
            ],
        ),
    ))
}

fn presets(_project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 1)?;
    let category = args.opt(0);
    if let Some(c) = category
        && !CATEGORIES.contains(&c)
    {
        return Err(args.invalid(format!(
            "unknown category `{c}`; choose from {}",
            CATEGORIES.join(", ")
        )));
    }
    let lines: Vec<String> = factory()
        .iter()
        .filter(|p| category.is_none_or(|c| p.category == c))
        .map(|p| format!("{}\t{}\t{}", p.name, p.category, p.tags.join(" ")))
        .collect();
    Ok(Outcome::done(lines.join("\n")))
}
