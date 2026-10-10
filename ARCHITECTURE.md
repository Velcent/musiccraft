# MusicCraft architecture

## Goals

- Pure Rust, cross-platform (Linux, macOS, Windows), headless-testable.
- The audio engine is isolated from everything else and obeys real-time rules.
- A single command registry is the only way to change a project. UI, shortcuts,
  CLI, and MCP are thin front ends over it, so Claude Code can drive and verify the
  whole app without a GUI.

## Crate graph

```
musiccraft-app ──► musiccraft-ui ──► musiccraft-commands ──► musiccraft-project
      │                                      ▲
      ├──► musiccraft-engine ──► musiccraft-dsp
      │
musiccraft-cli ──┐
musiccraft-mcp ──┴──► musiccraft-commands
```

| Crate | Responsibility |
| --- | --- |
| `musiccraft-project` | Project data model (tempo, transport, later tracks, patterns, clips). No audio, no UI. |
| `musiccraft-commands` | Command trait, registry, invocation parsing, undo/redo `Session`. |
| `musiccraft-engine` | Real-time audio engine: transport, sequencing, mixing. Takes plain `Copy` messages. |
| `musiccraft-dsp` | Oscillators, filters, effects. Allocation-free processing blocks. |
| `musiccraft-cli` | Headless front end: run commands and scripts, print state. |
| `musiccraft-mcp` | MCP server exposing the registry (planned, M7). |
| `musiccraft-ui` | egui-based interface and keymap (planned). |
| `musiccraft-app` | Wires commands, engine, audio device, and UI together. |

## Command registry

- `Command` trait: `name`, `summary`, `usage`, `execute(&mut Project, &[String])`.
- `Invocation` is a command name plus string arguments. It is what a shortcut, a
  menu item, a CLI line, and an MCP tool call all become.
- `execute` returns an `Outcome` with a human-readable message and an optional
  **inverse invocation**. Undoable commands return their inverse; the `Session`
  keeps undo and redo stacks of invocations.
- `edit.undo` and `edit.redo` are handled by `Session` itself.
- Naming: `area.action` (`transport.play`, `project.set_tempo`).
- Transport commands are deliberately not undoable; edits are.

## Threading model

- **Audio thread**: owns the `Engine`. Runs inside the audio callback. No allocation,
  locks, I/O, or panics.
- **Main/UI thread**: owns the `Session` and `Project`. After a command succeeds,
  the app translates the change into an `EngineMessage` and pushes it through a
  lock-free single-producer single-consumer queue (`rtrb`, planned M1).
- The engine never reads `Project`. Data the engine needs (e.g. sample buffers) is
  prepared on the main thread and handed over by message.

## Testing strategy

- Unit tests for every command, including error paths.
- Engine and DSP tests render offline and assert on exact sample positions or
  golden output.
- CLI integration test spawns the real binary and checks output and exit code.
- Planned: `cargo xtask parity` reporting how many specced features are implemented.

## Planned dependencies (justify before adding)

| Dependency | Why | When |
| --- | --- | --- |
| `cpal` | Cross-platform audio output | M1 |
| `rtrb` | Lock-free SPSC queue for UI → audio messages | M1 |
| `egui` / `eframe` | Fast cross-platform UI; keep swappable behind commands | UI |
| `symphonia`, `hound` | Sample decoding and WAV I/O | M2, M6 |
| `midir` | MIDI input | M4 |

The current workspace has **no external dependencies**.

## Deferred decisions

- Custom `wgpu` renderer if egui cannot reach the target look.
- Project file format (own format; versioned; M6).
- Plugin hosting (CLAP first).
- Transport position semantics beyond play/stop (rewind, loop, pause vs stop).
