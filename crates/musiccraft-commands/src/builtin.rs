use musiccraft_project::{Project, Transport};

use crate::{Command, CommandError, Invocation, Outcome};

/// All commands registered by default.
pub(crate) fn all() -> Vec<Box<dyn Command>> {
    vec![
        Box::new(TransportPlay),
        Box::new(TransportStop),
        Box::new(TransportToggle),
        Box::new(SetTempo),
    ]
}

fn usage_error(command: &dyn Command) -> CommandError {
    CommandError::Usage {
        command: command.name().to_owned(),
        usage: command.usage().to_owned(),
    }
}

fn expect_no_args(command: &dyn Command, args: &[String]) -> Result<(), CommandError> {
    if args.is_empty() {
        Ok(())
    } else {
        Err(usage_error(command))
    }
}

struct TransportPlay;

impl Command for TransportPlay {
    fn name(&self) -> &'static str {
        "transport.play"
    }

    fn summary(&self) -> &'static str {
        "Start playback."
    }

    fn usage(&self) -> &'static str {
        "transport.play"
    }

    fn execute(&self, project: &mut Project, args: &[String]) -> Result<Outcome, CommandError> {
        expect_no_args(self, args)?;
        let previous = project.set_transport(Transport::Playing);
        Ok(Outcome::done(if previous.is_playing() {
            "already playing"
        } else {
            "playing"
        }))
    }
}

struct TransportStop;

impl Command for TransportStop {
    fn name(&self) -> &'static str {
        "transport.stop"
    }

    fn summary(&self) -> &'static str {
        "Stop playback."
    }

    fn usage(&self) -> &'static str {
        "transport.stop"
    }

    fn execute(&self, project: &mut Project, args: &[String]) -> Result<Outcome, CommandError> {
        expect_no_args(self, args)?;
        let previous = project.set_transport(Transport::Stopped);
        Ok(Outcome::done(if previous.is_playing() {
            "stopped"
        } else {
            "already stopped"
        }))
    }
}

struct TransportToggle;

impl Command for TransportToggle {
    fn name(&self) -> &'static str {
        "transport.toggle"
    }

    fn summary(&self) -> &'static str {
        "Play if stopped, stop if playing."
    }

    fn usage(&self) -> &'static str {
        "transport.toggle"
    }

    fn execute(&self, project: &mut Project, args: &[String]) -> Result<Outcome, CommandError> {
        expect_no_args(self, args)?;
        if project.transport().is_playing() {
            project.set_transport(Transport::Stopped);
            Ok(Outcome::done("stopped"))
        } else {
            project.set_transport(Transport::Playing);
            Ok(Outcome::done("playing"))
        }
    }
}

struct SetTempo;

impl Command for SetTempo {
    fn name(&self) -> &'static str {
        "project.set_tempo"
    }

    fn summary(&self) -> &'static str {
        "Set the project tempo in beats per minute (undoable)."
    }

    fn usage(&self) -> &'static str {
        "project.set_tempo <bpm>"
    }

    fn execute(&self, project: &mut Project, args: &[String]) -> Result<Outcome, CommandError> {
        let [raw] = args else {
            return Err(usage_error(self));
        };
        let bpm: f64 = raw.parse().map_err(|_| CommandError::InvalidArgument {
            command: self.name().to_owned(),
            reason: format!("`{raw}` is not a number"),
        })?;
        let previous = project.set_tempo_bpm(bpm)?;
        let message = format!("tempo {bpm} BPM");
        if (previous - bpm).abs() < f64::EPSILON {
            Ok(Outcome::done(message))
        } else {
            Ok(Outcome::undoable(
                message,
                Invocation::new(self.name(), [previous.to_string()]),
            ))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }

    #[test]
    fn command_names_are_unique() {
        let commands = all();
        let mut names: Vec<_> = commands.iter().map(|c| c.name()).collect();
        names.sort_unstable();
        let total = names.len();
        names.dedup();
        assert_eq!(names.len(), total);
    }

    #[test]
    fn play_and_stop_change_transport() {
        let mut project = Project::default();
        let out = TransportPlay.execute(&mut project, &[]).unwrap();
        assert_eq!(out.message, "playing");
        assert!(project.transport().is_playing());
        let out = TransportPlay.execute(&mut project, &[]).unwrap();
        assert_eq!(out.message, "already playing");
        let out = TransportStop.execute(&mut project, &[]).unwrap();
        assert_eq!(out.message, "stopped");
        let out = TransportStop.execute(&mut project, &[]).unwrap();
        assert_eq!(out.message, "already stopped");
        assert!(out.inverse.is_none());
    }

    #[test]
    fn toggle_flips_state() {
        let mut project = Project::default();
        TransportToggle.execute(&mut project, &[]).unwrap();
        assert!(project.transport().is_playing());
        TransportToggle.execute(&mut project, &[]).unwrap();
        assert!(!project.transport().is_playing());
    }

    #[test]
    fn transport_commands_reject_arguments() {
        let mut project = Project::default();
        let err = TransportPlay
            .execute(&mut project, &args(&["now"]))
            .unwrap_err();
        assert!(matches!(err, CommandError::Usage { .. }));
        assert!(!project.transport().is_playing());
    }

    #[test]
    fn set_tempo_changes_tempo_and_returns_inverse() {
        let mut project = Project::default();
        let out = SetTempo.execute(&mut project, &args(&["140"])).unwrap();
        assert!((project.tempo_bpm() - 140.0).abs() < f64::EPSILON);
        assert_eq!(
            out.inverse,
            Some(Invocation::new("project.set_tempo", ["130"]))
        );
    }

    #[test]
    fn set_tempo_to_same_value_records_no_inverse() {
        let mut project = Project::default();
        let out = SetTempo.execute(&mut project, &args(&["130"])).unwrap();
        assert!(out.inverse.is_none());
    }

    #[test]
    fn set_tempo_rejects_bad_input_without_changing_project() {
        let mut project = Project::default();
        for bad in [
            &[][..],
            &["1", "2"][..],
            &["fast"][..],
            &["5"][..],
            &["NaN"][..],
        ] {
            let result = SetTempo.execute(&mut project, &args(bad));
            assert!(result.is_err(), "{bad:?} should fail");
        }
        assert!((project.tempo_bpm() - 130.0).abs() < f64::EPSILON);
    }
}
