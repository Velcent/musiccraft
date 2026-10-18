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

## Decisions needed before work starts

| # | Question | Recommendation |
| --- | --- | --- |
| D1 | **`unsafe` for plugin export.** Exporting a plugin means exporting C entry points, as `musiccraft-clap-testplugin` and `musiccraft-vst3-testplugin` already do. A new crate would need the same exception (CLAUDE.md requires your approval). | Approve one new crate, `musiccraft-plugin-export`, under the same rules as the hosting crates: every `unsafe` carries a `SAFETY:` comment, and nothing else changes. |
| D2 | **Framework or our own wrapper.** nih-plug (ISC licence) does CLAP and VST3 but is a large dependency with its own GUI and threading conventions. | Our own thin wrapper on the bindings we already use (`clap-sys`, `vst3`). The test plugins prove it works, and the wrapper stays small and fits our audio-thread rules. |
| D3 | **Name of the synth.** We cannot use "Serum". | Pick a name (ideas: *Prism*, *Facet*, *Tessera*). The plan calls it **the wavetable synth** until then. |
| D4 | **Wavetable import format.** Third-party wavetable packs are WAV files of 2048-sample frames. Many carry a `clm ` chunk that gives the frame size. That chunk is a de-facto convention, publicly described and read by several open-source synths. | Read plain WAV wavetables and honour the `clm ` frame size when present. Do **not** read the reference synth's preset or wavetable bank formats. |
| D5 | **Plugin GUIs.** Embedding our egui editors in another host's window is the largest UI risk. It is X11 first, as our plugin hosting already is. | Phase it: ship with the host's generic parameter UI first, then embedded editors on Linux, then Windows and macOS. |
| D6 | **Formats and platforms.** | CLAP first, on Linux, Windows and macOS. Then VST3. LV2 later only if wanted. |

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
- **1.2 Parameter tables per instrument.** Give each instrument its own `ParamSpec` table with **stable IDs**, instead of the shared 160-slot table. The new synth needs about 400 parameters. Project files keep loading through a name-based migration.
- **1.3 Plugin parameter model.** Add stable IDs, normalized values, automation flags, value text and parsing, choice lists, and grouping into modules. All of it is derived from the parameter tables.

### Phase 2: wavetable synth core (L)

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

### Phase 7: plugin export (L, needs D1 and D2)

- **CLAP wrapper (`musiccraft-plugin-export`):**
  - Every instrument (synth, FM, additive, granular, the wavetable synth and others) and every effect, except `Send`/`BandSplit`, becomes a plugin.
  - Plugin IDs look like `org.musiccraft.<name>`.
- **CLAP extensions:** audio and note ports, params (with modulation where hosts support it), state (JSON, versioned), latency, tail, voice info, and transport/tempo for synced effects. It can also be one `.clap` file with a factory listing every plugin.
- **Real-time rules:** the processor uses the same no-allocation rules as the engine. Parameter changes go through lock-free queues, and sample-rate changes rebuild on the main thread through `activate`.
- **Packaging:** `cargo xtask bundle` builds `.clap` (and later `.vst3`) bundles for each platform.
- **VST3 wrapper** after CLAP, sharing the same core: a combined processor and controller with IDs derived from the plugin IDs.
- **Editors:** first the host's generic UI (D5); then our egui editors embedded through `musiccraft-plugin-window` (X11), then Windows (HWND) and macOS (NSView).

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

1. **D1–D6 answered.**
2. **Phase 1**, which unblocks both goals.
3. **Track A (synth):** Phase 2, then 3, then 5, then 6, then 4.
4. **Track B (plugins):** Phase 7 with **effects first** (they are already engine-free, which proves the pipeline early), then instruments, then the wavetable synth, then Phase 8.
5. **Phase 9.**

A first public release can ship with Phases 1, 2, 3, 6 and the CLAP part of 7
and 8. Spectral, granular and multisample oscillators (Phase 4), VST3 and
embedded editors on Windows and macOS can follow in a point release.

## Risks

- **Scope.** The reference synth is many years of work. We match its *workflow and capabilities*, not every detail. Everything in Phase 4 can slip.
- **CPU.** Unison × 3 oscillators × mipmapped tables needs care: SIMD-friendly loops and control-rate modulation. Benchmarks are added early.
- **Embedded GUIs** differ per host and platform (D5). Generic UIs keep plugins usable meanwhile.
- **Parameter migration** (1.2) touches every project file. It needs good round-trip tests on old files.

## Sources

- The reference synth's public product page and user manual: feature lists only.
- The CLAP specification (MIT) and the VST3 SDK documentation (GPLv3 option).
- `docs/clean-room.md`.
- The survey of this repository (2026-10-18).
