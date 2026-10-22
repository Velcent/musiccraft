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

## Value text

Hints and captions share one formatter per kind of value
(`musiccraft_editors::format`):

- Levels (mixer faders, channel volume, the master knob) read in dB with one
  decimal and a true minus sign: "−6.0 dB", "+1.6 dB", "0.0 dB", and
  "−∞ dB" at silence.
- Pans read as a percent with the side: "25% left", "Centre", "100% right".

## Driven controls

A knob or fader that an automation clip or a project controller drives shows
a small dot in the automation tone: just outside a knob's track at about one
o'clock, or at the top-right corner of a fader's cap. Its hint ends with the
source, for example "Level 0.0 dB · automated (clip 'Master fade')" or
"Pan Centre · driven by LFO 1". A clip wins over a controller when both
drive the same control.

## Control menu

Right-clicking any automatable knob or fader opens the same menu:

- "Create automation clip", then "Open its automation…" (disabled until an
  automation exists for the control; it opens the automation editor).
- Under a dim "Control from" heading: "Add controller" with its kinds, and
  "Remove controller" while one drives the control.
- Under a dim "Value" heading: "Copy", "Paste" (disabled until something is
  copied) and "Reset to default".

Paste and Reset run `control.set <target> <position 0-1>` and
`control.reset <target>`, each one undo step. Targets are named as for
`automation.create` (`channel:2:pan`, `mixer:0:volume`, `mixer:3:1:mix`).
Copy keeps the control's position along its range, so a value copied from
one control pastes to the same share of another's range.

## Sources

- Own design. Middle-click and double-click to reset are common conventions
  in audio software; the rule that only the left button drags is our own.
- Value text formats: the interface style guide, `docs/spec/style/controls.md`
  ("Value formats"); own design.
