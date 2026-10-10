//! MCP server exposing the command registry as tools.
//!
//! Speaks JSON-RPC 2.0 over stdio with one JSON message per line (the MCP stdio
//! transport). Every registered command becomes a tool whose name replaces the
//! dot with an underscore (`transport.play` → `transport_play`) and whose input is
//! `{"args": [..]}`. Extra tools run whole lines and scripts, return the project,
//! and render the project to report levels, so an agent can drive and verify
//! MusicCraft without a GUI.

use musiccraft_commands::{Invocation, SESSION_COMMANDS, Session};
use musiccraft_project::Project;
use musiccraft_project::params::TICKS_PER_BAR;
use musiccraft_render::{RenderOptions, full_registry, render};
use serde_json::{Value, json};

/// Protocol revisions this server understands, newest first.
pub const PROTOCOL_VERSIONS: [&str; 3] = ["2025-06-18", "2025-03-26", "2024-11-05"];

const INSTRUCTIONS: &str = "\
MusicCraft is a pattern-based DAW. Every change is a command. Channels, patterns, \
playlist tracks and effect slots are numbered from 1; mixer track 0 is the master. \
Note times are in steps (1/16 notes) and playlist times in bars; fractions are \
allowed and `<n>t` is exact ticks (96 per beat). Keys are MIDI numbers or names \
with C5 = 60. Start with `run_script` for multi-step work, check results with \
`get_state` or `get_project`, and listen with `analyze_audio`. Every edit is \
undoable with `edit_undo`. Automate a parameter with `automation_create` (targets \
like channel:1:cutoff, mixer:0:volume, mixer:2:1:mix), shape it with \
`automation_add_point` (time in steps, value 0-1), then place it with \
`playlist_add_automation_clip`; automation plays in song mode.";

/// Converts a command name to a tool name.
#[must_use]
pub fn tool_name(command: &str) -> String {
    command.replace('.', "_")
}

/// A server holding one project session.
pub struct Server {
    session: Session,
}

impl Default for Server {
    fn default() -> Self {
        Self::new()
    }
}

fn text_result(text: impl Into<String>, is_error: bool) -> Value {
    json!({
        "content": [{ "type": "text", "text": text.into() }],
        "isError": is_error,
    })
}

fn rpc_error(id: Value, code: i64, message: impl Into<String>) -> Value {
    json!({ "jsonrpc": "2.0", "id": id, "error": { "code": code, "message": message.into() } })
}

fn args_schema(usage: &str) -> Value {
    json!({
        "type": "object",
        "properties": {
            "args": {
                "type": "array",
                "items": { "type": "string" },
                "description": format!("Positional arguments. Usage: {usage}"),
            }
        },
        "additionalProperties": false,
    })
}

fn rms(data: &[f32]) -> f32 {
    if data.is_empty() {
        return 0.0;
    }
    (data.iter().map(|s| s * s).sum::<f32>() / data.len() as f32).sqrt()
}

fn db(level: f32) -> f64 {
    if level <= 1e-7 {
        -144.0
    } else {
        (20.0 * f64::from(level).log10() * 10.0).round() / 10.0
    }
}

impl Server {
    /// A server with a fresh project and every command.
    #[must_use]
    pub fn new() -> Self {
        Self {
            session: Session::new(Project::default(), full_registry()),
        }
    }

    /// The session (for tests and embedding).
    #[must_use]
    pub fn session(&self) -> &Session {
        &self.session
    }

    /// Handles one JSON-RPC message. Returns the response, or `None` for
    /// notifications.
    pub fn handle(&mut self, message: &Value) -> Option<Value> {
        let id = message.get("id").cloned();
        let method = message.get("method").and_then(Value::as_str).unwrap_or("");
        let params = message.get("params").cloned().unwrap_or(Value::Null);
        let Some(id) = id else {
            // Notifications (initialized, cancelled, ...) need no reply.
            return None;
        };
        let result = match method {
            "initialize" => Ok(self.initialize(&params)),
            "ping" => Ok(json!({})),
            "tools/list" => Ok(self.list_tools()),
            "tools/call" => self.call_tool(&params),
            "resources/list" => Ok(json!({ "resources": [] })),
            "prompts/list" => Ok(json!({ "prompts": [] })),
            other => Err((-32601, format!("method not found: {other}"))),
        };
        Some(match result {
            Ok(result) => json!({ "jsonrpc": "2.0", "id": id, "result": result }),
            Err((code, message)) => rpc_error(id, code, message),
        })
    }

    fn initialize(&self, params: &Value) -> Value {
        let requested = params
            .get("protocolVersion")
            .and_then(Value::as_str)
            .unwrap_or(PROTOCOL_VERSIONS[0]);
        let version = PROTOCOL_VERSIONS
            .iter()
            .find(|v| **v == requested)
            .copied()
            .unwrap_or(PROTOCOL_VERSIONS[0]);
        json!({
            "protocolVersion": version,
            "capabilities": { "tools": { "listChanged": false } },
            "serverInfo": { "name": "musiccraft", "version": env!("CARGO_PKG_VERSION") },
            "instructions": INSTRUCTIONS,
        })
    }

    fn list_tools(&self) -> Value {
        let mut tools: Vec<Value> = self
            .session
            .registry()
            .iter()
            .map(|c| {
                json!({
                    "name": tool_name(c.name()),
                    "description": format!("{} Usage: {}", c.summary(), c.usage()),
                    "inputSchema": args_schema(c.usage()),
                })
            })
            .collect();
        for (name, usage, summary) in SESSION_COMMANDS {
            tools.push(json!({
                "name": tool_name(name),
                "description": format!("{summary} Usage: {usage}"),
                "inputSchema": args_schema(usage),
            }));
        }
        let object = |properties: Value, required: Vec<&str>| json!({ "type": "object", "properties": properties, "required": required });
        tools.push(json!({
            "name": "run",
            "description": "Run one command line, e.g. `note.add 1 0 4 C5`. Quote arguments with spaces.",
            "inputSchema": object(json!({ "line": { "type": "string" } }), vec!["line"]),
        }));
        tools.push(json!({
            "name": "run_script",
            "description": "Run one command per line (# comments allowed). Stops at the first error.",
            "inputSchema": object(json!({ "script": { "type": "string" } }), vec!["script"]),
        }));
        tools.push(json!({
            "name": "get_state",
            "description": "Summarize the project: tempo, mode, channels, patterns with notes, mixer, playlist.",
            "inputSchema": object(json!({}), vec![]),
            "annotations": { "readOnlyHint": true },
        }));
        tools.push(json!({
            "name": "get_project",
            "description": "Return the whole project in the JSON file format.",
            "inputSchema": object(json!({}), vec![]),
            "annotations": { "readOnlyHint": true },
        }));
        tools.push(json!({
            "name": "list_commands",
            "description": "List every command with its usage.",
            "inputSchema": object(json!({}), vec![]),
            "annotations": { "readOnlyHint": true },
        }));
        tools.push(json!({
            "name": "analyze_audio",
            "description": "Render the pattern or song (by play mode) offline and report duration, peak and RMS overall and per bar, so you can check what you built sounds as intended.",
            "inputSchema": object(json!({ "loops": { "type": "integer", "minimum": 1, "maximum": 16 } }), vec![]),
            "annotations": { "readOnlyHint": true },
        }));
        json!({ "tools": tools })
    }

    fn call_tool(&mut self, params: &Value) -> Result<Value, (i64, String)> {
        let name = params
            .get("name")
            .and_then(Value::as_str)
            .ok_or((-32602, "missing tool name".to_owned()))?;
        let arguments = params.get("arguments").cloned().unwrap_or(json!({}));
        let string_arg = |key: &str| -> Result<String, (i64, String)> {
            arguments
                .get(key)
                .and_then(Value::as_str)
                .map(str::to_owned)
                .ok_or((-32602, format!("`{key}` must be a string")))
        };
        Ok(match name {
            "run" => self.run_line(&string_arg("line")?),
            "run_script" => self.run_script(&string_arg("script")?),
            "get_state" => text_result(self.state_summary(), false),
            "get_project" => match self.session.project().to_json() {
                Ok(json) => text_result(json, false),
                Err(err) => text_result(err.to_string(), true),
            },
            "list_commands" => text_result(self.command_list(), false),
            "analyze_audio" => {
                let loops = arguments.get("loops").and_then(Value::as_u64).unwrap_or(1);
                text_result(
                    self.analyze(u32::try_from(loops.clamp(1, 16)).unwrap_or(1)),
                    false,
                )
            }
            tool => {
                let command = self
                    .session
                    .registry()
                    .iter()
                    .map(|c| c.name())
                    .chain(SESSION_COMMANDS.iter().map(|(n, _, _)| *n))
                    .find(|n| tool_name(n) == tool)
                    .ok_or((-32602, format!("unknown tool `{tool}`")))?;
                let args: Vec<String> = match arguments.get("args") {
                    None | Some(Value::Null) => Vec::new(),
                    Some(Value::Array(items)) => items
                        .iter()
                        .map(|v| match v {
                            Value::String(s) => s.clone(),
                            other => other.to_string(),
                        })
                        .collect(),
                    Some(_) => return Err((-32602, "`args` must be an array".to_owned())),
                };
                let invocation = Invocation::new(command, args);
                match self.session.run(&invocation) {
                    Ok(outcome) => text_result(outcome.message, false),
                    Err(err) => text_result(err.to_string(), true),
                }
            }
        })
    }

    fn run_line(&mut self, line: &str) -> Value {
        match self.session.run_line(line) {
            Ok(outcome) => text_result(outcome.message, false),
            Err(err) => text_result(err.to_string(), true),
        }
    }

    fn run_script(&mut self, script: &str) -> Value {
        let mut log = Vec::new();
        for line in script.lines() {
            let invocation = match Invocation::parse(line) {
                Ok(invocation) => invocation,
                Err(musiccraft_commands::CommandError::EmptyLine) => continue,
                Err(err) => {
                    log.push(format!("> {}\nerror: {err}", line.trim()));
                    return text_result(log.join("\n"), true);
                }
            };
            match self.session.run(&invocation) {
                Ok(outcome) => log.push(format!("> {invocation}\nok: {}", outcome.message)),
                Err(err) => {
                    log.push(format!("> {invocation}\nerror: {err}"));
                    return text_result(log.join("\n"), true);
                }
            }
        }
        text_result(log.join("\n"), false)
    }

    fn command_list(&self) -> String {
        let mut lines: Vec<String> = self
            .session
            .registry()
            .iter()
            .map(|c| format!("{} — {}", c.usage(), c.summary()))
            .collect();
        lines.extend(
            SESSION_COMMANDS
                .iter()
                .map(|(_, usage, summary)| format!("{usage} — {summary}")),
        );
        lines.join("\n")
    }

    /// A readable overview of the project.
    #[must_use]
    pub fn state_summary(&self) -> String {
        use musiccraft_project::music::key_name;
        let p = self.session.project();
        let mut out = vec![format!(
            "{} — {} BPM, {} mode, transport {}, pattern {}/{} selected",
            p.name(),
            p.tempo_bpm(),
            p.play_mode.label(),
            p.transport(),
            p.current_pattern + 1,
            p.patterns.len()
        )];
        out.push("Channels:".to_owned());
        for (i, c) in p.channels.iter().enumerate() {
            let source = match &c.instrument {
                musiccraft_project::Instrument::Sampler { sample } => format!("sampler {sample}"),
                musiccraft_project::Instrument::Synth => "synth".to_owned(),
            };
            out.push(format!(
                "  {} {} ({source}) vol {:.2} pan {:+.2}{} → mixer {}",
                i + 1,
                c.name,
                c.volume,
                c.pan,
                if c.mute { " MUTED" } else { "" },
                c.mixer_track
            ));
        }
        out.push("Patterns:".to_owned());
        for (i, pattern) in p.patterns.iter().enumerate() {
            out.push(format!(
                "  {} {} ({} steps, {} ticks)",
                i + 1,
                pattern.name,
                pattern.length_steps,
                pattern.length_ticks()
            ));
            for (ci, c) in p.channels.iter().enumerate() {
                let notes: Vec<String> = pattern
                    .notes_of(c.id)
                    .map(|n| format!("{}t:{}x{}t", n.start, key_name(n.key), n.length))
                    .collect();
                if !notes.is_empty() {
                    out.push(format!("    ch{} {}: {}", ci + 1, c.name, notes.join(" ")));
                }
            }
        }
        out.push("Mixer:".to_owned());
        for (i, t) in p.mixer.iter().enumerate() {
            let used =
                i == 0 || !t.effects.is_empty() || p.channels.iter().any(|c| c.mixer_track == i);
            if !used {
                continue;
            }
            let fx: Vec<&str> = t.effects.iter().map(|e| e.kind.name()).collect();
            out.push(format!(
                "  {i} {} vol {:.2} pan {:+.2}{}{} fx [{}]",
                t.name,
                t.volume,
                t.pan,
                if t.mute { " MUTED" } else { "" },
                if t.solo { " SOLO" } else { "" },
                fx.join(", ")
            ));
        }
        if !p.automations.is_empty() {
            out.push("Automations:".to_owned());
            for (i, a) in p.automations.iter().enumerate() {
                let points: Vec<String> = a
                    .points
                    .iter()
                    .map(|pt| format!("{}t={:.2}", pt.tick, pt.value))
                    .collect();
                out.push(format!(
                    "  {} {} [{:?}]: {}",
                    i + 1,
                    a.name,
                    a.target,
                    points.join(" ")
                ));
            }
        }
        out.push(format!("Playlist ({} clips):", p.playlist.clips.len()));
        for clip in &p.playlist.clips {
            let name = match clip.source {
                musiccraft_project::ClipSource::Pattern(id) => {
                    p.pattern_position(id).map_or("?".to_owned(), |i| {
                        format!("pattern {}", p.patterns[i].name)
                    })
                }
                musiccraft_project::ClipSource::Automation(id) => {
                    p.automation_position(id).map_or("?".to_owned(), |i| {
                        format!("automation {} ({})", i + 1, p.automations[i].name)
                    })
                }
                musiccraft_project::ClipSource::Audio(id) => {
                    p.channel_position(id).map_or("?".to_owned(), |i| {
                        format!("audio of channel {} ({})", i + 1, p.channels[i].name)
                    })
                }
            };
            out.push(format!(
                "  track {} bar {:.2} len {:.2} bars: {name}",
                clip.track + 1,
                f64::from(clip.start) / f64::from(TICKS_PER_BAR),
                f64::from(clip.length) / f64::from(TICKS_PER_BAR)
            ));
        }
        out.join("\n")
    }

    fn analyze(&self, loops: u32) -> String {
        let project = self.session.project();
        let options = RenderOptions {
            loops,
            ..RenderOptions::default()
        };
        let (left, right, warnings) = render(project, options);
        let sr = options.sample_rate as f32;
        let bar_seconds = 240.0 / project.tempo_bpm() as f32;
        let bar_frames = (bar_seconds * sr).max(1.0) as usize;
        let mono: Vec<f32> = left
            .iter()
            .zip(&right)
            .map(|(l, r)| (l + r) * 0.5)
            .collect();
        let peak = left
            .iter()
            .chain(&right)
            .fold(0.0_f32, |p, s| p.max(s.abs()));
        let mut out = vec![format!(
            "{:.2} s rendered, peak {} dBFS, RMS {} dBFS{}",
            left.len() as f32 / sr,
            db(peak),
            db(rms(&mono)),
            if peak >= 1.0 { " (CLIPPING)" } else { "" }
        )];
        for (bar, chunk) in mono.chunks(bar_frames).enumerate().take(64) {
            let p = chunk.iter().fold(0.0_f32, |p, s| p.max(s.abs()));
            out.push(format!(
                "bar {:>2}: peak {:>6} dB, RMS {:>6} dB",
                bar + 1,
                db(p),
                db(rms(chunk))
            ));
        }
        out.extend(warnings.into_iter().map(|w| format!("warning: {w}")));
        out.join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn call(server: &mut Server, name: &str, arguments: Value) -> (String, bool) {
        let response = server
            .handle(&json!({
                "jsonrpc": "2.0", "id": 7, "method": "tools/call",
                "params": { "name": name, "arguments": arguments }
            }))
            .unwrap();
        let result = &response["result"];
        (
            result["content"][0]["text"].as_str().unwrap().to_owned(),
            result["isError"].as_bool().unwrap(),
        )
    }

    #[test]
    fn initialize_negotiates_version() {
        let mut server = Server::new();
        let response = server
            .handle(&json!({
                "jsonrpc": "2.0", "id": 1, "method": "initialize",
                "params": { "protocolVersion": "2024-11-05", "capabilities": {}, "clientInfo": { "name": "t", "version": "1" } }
            }))
            .unwrap();
        assert_eq!(response["result"]["protocolVersion"], "2024-11-05");
        assert_eq!(response["result"]["serverInfo"]["name"], "musiccraft");
        let unknown = server
            .handle(&json!({ "jsonrpc": "2.0", "id": 2, "method": "initialize", "params": { "protocolVersion": "1999-01-01" } }))
            .unwrap();
        assert_eq!(unknown["result"]["protocolVersion"], PROTOCOL_VERSIONS[0]);
        assert!(
            server
                .handle(&json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }))
                .is_none()
        );
    }

    #[test]
    fn every_command_is_a_tool_with_a_valid_name() {
        let mut server = Server::new();
        let response = server
            .handle(&json!({ "jsonrpc": "2.0", "id": 1, "method": "tools/list" }))
            .unwrap();
        let tools = response["result"]["tools"].as_array().unwrap();
        let names: Vec<&str> = tools.iter().map(|t| t["name"].as_str().unwrap()).collect();
        for expected in [
            "transport_play",
            "note_add",
            "project_export_wav",
            "edit_undo",
            "edit_batch",
            "run_script",
            "analyze_audio",
        ] {
            assert!(names.contains(&expected), "{expected} missing");
        }
        for name in names {
            assert!(
                !name.is_empty()
                    && name.len() <= 64
                    && name
                        .chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '-'),
                "bad tool name {name}"
            );
        }
    }

    #[test]
    fn tools_change_and_report_the_project() {
        let mut server = Server::new();
        let (text, error) = call(
            &mut server,
            "channel_add_sampler",
            json!({ "args": ["builtin:kick"] }),
        );
        assert!(!error, "{text}");
        let (_, error) = call(
            &mut server,
            "run_script",
            json!({ "script": "step.toggle 1 0\nstep.toggle 1 8\nproject.set_tempo 120" }),
        );
        assert!(!error);
        let (state, _) = call(&mut server, "get_state", json!({}));
        assert!(state.contains("120 BPM"), "{state}");
        assert!(state.contains("ch1 Kick: 0t:C5x24t 192t:C5x24t"), "{state}");
        let (audio, _) = call(&mut server, "analyze_audio", json!({}));
        assert!(
            audio.starts_with("2."),
            "one bar at 120 BPM plus tail: {audio}"
        );
        assert!(audio.contains("bar  1"));
        let (_, error) = call(&mut server, "edit_undo", json!({}));
        assert!(!error);
        assert!((server.session().project().tempo_bpm() - 130.0).abs() < f64::EPSILON);
    }

    #[test]
    fn errors_are_reported_in_results_or_as_rpc_errors() {
        let mut server = Server::new();
        let (text, error) = call(
            &mut server,
            "project_set_tempo",
            json!({ "args": ["5000"] }),
        );
        assert!(error && text.contains("outside"));
        let (text, error) = call(&mut server, "run", json!({ "line": "nope.nothing" }));
        assert!(error && text.contains("unknown command"));
        let response = server
            .handle(&json!({ "jsonrpc": "2.0", "id": 3, "method": "tools/call", "params": { "name": "bogus_tool" } }))
            .unwrap();
        assert_eq!(response["error"]["code"], -32602);
        let response = server
            .handle(&json!({ "jsonrpc": "2.0", "id": 4, "method": "no/such" }))
            .unwrap();
        assert_eq!(response["error"]["code"], -32601);
        let (_, error) = call(&mut server, "transport_set_mode", json!({ "args": [1] }));
        assert!(error, "numbers are converted to strings and validated");
    }
}
