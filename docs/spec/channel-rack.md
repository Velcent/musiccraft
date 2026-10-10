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

## Built-in sounds

`kick`, `snare`, `clap`, `hat`, `open_hat`, `tom`, `rim`, `crash`. Each is
synthesized deterministically at load time (swept sines, filtered noise, metallic
square partials), normalized to a 0.9 peak and faded out at the end.

## Sources

- Channel rack layout (rows of instruments with step buttons, volume, pan, mute
  and mixer routing) follows the general workflow described in the FL Studio
  online manual, "Channel Rack" section. Command names, value ranges and defaults
  are our own.
- Built-in sounds: own design; no third-party audio is used.
- WAV decoding: the RIFF/WAVE format via the `hound` crate.
