# Spec: recording and step edit

## Arming and options

- Recording is **armed** with `record.toggle` (default key R, or the REC
  button) or `record.set on|off`. The armed state belongs to the session: it is
  never saved and is not part of undo history.
- Recording options are saved with the project but, like the metronome, are
  not undoable:
  - `record.toggle_count_in` (Ctrl+P): **count-in**. When playback starts while
    recording is armed, the metronome clicks one bar of the time signature
    where playback starts (its beats, the first one accented) before the
    playhead moves. The click sounds even with the
    metronome off. Notes played during the count-in are not recorded.
  - `record.toggle_wait` (Ctrl+I): **wait for input**. When playback starts
    while recording is armed, the playhead waits until the first live note
    (MIDI keyboard or typing keyboard) arrives, then starts with that note.
    That first note is recorded. Wait for input takes over from the count-in
    when both are on.
  - `record.set_mode blend|replace`: **blend** (default) adds recorded notes
    to what the pattern already has. **Replace**: while recording, every note
    of the recording channel in the recording pattern that the playhead passes
    over is removed, unless it was recorded in the same take. Removals are
    undoable, one step per screen refresh that removed something.
  - `record.toggle_loop`: **loop recording** (default on). When off, playback
    stops at the end of the pattern (pattern mode), of the loop region, or of
    the song (song mode) while recording is armed, instead of looping. The
    transport then reads `stopped` and the playhead returns to the start.
- Count-in, wait for input and loop recording off apply only while recording
  is armed; otherwise Play behaves as usual.

## Recording notes

- With recording armed and playback running, notes played live are written into
  the selected pattern on the selected channel (see `piano-roll.md`, "Chords,
  scales and recording"). A **take** runs from when playback starts (or
  recording is armed during playback) until playback stops or recording is
  disarmed.

## Recording knob movements (automation)

- With recording armed and playback running in **song mode**, moving a control
  that automation can drive (channel volume, pan or instrument parameter; mixer
  track volume or pan; effect parameter) records it as automation at the
  playhead.
- `automation.record <target> <tick>` (undoable) writes the target's current
  value as a point at song position `tick`. It uses an automation clip of that
  target that covers `tick` when there is one. Otherwise it creates an
  automation (named like `automation.create` names it) whose only point is the
  value at `tick`, and places its clip on the first playlist track that is
  empty over the whole song, from the song start to its end (rounded up to a
  bar), so later points of the same drag land in it even after the song
  wraps.
- The front end runs the control's own command and `automation.record` as one
  batch, coalesced per gesture, so one drag is one undo step that also removes
  the points (and a new automation) it recorded.
- An automation clip drives its parameter over the whole clip (holding its
  first value before the first point and its last value after the last), so
  after a drag the parameter stays where the control was left.

## Automation inside patterns

- A pattern can hold automation of its own: lanes (one per target) of points
  in pattern ticks, saved with the pattern and copied when it is cloned or
  made unique. They play wherever the pattern plays: in pattern mode, and in
  song mode once per repeat of the pattern inside each of its clips (a clip's
  offset and length cut the lane to its window). Unlike automation clips, a
  pattern lane drives its parameter only between its first and last point.
- With recording armed and playback running in **pattern mode**, moving a
  control records into the selected pattern: `pattern.record_event <target>
  <tick>` (tick in steps or `<n>t`, folded into the pattern) writes the
  target's current value. The front end batches it with the control's command
  and coalesces per gesture, as in song mode.
- `pattern.clear_events <pattern> [target]` removes all or one target's lanes
  (pattern list → right-click → Clear automation); `pattern.set_events
  <pattern> <json>` restores them exactly (undo).
- A mixer track or effect a pattern lane drives cannot be removed, as for
  automation clips; slot moves and track renumbering update pattern lanes too.

## Step edit

- `record.toggle_step_edit` (Ctrl+E) turns **step edit** on or off (session
  state, not saved).
- While step edit is on and playback is stopped or paused, a note played on a
  MIDI keyboard or the typing keyboard is entered at the playhead: in the
  selected pattern, on the selected channel, one snap step long (the piano
  roll's snap; one step when snap is off), at the playhead snapped down to the
  grid (song positions fold into the pattern). Notes held together form a
  chord at the same step. When every key is released the playhead moves one
  snap step on, wrapping at the pattern end, so the next note goes after it.
  Each note is one undo step.
- While playing, step edit does nothing; armed recording records as usual.
- Step edit is also how **step recording** works: there is no separate mode.

## Engine mapping

- `SetStart(Start)` tells the engine how the next Play begins: `Now`,
  `CountIn(bars)` or `WaitForInput`. The bridge sends it whenever the project's
  armed state or options change it. Stop and Pause cancel a pending count-in or
  wait. The engine reports `counting_in` and `waiting` in its meters; it is not
  `playing` during either.
- The count-in is sample-accurate: the processing block is split where the
  count-in ends, and the playhead starts on that frame.
- `SetStopAtEnd(bool)`: instead of wrapping at the loop end the engine stops
  there, rewinds to its home position and counts the stop in its meters; the
  app then runs `transport.stop` to match.

## Sources

- Count-in (Ctrl+P), wait for input (Ctrl+I), blend and replace recording,
  loop recording, step edit (Ctrl+E) and recording knob movements as automation
  are recording options and shortcuts described in the FL Studio online manual,
  "Recording" and "Keyboard shortcuts" sections; automation inside patterns
  mirrors its per-pattern event automation ("Event editor"). Behaviour is
  described in our own words; the one-bar count-in, the replace sweep rule,
  the automation clip placement, pattern lanes and the command names are our
  own design.
