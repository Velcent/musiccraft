use std::io::{self, Read};
use std::process::ExitCode;

use musiccraft_cli::{list_commands, new_session, run_script};

const USAGE: &str = "\
usage:
  musiccraft-cli list                    list available commands
  musiccraft-cli run <command> [args]    run one command
  musiccraft-cli script <file|->         run one command per line (# comments allowed)

Each invocation starts from a fresh project; use `script` for multi-step work.";

fn read_script(path: &str) -> io::Result<String> {
    if path == "-" {
        let mut text = String::new();
        io::stdin().read_to_string(&mut text)?;
        Ok(text)
    } else {
        std::fs::read_to_string(path)
    }
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut session = new_session();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let result = match args.first().map(String::as_str) {
        Some("list") if args.len() == 1 => list_commands(&session, &mut out).map(|()| true),
        Some("run") if args.len() > 1 => run_script(&mut session, &args[1..].join(" "), &mut out),
        Some("script") if args.len() == 2 => match read_script(&args[1]) {
            Ok(script) => run_script(&mut session, &script, &mut out),
            Err(err) => {
                eprintln!("cannot read script `{}`: {err}", args[1]);
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
