# Spec: knobs, faders and sliders

How every value control in the interface responds to the mouse. One set of
rules applies everywhere (channel rack, mixer, channel settings, effect
windows, plugin panels, the toolbar), so a gesture learned once works
everywhere.

## Behaviour

- **Left-drag** up or down changes the value; hold Shift for fine steps.
- **Scroll** over a knob or fader nudges it.
- **Middle-click** resets the control to its default: a fader to unity
  (0 dB), a channel volume to the default channel level, a pan to centre, an
  effect or synth parameter to its table default, a mixer send to unity.
  **Double-click** does the same.
- **Right-click** opens the control's menu (create automation, add
  controller, reset). Right or middle dragging never moves the value, so
  opening the menu or resetting cannot nudge it by accident.
- A reset is one undo step, like any other change.

## Sources

- Own design. Middle-click and double-click to reset are common conventions
  in audio software; the rule that only the left button drags is our own.
