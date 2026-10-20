# Spec: MusicCraft's effects as plugins (CLAP, VST3, standalone)

Phase 7 of `docs/roadmap-wavetable-and-plugins.md`, effects first. Instruments
and Antiserum follow with the same wrapper and editor adapter.

## Behavior

- **What is exported.** Every built-in effect in `EFFECT_KINDS` except the
  three the mixer routes (`send`, `band_split`, `rack`): 44 plugins. They all
  live in one library, **MusicCraft Effects**, bundled as
  `MusicCraft Effects.clap` and `MusicCraft Effects.vst3`. A host lists each
  effect as its own plugin.
- **Names and IDs.**
  - The plugin name is the effect's title as the app shows it ("Hall reverb",
    "Multi-tap delay"); the vendor is `MusicCraft`.
  - The CLAP ID is `org.musiccraft.<name>`, where `<name>` is the effect's
    stable identifier (`org.musiccraft.hall_reverb`).
  - The VST3 class ID is `MCfx` followed by the first 12 bytes of a 128-bit
    FNV-1a hash of the CLAP ID. It is computed at compile time, and a test
    freezes every ID, since a host stores it in its projects.
  - CLAP features: `audio-effect`, `stereo`, plus a category (`reverb`,
    `delay`, `filter`, `equalizer`, `compressor`, `distortion`, …). VST3
    subcategories: `Fx` plus the matching category.
- **Audio.** Stereo in, stereo out. The DSP is exactly the app's
  `musiccraft_dsp::Effect`, so a plugin sounds like the same effect in the
  mixer.
- **Parameters.**
  - One host parameter per entry of the effect's `ParamSpec` table, in table
    order. The **parameter ID is the parameter name** (`cutoff`, `mix`), the
    same stable name that project files and commands use; names are frozen by
    the `frozen_names` test, so automation in host projects keeps working.
  - Values are the app's plain values (Hz, dB, seconds, 0–1 amounts).
    Integer parameters are integer host parameters. Frequencies are skewed so
    the middle of the range is the geometric mean, times are skewed with a
    cube root (as in the app), and the rest are linear. Hosts see continuous
    parameters as 0–1 through that skew (nih-plug normalizes them) and show
    our text for them.
  - Display text and typed text use the same descriptions as the app's knobs
    (`musiccraft_dsp::describe`): choice names (`low-pass`, `1/8`, `tanh`),
    units, `off` for switched-off taps.
  - **Routing parameters do not exist in plugins**, since a plugin has no
    mixer: the sidechain source track, send targets and the pitch corrector's
    followed channel. Their replacements:
    - The **compressor** and the **vocoder** have a stereo **sidechain input**
      and a `sidechain` switch (off/on). When on, the sidechain input is the
      key, as a sidechain track is in the app.
    - The **pitch corrector** accepts MIDI notes and has a `follow` switch
      (off/on). When on, it corrects toward the highest held note, as when it
      follows a channel in the app.
  - Every plugin has a **bypass** parameter (`bypass`), flagged as the host's
    bypass. A bypassed effect passes audio through and keeps its state.
- **Processing.** Blocks are split at parameter changes (sample-accurate
  automation) and at most 512 frames long. Parameters are read once per
  sub-block, as the app's engine does once per block. Nothing allocates,
  locks or does I/O on the audio thread.
- **Transport.** Tempo comes from the host (120 BPM when it gives none). While
  the host plays, synced effects (delay, gate, auto-pan, stutter, …) follow
  the host's beat position; while it is stopped they run on a free clock, as
  in the app.
- **Convolution reverb.** The impulse is one of the app's generated rooms
  (`room`, `chamber`, `plate`, `hall`, `cathedral`) or a WAV file, as in the
  app's mixer. The editor has the room list and a path field with **Load**
  (Enter also loads); the picker then shows the file's name. The choice is
  stored in the state (a room name, or the file's path) and is not
  automatable. A file is read and resampled to the plugin's rate off the
  audio thread; one that cannot be read keeps the previous impulse and the
  editor shows why. Loading the same file again re-reads it. A new impulse
  is built off the audio thread and handed over through a lock-free queue;
  the old one is dropped off the audio thread. A project that moves to
  another computer needs the file at the same path; when it is missing the
  plugin keeps its default room and shows the error.
- **State.** The host saves nih-plug's parameter map plus a `musiccraft` field
  holding our own versioned JSON:
  `{"format":1,"effect":"<name>","values":{"<param>":value,…},"impulse":"hall"}`.
  It is the same shape as an app preset (name → value), so effect presets are
  shared. On load the values come back through nih-plug's parameter map, which
  uses the same names: unknown names are ignored and missing ones keep their
  defaults. The room is read from our field; a field for another effect is
  ignored.
- **Editor.** Every plugin opens our own editor (D5), the same one the mixer's
  effect window shows: the preset bar (bypass, previous/next, the effect's
  factory presets), the drawn graph you can drag (filter and EQ curves,
  dynamics curves, delay taps, gate steps, LFO strips…) and the knobs. The
  tuner and the pitch corrector show the detected pitch. The convolution
  reverb has its room picker. Edits are reported to the host as gestures
  (begin, set, end), so hosts record automation and undo them. When the host
  changes a parameter, the editor shows it on the next frame. The window is
  fixed-size per effect and scales with the host's scale factor. If the
  system cannot give the window an OpenGL context, the editor stays empty
  instead of taking the host down.
- **Standalone.** `musiccraft-fx-standalone <effect>` runs one effect as a
  standalone app (nih-plug's standalone wrapper: JACK, ALSA or a dummy
  backend, audio input to output, with the editor). It lives in `standalone/`,
  a workspace of its own, because nih-plug's wrapper uses an older ALSA
  binding than the app and Cargo allows one per workspace:
  `cargo build --release --manifest-path standalone/Cargo.toml`.
- **Packaging.** `cargo xtask bundle [--release]` builds the library with its
  `effects` feature and lays out `target/bundled/MusicCraft Effects.clap` and
  `target/bundled/MusicCraft Effects.vst3` for the current platform (nih-plug's
  bundler); the same command builds the instruments bundle
  (`docs/spec/plugin-instruments.md`). On macOS, `cargo xtask
  bundle-universal` builds bundles holding Apple Silicon and Intel code.
  Install by copying them to the platform's CLAP and VST3 folders.
- **Release downloads.** Each release has `MusicCraft-Plugins-<v>-<os>.zip`
  for Linux (x86-64), Windows (x64) and macOS (universal, ad-hoc signed),
  with both bundles in both formats, the license and a `README.txt` naming
  each system's plugin folders (`packaging/portable/README-plugins.txt`).
  The Release workflow checks every bundle before it publishes: present,
  the right binary inside (universal on macOS), the CLAP entry point
  exported on Linux.

## Tests

- Every exported effect: the IDs are unique and frozen; the parameter list is
  the spec table minus routing parameters plus the plugin's own switches.
- Normalized values round-trip for every parameter, display text matches the
  app's knob text, and typed text parses back.
- Our own CLAP host and VST3 host load the bundled library from disk, list all
  44 effects, and the output of each effect processed through the plugin
  matches `Effect::process` directly, block for block.
- State saved by one instance and loaded into another restores the values,
  through both hosts; the convolution room survives the state.
- The shared effect editor's edits (drags as one gesture, clicks as single
  sets) are tested headlessly with egui. With an X display (Xvfb), our CLAP
  host opens a plugin's editor in a host window.
- Our hosts feed a plugin's first auxiliary input from
  `External::sidechain` (silence in blocks without a call); the VST3 host
  activates every audio input bus. Through both hosts, the compressor and the
  vocoder with `sidechain` on match the mixer's effects keyed by the same
  signal (and the key changes the sound), and the pitch corrector with
  `follow` on, sent a note, matches the mixer's corrector following that
  note.
- An impulse file is loaded, resampled and handed to the audio thread; an
  unreadable path keeps the previous impulse and reports an error.
- Validators, render round trips, denormal and CPU checks:
  `docs/spec/validation.md`. Effect plugins zero subnormal input samples
  before processing.

## Sources

- Own design, on top of nih-plug (ISC, with GPLv3 VST3 export), the open CLAP
  specification and the VST3 SDK under its GPLv3 option.
- The effects' behavior is the app's (`docs/spec/mixer.md`).
- Roadmap decisions D1, D2, D5 and D6 (`docs/roadmap-wavetable-and-plugins.md`).
