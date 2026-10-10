# MusicCraft

A clean-room, pure-Rust digital audio workstation with a pattern-based workflow:
channel rack, piano roll, playlist and mixer, with familiar default shortcuts.
Runs on Linux, macOS and Windows. Everything the app can do is a command, so it
can be driven from the GUI, scripts, the CLI, or an AI agent over MCP.

## Build and run

```sh
cargo run --release -p musiccraft-app              # the desktop app
cargo run --release -p musiccraft-app -- song.mcproj
```

Linux needs the ALSA development package (`alsa-lib` / `libasound2-dev`).

Options: `--empty` (no starter drum kit), `--keymap <file>`, `--open
channel_rack,piano_roll,playlist,mixer,browser`.

## Using it

| Key | Action |
| --- | --- |
| Space | Play / stop |
| L | Pattern / song mode |
| F5 / F6 / F7 / F9 | Playlist / channel rack / piano roll / mixer |
| F4, Plus, Minus | New / next / previous pattern |
| Ctrl+Z, Ctrl+Y | Undo / redo |
| Ctrl+S, Ctrl+O, Ctrl+R | Save / open / export WAV |
| Ctrl+M, Ctrl+E, Ctrl+T | Metronome / record / typing keyboard |

- **Channel rack**: click steps; knobs for pan and volume (double-click resets);
  the number routes the channel to a mixer insert; right-click a name for more.
- **Piano roll**: click to draw, drag to move, drag the right edge to resize,
  right-click to delete; chord and scale menus; velocity lane at the bottom.
- **Playlist**: pick a pattern on the left, click to place it, drag to move or
  resize, right-click to delete.
- **Mixer**: faders, pan, mute/solo, eight effect slots per track (filter, EQ,
  delay, reverb, chorus, compressor, limiter, distortion), sends to buses.
- A MIDI keyboard plays the selected channel; arm REC while playing to record.

Built-in sounds (`builtin:kick`, `snare`, `clap`, `hat`, `open_hat`, `tom`,
`rim`, `crash`) are synthesized at start-up; WAV files can be loaded as samples.

## Headless: CLI and scripts

```sh
cargo run -p musiccraft-cli -- list                        # every command
cargo run -p musiccraft-cli -- script examples/basic-beat.mcs
cargo run -p musiccraft-cli -- play examples/basic-beat.mcs 10
cargo run -p musiccraft-cli -- dump examples/basic-beat.mcs  # project JSON
```

Add `project.export_wav beat.wav` or `project.export_stems stems/` to a script to
render audio. See `docs/spec/commands.md` for argument conventions.

## AI agents (MCP)

`musiccraft-mcp` is a Model Context Protocol server over stdio. Every command is
a tool, plus `run_script`, `get_state`, `get_project` and `analyze_audio`:

```json
{ "mcpServers": { "musiccraft": { "command": "/path/to/target/release/musiccraft-mcp" } } }
```

## Development

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo xtask parity          # feature checklist (docs/parity.md)
```

- `CLAUDE.md` — working rules; `ARCHITECTURE.md` — crates and threading;
  `docs/spec/` — behavior specs; `docs/clean-room.md` — provenance rules.
- `MUSICCRAFT_SCREENSHOT=out.png musiccraft …` saves one frame and exits.

MusicCraft is an independent project and is not affiliated with any other DAW.
