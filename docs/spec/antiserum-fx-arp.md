# Antiserum: effects rack, arpeggiator and sequencer

Roadmap Phase 5. Everything here lives in `musiccraft-antiserum` and follows
its rules (`docs/spec/antiserum.md`): built in `Antiserum::new` on the main
thread, then processed without allocating, locking or panicking.

## Parameters

All new parameters are appended to the table, after `mod32_aux`, in this
order: the arpeggiator, the sequencer, the ten effect slots, then each effect
module's parameters. Names and order are frozen like the others
(`frozen-names.txt`). A mod-matrix destination can be any continuous one of
them.

## Effects rack

Ten **slots** set the order of the effects after the voices are mixed:
`fx_slot1` … `fx_slot10`, each a choice of `none` or one of the modules below
(default `none`, so a new patch is dry). A module plays at most once, in the
first slot that names it; a later slot naming it again is skipped.

Each **module** is one effect with its own parameters, named
`fx_<module>_<parameter>`. Module parameters keep their values when the
module is taken out of the rack, so moving it to another slot changes
nothing else.

| Module | Effect (`musiccraft-dsp`) | Parameters |
| --- | --- | --- |
| `dimension` | new, below | `size`, `mix` |
| `distortion` | Distortion | `drive`, `mix`, `output`, `mode`, `threshold` |
| `chorus` | Chorus | `rate`, `depth`, `delay`, `mix`, `voices`, `tone` |
| `flanger` | Flanger | `rate`, `depth`, `delay`, `feedback`, `mix` |
| `phaser` | Phaser | `rate`, `depth`, `feedback`, `stages`, `mix`, `warm` |
| `delay` | Delay | `time`, `feedback`, `mix`, `ping_pong`, `damping`, `offset`, `mod_rate`, `mod_depth`, `low_cut`, `diffusion`, `saturation` |
| `compressor` | Compressor | `threshold`, `ratio`, `attack`, `release`, `makeup` (no sidechain) |
| `multiband` | Multiband | its 14 parameters |
| `eq` | ParametricEq | its 7 bands × (frequency, gain, Q) and `output` |
| `filter` | Filter | `mode`, `cutoff`, `resonance`, `mix` |
| `reverb` | HallReverb | `predelay`, `decay`, `size`, `damping`, `low_cut`, `early`, `mod_depth`, `mod_rate`, `width`, `mix` |
| `shift` | FreqShift | `shift`, `mix` |
| `utility` | Stereo, plus a gain | `width`, `balance`, `invert_left`, `invert_right`, `gain` (−24..24 dB) |

- Each module's ranges, defaults, display text and choices are those of its
  mixer effect (`musiccraft_dsp::describe`), so the synth's effects behave
  like the mixer's. A module takes the parameters its effect has today; a
  parameter added to the mixer effect later stays at its default inside the
  synth unless it is appended here.
- Effects follow the synth's tempo (`set_tempo`) and a beat clock that runs
  from the synth's creation (or from `set_beat_position`, which a host calls
  each block while its transport plays).
- `reset()` also clears the effects' tails; `all_notes_off()` does not.

**Dimension** (new, small): a widener in the style of classic "dimension"
chorus units. The mid signal feeds two short delays (5–25 ms, set by `size`)
swept slowly in opposite directions; one is added to the left channel and
the other subtracted from the right. `mix` sets how much. Because the added
signals are opposite, they mostly cancel when the output is summed to mono.

**Modulation.** Voices are modulated one by one, but the effects are shared.
The effects follow the modulation of the most recently started voice that is
still playing. When no voice plays, the last values hold, so tails don't
jump.

## Arpeggiator

The same modes, rates, range and gate as the channel arpeggiator, but run in
real time on the keys held now (the channel one is applied when sequences
are built).

| Parameter | Values |
| --- | --- |
| `arp_mode` | off, up, down, up-down, random |
| `arp_rate` | 1/32, 1/16, 1/8, 1/4, 1/2 |
| `arp_range` | 1–4 octaves |
| `arp_gate` | 0.05–1 of a step |

- While the arpeggiator (or the sequencer) is on, held keys don't play
  themselves: they feed the pattern.
- The order is every held key, repeated an octave up for each extra octave of
  range, sorted, then played up, down, up and down (without repeating the top
  and bottom), or in a random order that repeats (a hash of the step number).
- Pressing a key when none is held starts the pattern at once, on step 0.
  Steps then follow the tempo. Keys added or released while it runs take
  effect from the next step. Releasing the last key stops the pattern and
  releases its note.
- Each note uses the velocity its key was pressed with and lasts `arp_gate`
  of a step. Events are sample-accurate: a block is split at each step.

## Sequencer

A per-patch note pattern of up to 64 steps. Each held key plays the
pattern, transposed from that key (a chord plays the pattern as a chord).

| Parameter | Values |
| --- | --- |
| `seq_on` | off, on |
| `seq_length` | 1–64 steps (default 16) |
| `seq_rate` | 1/32 … 1/2, as the arpeggiator |

The steps themselves are not parameters (like the LFO shapes):
`set_sequence(Sequence)` and `sequence()`. A `Sequence` holds 64 `Step`s,
each with:

- `on`
- `transpose`: −24..24 semitones
- `velocity`: 0..1, multiplied by the key's velocity
- `gate`: 0.05..1 of a step

The default sequence has every step on, with no transposition, velocity 1
and gate 0.5. While `seq_on` is on, the arpeggiator is ignored. Starting,
stopping and timing work as for the arpeggiator. Notes above 127 or below 0
are skipped.

## Tests

- Each module processes when placed in a slot and is skipped otherwise. A
  module named twice plays once. Slot order matters: distortion then filter
  differs from filter then distortion.
- The synth's distortion module gives the same output as the mixer's
  Distortion effect.
- Dimension: a mono input becomes wider (the side signal grows), and the
  mono sum stays close to the input.
- A macro modulating an effect parameter changes it.
- Arpeggiator: up, down, up-down and random orders, with octaves; steps
  start on exact samples at 120 BPM; gate length; stopping when keys are
  released.
- Sequencer: transposition, skipped steps, length, velocity scaling, and a
  chord.

## Sources

- Own design. Effects racks with reorderable modules, arpeggiators and step
  sequencers are common features of software synthesizers, as described in
  their public manuals. No layout, preset or name is taken from any of them.
- The dimension widener follows the general idea of opposite-polarity
  modulated delays, as described in public articles on stereo chorus and
  widening; the values are our own.
- The arpeggiator modes and rates are MusicCraft's own channel arpeggiator
  (`docs/spec/channel-rack.md`).
