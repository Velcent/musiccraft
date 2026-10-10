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

## Pattern length

- A pattern has a minimum length in steps (default 16). When notes start past it,
  the pattern extends to whole bars to fit them.

## Synth

- Two oscillators (`wave1`, `wave2`: sine, saw, square, triangle, noise; saw and
  square are band-limited with PolyBLEP). Oscillator 2 is detuned by `detune`
  cents; `osc_mix` crossfades between them.
- A resonant low-pass state-variable filter (`cutoff`, `resonance`) whose cutoff is
  modulated by the amplitude envelope by `env_amount` octaves.
- Linear ADSR envelope (`attack`, `decay`, `sustain`, `release`); a minimum 1 ms
  release prevents clicks. `pitch` transposes in semitones.

## Editor (UI)

- Vertical keyboard on the left, time grid snapped to steps by default.
- Left-click on empty grid adds a note with the last used length; dragging a note
  moves it; dragging its right edge resizes it; right-click deletes.
- Each finished gesture is one undoable command.
- Clicking a key on the keyboard previews the note on the selected channel.

## Sources

- Piano roll interaction (draw with left click, delete with right click, resize
  from the note edge) follows conventions described in the FL Studio online
  manual, "Piano roll" section, and common to most DAWs. Implementation and
  layout are our own.
- Synth architecture: textbook subtractive synthesis. PolyBLEP and the
  trapezoidal SVF follow publicly published papers (Välimäki et al.; Simper).
