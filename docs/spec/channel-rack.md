# Spec: channel rack, steps and the sampler (M2)

## Behavior

- A project has an ordered list of **channels**. Each channel has a name, an
  instrument (sampler or synth), instrument parameters, volume (linear gain 0 to
  1.25, default 0.8), pan (-1 to 1, balance law), mute, and the mixer track it
  feeds (0 = master).
- Channels are numbered from 1 in display order. Internally each has a stable id
  so that reordering or removing channels never reattaches notes to the wrong
  channel.
- `channel.add_sampler <sample> [name]` adds a sampler. A sample is a WAV file
  path (absolute, or relative to the working directory or the project file) or a
  built-in sound `builtin:<name>`. The default name is the file stem or the
  built-in sound's name.
- `channel.add_synth [name]` adds a synth channel (see the piano roll spec).
- `channel.remove`, `channel.clone`, `channel.move`, `channel.rename`,
  `channel.set_volume`, `channel.set_pan`, `channel.set_mute`,
  `channel.toggle_mute`, `channel.solo`, `channel.route`, `channel.set_sample` and
  `channel.set_param` edit channels. All are undoable. Removing a channel removes
  its notes from every pattern; undo restores both.
- `channel.solo <n>` mutes every other channel; running it again on an already
  soloed channel unmutes all channels.
- **Steps** are sixteenth notes. `step.toggle <channel> <step>` (0-based step)
  adds a one-step note at key C5 (60) and velocity 100 if the channel has no note
  starting there, otherwise removes every note of that channel starting there.
  `step.set` forces a state. Steps and piano-roll notes are the same data.

- **Swing**: `pattern.set_swing <pattern> <0-100>` delays notes that start
  exactly on an off-beat sixteenth (steps 1, 3, 5, …) by up to half a step
  (12 ticks at 100%). Other notes are unaffected. Undoable; saved per pattern.

## Sampler

- Plays the sample from its start. The pitch follows the note key relative to the
  `root_key` parameter (default 60) plus the `pitch` parameter in semitones.
  Samples of any rate are resampled by linear interpolation.
- With `one_shot` on (the default) the whole sample plays regardless of note
  length; otherwise the note's end starts the envelope release.
- The sampler applies the shared ADSR envelope; new samplers default to instant
  attack, full sustain and a short release so samples play unshaped.
- Missing or undecodable files play silence and produce a warning.

## Slicing

- The sampler's `slices` parameter (1–64, default 1) cuts the sample into
  equal slices. With more than one, the key `root_key + i` plays slice `i`
  (wrapping) at its original pitch, from the slice start to its end, as a
  one-shot; `pitch` still transposes.
- `channel.slice <channel> <slices> [@pattern]` sets `slices` and rewrites the
  channel's notes in the pattern so slice `i` plays at `i` × (pattern length
  ÷ slices): the loop plays back as it was, now editable note by note. One
  undo step.

## Drum synth

- `channel.add_drum [kick|snare|hat|tom|clap] [name]` adds a channel that
  synthesizes a drum on every note (default kick). Parameters: `drum_type`
  (0 kick, 1 snare, 2 hat, 3 tom, 4 clap), `drum_tune` (±24 semitones),
  `drum_decay` (0.02–2 s), `drum_sweep` (pitch drop in semitones, 0–48),
  `drum_tone` (noise against tone for snares; brightness for hats and claps),
  `drum_click` (a short noise attack, 0–1).
- Kicks and toms are a sine whose pitch falls by `drum_sweep` semitones in the
  first ~40 ms; the note key sets the pitch (C5 plays the tuned default, two
  octaves down for kicks and one for toms). Snares mix a tone with noise;
  hats are high-passed noise; claps are three short noise bursts then a tail.
  Every hit decays exponentially over `drum_decay` and ignores note length.
- Presets: a set of kicks, snares, hats, toms and claps.

## Pluck

- `channel.add_pluck [name]` adds a plucked-string channel (Karplus–Strong:
  a burst of noise circulating in a tuned, damped delay line). Parameters:
  `pluck_decay` (time to fade by 60 dB, 0.1–10 s), `pluck_damping` (0
  bright to 1 dark), `pluck_tone` (brightness of the initial burst). Ending
  the note damps the string quickly. Presets: guitar, harp, bass, muted.

## FM synth

- `channel.add_fm [name]`: four sine operators. `fm_algo` picks how they
  connect (0: 4→3→2→1; 1: 4→3, 3 and 2 → 1; 2: two stacks 4→3 and 2→1;
  3: 4 modulates 1, 2 and 3; 4: 2→1 plus 3 and 4 as tones; 5: all four as
  tones). Each operator has `opN_ratio` (0.25–16 × the note), `opN_level`,
  `opN_decay` (s) and `opN_sustain`; operator 4 has `fm_feedback`. Tones
  follow the channel envelope; modulators shape the timbre over time.
  Presets: e-piano, bell, bass, brass, organ, pluck, metallic.

## Keys

- `channel.add_keys [piano|epiano|organ] [name]`: `keys_type`, `keys_tone`
  (brightness), `keys_decay` (how long a held note rings), `keys_tremolo`
  and `keys_hammer` (attack noise; organ: percussion). The piano sums slightly
  stretched partials whose upper ones fade faster; the e-piano is a decaying
  FM tine with a bell overtone; the organ mixes nine drawbar-like harmonics.
  Release follows the channel envelope.

## Additive

- `channel.add_additive [name]`: up to 32 harmonics (`add_partials`) with
  `add_bright` (spectral slope), `add_even` (level of even harmonics),
  `add_inharm` (stretch) and `add_decay` (upper harmonics fade faster).

## Third oscillator

- The synth has a third oscillator: `wave3`, `osc3_level` (0 = off) and
  `osc3_coarse` (±24 semitones), mixed after the first two.

## Granular sampler

- `channel.add_granular <sample> [name]` plays a sample as overlapping
  windowed grains: `gran_size` (5–500 ms), `gran_density` (grains per
  second), `gran_position` (0–1 in the sample), `gran_spread` (random
  position range), `gran_pitch_rand` (random detune, semitones) and
  `gran_scan` (how fast the position moves while held, −2…2 × real time).
  Pitch follows the key around `root_key`.

## Drum kit

- `channel.add_kit [name]` adds a 16-pad sample kit; key `root_key + n`
  plays pad `n + 1` (one-shot). `channel.set_pad <channel> <pad> <sample>`
  loads a pad (`-` clears it). A new kit loads the built-in sounds on pads
  1–8.

## Layer

- `channel.add_layer [name]` adds a channel that plays nothing itself but
  passes every note (sequenced and live) to the channels set with
  `channel.set_layer <channel> <channel,channel,…>` (`-` for none). Layers
  do not nest.

## MIDI Out

- `channel.add_midi_out [1-16] [name]` adds a channel that makes no sound but
  sends its notes to external MIDI gear on a MIDI channel (default 1).
  `channel.set_midi_channel <channel> <1-16>` changes it (undoable).
- Sequenced notes send note on (velocity 1–127 from the note's velocity)
  at their start and note off after their length; live notes (MIDI keyboard,
  typing keyboard, rack audition) send note on and off as they are played.
  Layers can play MIDI Out channels like any other.
- A muted MIDI Out channel sends nothing new; notes already sounding still
  get their note off. Stop, pausing and rebuilding the channels send note off
  for every held note. Volume, pan, mixer routing and instrument parameters
  do not apply.
- The engine queues the messages on the audio thread (fixed-size queue, a
  full queue drops messages) with their offset into the block; the app sends
  them from a separate thread to the **MIDI output port** chosen in
  Settings → MIDI (none by default). Timing is accurate to about one audio
  buffer. Offline renders and exports leave MIDI Out channels silent and
  send nothing.

## Built-in sounds

`kick`, `snare`, `clap`, `hat`, `open_hat`, `tom`, `rim`, `crash`. Each is
synthesized deterministically at load time (swept sines, filtered noise, metallic
square partials), normalized to a 0.9 peak and faded out at the end.

## Sources

- Channel rack layout (rows of instruments with step buttons, volume, pan, mute
  and mixer routing) follows the general workflow described in the FL Studio
  online manual, "Channel Rack" section. Command names, value ranges and defaults
  are our own.
- MIDI Out: note on/off messages per the MIDI 1.0 specification (MIDI
  Association); the channel kind mirrors the "send notes to external gear"
  generator listed in the FL Studio online manual's plugin list. Design ours.
- Built-in sounds: own design; no third-party audio is used.
- WAV decoding: the RIFF/WAVE format via the `hound` crate.
