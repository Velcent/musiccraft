# MusicCraft

A clean-room, pure-Rust digital audio workstation with a pattern-based workflow:
channel rack, piano roll, playlist and mixer, with familiar default shortcuts.
Runs on Linux, macOS and Windows. Everything the app can do is a command, so it
can be driven from the GUI, scripts, the CLI, or an AI agent over MCP.

![Playlist arrangement with markers and automation, and the docked mixer](docs/screenshots/playlist-mixer.png)

<table>
  <tr>
    <td><img src="docs/screenshots/channel-rack.png" alt="Channel rack step sequencer"></td>
    <td><img src="docs/screenshots/piano-roll.png" alt="Piano roll with a melody over chords and the velocity lane"></td>
  </tr>
  <tr>
    <td align="center">Channel rack</td>
    <td align="center">Piano roll</td>
  </tr>
</table>

The screenshots show [`examples/demo-song.mcs`](examples/demo-song.mcs).

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

- **Playlist** (main workspace): paint patterns, automation or audio clips;
  click, Shift+click or Ctrl+drag to select; drag to move or resize; Delete,
  Ctrl+B duplicate. Click the ruler to seek, Shift+drag it to set a loop,
  right-click it for markers.
- **Channel rack**: click steps; knobs for pan and volume (double-click resets);
  click a channel name for its settings (envelope, filter, glide, sample view);
  melodic channels show a mini piano roll.
- **Piano roll**: click to draw (single notes or chords), drag to move, drag the
  right edge to resize, right-click to delete; selection with copy, paste,
  Ctrl+B and arrow keys; velocity/pan lane at the bottom; scale highlighting.
- **Mixer** (docked): faders with a dB curve, pan, on/solo, eight effect slots
  (filter, EQ, delay, reverb, chorus, compressor with sidechain, limiter,
  distortion), sends via the arrows under each strip.
- **Automation**: right-click any knob or fader → Create automation clip;
  double-click the clip to edit its curve.
- **Browser**: built-in sounds, your folders, search; click previews,
  double-click adds, drag onto the rack (new channel) or the playlist (audio
  clip).
- **Recording**: REC + playing records a MIDI or typing keyboard into the
  pattern; with File → Record audio input, song-mode takes become audio clips.
- **Export**: WAV (16/24/32f), FLAC (16/24), or one stem per mixer insert.
- **Plugins (CLAP)**: instruments and effects from `~/.clap`, `/usr/lib/clap`
  and `CLAP_PATH` show up in the browser, the rack's add menu and mixer slots.
  Each gets a parameter panel and can open its own window; parameters and
  state are saved in the project and exports render them. Tested with
  Surge XT and LSP plugins.

Built-in sounds (`builtin:kick`, `snare`, `clap`, `hat`, `open_hat`, `tom`,
`rim`, `crash`) are synthesized at start-up; WAV files can be loaded as samples.

## Headless: CLI and scripts

```sh
cargo run -p musiccraft-cli -- list                        # every command
cargo run -p musiccraft-cli -- script examples/basic-beat.mcs
cargo run -p musiccraft-cli -- play examples/basic-beat.mcs 10
cargo run -p musiccraft-cli -- dump examples/basic-beat.mcs  # project JSON
```

Add `project.export_wav beat.wav`, `project.export_flac beat.flac` or
`project.export_stems stems/` to a script to render audio. See `docs/spec/commands.md` for argument conventions.

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
