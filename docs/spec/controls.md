# Spec: knobs, faders and sliders

How every value control in the interface responds to the mouse. One set of
rules applies everywhere (channel rack, mixer, channel settings, effect
windows, plugin panels, the toolbar), so a gesture learned once works
everywhere.

## Behaviour

- **Left-drag** up or down changes the value; hold Shift for fine steps.
  Every knob, tiny or medium, crosses its whole range in 150 points of drag
  (600 with Shift) and 600 points of wheel, and lightens while hovered.
- **Scroll** over a knob or fader nudges it.
- **Middle-click** resets the control to its default: a fader to unity
  (0 dB), a channel volume to the default channel level, a pan to centre, an
  effect or synth parameter to its table default, a mixer send to unity.
  **Double-click** and **Alt-click** do the same.
- **Default detent.** A drag that reaches or crosses a knob's default (a
  fader's unity) stops there until the pointer has travelled a further
  tenth of the range, while a small light marks the default. Holding Ctrl
  while dragging skips the pause.
- **Right-click** opens the control's menu (see "Control menu"). Right or middle dragging never moves the value, so
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
  copied), "Enter exact value…" and "Reset to default", which shows the
  default at its right in dim text, as a percentage of the range like the
  type-in box.

Paste and Reset run `control.set <target> <position 0-1>` and
`control.reset <target>`, each one undo step. Targets are named as for
`automation.create` (`channel:2:pan`, `mixer:0:volume`, `mixer:3:1:mix`).
Copy keeps the control's position along its range, so a value copied from
one control pastes to the same share of another's range.

## Type-in value box

"Enter exact value…" opens a small box just under the control: a value line
in percent of the control's range with the text ready to replace, a fine
slider bound to the same value, then a tick, a cross and the control's name.
Enter, the tick or a click outside the box applies the value with
`control.set` as one undo step; values outside 0–100 % are clamped and text
that is not a number changes nothing. Esc or the cross closes it unchanged.
Values are typed in percent for now; the control's own units come later.

## Rename in place

Double-clicking a channel's name in the channel rack, a mixer track's name,
a playlist track header or a pattern in the playlist's clip list turns the
name into a text field where it is drawn (on a mixer strip, a row at the top
of the upright name), with the whole name selected. Enter or a click
elsewhere renames it as one undo step; Esc, or an empty name, leaves it as
it was. The "Rename…" menu items keep their dialog. The first click on a
channel name still selects it and opens its settings, which stay open.

## Sources

- Own design. Middle-click and double-click to reset are common conventions
  in audio software; the rule that only the left button drags is our own.
- Value text formats: the interface style guide, `docs/spec/style/controls.md`
  ("Value formats"); own design.
