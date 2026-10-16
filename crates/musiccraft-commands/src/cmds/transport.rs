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
            "transport.pause",
            "transport.pause",
            "Halt playback and keep the playhead where it is.",
            pause,
        ),
        cmd(
            "transport.play_pause",
            "transport.play_pause",
            "Pause if playing, otherwise play from the playhead.",
            play_pause,
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
            "transport.set_loop",
            "transport.set_loop <start bar> <end bar>",
            "Loop a range of the song in song mode (bars from 0; <n>t for ticks).",
            set_loop,
        ),
        cmd(
            "transport.clear_loop",
            "transport.clear_loop",
            "Play the whole song again.",
            clear_loop,
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
    Ok(Outcome::done(if previous != Transport::Stopped {
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

fn pause(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    if project.transport().is_playing() {
        project.set_transport(Transport::Paused);
        Ok(Outcome::done("paused"))
    } else {
        Ok(Outcome::done("not playing"))
    }
}

fn play_pause(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    if project.transport().is_playing() {
        project.set_transport(Transport::Paused);
        Ok(Outcome::done("paused"))
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

fn loop_inverse(previous: Option<(u32, u32)>) -> crate::Invocation {
    match previous {
        Some((start, end)) => crate::Invocation::new(
            "transport.set_loop",
            [format!("{start}t"), format!("{end}t")],
        ),
        None => crate::Invocation::bare("transport.clear_loop"),
    }
}

fn set_loop(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(2, 2)?;
    let bar = musiccraft_project::params::TICKS_PER_BAR;
    let start = args.time(0, bar)?;
    let end = args.time(1, bar)?;
    if end <= start {
        return Err(args.invalid("the loop must end after it starts"));
    }
    let previous = project.playlist.loop_range.replace((start, end));
    let message = format!("loop {start}t to {end}t");
    if previous == Some((start, end)) {
        return Ok(Outcome::done(message));
    }
    Ok(Outcome::undoable(message, loop_inverse(previous)))
}

fn clear_loop(project: &mut Project, args: &Args<'_>) -> Result<Outcome, CommandError> {
    args.expect(0, 0)?;
    match project.playlist.loop_range.take() {
        Some(previous) => Ok(Outcome::undoable(
            "loop cleared",
            loop_inverse(Some(previous)),
        )),
        None => Ok(Outcome::done("no loop set")),
    }
}
