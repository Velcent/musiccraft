# FL-manual parity TODO

Gaps and mismatches found by comparing MusicCraft against the public FL Studio
online manual (October 2026). Behaviour is described in our own words; each
item becomes a spec in `docs/spec/` before code, per `docs/clean-room.md`.
When an item lands, tick it here and add or update its row in `docs/parity.md`.

## 1. Misinterpretations: FL does it differently

### Keyboard (first: FL users hit these immediately)
- [x] Ctrl+N saves a new numbered version of the project (FL), not "new project".
  New project stays in the File menu.
- [x] R arms / disarms recording (FL); Ctrl+E is reserved for step edit mode.
- [x] Piano roll: Shift+arrows move the selection (semitone / snap step),
  Ctrl+Up/Down move it an octave. Plain arrows no longer move notes.
- [ ] Ctrl+E toggles step edit mode (notes played on a MIDI or typing keyboard
  are entered at the selected step and the cursor advances).

### Sound
- [x] Slide notes:
  - [x] A slide note that overlaps no other note is silent.
  - [x] The overlap length sets the slide time.
  - [x] The slide stops when the note being slid ends.
  - [x] Slides only affect notes of the same colour group (up to 16
    independent slides in a chord).
  - [x] In a chord, the top note sets the pitch offset and every note keeps
    that offset after the slide.
- [x] Pan law: a circular default (-3 dB at centre) with a triangular option,
  instead of a plain balance control.
- [x] Swing: one global swing amount, scaled by a per-channel swing mix
  (default 100%); add a "truncate" option.

### Smaller
- [x] F4 jumps to the next *empty* pattern and offers to name it.
- [ ] Tempo range: FL allows 10–522 BPM; decide whether to lower our 20 BPM
  floor (999 BPM ceiling can stay).

## 2. Missing: high impact

- [x] Cut groups / choke (a closed hi-hat cuts an open one).
- [x] Per-channel mono mode and maximum polyphony.
- [ ] Recording:
  - [ ] Count-in (Ctrl+P).
  - [ ] Wait for input before recording starts (Ctrl+I).
  - [ ] Blend vs replace.
  - [ ] Loop recording.
  - [ ] Step recording.
  - [ ] Record knob movements as automation.
- [ ] Transport:
  - [x] Pause with Ctrl+Space (`transport.pause`, `transport.play_pause`).
  - [x] Panic with Ctrl+H (all notes off).
  - [x] Home jumps to the start.
- [ ] Playlist tracks: names, colours, mute and solo.
- [ ] Playlist clip editing:
  - [ ] Slice clips.
  - [ ] Make unique.
  - [ ] Merge.
  - [ ] Consolidate / render to audio.
  - [ ] Reverse.
  - [ ] Fade in / out.
  - [ ] Insert / delete space (Ctrl+Ins / Ctrl+Del).
- [ ] Single-key tools: P draw, B paint, D delete, E select, C slice, T mute,
  Y play, Z zoom; plus S slide and O porta for new piano-roll notes.
- [ ] Channel rack keys:
  - [ ] 1–0 mute a channel, Ctrl+1–0 solo one.
  - [ ] Alt+C clone, Alt+Del delete, Alt+Up/Down move.
  - [ ] Ctrl+L sends to a free mixer track.
  - [ ] Shift+Ctrl+Left/Right shifts steps.
- [ ] MIDI export (Ctrl+Shift+M).
- [ ] Mixer track options: stereo separation, phase invert, swap left/right.

## 3. Missing: lower priority

- [ ] Per-step graph editor (pitch, pan, velocity per step).
- [ ] Per-channel time shift, gate and note echo.
- [ ] Piano roll tools:
  - [ ] Limit.
  - [ ] LFO.
  - [ ] Scale levels.
  - [ ] Flam.
  - [ ] Full randomizer.
  - [ ] Chord / pattern generator.
  - [ ] Articulate.
  - [ ] Claw machine.
- [ ] Master pitch / A4 tuning.
- [ ] Tap tempo.
- [ ] MP3 export (already a `todo` in parity.md).
- [ ] Pattern select with keys 1–9.
- [ ] Automation of individual notes or events inside a pattern.
- [ ] Channel display filter groups.
- [ ] Per-track arm and audio input selection.
- [ ] Time signatures (known deferred).

## Sources

The FL Studio online manual pages listed below are under
https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/:

- `basics_shortcuts.htm`
- `channelrack.htm`
- `pianoroll.htm`
- `playlist.htm`
- `mixer.htm`
- `chansettings_misc.htm`
- `toolbar_panels.htm`
- `recording.htm`
- `songsettings_settings.htm`
