use std::io::{self, Read, Write};
use std::process::ExitCode;
use std::time::{Duration, Instant};

use musiccraft_audio::OutputDevice;
use musiccraft_cli::{list_commands, new_session, run_script};
use musiccraft_commands::Session;
use musiccraft_engine::{ChannelParams, EngineMessage, Graph, Instrument, engine_pair};
use musiccraft_project::Transport;
use musiccraft_render::Bridge;

const USAGE: &str = "\
usage:
  musiccraft-cli list                         list available commands
  musiccraft-cli run <command> [args]         run one command
  musiccraft-cli script <file|->              run one command per line (# comments allowed)
  musiccraft-cli dump <file|->                run a script, then print the project as JSON
  musiccraft-cli play <file|-> [seconds]      run a script, then play it on the sound card
  musiccraft-cli tone [seconds]               play a test tone (A4) on the sound card

Each invocation starts from a fresh project; use a script for multi-step work.
Export audio from a script with `project.export_wav <path>`.";

fn read_script(path: &str) -> io::Result<String> {
    if path == "-" {
        let mut text = String::new();
        io::stdin().read_to_string(&mut text)?;
        Ok(text)
    } else {
        std::fs::read_to_string(path)
    }
}

fn parse_seconds(arg: Option<&String>, default: f64) -> Result<f64, String> {
    match arg {
        None => Ok(default),
        Some(raw) => raw
            .parse::<f64>()
            .ok()
            .filter(|s| s.is_finite() && *s > 0.0 && *s <= 3_600.0)
            .ok_or_else(|| format!("`{raw}` is not a duration in seconds")),
    }
}

/// Plays the session's project through the default output device.
fn play(session: &Session, seconds: f64, out: &mut impl Write) -> Result<(), String> {
    let device = OutputDevice::default_device().map_err(|e| e.to_string())?;
    let (engine, mut handle) = engine_pair(device.sample_rate());
    let mut bridge = Bridge::new(device.sample_rate());
    let mut project = session.project().clone();
    project.set_transport(Transport::Playing);
    let report = bridge.sync(&project, &mut handle);
    for warning in &report.warnings {
        let _ = writeln!(out, "warning: {warning}");
    }
    let output = device.start(engine).map_err(|e| e.to_string())?;
    let _ = writeln!(
        out,
        "playing on {} ({} Hz) for {seconds} s",
        output.name(),
        output.sample_rate()
    );
    let start = Instant::now();
    while start.elapsed().as_secs_f64() < seconds {
        std::thread::sleep(Duration::from_millis(50));
        handle.collect_garbage();
    }
    let meters = handle.meters();
    let _ = writeln!(
        out,
        "done at beat {:.2}, {} stream errors",
        meters.position_ticks() / 96.0,
        output.error_count()
    );
    Ok(())
}

/// Plays a sine-ish synth tone: the M1 smoke test for the audio path.
fn tone(seconds: f64, out: &mut impl Write) -> Result<(), String> {
    let device = OutputDevice::default_device().map_err(|e| e.to_string())?;
    let (engine, mut handle) = engine_pair(device.sample_rate());
    let mut graph = Graph::new();
    let mut params = ChannelParams::default();
    params.instrument[musiccraft_project::params::inst::WAVE1] = 0.0;
    params.instrument[musiccraft_project::params::inst::OSC_MIX] = 0.0;
    graph.add_channel(Instrument::Synth, params);
    let messages = [
        EngineMessage::SetGraph(Box::new(graph)),
        EngineMessage::NoteOn {
            channel: 0,
            key: 69,
            velocity: 0.8,
        },
    ];
    for message in messages {
        handle.send(message).map_err(|_| "engine queue full")?;
    }
    let output = device.start(engine).map_err(|e| e.to_string())?;
    let _ = writeln!(out, "A4 on {} for {seconds} s", output.name());
    std::thread::sleep(Duration::from_secs_f64(seconds));
    let _ = handle.send(EngineMessage::AllNotesOff);
    std::thread::sleep(Duration::from_millis(200));
    Ok(())
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut session = new_session();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let script_arg = |i: usize| -> Result<String, ExitCode> {
        let path = &args[i];
        read_script(path).map_err(|err| {
            eprintln!("cannot read script `{path}`: {err}");
            ExitCode::from(2)
        })
    };

    let result = match args.first().map(String::as_str) {
        Some("list") if args.len() == 1 => list_commands(&session, &mut out).map(|()| true),
        Some("run") if args.len() > 1 => {
            let line = musiccraft_commands::Invocation::new(&args[1], args[2..].iter().cloned());
            run_script(&mut session, &line.to_string(), &mut out)
        }
        Some("script") if args.len() == 2 => match script_arg(1) {
            Ok(script) => run_script(&mut session, &script, &mut out),
            Err(code) => return code,
        },
        Some("dump") if args.len() == 2 => match script_arg(1) {
            Ok(script) => {
                let mut sink = io::sink();
                match run_script(&mut session, &script, &mut sink) {
                    Ok(true) => match session.project().to_json() {
                        Ok(json) => writeln!(out, "{json}").map(|()| true),
                        Err(err) => {
                            eprintln!("{err}");
                            Ok(false)
                        }
                    },
                    other => {
                        eprintln!("script failed; run it with `script` to see why");
                        other
                    }
                }
            }
            Err(code) => return code,
        },
        Some("play") if (2..=3).contains(&args.len()) => {
            let seconds = match parse_seconds(args.get(2), 8.0) {
                Ok(s) => s,
                Err(err) => {
                    eprintln!("{err}");
                    return ExitCode::from(2);
                }
            };
            match script_arg(1) {
                Ok(script) => match run_script(&mut session, &script, &mut out) {
                    Ok(true) => match play(&session, seconds, &mut out) {
                        Ok(()) => Ok(true),
                        Err(err) => {
                            eprintln!("audio: {err}");
                            Ok(false)
                        }
                    },
                    other => other,
                },
                Err(code) => return code,
            }
        }
        Some("tone") if args.len() <= 2 => match parse_seconds(args.get(1), 1.0) {
            Ok(seconds) => match tone(seconds, &mut out) {
                Ok(()) => Ok(true),
                Err(err) => {
                    eprintln!("audio: {err}");
                    Ok(false)
                }
            },
            Err(err) => {
                eprintln!("{err}");
                return ExitCode::from(2);
            }
        },
        _ => {
            eprintln!("{USAGE}");
            return ExitCode::from(2);
        }
    };

    match result {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(err) => {
            eprintln!("output error: {err}");
            ExitCode::from(2)
        }
    }
}
