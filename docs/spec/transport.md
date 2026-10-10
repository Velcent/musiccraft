# Spec: transport and tempo (M0)

## Behavior

- A project has a **tempo** in beats per minute (BPM) and a **transport** state that
  is either `stopped` or `playing`.
- New projects start `stopped` at 130 BPM.
- Supported tempo range is 20 to 999 BPM inclusive. Values outside the range, and
  non-finite values, are rejected and leave the project unchanged.
- `transport.play` starts playback. If already playing it does nothing and reports so.
- `transport.stop` halts playback. If already stopped it does nothing and reports so.
- `transport.toggle` plays when stopped and stops when playing. The default keymap
  will bind it to the space bar.
- `project.set_tempo <bpm>` sets the tempo. It is undoable; setting the same value
  records no history entry.
- Transport commands are not part of undo history.

## Engine mapping

- The engine tracks position in beats. While playing, each processed block advances
  position by `frames * bpm / 60 / sample_rate`. Tempo changes apply from the next
  block; position never jumps.
- While stopped the engine writes silence and does not advance.
- Stop keeps the current position. Rewind, looping, and pause semantics are
  deferred.

## Sources

- Own design. The range and defaults are our own choices.
- Space-bar play/stop is a common convention in DAWs, described in public
  documentation of many products.
