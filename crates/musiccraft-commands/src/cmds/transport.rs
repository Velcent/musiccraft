use musiccraft_project::{PlayMode, Project, Transport};

use crate::args::Args;
use crate::command::cmd;
use crate::{Command, CommandError, Outcome};

pub(super) fn commands() -> Vec<Box<dyn Command>> {
    vec![
        cmd("transport.play", "transport.play", "Start playback.", play),
        cmd(
            "transport.stop",
            "transport.stop",
            "Stop playback and return to the start.",
            stop,
        ),
        cmd(
            "transport.toggle",
            "transport.toggle",
            "Play if stopped, stop if playing.",
            toggle,
        ),
        cmd(
            "transport.set_mode",
            "transport.set_mode <pattern|song>",
            "Choose whether playback loops the selected pattern or plays the song.",
            set_mode,
        ),
        cmd(
            "transport.toggle_metronome",
            "transport.toggle_metronome",
            "Turn the metronome click on or off.",
            toggle_metronome,
        ),
        cmd(
            "transport.toggle_mode",
            "transport.toggle_mode",
            "Switch between pattern and song playback.",
            toggle_mode,
        ),
    ]
}

fn play(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    let previous = project.set_transport(Transport::Playing);
    Ok(Outcome::done(if previous.is_playing() {
        "already playing"
    } else {
        "playing"
    }))
}

fn stop(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    let previous = project.set_transport(Transport::Stopped);
    Ok(Outcome::done(if previous.is_playing() {
        "stopped"
    } else {
        "already stopped"
    }))
}

fn toggle(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    if project.transport().is_playing() {
        project.set_transport(Transport::Stopped);
        Ok(Outcome::done("stopped"))
    } else {
        project.set_transport(Transport::Playing);
        Ok(Outcome::done("playing"))
    }
}

fn set_mode(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(1, 1)?;
    project.play_mode = match args.str(0)? {
        "pattern" => PlayMode::Pattern,
        "song" => PlayMode::Song,
        other => return Err(args.invalid(format!("`{other}` is not pattern or song"))),
    };
    Ok(Outcome::done(format!("{} mode", project.play_mode.label())))
}

fn toggle_mode(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    project.play_mode = match project.play_mode {
        PlayMode::Pattern => PlayMode::Song,
        PlayMode::Song => PlayMode::Pattern,
    };
    Ok(Outcome::done(format!("{} mode", project.play_mode.label())))
}

fn toggle_metronome(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    project.metronome = !project.metronome;
    Ok(Outcome::done(if project.metronome {
        "metronome on"
    } else {
        "metronome off"
    }))
}
