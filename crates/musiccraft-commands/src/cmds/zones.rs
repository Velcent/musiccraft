//! The zone sampler: `channel.add_zones` and the commands that edit its
//! zones (see `docs/spec/channel-rack.md`).

use musiccraft_project::{Channel, ChannelId, Instrument, Project, SampleZone};

use super::channel::{channel_arg, check_sample, name_or, push_channel};
use crate::args::Args;
use crate::command::cmd;
use crate::{Command, CommandError, Invocation, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "channel.add_zones",
            "channel.add_zones [name]",
            "Add a sampler that maps several samples to key and velocity zones.",
            add_zones,
        ),
        cmd(
            "channel.add_zone",
            "channel.add_zone <channel> <sample> [low] [high] [root]",
            "Add a zone to a zone sampler (defaults: every key, root 60).",
            add_zone,
        ),
        cmd(
            "channel.remove_zone",
            "channel.remove_zone <channel> <zone>",
            "Remove a zone.",
            remove_zone,
        ),
        cmd(
            "channel.insert_zone",
            "channel.insert_zone <channel> <zone> <json>",
            "Insert a zone from its JSON form (used by undo).",
            insert_zone,
        ),
        cmd(
            "channel.set_zone",
            "channel.set_zone <channel> <zone> <low|high|root|vel_low|vel_high|sample> <value>",
            "Change a zone's key range, root key, velocity range or sample.",
            set_zone,
        ),
    ]
}

fn add_zones(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    let name = name_or(args, 0, "Zones")?;
    let id = ChannelId(project.alloc_id());
    Ok(push_channel(
        project,
        Channel::new(id, name, Instrument::Zones { zones: Vec::new() }),
    ))
}

/// The zones of channel `index`, or an error if it is not a zone sampler.
fn zones_mut<'a>(
    project: &'a mut Project,
    args: &Args<'_>,
    index: usize,
) -> Result<&'a mut Vec<SampleZone>, CommandError> {
    match &mut project.channels[index].instrument {
        Instrument::Zones { zones } => Ok(zones),
        _ => Err(args.invalid(format!("channel {} is not a zone sampler", index + 1))),
    }
}

fn key_arg(args: &Args<'_>, i: usize, what: &str) -> Result<u8, CommandError> {
    let key: u8 = args.parse(i, what)?;
    if key > 127 {
        return Err(args.invalid(format!("{what} must be 0–127")));
    }
    Ok(key)
}

fn add_zone(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 5)?;
    let index = channel_arg(project, args, 0)?;
    let sample = args.str(1)?;
    check_sample(project, args, sample)?;
    let mut zone = SampleZone::new(sample);
    if args.len() > 2 {
        zone.low = key_arg(args, 2, "low key")?;
    }
    if args.len() > 3 {
        zone.high = key_arg(args, 3, "high key")?;
    }
    if args.len() > 4 {
        zone.root = key_arg(args, 4, "root key")?;
    }
    if zone.low > zone.high {
        return Err(args.invalid("the low key is above the high key"));
    }
    let zones = zones_mut(project, args, index)?;
    zones.push(zone);
    let number = zones.len();
    Ok(Outcome::undoable(
        format!("channel {} zone {number} added", index + 1),
        Invocation::new(
            "channel.remove_zone",
            [(index + 1).to_string(), number.to_string()],
        ),
    ))
}

fn zone_arg(zones: &[SampleZone], args: &Args<'_>, i: usize) -> Result<usize, CommandError> {
    let number = args.number(i, "zone")?;
    if number > zones.len() {
        return Err(args.invalid(format!("there is no zone {number}")));
    }
    Ok(number - 1)
}

fn remove_zone(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let index = channel_arg(project, args, 0)?;
    let zones = zones_mut(project, args, index)?;
    let zone = zone_arg(zones, args, 1)?;
    let removed = zones.remove(zone);
    let json = serde_json::to_string(&removed).map_err(|e| args.invalid(e.to_string()))?;
    Ok(Outcome::undoable(
        format!("channel {} zone {} removed", index + 1, zone + 1),
        Invocation::new(
            "channel.insert_zone",
            [(index + 1).to_string(), (zone + 1).to_string(), json],
        ),
    ))
}

fn insert_zone(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(3, 3)?;
    let index = channel_arg(project, args, 0)?;
    let position = args.number(1, "zone")?;
    let zone: SampleZone =
        serde_json::from_str(args.str(2)?).map_err(|e| args.invalid(e.to_string()))?;
    let zones = zones_mut(project, args, index)?;
    if position > zones.len() + 1 {
        return Err(args.invalid(format!("zone {position} is past the end")));
    }
    zones.insert(position - 1, zone);
    Ok(Outcome::undoable(
        format!("channel {} zone {position} inserted", index + 1),
        Invocation::new(
            "channel.remove_zone",
            [(index + 1).to_string(), position.to_string()],
        ),
    ))
}

fn set_zone(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(4, 4)?;
    let index = channel_arg(project, args, 0)?;
    let field = args.str(2)?;
    if field == "sample" {
        check_sample(project, args, args.str(3)?)?;
    }
    let zones = zones_mut(project, args, index)?;
    let zone = zone_arg(zones, args, 1)?;
    let before = zones[zone].clone();
    let mut after = before.clone();
    let velocity = |args: &Args<'_>| -> Result<u8, CommandError> {
        let v: u8 = args.parse(3, "velocity")?;
        if !(1..=127).contains(&v) {
            return Err(args.invalid("velocities are 1–127"));
        }
        Ok(v)
    };
    match field {
        "low" => after.low = key_arg(args, 3, "low key")?,
        "high" => after.high = key_arg(args, 3, "high key")?,
        "root" => after.root = key_arg(args, 3, "root key")?,
        "vel_low" => after.vel_low = velocity(args)?,
        "vel_high" => after.vel_high = velocity(args)?,
        "sample" => after.sample = args.str(3)?.to_owned(),
        other => {
            return Err(args.invalid(format!(
                "zones have no `{other}`; choose low, high, root, vel_low, vel_high or sample"
            )));
        }
    }
    if after.low > after.high || after.vel_low > after.vel_high {
        return Err(args.invalid("a zone's low end must not be above its high end"));
    }
    let previous = match field {
        "low" => before.low.to_string(),
        "high" => before.high.to_string(),
        "root" => before.root.to_string(),
        "vel_low" => before.vel_low.to_string(),
        "vel_high" => before.vel_high.to_string(),
        _ => before.sample.clone(),
    };
    zones[zone] = after;
    Ok(Outcome::undoable(
        format!("channel {} zone {} {field} set", index + 1, zone + 1),
        Invocation::new(
            args.name(),
            [
                (index + 1).to_string(),
                (zone + 1).to_string(),
                field.to_owned(),
                previous,
            ],
        ),
    ))
}
