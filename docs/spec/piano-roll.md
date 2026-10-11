# Spec: piano roll, notes and the synth (M4)

## Notes

- A pattern holds notes for any number of channels. A note has a start and length
  in ticks (96 per beat, 24 per step), a key 0–127, and a velocity 1–127.
- At most one note per channel, start and key: adding a note at an occupied
  position replaces it.
- Key arguments accept MIDI numbers or names with key 60 = `C5` (octave = key/12,
  sharps `#`, flats `b`).
- Time arguments are in steps; fractions are allowed and `<n>t` is an exact tick
  count.
- Commands: `note.add`, `note.remove`, `note.move`, `note.resize`,
  `note.set_velocity`, `note.clear`, `note.set_all`, `note.transpose`,
  `note.quantize`. Every note and step command acts on the selected pattern unless
  a trailing `@<pattern>` names another. Their inverses always name the pattern,
  so undo is correct after switching patterns.

## MIDI file import

- `note.import_midi <channel> <file.mid> [@pattern]` adds every note of a
  Standard MIDI File (format 0 or 1; all tracks and MIDI channels) to one channel
  in the pattern. File ticks are scaled to 96 per beat. Notes left sounding at a
  track's end stop there. SMPTE-timed files are rejected. One undo step.

## Chords, scales and recording

- `note.add_chord <channel> <start> <length> <root> <quality> [velocity]` adds
  the notes of a chord (major, minor, dim, aug, sus2, sus4, maj7, min7, dom7,
  dim7, add9) as one undo step. The piano roll's chord menu makes a click draw
  the chosen chord.
- The scale menu tints the rows of a scale (major, natural and harmonic minor,
  dorian, mixolydian, major and minor pentatonic) on any root; the root row is
  tinted more strongly. Display only.
- With recording armed (REC button or Ctrl+E) and playback running, notes played on
  a MIDI keyboard or the typing keyboard are written into the selected pattern on
  the selected channel at the playhead position (folded into the pattern length
  in song mode). Each recorded note is one undo step. Notes held across the loop
  point keep their full length.

## Note properties and kinds

- Besides start, length, key, velocity, pan and slide, every note has:
  `release` (release velocity 0–127, default 64: lower is a shorter release,
  higher a longer one, up to ×4), `fine` (−100…100 cents), `mod_x` and `mod_y`
  (0–127, default 64; on the synth they offset the filter cutoff by up to ±3
  octaves and the resonance), `color` (1–16, a colour group for display;
  plugins receive notes on one channel), `porta` and `muted`.
- `note.set_prop <channel> <start> <key> <prop> <value> [@pattern]` sets one
  (`release`, `fine`, `mod_x`, `mod_y`, `color`, `porta on|off`,
  `muted on|off`); undoable.
- **Portamento notes** start a new voice that glides from the pitch of the
  channel's previous note over the channel's `glide` time (0.1 s when that is
  0). Unlike slide notes they retrigger the envelope.
- **Muted notes** stay in the pattern but do not play; they are drawn hollow.
  The piano roll's Mute tool toggles muting on the notes it clicks or sweeps.
- The bottom lane edits velocity, pan, release, fine pitch, Mod X or Mod Y.
- New notes take the toolbar's colour and Portamento setting. Fine pitch,
  release velocity and Mod X/Y apply to the built-in instruments; plugins get
  the note's key and velocity.
- Project files store only properties that differ from their defaults.

## Slide notes

- A note can be a **slide note** (`note.set_slide <channel> <start> <key>
  <on|off>`, undoable; stored as `"slide": true`). When it starts while
  another note of the same channel is sounding, it does not start a new
  voice: the sounding note glides to the slide note's key over the slide
  note's length and keeps sounding until the slide note ends. With nothing
  sounding, a slide note plays like a normal note.
- Works for the synth and the sampler; plugins get a normal note.
- In the piano roll, the Slide toggle on the toolbar makes new notes slide
  notes, and Tools → Toggle slide flips the selected notes. Slide notes are
  drawn with a ramp mark.

## Arpeggiator

- Every channel has an arpeggiator (channel settings → ARPEGGIATOR):
  `arp_mode` (off, up, down, up-down, random), `arp_rate` (1/32, 1/16, 1/8,
  1/4 or 1/2 note), `arp_range` (1–4 octaves) and `arp_gate` (length of each
  arpeggio note as a share of the rate).
- When on, the channel's sequenced notes become an arpeggio when the
  sequence is built: on every rate step of the song grid, the keys held during
  that step (spread over the octave range) play one at a time in the mode's
  order, at the loudest held velocity. The order restarts when nothing is
  held. Random order is the same every time it plays.
- It applies to pattern and song playback and to export, for every kind of
  instrument. Live MIDI notes are not arpeggiated.
- Presets leave the arpeggiator settings alone.

## Note tools

Each tool rewrites all notes of one channel in a pattern as one undo step
(the inverse is `note.set_all`). Times are steps unless written `<n>t`.

- `note.strum <channel> <spread> [up|down]`: notes that start together are
  staggered by `spread` per note, lowest first (`up`, default) or highest
  first; each keeps its end time (but stays at least 1 tick long).
- `note.humanize <channel> <timing> <velocity> [seed]`: moves each start by up
  to ±`timing` and each velocity by up to ±`velocity` (1–127). The same seed
  (default 1) always gives the same result.
- `note.legato <channel>`: every note lasts until the next later start; the
  last notes keep their length.
- `note.chop <channel> <length>`: splits notes into pieces of `length`; a
  shorter remainder is kept.
- `note.glue <channel>`: joins notes of the same key where one ends exactly
  where the next begins, keeping the first one's velocity and pan.
- `note.flip <channel> time|pitch`: mirrors the notes in time within the
  pattern length, or upside down around the middle of their key range.
- `note.to_scale <channel> <root> <scale>`: moves each note to the nearest
  key of the scale (ties go down). Scales as in the piano roll's highlight.
- `note.arpeggiate <channel> <step> [up|down|updown|random] [seed]`: every
  group of notes starting together becomes one note at a time, `step` long,
  cycling through the group's keys for the group's longest duration.
- Step tools: `step.fill <channel> <every> [offset]` turns on every `every`th
  step from `offset` (0-based) across the pattern, keeping existing notes;
  `step.shift <channel> <steps>` rotates the channel's notes by whole steps,
  wrapping at the pattern length.
- Notes that end up on the same start and key keep only the first.
- In the piano roll these live in the toolbar's Tools menu; in the channel
  rack, in a channel's right-click menu.

## Pattern length

- A pattern has a minimum length in steps (default 16). When notes start past it,
  the pattern extends to whole bars to fit them.

## Synth

- Two oscillators (`wave1`, `wave2`: sine, saw, square, triangle, noise; saw and
  square are band-limited with PolyBLEP). Oscillator 2 is detuned by `detune`
  cents; `osc_mix` crossfades between them.
- `unison` (1–7) stacks copies of both oscillators detuned across `spread`
  cents and spread across the stereo field; the stack is normalized so more
  voices are not louder. `sub` adds a sine one octave below.
- `drive` saturates (tanh) before the filter.
- A resonant state-variable filter per side (`filter_type`: low-, band- or
  high-pass; `cutoff`, `resonance`) whose cutoff is modulated by the amplitude
  envelope by `env_amount` octaves.
- A sine LFO per voice (`lfo_rate` Hz, `lfo_depth` 0–1, `lfo_target`): pitch
  (up to ±2 semitones), cutoff (up to ±3 octaves) or volume (tremolo).
  Modulation, glide and filter coefficients update every 32 samples.
- **Presets**: the synth editor offers factory presets (bass, lead, pad, pluck,
  keys, brass, riser…). Applying one sets every synth parameter in one undo
  step (`edit.batch` of `channel.set_param`); the sampler-only parameters are
  left alone. The editor shows the matching preset or "(custom)".
- Linear ADSR envelope (`attack`, `decay`, `sustain`, `release`); a minimum 1 ms
  release prevents clicks. `pitch` transposes in semitones.
- `glide` (seconds, default 0) is portamento: a note that starts while another
  note on the same channel is still held slides linearly (in semitones) from that
  note's current pitch over the glide time.

## Editor (UI)

- Vertical keyboard on the left (drawn as piano keys), time grid snapped to
  steps by default. Notes of other channels in the pattern show faintly in
  their channel colours (ghost notes).
- **Tools** (toolbar, left): Draw (the default behavior below), Paint (drag to
  lay notes end to end at the last length; one undo step), Delete (click or
  drag over notes; one undo step) and Select (drag a box without Ctrl; click
  empty space to clear). Starting a Paint drag on a note moves it as Draw would.
- Left-click on empty grid adds a note with the last used length; dragging a note
  moves it; dragging its right edge resizes it; right-click deletes.
- Right-click deletes a note; right-drag deletes every note the pointer passes
  over, as one undo step when the button is released (with any tool).
- Each finished gesture is one undoable command.
- Selection: click a note to select it, Shift+click toggles, Ctrl+drag on empty
  grid draws a selection rectangle (Shift adds to the selection). Dragging a
  selected note moves the whole selection; dragging its edge resizes all
  selected notes by the same amount. Selected notes are outlined.
- Keys while the pointer is over the piano roll: Delete/Backspace remove the
  selection; Ctrl+A selects all; Escape clears; Ctrl+C / Ctrl+X copy / cut;
  Ctrl+V pastes at the pointer; Ctrl+B duplicates the selection right after its
  end (rounded up to the snap); arrows move by one key or one snap step, and
  Shift+Up/Down by an octave. Every operation is one undo step (`edit.batch`),
  with moves ordered so notes of the selection never overwrite each other.
- Ctrl+scroll zooms horizontally.
- Notes have a pan, -100 to 100 (`note.set_pan`; `note.add` takes it as an
  optional sixth argument; `note.set_all` rows may carry it as a fifth value).
  It is applied as a balance on top of the channel pan.
- The bottom lane edits velocity or pan; click its label to switch. In pan mode
  stems grow up or down from the centre and snap to centre within ±3.
- A velocity lane is pinned to the bottom of the view: one stem per note of the
  selected channel. Dragging vertically sets the velocity of the note under the
  pointer (the latest-starting one when notes overlap); one drag is one undo step.
- Clicking a key on the keyboard previews the note on the selected channel.

## Sources

- Piano roll interaction (draw with left click, delete with right click, resize
  from the note edge) follows conventions described in the FL Studio online
  manual, "Piano roll" section, and common to most DAWs. Implementation and
  layout are our own.
- Synth architecture: textbook subtractive synthesis. PolyBLEP and the
  trapezoidal SVF follow publicly published papers (Välimäki et al.; Simper).
