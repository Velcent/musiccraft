# Antiserum: more oscillator types

Roadmap Phase 4. Each of Antiserum's three oscillators can switch from a
wavetable to a sample, a multisample, a granular or a spectral oscillator.
Everything here follows `docs/spec/antiserum.md`: sources are built on the
main thread, and voices play them without allocating, locking or panicking.

## Parameters

Appended after the effect modules, for each oscillator in turn (`osc1_…`,
`osc2_…`, `osc3_…`):

| Parameter | Range | Default | Used by |
| --- | --- | --- | --- |
| `type` | wavetable, sample, multisample, granular, spectral | wavetable | all |
| `root` | 0–127 | 60 | sample, granular |
| `sample_mode` | repitch, fixed, stretch | repitch | sample |
| `loop` | off, forward, ping-pong | off | sample, multisample, spectral (on or off) |
| `loop_start`, `loop_end` | 0–1 of the sample | 0, 1 | sample, multisample |
| `grain_size` | 5–500 ms | 60 ms | granular, stretch |
| `grain_density` | 1–100 grains/s | 20 | granular |
| `grain_spread` | 0–1 | 0.1 | granular |
| `grain_pitch` | 0–12 st (random) | 0 | granular |
| `grain_scan` | −2–2 | 0 | granular |
| `spectral_shift` | −1000–1000 Hz | 0 | spectral |
| `spectral_stretch` | −1–1 | 0 | spectral |
| `spectral_tilt` | −1–1 | 0 | spectral |
| `spectral_speed` | 0–4 | 1 | spectral |

The oscillator's existing controls still apply to every type: level, pan,
octave, semitone and fine (pitch), position, unison, the warps (below), filter sends, and the mod matrix. **Position** means:

- sample and multisample: where playback starts;
- granular: where scanning starts;
- spectral: the time in the analysis where playback starts.

### Unison

Unison works on every type, with the wavetable's detune, blend, width and
stack:

- sample and multisample: each unison voice is a detuned read of the sample;
- granular and the sample type's stretch mode: each new grain starts once per
  unison voice, detuned and panned like that voice;
- spectral: each unison voice plays its own set of partials, detuned like
  that voice.

### Warps

The two warps work on every type. On a wavetable they reshape the cycle as
before (`docs/spec/antiserum.md`). On the other types:

| Warp | Spectral | Sample, multisample, granular |
| --- | --- | --- |
| sync, bend ±, PWM, asym ±, mirror, flip, quantize, remap | each partial's phase is warped, as a one-partial wavetable cycle | see below |
| FM | moves each partial's phase by the mod source | moves the read position by the mod source, up to 2 cycles of the note |
| AM, RM | the output level, as for wavetables | the output level, as for wavetables |
| PWM, flip | as above | gates (PWM) or inverts (flip) the part of each note-pitch cycle the wavetable would |
| quantize | as above | holds each read value for up to 32 frames (a lower sample rate) |
| sync, bend ±, asym ±, mirror, remap | as above | no effect: they reshape a wave's cycle, and a sample has none. The editor dims them for these types. |

"Each note-pitch cycle" is a phase that runs at the oscillator's pitch (each
unison voice its own), so level warps follow the note as they do on a
wavetable.

## Sources

Each oscillator has a **source** besides its wavetable: one sample, and a
list of zones for the multisample. `Antiserum::set_source(osc, Arc<Source>)`
replaces it and returns the old one (like `set_wavetable`). A `Source` holds:

- the sample (a `SampleBuffer`), or none;
- its harmonic analysis (`musiccraft_dsp::resynth::HarmonicTable`, the same
  analysis the resynth instrument uses), measured when the source is built:
  one for each channel of a stereo sample, one for a mono sample;
- the zones: a sample each, with low and high key, root key, and low and
  high velocity (the zone sampler's fields).

An oscillator whose type needs a sample but has none is silent.

## Types

- **Sample.** The sample in stereo: its left channel goes left and its right
  channel right, then pan and unison width place it like any oscillator. A
  mono sample plays in both channels. (The mono mix feeds other oscillators'
  FM, AM and RM.)
  - *repitch:* plays faster or slower so that `root` sounds at its own
    pitch, as a sampler does.
  - *fixed:* plays at the sample's own speed whatever the key (drums, loops;
    octave, semitone and fine still transpose it).
  - *stretch:* plays at the key's pitch while moving through the sample at
    its own speed, made of overlapping grains (`grain_size`).
  - Loops: *forward* jumps from `loop_end` back to `loop_start`; *ping-pong*
    turns round at either end. Without a loop, playback stops at the end of
    the sample.
  - Unison voices play detuned copies, spread in the stereo field as for
    wavetables.
- **Multisample.** At note start, the first zone whose key and velocity
  ranges hold the note plays, as in the zone sampler. Its root key sets the
  pitch. The loop settings apply to that zone's sample.
- **Granular.** The granular sampler's method: Hann-windowed grains of
  `grain_size`, started `grain_density` times a second at the scan point.
  Each grain is offset at random by up to `grain_spread` of the sample and
  detuned at random by up to `grain_pitch` semitones. The scan point starts at
  *position* and moves at `grain_scan` times real time (0 holds it, negative
  moves backwards). Grains play at the key's pitch relative to `root`, in
  stereo like the sample type.
- **Spectral.** Resynthesis from the analysis: 32 sine partials whose levels
  follow the analysis from *position*, at `spectral_speed` times real time
  (looping when `loop` is on). Partial n sounds at
  `f · n^(1 + stretch/2) + shift`, where `f` is the note's frequency, and is
  weighted by `n^(2·tilt)` (darker below 0, brighter above). Partials above
  Nyquist are dropped. With a stereo sample each partial has a left and a
  right level, from the two channels' analyses, so the sample's stereo image
  carries over.

## Patch

A patch gains `sources`, one per oscillator:

```json
"sources": [
  { "sample": "builtin:crash" },
  {},
  { "zones": [ { "sample": "pads/c3.wav", "low": 0, "high": 59, "root": 48 } ] }
]
```

`sample` is a file path or a `builtin:` name. A zone's velocity range
defaults to 1–127. Sample paths are resolved like the sampler's (relative to
the project and through sample-path mappings). A sample that cannot be loaded
plays as silence, with a warning.

## Commands

| Command | Does | Undo |
| --- | --- | --- |
| `antiserum.set_sample <ch> <osc> <sample\|->` | Sets (or clears) an oscillator's sample. | previous sample |
| `antiserum.add_zone <ch> <osc> <sample> [low] [high] [root]` | Adds a multisample zone (0, 127, 60 by default). | removes it |
| `antiserum.set_zone <ch> <osc> <zone> <low> <high> <root>` | Changes a zone's ranges and root. | previous values |
| `antiserum.remove_zone <ch> <osc> <zone>` | Removes a zone. | adds it back |

The type and the other settings are parameters (`channel.set_param`).

## Editor

On the Oscillators page, each oscillator has a **type** menu. Its knobs and
its picture follow the type:

- *wavetable:* as before;
- *sample* and *granular:* the sample's waveform with the start and loop
  points, a *Sample…* button (a path prompt) and the type's knobs;
- *multisample:* the zones, each with its sample and its low, high and root
  keys, plus *Add zone…* and remove buttons;
- *spectral:* the partial levels at the start position, *Sample…* and the
  spectral knobs.

## Tests

- Repitch plays the root key at the sample's own speed and an octave up at
  twice the speed. Fixed ignores the key. Stretch keeps the duration.
- Forward and ping-pong loops keep a short sample sounding; no loop stops.
- Multisample picks the zone by key and velocity.
- Granular makes sound from a sample, and `grain_scan` 0 holds still.
- Spectral sounds at the note's pitch; shift moves the partials; tilt changes
  the brightness.
- Every type, with no sample, is silent and stays finite.
- A stereo sample keeps its sides apart in the sample, granular and
  spectral types.
- Unison widens and thickens granular, stretch and spectral oscillators.
- FM, AM and quantize change sample-type output; sync leaves it unchanged;
  every warp changes a spectral oscillator.
- Commands undo and redo exactly; a sample swap reaches a running synth.

## Sources

- Own design. Sample, multisample, granular and spectral oscillators are
  common in software synthesizers, as their public manuals describe; no
  layout or names are taken from any of them.
- The granular and resynthesis methods are MusicCraft's own (the granular
  sampler and resynth instruments, `docs/spec/channel-rack.md`), after the
  textbook methods: windowed grains, and additive resynthesis of measured
  harmonics.
