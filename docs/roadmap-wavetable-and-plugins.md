# Roadmap: a wavetable synth, and built-in plugins outside the app

Two goals for the next release:

1. **A built-in wavetable synth** that covers what people use the most popular
   commercial wavetable synth for (Xfer Serum 2 is the reference). It is a
   clean-room design with its own name, artwork, wavetables and presets.
2. **MusicCraft's instruments and effects as plugins** (CLAP, then VST3), so
   they load in other DAWs.

Clean-room rules (`docs/clean-room.md`) apply throughout.

- Allowed sources: the reference synth's **public** feature list and manual
  (what it does), textbook DSP and our own design.
- Never use its binaries, factory wavetables, presets, skins or name.
- Never use its preset file format.
- Plugin formats: the open CLAP specification and the VST3 SDK under its GPLv3
  licence option.

## Decisions (owner, 2026-10-18)

| # | Question | Decision |
| --- | --- | --- |
| D1 | **`unsafe` for plugin export.** | **Approved**, for `musiccraft-plugin-export` and its editor adapter `musiccraft-plugin-editor`, under the hosting crates' rules (every `unsafe` carries a `SAFETY:` comment). Recorded in CLAUDE.md and ARCHITECTURE.md. |
| D2 | **Framework or our own wrapper.** | **nih-plug**, which is Rust. Its core is ISC-licensed and its VST3 export is GPLv3; both are compatible with our GPL-3.0-or-later. It comes from GitHub (not crates.io) and is pinned in `Cargo.lock`. Its egui adapter is egui 0.31 while our widgets are 0.36, so we write a small editor adapter of our own on the same window layer (baseview) with egui 0.36. The app and the plugins then share one set of widgets. |
| D3 | **Name of the synth.** | **Antiserum** (a medical term for a serum of antibodies). The owner chose it as a dictionary word. Our own artwork and presets keep it distinct, and no other product's name, logo or styling is used. |
| D4 | **Wavetable import.** | Read WAV wavetables and honour the `clm ` frame-size chunk when present. That is interoperability with a public convention, which is lawful. Never read the reference synth's own preset or bank formats. |
| D5 | **Plugin GUIs.** | **Full editors everywhere**: the plugins must be usable in any host, with our own editor on Linux (X11), Windows and macOS from the first plugin release. The host's generic UI is only a fallback. |
| D6 | **Formats.** | The most accessible across platforms: **CLAP and VST3** on Linux, Windows and macOS (VST3 is the most widely supported, CLAP the most capable), plus nih-plug's **standalone** app for each plugin. AU (Logic) is not supported by nih-plug; it could come later through the open-source clap-wrapper. |

## Where we start (survey, 2026-10-18)

- **Synth today** (`engine/voice.rs`):
  - Oscillators: 2 unison oscillators (up to 7 voices, PolyBLEP), a sub and a third oscillator.
  - Filters: 2 state-variable filters (low, band, high pass), in series, parallel or split.
  - Modulation: an amp ADSR, an attack/decay mod envelope, 2 LFOs, and a 3-slot mod matrix.
  - There is no wavetable code.
- **Instrument DSP is inside the engine** (`Voice` is crate-private and reads `ChannelParams`). It has to move into an engine-free crate before it can become a plugin.
- **Effect DSP is already engine-free.** `musiccraft-dsp::Effect` has 47 effects behind `Effect::new(kind, rate)` and `process(l, r, params, ctx)`, which is plugin-ready. Only `Send` and `BandSplit` need the mixer.
- **Parameters:**
  - Instruments share one flat table of 160 parameters (153 in use).
  - Effects have up to 24 parameters each.
  - `ParamSpec` (name, range, default, unit, integer) maps cleanly to plugin parameters. Stable numeric IDs are missing.
- **Reusable pieces:**
  - `musiccraft-formula`: expressions that allocate nothing.
  - `fft.rs` and `resynth.rs`: spectral analysis.
  - Granular, sample and zone playback.
  - The preset system (`params/presets.rs`).
  - Our own CLAP and VST3 hosts, which can load exported plugins in tests.

## Phases

Each phase ends with its spec written first, tests, the quality gates passing,
and a PR. Sizes are rough: S = days, M = about a week, L = several weeks.

### Phase 1: groundwork (M)

- **1.1 Engine-free instruments.** Move every instrument's voice DSP into a new crate, `musiccraft-instruments`, with an API like the effects: `new(rate)`, then `note_on/off`, `set_param` and `process`. The engine keeps voice allocation and routing and calls into it. Output must be bit-identical: golden renders of every instrument before and after.
- **1.2 Parameters per instrument (done).** Parameter names are the stable IDs. Project files, commands and plugin parameter IDs already use them, so nothing has to migrate. The `frozen_names` test (`musiccraft-params/frozen-names.txt`) fails if a name is renamed or removed. `musiccraft_instruments::Kind` lists the parameters each instrument uses, in this order: the envelope, its sound parameters, then the host's `NOTE_FUNCTIONS` (arpeggiator, swing, timing, echo, cut groups and polyphony; a plugin wrapper implements these around the voices). The existing instruments keep the shared 160-slot table. Antiserum gets its own table (about 400 parameters) in Phase 2.
- **1.3 Plugin parameter model (done).** `musiccraft_params::describe` provides labels, choice lists, display text and parsing of typed text (units and choice names). Normalized values come from `ParamSpec::to_unit` and `from_unit`. Grouping into modules follows each kind's list order.

### Phase 2: Antiserum core (L, done)

Done in `musiccraft-antiserum` (spec: `docs/spec/antiserum.md`), with these choices:
- **Parameters:** 398 of them; names and order are frozen.
- **Warps:** the FM, AM and RM source is chosen per oscillator.
- **Wavetables:** the default is a basic sine–triangle–saw–square morph; the factory tables come in Phase 3.
- **Benchmark:** about 4× real time on one core of the CI-class machine (8 notes × 3 × 16 unison, both filters, bend warp). `--example bench` runs in CI.


- **Oscillators:**
  - 3 wavetable oscillators. Each table holds up to 256 frames of 2048 samples, band-limited per octave (mipmaps from an FFT) so it does not alias.
  - Wavetable position, with modulation and morphing between frames.
  - Per oscillator: level, pan, octave/semi/fine, phase and random phase.
- **Warp modes**, two per oscillator: sync, bend (+/−), PWM, asymmetric, mirror, flip, quantize, remap, and FM/AM/RM from another oscillator or the sub/noise.
- **Unison:** up to 16 voices with detune, blend, stereo width and stacked octaves.
- **Sub and noise:** a sub oscillator (sine, triangle, saw, square; octave), and a noise oscillator with a set of generated noise tables, pitch tracking and one-shot mode.
- **Filters:** 2 filters, each with many types:
  - low, high, band, notch and peak pass, at 12 and 24 dB
  - ladder, diode and comb
  - formant and vowel
  - phaser-style and flanger-style
  - Each filter has drive, key tracking and mix, with serial or parallel routing and per-oscillator filter sends.
- **Modulation:**
  - 4 envelopes (ADSR with curves) and 8 LFOs (drawable shapes; trigger, envelope and free modes; tempo sync; delay and rise).
  - 8 macros, velocity, note, aftertouch, mod wheel, and random values per note.
- **Mod matrix:** 32 slots, each with source, destination, amount and curve. Bipolar or unipolar, with an auxiliary source as a multiplier.
- **Voicing:** polyphony, mono, legato, glide (always or only when legato, with a curve), and voice stealing. Control-rate modulation runs every 32 samples, with audio-rate paths for FM and sync.
- **Performance targets:** 16 unison voices × 3 oscillators × 8 notes running in real time. Benchmarks are added to CI.

### Phase 3: wavetable sources and editor (L)

- **Generated factory tables**, made by code with no files copied:
  - analogue shapes and PWM sweeps
  - additive series and formant/vowel sweeps
  - FM and sync sweeps
  - noise and harmonic morphs
- **Import:**
  - WAV wavetables (D4).
  - Any audio file, sliced into frames by pitch or by fixed size.
  - Single-cycle files.
- **Editor:**
  - Draw frames.
  - Edit harmonics with the FFT (magnitude and phase per partial).
  - Formula tables through `musiccraft-formula` (for example `sin(x*pi*2) * (1-y)`).
  - Morph between frames (crossfade or spectral).
  - Normalize, remove DC, reverse.
- The editor UI shows a 3D frame stack.

### Phase 4: more oscillator types (M)

Each oscillator can switch to one of these:

- **Sample:** one sample, with start, loop and warp modes.
- **Multisample:** reuses the zone sampler.
- **Granular:** reuses the granular voice.
- **Spectral:** resynthesis from `resynth.rs`, with spectral warp such as shift, stretch and filter.

### Phase 5: built-in effects, arpeggiator and sequencer (M)

- **An effects rack inside the synth:** 10 slots of existing `musiccraft-dsp` effects, among them:
  - distortion, chorus, flanger, phaser and delay
  - compressor and multiband
  - EQ, filter, reverb and frequency shift
  - a "dimension"-style widener (new and small)
  - utility
- **Arpeggiator and clip sequencer:** reuse the channel arpeggiator, plus a per-patch note pattern of 16 to 64 steps with gate, velocity and transpose.

### Phase 6: synth UI and presets (L)

- **Pages:** oscillators, mixer, filters, modulation, effects, matrix, global.
- **Interaction:**
  - Drag a modulation source onto any knob.
  - Modulation rings on knobs.
  - Draw LFO shapes, with tempo-synced views.
  - A wavetable view per oscillator.
- **Factory bank:** about 150 presets of our own sound design (basses, leads, pads, plucks, keys, FX, drums), with tags and search. Presets save as MusicCraft's own JSON, shared with the plugin version.
- **Integration:** a browser entry, a channel settings page, and automation targets for macros and parameters.

### Phase 7: plugin export (L)

**Effects: done** (spec: `docs/spec/plugin-export.md`), with these choices:
- **One library** exports all 44 effects (everything but `send`, `band_split` and `rack`): `MusicCraft Effects.clap` and `.vst3`.
- **Routing parameters** (sidechain source, followed channel) become plugin switches: a sidechain input for the compressor and vocoder, MIDI notes for the pitch corrector.
- **Editors:** the effect editor moved into a new crate, `musiccraft-editors` (not a module of `musiccraft-ui`), so plugins don't pull in projects and commands.
- **Standalone** apps live in `standalone/`, a separate workspace, because nih-plug's standalone wrapper needs an older ALSA binding than the app.
- **Convolution** offers the generated rooms only; loading an impulse file comes later.
- **Tests:** our own CLAP and VST3 hosts load the bundle from disk, and every effect matches the mixer's output. Phase 8's round trip is therefore done for effects.

Still to do: instruments, then Antiserum.

- **`musiccraft-plugin-export`, built on nih-plug:**
  - One `nih_plug::Plugin` per MusicCraft instrument (synth, FM, additive, granular, Antiserum and the others) and per effect, except `Send`/`BandSplit`.
  - Exported with `nih_export_clap!` and `nih_export_vst3!`, plus standalone binaries.
  - IDs look like `org.musiccraft.<name>`.
- **Parameters:** derived from our `ParamSpec` tables with stable IDs (Phase 1), with smoothing and automation through nih-plug.
- **State:** our own versioned JSON, so presets are shared with the app.
- **Notes and transport:** note events and transport/tempo, for synced effects.
- **Real-time rules:** nih-plug keeps processing allocation-free, and our DSP already is.
- **Packaging:** `cargo xtask bundle` builds the `.clap`, `.vst3` and standalone files for each platform through nih-plug's bundler.
- **Editors (`musiccraft-plugin-editor`, D5):** an adapter between nih-plug's `Editor`, baseview (X11, Win32, Cocoa) and egui 0.36 with the glow renderer. Every plugin opens our own editor in every host. The instrument and effect editors move into a part of `musiccraft-ui` that the app and the plugins share.

### Phase 8: validation (M)

- **Round trips:** our own CLAP and VST3 hosts load every exported plugin in tests. An offline render inside a plugin must match the in-app render.
- **Validators:** `clap-validator` (MIT) and the VST3 SDK validator run in CI on Linux.
- **Manual smoke tests** in two or three free or open hosts: Ardour, Bitwig's demo, REAPER's evaluation version.
- **Performance and denormal checks.**

### Phase 9: release (S)

- Plugin bundles in the release artifacts for Linux, Windows and macOS.
- Install docs: CLAP and VST3 folders per OS.
- `docs/parity.md` rows and a changelog.

## Suggested order and parallel tracks

1. **Phase 1**, which unblocks both goals.
2. **Track A (Antiserum):** Phase 2, then 3, then 5, then 6, then 4.
3. **Track B (plugins):** Phase 7 with **effects first** (they are already engine-free, which proves the pipeline and the editor adapter early), then instruments, then Antiserum, then Phase 8.
4. **Phase 9.**

The release ships CLAP, VST3 and standalone builds with our own editors on all
three platforms (D5, D6). Antiserum's spectral, granular and multisample
oscillators (Phase 4) can follow in a point release.

## Risks

- **Scope.** The reference synth is many years of work. We match its *workflow and capabilities*, not every detail. Everything in Phase 4 can slip.
- **CPU.** Unison × 3 oscillators × mipmapped tables needs care: SIMD-friendly loops and control-rate modulation. Benchmarks are added early.
- **Embedded GUIs** differ per host and platform (D5). baseview already handles X11, Win32 and Cocoa for many nih-plug plugins; our risk is the egui 0.36 adapter, which is proved early on the effects.
- **Git dependencies.** nih-plug, baseview and their forks come from GitHub, pinned by revision in `Cargo.lock`. Builds need GitHub access, and updates are deliberate.
- **Parameter renames** would break project files and plugin automation. The names are frozen by a test (1.2), so a rename can't happen by accident.

## Sources

- The reference synth's public product page and user manual: feature lists only.
- The CLAP specification (MIT) and the VST3 SDK documentation (GPLv3 option).
- `docs/clean-room.md`.
- The survey of this repository (2026-10-18).
