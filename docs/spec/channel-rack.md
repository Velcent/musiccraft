# Spec: channel rack, steps and the sampler (M2)

## Behavior

- A project has an ordered list of **channels**. Each channel has a name, an
  instrument (sampler or synth), instrument parameters, volume (linear gain 0 to
  1.25, default 0.8), pan (-1 to 1, using the project's pan law; see `mixer.md`), mute, and the mixer track it
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
- `channel.route_free <n>` routes a channel to the first mixer insert that no
  other channel feeds and that has no effects, adding an insert when none is
  free, and names that insert after the channel (one undo step).
- Keys over the rack (see `keymap.md`): 1–0 mute channels 1–10, Ctrl+1–0 solo
  them; Alt+C, Alt+Delete and Alt+Up/Down clone, delete and move the selected
  channel; Ctrl+L runs `channel.route_free`; Shift+Ctrl+Left/Right shift its
  steps.
- **Step graph editor**: the rack's Graph toggle opens a strip under the rows
  for the selected channel, one bar per step of the pattern, showing the note
  that starts on that step (the highest, when several do): velocity, pan,
  release, fine pitch, Mod A or Mod B. Dragging a bar sets that note's value
  through the usual note commands; one drag is one undo step.
- **Display filter groups**: `channel.set_group <n> [group]` (undoable,
  saved as `"group"`; no name removes it) puts a channel in a named group. The
  rack's group menu (always at the top left of the rack) shows All, one
  group, or No group (channels without one). Right-click a channel → Group…
  sets it. Filtering only hides rows; numbering and keys stay the same.
- `channel.solo <n>` mutes every other channel; running it again on an already
  soloed channel unmutes all channels.
- **Steps** are sixteenth notes. `step.toggle <channel> <step>` (0-based step)
  adds a one-step note at key C5 (60) and velocity 100 if the channel has no note
  starting there, otherwise removes every note of that channel starting there.
  `step.set` forces a state. Steps and piano-roll notes are the same data.
- **Step buttons** (UI): left-click toggles a step; dragging with the left
  button across a channel's row paints every step it passes to the state
  the first click gave (on or off). Right-click turns a step off, and
  dragging with the right button erases every step it passes. A whole drag
  is one undo step. Turning a step on previews the channel's sound.
  A lit step is filled with the channel colour, one step darker in odd beat
  groups so the grouping survives, and dimmer for a lower velocity (with a
  readable minimum); lit steps under the playhead light one step brighter.
  Melodic rows show a miniature roll whose alternate bars are shaded; a click
  on it opens the piano roll on that channel, or closes it when it already
  shows that channel.
- Shift+wheel over the rack's header row steps to the next or previous
  pattern (`pattern.next`, `pattern.prev`).
- The mute light: a click mutes or unmutes, Ctrl+click solos the channel
  (`channel.solo`), and right-click offers Mute or Unmute and Solo.
  The pattern length box takes 1 to 1024 steps.
- **Row layout** (UI), left to right: mute dot, pan knob, volume knob, the
  mixer track box, the name button, the steps (or a miniature roll), and a
  **selection switch** that selects the channel without opening its
  settings (lit for the selected channel).
- **Mixer track box** (UI): each row shows the mixer track the channel plays
  to as a small number box: "M" for the master track, "3" for insert 3. Drag
  it up or down (or type) to change the track; the tooltip names the track.
- **Browser** (UI): in the plugin database, a single click only selects an
  entry (it is highlighted). Double-clicking a generator adds it as a new
  channel; dragging it onto the channel rack does the same. Double-clicking
  an effect adds it to the selected mixer track. Sounds in packs and folders
  preview on a single click and are added on a double click or by dragging,
  as before.

- **Next empty pattern**: `pattern.next_empty` selects the first pattern
  after the selected one that has no notes; when there is none it adds a new
  pattern (as `pattern.add`, undoable) and selects it. Selecting an existing
  pattern is not undoable. F4 runs it and then offers to name the pattern
  (a rename prompt; Escape keeps the name).
- **Swing** is one amount for the whole project: `project.set_swing <0-100>`
  (undoable; saved as `"swing"`, 0–1). Notes that start exactly on an off-beat
  sixteenth (steps 1, 3, 5, …) are delayed by up to half a step: 12 ticks ×
  swing × the channel's **swing mix**. Other notes are unaffected. It applies
  to pattern notes in pattern and song mode, not to audio clips.
  - `swing_mix` (channel parameter, 0–1, default 1 = 100%; `channel.set_param
    <n> swing_mix <v>`) scales the global swing per channel: 0 keeps a channel
    straight.
  - `swing_truncate` (channel parameter, 0 or 1, default 0): a swung note
    keeps its original end and so gets shorter by the delay (never below one
    tick) instead of moving whole, so it cannot run into the next step.
  - Both are in channel settings → Functions → SWING. The channel rack's swing box sets
    the global amount.
  - Older files stored swing per pattern. On load, a project without a global
    swing takes the largest pattern swing.

## Timing and echo

Channel parameters (channel settings → Functions → TIMING AND ECHO), applied when the
sequence is built, after swing and the arpeggiator; they change what the
channel plays, not the stored notes:

- `time_shift` (-96…96 ticks) moves every note of the channel later
  (negative: earlier, never before the loop start).
- `gate` (0…384 ticks, 0 = off) gives every note that fixed length.
- Note echo: `echo_count` (0–8) repeats each note `echo_time` ticks apart
  (default 48), each repeat's velocity multiplied by `echo_feed` (0–1,
  default 0.5) and its key moved by `echo_pitch` semitones (-12…12). Repeats
  past the loop end, below velocity 1 or outside the key range are dropped.
  Echoes are never slide notes.

## Voices: cut groups, mono and polyphony

Channel parameters (`channel.set_param <n> <name> <value>`, undoable; channel
settings → VOICES). They apply to the built-in instruments; plugin and MIDI
Out channels voice their own notes and are never cut.

- **Cut groups** (choke): `cut_group` (0–16, 0 = none) is the group a
  channel's notes cut; `cut_by` (0–16, 0 = none) is the group that cuts the
  channel. When a note starts on a channel whose `cut_group` is G, every
  sounding voice of every channel whose `cut_by` is G is cut, on the new
  note's own frame. A closed hi-hat with `cut_group 1` and an open hi-hat with
  `cut_by 1` makes the closed hat choke the open one; giving both channels
  `cut_group 1` and `cut_by 1` makes each choke the other (and itself).
- **Mono** (`mono` 0 or 1): a new note cuts every other voice of the channel,
  so one note sounds at a time. With the synth's `glide`, the new note still
  glides from the note it replaces.
- **Max polyphony** (`poly_max`, 0–64, 0 = no limit): when a note starts and
  the channel already has that many voices sounding, the oldest are cut until
  the new note fits. Mono is the same as a limit of 1.
- A **cut** fades the voice out over 3 ms instead of its release time (one-shot
  samples included), so it never clicks. A voice fading out after a cut no
  longer counts toward the polyphony limit.
- Slide notes start no voice, so they cut nothing.

## Sampler

- `reverse` (0 or 1) plays the sample backwards from its end; `fade_in` and
  `fade_out` (0–10 s) fade each note in from its start and out over its last
  seconds (before the note or the sample ends). Used by audio clips (see
  `playlist.md`, "Clip editing"); sliced samplers and kit pads ignore
  `reverse`.
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
  the note damps the string quickly. Presets: guitar, harp, bass, muted,
  bowed and twelve-string.
- `pluck_bow` (0–1) bows the string: while the note is held, low-passed
  noise (brightness from `pluck_tone`, level following the channel
  envelope) keeps feeding the string, so it sustains; releasing stops the
  bow and the string damps as usual. `pluck_strings` (1–3) sounds extra
  strings `pluck_detune` cents above and below the note (each with its own
  burst), mixed at equal power.

## FM synth

- `channel.add_fm [name]`: four sine operators. `fm_algo` picks how they
  connect (0: 4→3→2→1; 1: 4→3, 3 and 2 → 1; 2: two stacks 4→3 and 2→1;
  3: 4 modulates 1, 2 and 3; 4: 2→1 plus 3 and 4 as tones; 5: all four as
  tones). Each operator has `opN_ratio` (0.25–16 × the note), `opN_level`,
  `opN_decay` (s) and `opN_sustain`; operator 4 has `fm_feedback`. Tones
  follow the channel envelope; modulators shape the timbre over time.
  Presets: e-piano, bell, bass, brass, organ, pluck, metallic, plus six-operator
  ones.
- Six operators: algorithms 6–11 also use operators 5 and 6 (same four
  parameters each): 6→5→4→3→2→1; two stacks 6→5→4 and 3→2→1; three pairs
  6→5, 4→3, 2→1; 2–6 all into 1; 6 into each of 1–5; all six as tones.
  With six operators the feedback is on operator 6. Algorithms 0–5 ignore
  operators 5 and 6, so older sounds are unchanged.
- `opN_wave` (operators 1–6): sine, half sine, soft square or soft saw.
- `fm_cutoff` and `fm_resonance`: a low-pass filter on the output, off at
  20 kHz.

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
- A second spectrum (`add_bright_b`, `add_even_b`, `add_inharm_b`) and a
  morph between them: `add_morph` (0 the first spectrum, 1 the second),
  `add_morph_time` (seconds over which each note moves from `add_morph` to
  the second spectrum; 0 off) and a sine LFO on the position
  (`add_morph_rate` Hz, `add_morph_depth`). Each harmonic's level and the
  stretch are interpolated; both spectra are normalized first, so morphing
  keeps the level.

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

## Resynth

- `channel.add_resynth <sample> [name]` measures the sample's harmonics once
  (when the channel is built): the median pitch it hears is its fundamental
  (220 Hz if it hears none), and Hann-windowed spectra up to 512 times along
  the sample give the level of harmonics 1–32 (the strongest bin within two
  of each harmonic), normalized to the loudest. Notes play those levels as
  sines on the note's own harmonics, so any sample becomes a pitched
  instrument that keeps its tone changing over time.
- `resyn_speed` (0 freezes, 1 as recorded, up to 4), `resyn_position`
  (start, 0–1 of the sample), `resyn_blur` (0–1: levels follow the analysis
  over up to half a second, smearing changes), `resyn_loop` (hold the last
  frame or loop), and from the additive synth `add_partials` and
  `add_inharm` (stretches the harmonics apart). Levels update every 32
  samples. `channel.set_sample` changes the sample. Browser samples offer
  "Add as resynth" in their right-click menu.

## Zone sampler

- `channel.add_zones [name]` adds a sampler that plays several samples, each
  in a **zone**: a key range (`low`–`high`), a velocity range (`vel_low`–
  `vel_high`, 1–127) and the `root` key at which the sample plays at its
  recorded pitch. A note plays the first zone (in list order) whose ranges
  contain its key and velocity; when none does it is silent. One-shot,
  fades, reverse, envelope and pitch work as on the sampler.
- `channel.add_zone <channel> <sample> [low] [high] [root]` appends a zone
  (defaults: the whole keyboard, root C5 = 60, all velocities);
  `channel.remove_zone <channel> <zone>`, `channel.insert_zone <channel>
  <zone> <json>` (undo) and `channel.set_zone <channel> <zone>
  <low|high|root|vel_low|vel_high|sample> <value>` edit them. All undoable.
- The editor lists the zones with their sample and ranges as dragged
  numbers, a delete button per zone and "Add zone…" (asks for a sample).

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
- Eight control-change knobs (`midi_cc1` … `midi_cc8`, 0–127; −1, the
  default, sends nothing) with their controller numbers (`midi_cc1_number`
  …, defaults 1, 7, 10, 11, 71, 74, 91, 93) and `midi_program` (0 none,
  1–128). The engine sends a control change or program change whenever the
  value differs from the last one it sent (and again after the channels are
  rebuilt), so automation and controllers drive external gear; a muted
  channel sends none.
- **MIDI clock** (Settings → MIDI, off by default): while playing, 24 clock
  messages per quarter note go to the MIDI output, with Start when playback
  begins at the song start, Continue when it resumes elsewhere and Stop when
  it stops or pauses.

## Built-in sounds

`kick`, `snare`, `clap`, `hat`, `open_hat`, `tom`, `rim`, `crash`. Each is
synthesized deterministically at load time (swept sines, filtered noise, metallic
square partials), normalized to a 0.9 peak and faded out at the end.

## Channel settings window

- Clicking a channel's name opens its
  settings window. A **fixed header** stays at the top: the channel name and
  instrument, volume, pan and pitch knobs, mute, and the mixer track it feeds
  as a number box (drag or type; `channel.route`).
- Below it, **tabs**: *Sound* (the instrument's own sections: sample, oscillators,
  operators, plugin parameters…), *Envelope* (the amplitude envelope, for
  instruments that use it) and *Functions* (arpeggiator, swing, timing and
  echo, voices). Only the tab's contents scroll. The last tab used stays
  selected while the window is open.
- MIDI Out channels have no sound settings: the window shows their MIDI page
  only.

## Sources

- The channel settings window's fixed header and tabbed pages follow the
  layout described in the FL Studio online manual, "Channel settings"
  (sampler and instrument pages, miscellaneous functions), with our own
  grouping and names.

- Channel rack layout (rows of instruments with step buttons, volume, pan, mute
  and mixer routing) follows the general workflow described in the FL Studio
  online manual, "Channel Rack" section. Command names, value ranges and defaults
  are our own.
- Step buttons that toggle on left-click, erase on right-click and paint or
  erase when dragged follow the step sequencer workflow described in the FL
  Studio online manual, "Channel Rack". The routing box's labels and the
  browser's select-then-double-click behaviour are our own design.
- F4 selecting the next empty pattern and offering to name it follows the FL
  Studio online manual, "Keyboard shortcuts" and the pattern selector.
  "Empty" meaning "no notes" and adding a pattern when none is left are our
  own choices.
- Cut groups ("cut" and "cut by"), mono and maximum polyphony are channel
  settings described in the FL Studio online manual, "Channel settings",
  miscellaneous functions. The 3 ms fade, the group range and the counting
  rules are our own.
- Swing: the FL Studio online manual describes one main swing control, a
  per-channel swing mix (default 100%) and a truncate option for swung
  notes. The delay curve, the truncate rule and the migration are our own.
- MIDI Out: note on/off messages per the MIDI 1.0 specification (MIDI
  Association); the channel kind mirrors the "send notes to external gear"
  generator listed in the FL Studio online manual's plugin list. Design ours.
- Built-in sounds: own design; no third-party audio is used.
- WAV decoding: the RIFF/WAVE format via the `hound` crate.
- Time shift, gate and note echo are channel settings described in the FL
  Studio online manual, "Channel settings" (miscellaneous functions: time,
  echo delay). Units, ranges and rules are our own.
- Display filter groups follow the channel rack filter groups described in the
  FL Studio online manual, "Channel Rack". "No group" and the menu are our own.
- The step graph editor follows the channel rack graph editor described in the
  FL Studio online manual, "Channel Rack" (graph editor). Its properties match
  our piano roll lane; the layout is our own.
