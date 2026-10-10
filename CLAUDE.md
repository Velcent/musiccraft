# MusicCraft

A clean-room, pure-Rust digital audio workstation with a pattern-based workflow
(channel rack, piano roll, playlist, mixer) and familiar default keyboard
shortcuts. Cross-platform: Linux, macOS, Windows. Developed primarily on CachyOS.

## Read first

- `docs/clean-room.md` — binding rules about what we may and may not look at or copy.
- `ARCHITECTURE.md` — crate graph, threading model, command registry.
- `docs/spec/` — behavior specs. **Write or update the spec before the code.**

## Clean-room rules (summary — full text in `docs/clean-room.md`)

- Work only from: the publicly published user manual, observable behavior of
  software a contributor lawfully uses, and open standards (MIDI, CLAP, VST3 SDK
  terms permitting, WAV, etc.).
- Never decompile, disassemble, or inspect the binaries or internals of any
  commercial DAW. Never use leaked or proprietary source, assets, or samples.
- Do not read, parse, or reverse-engineer proprietary project formats (e.g. `.flp`).
  MusicCraft has its own project format.
- Do not copy icons, skins, fonts, sample packs, logos, or names. Match layout and
  default shortcuts through our own implementation and our own artwork.
- Every spec in `docs/spec/` ends with a **Sources** section naming where the
  behavior came from (public manual section, own design, open standard).
- If unsure whether something is allowed, stop and ask the user.

## Audio-thread rules (the real-time callback is sacred)

Inside anything that runs on the audio thread (`musiccraft-engine`, `musiccraft-dsp`):

- No heap allocation, no `Vec` growth, no `String`, no `Box::new`.
- No locks (`Mutex`, `RwLock`), no blocking channels, no I/O, no logging.
- No panics: avoid `unwrap`, `expect`, indexing that can go out of bounds, and `todo!`.
- Communicate with the rest of the app only through lock-free, fixed-size messages
  (plain `Copy` data) and pre-allocated buffers.
- `unsafe` is forbidden workspace-wide. If a future dependency forces an exception,
  raise it with the user first.

## Architecture rules

- **Every user-visible action is a command** in `musiccraft-commands`. The UI,
  keyboard shortcuts, CLI, and MCP server only invoke commands. Nothing else
  mutates the project.
- Commands that should be undoable return their inverse invocation.
- `musiccraft-engine` must not depend on `musiccraft-project`; the app translates
  commands into engine messages.
- UI crates contain no business logic. Keyboard shortcuts are data (a keymap that
  maps keys to command names), never hard-coded behavior.
- Adding a dependency requires a one-line justification in `ARCHITECTURE.md`.

## Quality gates (must pass before a session ends)

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
```

## Working style

- One milestone per session. Plan first, then implement.
- Add a test for every command and every DSP block. Prefer offline renders compared
  against golden output for audio.
- Verify behavior headlessly through `musiccraft-cli` before building UI for it.
- Keep functions small and crates focused; prefer many small crates over one large one.

## Milestones

- [x] M0 Workspace, docs, command registry, CLI (set tempo, play/stop/toggle, undo/redo)
- [ ] M1 Audio callback (cpal) with sine oscillator driven by engine messages
- [ ] M2 Step sequencer with sample playback
- [ ] M3 Mixer: channel strips, volume/pan, first effects
- [ ] M4 Piano roll with a basic synth and default shortcuts
- [ ] M5 Playlist with patterns and clips
- [ ] M6 Project save/load, WAV export
- [ ] M7 MCP server, plugin hosting (CLAP first)
- [ ] M8 Parity checklist (`cargo xtask parity`) and UI polish
