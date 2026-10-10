# MusicCraft architecture

## Goals

- Pure Rust, cross-platform (Linux, macOS, Windows), headless-testable.
- The audio engine is isolated from everything else and obeys real-time rules.
- A single command registry is the only way to change a project. UI, shortcuts,
  CLI, and MCP are thin front ends over it, so Claude Code can drive and verify the
  whole app without a GUI.

## Crate graph

```
musiccraft-app ──► musiccraft-ui ──► musiccraft-commands ──► musiccraft-project ──► musiccraft-params
      │                                     ▲                                         ▲
      ├──► musiccraft-render ───────────────┤                                         │
      │        ├──► musiccraft-engine ──► musiccraft-dsp ─────────────────────────────┤
      │        └──► musiccraft-samples ──► musiccraft-dsp                              │
      ├──► musiccraft-audio ──► musiccraft-engine                                      │
      │                                                                                │
musiccraft-cli ──► commands, render, audio                                             │
musiccraft-mcp ──► commands, render                                                    │
xtask ──► commands, render, ui (keymap) ───────────────────────────────────────────────┘
```

| Crate | Responsibility |
| --- | --- |
| `musiccraft-params` | Shared tables: timing (PPQ), instrument and effect parameter specs, built-in sound names. No logic. |
| `musiccraft-project` | Project data model and JSON file format. No audio, no UI. |
| `musiccraft-commands` | Command trait, ~70 built-in commands, invocation parsing, undo/redo `Session`. |
| `musiccraft-dsp` | Oscillators, envelopes, filters, effects, sample buffers. Allocation-free processing. |
| `musiccraft-engine` | Real-time engine: voices, sequencing, mixing, meters. Driven only by messages. |
| `musiccraft-samples` | WAV decoding/encoding, FLAC encoding, sample cache, synthesized built-in sounds. Main thread only. |
| `musiccraft-render` | Bridge: diffs a project against engine state and sends messages; offline render; `project.export_wav`. |
| `musiccraft-audio` | Opens the output device with `cpal` and runs the engine in its callback; captures input through a lock-free ring for recording. |
| `musiccraft-cli` | Headless front end: run commands and scripts, dump JSON, play, export. |
| `musiccraft-mcp` | MCP server (JSON-RPC over stdio) exposing every command as a tool. |
| `musiccraft-ui` | egui views (channel rack, piano roll, playlist, mixer) and the keymap. Emits invocations only. |
| `musiccraft-app` | Wires session, bridge, audio device, MIDI input and UI together. |
| `xtask` | `cargo xtask parity`: checks the feature checklist against the registry and keymap. |

## Command registry

- `Command` trait: `name`, `summary`, `usage`, `execute(&mut Project, &[String])`.
- `Invocation` is a command name plus string arguments. It is what a shortcut, a
  menu item, a CLI line, and an MCP tool call all become. Its text form supports
  quoting, so file paths and JSON can be arguments.
- `execute` returns an `Outcome` with a message, an optional **inverse
  invocation**, and a `reset_history` flag (new/load). The `Session` keeps undo and
  redo stacks of invocations.
- `edit.undo`, `edit.redo` and `edit.batch` are handled by `Session` itself.
  Batches give multi-part edits a single, atomic undo step.
- Naming: `area.action` (`transport.play`, `project.set_tempo`).
- Transport, mode and selection commands are deliberately not undoable; edits are.
- Other crates add commands with `Registry::register` (the render crate adds
  `project.export_wav`).
- See `docs/spec/commands.md` for numbering, time units and undo details.

## Threading model

- **Audio thread**: owns the `Engine`. Runs inside the audio callback. No
  allocation, deallocation, locks, I/O, or panics.
- **Main/UI thread**: owns the `Session`, `Project` and `Bridge`. After commands
  run, the bridge diffs the project against what the engine last received and
  pushes messages through a lock-free SPSC queue (`rtrb`).
- **MIDI thread** (optional): sends live note events through a second SPSC queue.
- The engine never reads `Project`. Data the engine needs (sample buffers, note
  sequences, effect state) is prepared on the main thread.

### Message rule (refines the CLAUDE.md audio-thread rule)

Control messages are plain `Copy` data. Three messages, `SetGraph`,
`SetSequence` and `PlayPreview`, carry a `Box` allocated on the main thread. Moving a box is a
fixed-size pointer move, not an allocation. The value it replaces is pushed onto
a return queue and dropped on the main thread. The engine checks that the return
queue has room *before* taking such a message; if it does not, the message waits
for the next callback. The audio thread therefore never allocates or frees.
Shared sample data lives in `Arc`s that are only ever dropped by the main thread
(they travel inside returned graphs).

## Time

- 96 ticks per beat (PPQ), 24 ticks per sixteenth-note step, 4/4 bars of 384
  ticks. The engine keeps the playhead as fractional ticks and starts each note on
  the first frame at or after its tick.

## Testing strategy

- Unit tests for every command, including error paths. A round-trip test runs a
  script touching every undoable command, then undoes, redoes and undoes again,
  comparing the project to snapshots at every step.
- Engine and DSP tests render offline and assert on exact sample positions.
- Render tests check sequence building, diff-based sync and exported audio.
- CLI integration tests spawn the real binary.
- `cargo xtask parity` reports how many checklist features are implemented.

## Dependencies

| Dependency | Why |
| --- | --- |
| `cpal` | Cross-platform audio output device access. |
| `rtrb` | Wait-free SPSC ring buffer for main → audio messages, returns and live notes. |
| `serde`, `serde_json` | Project file format and JSON payloads in undo inverses and MCP. |
| `hound` | WAV decoding for samples and WAV encoding for export. |
| `eframe` (egui) | Cross-platform immediate-mode UI; kept swappable behind commands. |
| `midir` | MIDI keyboard input in the app. |

## Deferred decisions

- Custom `wgpu` renderer if egui cannot reach the target look.
- Plugin hosting (CLAP first). Loading plugins requires `unsafe` FFI, which is
  forbidden workspace-wide; this needs an explicit decision before it starts.
- Decoding compressed samples (FLAC, Ogg, MP3), likely via `symphonia`; lossy
  export formats.
- Time signatures other than 4/4, tempo automation, audio clips and recording.
