# Antiserum: the wavetable synth core

Antiserum is MusicCraft's wavetable synthesizer. This spec covers its sound
engine (roadmap Phase 2): the crate `musiccraft-antiserum`, which is
independent of the engine, the app and any plugin format. Wavetable sources
and the editor (Phase 3), more oscillator types (Phase 4), the built-in
effects and sequencer (Phase 5), the UI and presets (Phase 6) and plugin export
(Phase 7) build on it.

## API

- `Antiserum::new(sample_rate)` builds a synth with every parameter at its
  default and the basic wavetable on all three oscillators. It allocates. So do
  `set_wavetable` and `Wavetable::new`, which run on the main thread.
- These calls run on the audio thread and never allocate, lock or panic:
  - `set_param(index, value)` (the value is sanitized)
  - `note_on(key, velocity)` and `note_off(key)`
  - `set_pitch_bend(-1..1)`, `set_mod_wheel(0..1)`, `set_aftertouch(0..1)`
    (channel) and `set_poly_aftertouch(key, 0..1)`
  - `set_tempo(bpm)`, `set_lfo_shape(lfo, shape)`, `all_notes_off()`, `reset()`
  - `set_sequence(&Sequence)` and `set_beat_position(beats)` (the effects
    rack, arpeggiator and sequencer: `docs/spec/antiserum-fx-arp.md`)
  - `process(left, right)`, which overwrites the two buffers
- Events apply at the start of the next `process` call. A host that needs
  sample-accurate events splits its block at each event.
- `set_wavetable(osc, Arc<Wavetable>)` returns the table it replaces, so the
  caller drops it where freeing memory is allowed.
- `params()` is the parameter table (`ParamSpec`s). Parameter names are stable
  IDs, and their **order** is stable too: a mod-matrix destination stores a
  parameter's index. New parameters are only ever appended (the
  `frozen_names` test enforces this).

## Wavetables

- A table holds 1 to 256 frames of 2048 samples each.
- On creation, every frame goes through an FFT and is rebuilt at 11
  band-limited levels, one per octave, with 1024, 512, … down to 1 harmonic.
  Each level stores at least four samples per harmonic (64 at the least,
  2048 at the most). DC is removed.
- An oscillator reads the level whose highest harmonic stays below Nyquist
  for the pitch it is playing, so it does not alias. Reads interpolate
  linearly between samples and, by the wavetable position, between the two
  nearest frames (frame morphing).
- `Wavetable::basic()` is the default table: 64 frames morphing from sine to
  triangle to saw to square.

## Oscillators (`osc1_` … `osc3_`)

| Parameter | Range | Meaning |
| --- | --- | --- |
| `on` | off/on | Osc 1 is on by default; 2 and 3 are off. |
| `level`, `pan` | 0..1, −1..1 | |
| `octave`, `semi`, `fine` | −4..4, −12..12, −100..100 ct | |
| `position` | 0..1 | Wavetable position (frame). |
| `phase`, `random_phase` | 0..1 | Start phase, and how much of a cycle each note and unison voice randomizes it by. |
| `warp1`, `warp1_amount`, `warp2`, `warp2_amount` | mode, 0..1 | Two warps, applied in order. |
| `mod_source` | other osc A, other osc B, sub, noise | Source for the FM, AM and RM warps. "Other A" and "B" are the two other oscillators, in order. |
| `unison` | 1..16 | Unison voices. |
| `detune`, `blend`, `width` | 0..1 | Detune spreads voices up to ±100 cents (on a squared curve); blend sets the outer voices' level against the centre; width sets their stereo spread. |
| `stack` | off, +1 oct, +2 oct, −1 oct, ±1 oct | Moves every second unison voice by octaves. |
| `filter1`, `filter2` | 0..1 | Send to each filter (osc defaults: 1 and 0). |

Warp modes reshape the read phase `p` (0..1) with an amount `a`:

- **off**
- **sync**: hard sync, at up to 16 times the pitch.
- **bend+**, **bend−**: phase pushed later or earlier on a power curve.
- **PWM**: plays the cycle in the first part of the period and holds silence
  for the rest (up to 98 %).
- **asym+**, **asym−**: moves the cycle's midpoint later or earlier.
- **mirror**: blends towards the cycle played forward and then backward.
- **flip**: inverts the wave after a point that moves from the end towards the
  start.
- **quantize**: steps the phase (from 256 steps down to 2).
- **remap**: an S-curve on the phase.
- **FM**: through-zero phase modulation by the mod source.
- **AM**, **RM**: amplitude and ring modulation by the mod source.

Cross-modulation uses the source's previous sample, so the order of the
oscillators does not matter.

## Sub and noise

- **Sub (`sub_`)**: `on`, `shape` (sine, triangle, saw, square),
  `octave` (−4..0), `level`, `pan`, `filter1`, `filter2`.
- **Noise (`noise_`)**: `on`, `type`, `level`, `pan`, `pitch` (−48..48 st),
  `track` (key tracking), `one_shot`, `filter1`, `filter2`.
  - Types (generated tables of 65 536 samples): white, pink, brown, crackle,
    digital.
  - A looping noise starts at a random place. A one-shot starts at the
    beginning and stops at the end.
  - With tracking on, it plays at its natural rate at key 60.

## Filters (`filter1_`, `filter2_`)

- `on`, `type`, `cutoff` (20 Hz..20 kHz), `resonance`, `drive`, `key_track`
  (0..1; at 1 the cutoff follows the key, centred on key 60) and `mix`
  (dry/wet).
- Types:
  - low-, high- and band-pass, notch and peak, at 12 and 24 dB per octave
    (state-variable filters)
  - ladder (four nonlinear poles)
  - diode (three asymmetric nonlinear poles)
  - comb+ and comb− (feedback comb, tuned to the cutoff)
  - formant (two peaks a fixed ratio apart, following the cutoff)
  - vowel (three formant peaks morphing a–e–i–o–u by the cutoff)
  - phaser (four all-pass stages at the cutoff, with feedback)
  - flanger (a short delay tuned to the cutoff, mixed with the input)
- `filter_routing`: **serial** (filter 1, then filter 2; the oscillators'
  filter-2 sends join after filter 1) or **parallel**.
- A source's dry share (`1 − filter1 − filter2`, at least 0) goes straight to
  the output.
- A filter that is off passes its input unchanged.

## Modulation

- **Envelopes (`env1_` … `env4_`)**: `attack`, `hold`, `decay`, `sustain`,
  `release`, plus `attack_curve`, `decay_curve` and `release_curve` (−1..1;
  0 is linear). Envelope 1 is the amplitude envelope.
- **LFOs (`lfo1_` … `lfo8_`)**:
  - `shape`: sine, triangle, saw up, saw down, square, sample and hold, smooth
    random, or custom (a drawn shape of up to 32 points with curves).
  - `mode`: trigger (restarts on each note), envelope (plays one cycle and
    holds its end), or free (follows the synth's clock).
  - `sync` with `sync_rate` (8 bars … 1/32, with dotted and triplet values), or
    free `rate` (0.01..50 Hz).
  - `delay`, `rise` and `phase`.
- **Other sources**: velocity, note (key / 127), aftertouch (poly if sent,
  else channel), mod wheel, pitch bend (as 0..1), a random value per note,
  and the macros `macro1` … `macro8`. Every source is unipolar (0..1).
- **Mod matrix (`mod1_` … `mod32_`)**: `source`, `dest`, `amount` (−1..1),
  `curve` (−1..1), `bipolar` and `aux`.
  - A slot runs when its source and its destination are both set.
  - The curve bends the source (`s^(e^(−2.5·curve))`), and `bipolar` then maps
    it to −1..1.
  - A set `aux` source multiplies the result.
  - The destination is a parameter index plus one (0 means none). Only
    continuous parameters can be destinations.
  - Modulation adds `amount × source` to the destination's knob position
    (0..1, the same mapping as the UI), clamped to the range.
- Modulation runs per voice, every 32 samples (the control rate). The
  amplitude envelope is interpolated per sample. FM, AM, RM and sync run at
  audio rate.

## Voicing (global)

| Parameter | Range | Meaning |
| --- | --- | --- |
| `volume` | 0..1 | Output gain (squared). |
| `velocity` | 0..1 | How much velocity scales the level. |
| `polyphony` | 1..32 | |
| `voice_mode` | poly, mono, legato | |
| `glide`, `glide_mode`, `glide_curve` | 0..5 s; always or legato only; −1..1 | |
| `bend_range` | 0..48 st | |
| `macro1` … `macro8` | 0..1 | |

- **Poly**: each note takes a free voice. When none is free, the releasing
  voice that started first is stolen, else the oldest voice. A stolen voice
  fades out over 2 ms in a spare slot (8 spares), so stealing does not click.
- **Mono**: one voice. A new note restarts the envelopes. Releasing a key goes
  back to the most recent key still held (up to 16 are remembered).
- **Legato**: like mono, but a note played while another is held keeps the
  envelopes running.
- **Glide**: moves from the previous note's pitch over `glide` seconds. In
  "legato only" mode it glides only between overlapping notes. The curve is
  linear at 0, approaches faster when positive and slower when negative.
- **Pitch**: key + octave/semi/fine + bend × `bend_range` + glide.

## Performance

- 8 notes × 3 oscillators × 16 unison voices must render faster than real
  time at 48 kHz in a release build.
- `cargo run --release -p musiccraft-antiserum --example bench` reports the
  real-time factor, and CI runs it.

## Sources

- Own design. The feature set (wavetable oscillators with warps, unison, sub
  and noise, two multimode filters, envelopes, LFOs, macros and a mod matrix)
  is the common vocabulary of wavetable synthesizers, as described in their
  public product pages and manuals. No layout, preset, wavetable, artwork or
  name is taken from any of them.
- Band-limiting through per-octave mipmaps built with an FFT is standard
  practice in the public DSP literature.
- The state-variable filters use the topology-preserving transform from
  Andrew Simper's published paper ("Linear trapezoidal integrated SVF"), as
  `musiccraft-dsp` already does. The ladder and diode filters are simple
  nonlinear one-pole cascades of our own (after the general structure
  described in public papers on virtual-analogue filters, for example
  Välimäki and Huovilainen, "Oscillator and filter algorithms for virtual
  analog synthesis", 2006). The vowel formant frequencies are textbook
  averages for adult speakers.
