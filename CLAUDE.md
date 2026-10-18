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
- Do not reverse-engineer proprietary project formats yourself. The one exception
  (owner's decision, 2026-10-16): `.flp` import in `musiccraft-flp`, a Rust port of
  the GPL-3.0 PyFLP parser, kept in step with PyFLP. MusicCraft has its own project
  format.
- MusicCraft is licensed GPL-3.0-or-later (`LICENSE`). Code copied or ported from
  other projects must be GPL-3.0-compatible and keep its copyright notice.
- Do not copy icons, skins, fonts, sample packs, logos, or names. Match layout and
  default shortcuts through our own implementation and our own artwork.
- Every spec in `docs/spec/` ends with a **Sources** section naming where the
  behavior came from (public manual section, own design, open standard).
- If unsure whether something is allowed, stop and ask the user.

## Audio-thread rules (the real-time callback is sacred)

Inside anything that runs on the audio thread (`musiccraft-engine`, `musiccraft-dsp`,
`musiccraft-instruments`, `musiccraft-antiserum`):

- No heap allocation, no `Vec` growth, no `String`, no `Box::new`.
- No locks (`Mutex`, `RwLock`), no blocking channels, no I/O, no logging.
- No panics: avoid `unwrap`, `expect`, indexing that can go out of bounds, and `todo!`.
- Communicate with the rest of the app only through lock-free, fixed-size messages
  (plain `Copy` data) and pre-allocated buffers.
- `unsafe` is forbidden workspace-wide. If a future dependency forces an exception,
  raise it with the user first. The approved exceptions (plugin hosting: CLAP
  approved by the user on 2026-10-10, VST3 on 2026-10-16) are `musiccraft-clap`
  and `musiccraft-vst3` plus their test fixtures `musiccraft-clap-testplugin` and
  `musiccraft-vst3-testplugin`; and, for exporting MusicCraft's instruments and
  effects as CLAP/VST3 plugins (approved by the user on 2026-10-18),
  `musiccraft-plugin-export` and its editor adapter `musiccraft-plugin-editor`.
  Every `unsafe` block there carries a `SAFETY:` comment (enforced by clippy),
  and nothing else may use `unsafe`.

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

## File editing protocol

1. **Primary choice**: always try the native `Edit` tool first for surgical,
   localized line changes. It saves output tokens and keeps file history.
2. **Fallback**: only if an edit fails because of complex multi-line formatting,
   matching errors, or fragile whitespace or quotes, use a precise
   `python3 - <<'EOF'` script through the Bash tool to make the replacement.
3. **Condition**: before using the Python fallback, state the error the native
   tool hit.

## Milestones

- [x] M0 Workspace, docs, command registry, CLI (set tempo, play/stop/toggle, undo/redo)
- [x] M1 Audio callback (cpal) with sine oscillator driven by engine messages
- [x] M2 Step sequencer with sample playback
- [x] M3 Mixer: channel strips, volume/pan, first effects
- [x] M4 Piano roll with a basic synth and default shortcuts
- [x] M5 Playlist with patterns and clips
- [x] M6 Project save/load, WAV export
- [x] M7 MCP server
- [x] M7 Plugin hosting, CLAP and VST3 (`docs/spec/plugins.md`)
- [x] M8 Parity checklist (`cargo xtask parity`) and UI polish

Open items are tracked as `todo` rows in `docs/parity.md`.
