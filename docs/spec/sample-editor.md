# Spec: sample editor

Offline edits of the sample a channel plays (sampler, granular and resynth
channels): trimming, cutting, silencing, fades, gain, normalizing,
reversing, time-stretching, pitch-shifting and pitch correction. They cover
the editing jobs of an audio editor and of offline pitch and time tools,
with our own design.

## Behaviour

- `sample.edit <channel> <operation> [arguments]` reads the channel's
  sample, applies the operation and writes the result as a new 32-bit float
  WAV file at the sample's own rate in `edited/` beside the project
  (`<sample name>-<n>.wav`, the first free number), then points the channel
  at it (stored relative to the project). The original file is never
  changed. Undo points the channel back at the previous sample (the edited
  file stays on disk). The project must have been saved, so the files have
  a home.
- Times are in seconds from the start of the sample and are clamped to its
  length; a range needs `start` before `end`.
- Operations:
  - `trim <start> <end>`: keep only the range.
  - `cut <start> <end>`: remove the range, joining what is left.
  - `silence <start> <end>`: replace the range with silence.
  - `fade_in <start> <end>` / `fade_out <start> <end>`: the level ramps
    linearly from 0 to 1 (or 1 to 0) across the range; for a fade in,
    everything before the range is silenced, for a fade out everything
    after it.
  - `gain <dB> [start end]`: scale the level (−48 to 24 dB).
  - `normalize [peak dB]`: scale so the loudest sample reaches the peak
    (default 0 dB, −24 to 0).
  - `reverse [start end]`: play the range backwards.
  - `stretch <ratio>`: change the length by `ratio` (0.25–4) without
    changing the pitch (waveform-similarity overlap-add: 2048-sample
    Hann frames at half-frame hops, each moved by up to 512 samples to line
    up with the last).
  - `pitch <semitones>`: shift the pitch (−24 to 24) without changing the
    length (stretch, then resample back).
  - `tune <key> <scale> [speed ms]`: run the pitch corrector over the whole
    sample (key 0–11 from C, scale as for `pitch_correct`, speed default
    0 for a hard snap), for a monophonic voice or instrument.
- The channel editor shows the waveform; dragging across it selects a
  range (shown shaded, with its times). Buttons apply trim, cut, silence,
  fade in, fade out, reverse and gain ±3 dB to the selection (or the whole
  sample when nothing is selected), normalize, and stretch, pitch and tune
  with their values.

## Sources

- Own design. The set of operations mirrors what audio editors and offline
  pitch and time tools in pattern-based DAWs are publicly described as
  doing; WSOLA time-stretching is the published method of Verhelst and
  Roelands (1993); the pitch correction reuses our real-time corrector.
