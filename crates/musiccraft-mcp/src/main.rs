//! `musiccraft-mcp`: MCP server over stdio. Configure an MCP client to launch this
//! binary; each line on stdin is one JSON-RPC message. With `--connect` it relays
//! stdio to the project open in the desktop app instead (`docs/spec/live-control.md`).

use std::io::{self, BufRead, Write};
use std::path::PathBuf;

use musiccraft_mcp::Server;
use serde_json::{Value, json};

const USAGE: &str = "\
usage: musiccraft-mcp [--connect <host:port> [--control-token-file <file>]]
  (no options)                 MCP over stdio with its own project
  --connect <host:port>        work on the project open in MusicCraft, started
                               with `musiccraft --control <port>`
  --control-token-file <file>  the token file given to `musiccraft`
  --version                    print the version";

enum Mode {
    Stdio,
    Connect {
        addr: String,
        token: Option<PathBuf>,
    },
}

fn parse_args() -> Result<Mode, String> {
    let mut args = std::env::args().skip(1);
    let (mut addr, mut token) = (None, None);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--connect" => addr = Some(args.next().ok_or(USAGE)?),
            "--control-token-file" => token = Some(PathBuf::from(args.next().ok_or(USAGE)?)),
            "--version" | "-V" => {
                println!("musiccraft-mcp {}", env!("CARGO_PKG_VERSION"));
                std::process::exit(0);
            }
            "-h" | "--help" => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            other => return Err(format!("unknown option {other}\n{USAGE}")),
        }
    }
    match (addr, token) {
        (Some(addr), token) => Ok(Mode::Connect { addr, token }),
        (None, None) => Ok(Mode::Stdio),
        (None, Some(_)) => Err(format!("--control-token-file needs --connect\n{USAGE}")),
    }
}

fn main() {
    musiccraft_cli::serve_plugin_probe();
    match parse_args() {
        Ok(Mode::Stdio) => serve_stdio(),
        Ok(Mode::Connect { addr, token }) => {
            if let Err(err) = connect(&addr, token.as_deref()) {
                eprintln!("musiccraft-mcp: can't reach MusicCraft at {addr}: {err}");
                std::process::exit(1);
            }
        }
        Err(message) => {
            eprintln!("{message}");
            std::process::exit(2);
        }
    }
}

fn connect(addr: &str, token: Option<&std::path::Path>) -> io::Result<()> {
    let token = token.map(musiccraft_mcp::control::read_token).transpose()?;
    musiccraft_mcp::control::relay(
        addr,
        token.as_deref(),
        io::BufReader::new(io::stdin()),
        io::stdout().lock(),
    )
}

fn serve_stdio() {
    let mut server = Server::new();
    let stdin = io::stdin();
    let mut stdout = io::stdout().lock();
    for line in stdin.lock().lines() {
        let Ok(line) = line else {
            break;
        };
        if line.trim().is_empty() {
            continue;
        }
        let response = match serde_json::from_str::<Value>(&line) {
            Ok(Value::Array(batch)) => {
                let replies: Vec<Value> = batch.iter().filter_map(|m| server.handle(m)).collect();
                (!replies.is_empty()).then_some(Value::Array(replies))
            }
            Ok(message) => server.handle(&message),
            Err(err) => Some(json!({
                "jsonrpc": "2.0",
                "id": null,
                "error": { "code": -32700, "message": format!("parse error: {err}") }
            })),
        };
        if let Some(response) = response {
            if writeln!(stdout, "{response}")
                .and_then(|()| stdout.flush())
                .is_err()
            {
                break;
            }
        }
    }
}
