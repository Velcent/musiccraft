//! Recording: arming, options and step edit. None of these are undoable.

use musiccraft_project::{Project, RecordMode};

use crate::args::Args;
use crate::command::cmd;
use crate::{Command, CommandError, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd(
            "record.toggle",
            "record.toggle",
            "Arm or disarm recording.",
            toggle,
        ),
        cmd(
            "record.set",
            "record.set <on|off>",
            "Arm or disarm recording.",
            set,
        ),
        cmd(
            "record.toggle_count_in",
            "record.toggle_count_in",
            "Click one bar before playback starts while recording is armed.",
            toggle_count_in,
        ),
        cmd(
            "record.toggle_wait",
            "record.toggle_wait",
            "Start playback with the first live note while recording is armed.",
            toggle_wait,
        ),
        cmd(
            "record.set_mode",
            "record.set_mode <blend|replace>",
            "Add recorded notes to the pattern, or replace the notes the playhead passes.",
            set_mode,
        ),
        cmd(
            "record.toggle_loop",
            "record.toggle_loop",
            "Keep looping while recording, or stop at the loop end.",
            toggle_loop,
        ),
        cmd(
            "record.toggle_step_edit",
            "record.toggle_step_edit",
            "Enter played notes at the playhead and step on (while stopped).",
            toggle_step_edit,
        ),
    ]
}

fn on_off(on: bool) -> &'static str {
    if on { "on" } else { "off" }
}

fn toggle(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    let armed = !project.is_armed();
    project.set_armed(armed);
    Ok(Outcome::done(format!("recording {}", armed_label(armed))))
}

fn set(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    let armed = args.flag(0)?;
    project.set_armed(armed);
    Ok(Outcome::done(format!("recording {}", armed_label(armed))))
}

fn armed_label(armed: bool) -> &'static str {
    if armed { "armed" } else { "disarmed" }
}

fn toggle_count_in(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    project.record.count_in = !project.record.count_in;
    Ok(Outcome::done(format!(
        "count-in {}",
        on_off(project.record.count_in)
    )))
}

fn toggle_wait(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    project.record.wait_for_input = !project.record.wait_for_input;
    Ok(Outcome::done(format!(
        "wait for input {}",
        on_off(project.record.wait_for_input)
    )))
}

fn set_mode(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    project.record.mode = match args.str(0)? {
        "blend" => RecordMode::Blend,
        "replace" => RecordMode::Replace,
        other => return Err(args.invalid(format!("`{other}` is not blend or replace"))),
    };
    Ok(Outcome::done(format!(
        "{} recording",
        project.record.mode.label()
    )))
}

fn toggle_loop(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    project.record.loop_record = !project.record.loop_record;
    Ok(Outcome::done(format!(
        "loop recording {}",
        on_off(project.record.loop_record)
    )))
}

fn toggle_step_edit(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    let on = !project.step_edit();
    project.set_step_edit(on);
    Ok(Outcome::done(format!("step edit {}", on_off(on))))
}
