# Style guide: Controls and widgets

Part of the [interface style guide](README.md). Read it with the shared rules in [design language](design-language.md), [controls](controls.md) and the window header in [main window](main-window.md).

Every window in MusicCraft is built from a small set of controls: knobs, faders and sliders, LCD-style number wells, LED switches, push and toggle buttons, step buttons, drop-down fields, menus, text fields and scroll bars. The reference manual's interface reuses the same few kinds everywhere, so a gesture learned on one knob works on every knob, and we keep that rule. Aim for small, dense, quiet controls whose **value** is the brightest thing about them: a lit arc, a lit LED, bright digits in a dark well. The chrome around them stays matte and recessive. Every control explains itself in the hint bar, and every value control opens the same right-click menu. This section is the shared vocabulary the other sections mean when they say "a small knob", "an LCD well" or "the control menu". Behaviour already fixed in `docs/spec/controls.md` and `docs/spec/controllers.md` is restated only where it affects the look; anything new is marked **Proposed (needs `<spec>.md` updated first)**, and today's behaviour stays the default until that spec changes.

## Placement and proportions

Controls are not a window of their own. They appear inside every panel, so this subsection sets their **size classes** and how tightly they pack. Our absolute sizes come from `docs/spec/window-layout.md` and the existing widgets; everything below is relative to them.

**Base unit.** The base unit is the toolbar button (see window-layout.md, "Tool and transport buttons"). One **row** is one button tall. Most controls are sized in rows, and most panels are a whole number of rows tall.

**Knob size classes** (all round, all drawn the same way; only the size changes):

| Class | Relative size | Where it is used | Caption |
|---|---|---|---|
| **Tiny** (`widgets::knob`) | A little smaller than one row, so it fits inside a list row | Channel rack volume and pan, mixer pan, separation and send knobs, the mix knob in an effect slot, EQ knob rows | None. The value shows only in the hint bar. |
| **Small** | About one row | Toolbar master volume, knobs in a window's control strip | None, or a tiny dim caption under it when the row has room (plugin-windows section) |
| **Medium, captioned** (`widgets::labeled_knob`) | About two rows of knob plus a caption line under it, so the cell is a bit taller than wide | Every parameter knob in instrument and effect editors | One short dim word under the knob. While hovered or dragged, the caption shows the value instead. |
| **Large, feature** (optional) | About one and a half times the medium knob | At most one or two per editor, for the parameter that matters most (for example an effect's main amount) | As for medium |

- Knobs in a group sit on an even grid with equal gaps. A gap is about a quarter of the knob's own width. Captions never wrap. A caption longer than its cell is shortened, never set smaller.
- A row of medium knobs is the building block of an editor page. A group box holds one or two such rows (see Anatomy, S).

**Faders and sliders.**

- A **vertical fader** is many times taller than wide. On a mixer strip it fills about a third of the strip height (mixer section). Inside editors, short faders are about three medium knobs tall.
- A **horizontal slider** is one row tall or less and many times wider than tall. It is used where a long throw matters (sample start, crossfade, the fine slider in the type-in box). Its length is set by its container, never by its range.

**LCD-style number wells.**

- A **large well** is one row tall or a little taller and holds a big value with a tiny caption. The toolbar's time and tempo displays are the examples (toolbar section, which owns their exact layout).
- A **compact number box** is a little less than one row tall and two to three digits wide, with tiny step chevrons stacked at its right end. It holds small integers: the mixer track a channel feeds, a pitch range, a MIDI channel, a time-signature part. It is about the size of a tiny knob plus a digit or two, so it sits in the same rows as tiny knobs.

**Switches, buttons and fields.**

- **LED switches** (round on/off lights) are about half a row across and come in two variants:
  - **In knob rows** (control strips, mixer strips): the light with a tiny caption under it, on the same line as the knobs' captions so it lines up with them (plugin-windows section, "On light").
  - **In option lists**: the light with a body-size label to its right. A column of them stacks one per row.
- **Push and toggle buttons** are one row tall. Icon buttons are square or very slightly wider than tall (window-layout.md). Text buttons are as wide as their label plus a small margin.
- **Step buttons** are taller than wide (window-layout.md, "Step buttons").
- **Drop-down fields** and **text fields** are one row tall. Their width follows the longest expected text, typically six to twelve tiny-knob widths.
- **Scroll bars** follow the main-window section: a little less thick than a window header row, halved by the "thin scroll bars" setting. Their end buttons are square.

**Density.** The interface is compact. Controls sit a fraction of a row apart, group boxes are separated by a thin even gap rather than by open space, and no group has padding beyond about a quarter row. Prefer more rows over wider spacing.

**Popups.** Menus, the type-in box, the colour popover and tooltips float above everything. They open next to the control that summoned them, below it or to its right, and flip to stay on screen. They never dock and never resize. A menu is as wide as its widest row. The type-in box is about as wide as six medium knobs. The colour popover is as wide as its swatch grid.

## Anatomy

### A. Knob (all size classes)

Drawing, from back to front:

1. **Well**: a recessed disc a little larger than the body. Drawn only for medium and large knobs, it seats the knob in the panel.
2. **Track arc**: the whole sweep the knob can travel, from about the seven o'clock position clockwise to about the five o'clock position, leaving a gap at the bottom. It is a thin unlit arc just outside the body. Tiny knobs show it too, so a knob at its minimum still reads as a knob.
3. **Value arc**: the lit part of the track.
   - **Unipolar** (volume, cutoff, mix, attack): lit from the start of the sweep to the current value.
   - **Bipolar** (pan, detune, stereo separation, EQ gain, fine pitch): lit from the top centre (twelve o'clock) toward the value, left or right. At exactly the centre nothing is lit, so an untouched bipolar knob looks quiet.
   - **Default tick**: a tiny tick outside the track at the default position, drawn only when the default is neither the start of the sweep nor the centre.
4. **Body**: a plain filled disc. Flat, with no gradient and no bevel, as our `widgets::knob` draws it today.
5. **Pointer**: a short straight line toward the rim, pointing at the value. On tiny knobs it starts at the centre. On medium and larger knobs it starts a little out from the centre, so the middle stays a clean disc.
6. **Status mark** (only when the control is automated, linked or driven): a tiny dot just outside the track at about the one o'clock position. See M.
7. **Caption** (medium and large only): one short word centred under the knob in small dim text. While hovered or dragged it shows the formatted value in bright text, as `labeled_knob` does today.

Interaction. Today's rules from `docs/spec/controls.md` stay:

- **Left-drag up or down** changes the value; horizontal motion is ignored. Shift gives fine steps.
- **Wheel** nudges the value. Shift with the wheel gives finer steps.
- **Middle-click or double-click** resets to the default. The arc snaps there with no animation. A reset is one undo step.
- **Right-click** opens the control menu (L). Right- and middle-drags never move the value.
- **Hover**: the body lifts one step and the hint bar shows "Target: value" (see Labels). The pointer stays the normal arrow; there is no hover cursor change, because panels are full of knobs.

Additions, each **Proposed (needs `controls.md` updated first)**:

- **Default detent.** While dragging, the value pauses briefly at the default and, on bipolar knobs, at the centre, then carries on after a short stretch of pointer travel. Holding Ctrl while dragging skips the pause. While held at the detent, the default tick (or the twelve o'clock point) lights.
- **Alt-click** also resets, for users who expect it.
- **Hidden pointer while dragging.** The pointer is hidden and only the arc and pointer line move. The hint bar shows the live value, and medium knobs also show it in the caption. When the drag ends, the pointer reappears where the drag began, so a long drag never walks the mouse off the control.
- **Keyboard.** A knob that has keyboard focus (after a click) responds to Up/Down (one step), Page Up/Page Down (one tenth of the range) and Delete (reset). None of these keys is bound in `default.keymap`. Home and End are **left unbound** here because Home is already "jump to start" (`transport.jump_to_start`). Focus shows as a thin ring (see States).
- **Deliberately left out:** fine adjustment by holding both mouse buttons while dragging. It would contradict the `controls.md` rule that only the left button moves a value.

### B. Vertical fader

From top to bottom:

1. **Groove**: a narrow recessed slot the full length of the travel, darker than the panel.
2. **Scale ticks** (optional, wide strips only): short dim ticks on one side of the groove at round dB values. The **unity tick** is longer and sits at three quarters of the travel, as our existing fader curve puts it.
3. **Cap**: a flat rectangle wider than the groove, about as wide as the strip allows and much less tall than wide. A thin line across its middle is the reading line, lit in the accent.
4. **Value read-out** (wide mixer strips only): a small dB number under the fader (mixer section).

Interaction: as for knobs (drag the cap, Shift fine, wheel, middle- or double-click back to unity, right-click for the control menu). Dragging the cap is relative, so it never jumps. Hover lifts the cap one step. While dragging, the cap takes the dragging tone. **Proposed (needs `mixer.md` updated first)**, as in the mixer section: a press on the groove above or below the cap glides the cap toward the pointer at a steady speed while the button is held, instead of doing nothing.

### C. Horizontal slider

From left to right:

1. **End ticks**: a tiny vertical tick under each end of the track, marking minimum and maximum.
2. **Track**: a thin recessed line the full length of the control.
3. **Filled part**: the track is lit from its left end (unipolar) or from its centre (bipolar) up to the handle.
4. **Handle**: a small round cap with a short upright reading line, centred on the track.

Interaction (new widget, so its rules go into `controls.md` with it): drag the handle left or right (Shift fine). A press on the track elsewhere glides the handle toward the pointer, like the fader proposal. Wheel nudges. Middle- or double-click resets. Right-click opens the control menu. With keyboard focus, the arrow keys step.

### D. LCD-style displays

**D1. Large well** (the toolbar's time and tempo displays, and the general pattern for any large numeric read-out):

1. **Well**: a dark recessed rectangle with a thin edge and slightly rounded corners, the darkest chrome in its band.
2. **Digit groups**, left to right, separated by a dim separator character (`.` or `:`, as the format table below gives). The leading groups are full size. The **least significant group is noticeably smaller** and sits on the same baseline. Examples: tempo "124" large followed by ".000" small; position "12.3" large followed by ".4" small. Digits are monospaced, so the width never jitters as the value changes.
3. **Step chevrons**: two tiny chevrons stacked at the right inside edge, up above down, dim until the well is hovered.
4. **Caption**: a tiny dim word naming the unit or mode (`BPM`, `BAR.BEAT.STEP`, `MIN:SEC`). Its position follows the toolbar section, which puts it in the well's top-right corner.

Interaction:

- **Drag up or down on a digit group** changes that group only: for tempo, the large group moves whole BPM and the small group moves thousandths (toolbar section). Shift gives fine steps within the group.
- **Wheel** over the well steps the value (toolbar section).
- **Edge zones**: hovering near the top or bottom edge lights the matching chevron and turns the pointer into a small up or down arrow. A click there steps the value by one unit. **Proposed (needs `transport.md` updated first)**: holding the button repeats after a short delay.
- **Typing a value**: double-click turns the digits into a text caret in the same well (toolbar section). Enter commits, Esc cancels, focus loss commits. Invalid input flashes the well edge in the danger tone and keeps the old value. **Proposed (needs `controls.md` updated first)**: pressing and holding without moving, then typing, does the same.
- **Right-click**: the control menu (L). Displays with **modes** put their modes at the top as radio items. Tempo adds its own quick items (tap, a few common tempos, a half-tempo listening preview) inside the same single list; the toolbar section owns their wording and order.
- **Display-only wells** (the time display) take no drag. A click cycles their mode, and the caption names the current mode (toolbar section).

**D2. Compact number box** (small integers):

1. A small dark well, a little less than one row tall, with right-aligned digits at body-text size or a little larger.
2. Tiny stacked up/down chevrons at its right edge, dim until hovered.
3. A label: to the left in forms, or as a tiny caption under it in knob rows, styled like knob captions.

Interaction: as D1 with a single digit group (drag, wheel, edge chevrons, double-click to type, right-click menu). Out-of-range values clamp, and the clamp shows as a brief flash of the edge. Boxes that allow "nothing", such as a channel routed to no mixer track, show a dim long dash in place of the digits.

### E. LED switches

1. **Light**: a small round light with a thin dim ring. **Off** is a recessed dark fill. **On** is a lit dot that nearly fills the ring.
2. **Label**: the two variants from Placement. In option lists the label sits to the right, and clicking the label toggles too. In knob rows the caption sits under the light.

Used for independent options ("Reverse", "Normalize", "Loop") and, in exclusive groups, as radio choices. A **radio group** stacks its lights one per row, or wraps into two columns when there are many. Exactly one is lit, and clicking the lit one does nothing.

Interaction: left-click toggles. Right-click opens the control menu when the switch is automatable (on/off parameters are). Hover brightens the ring. Disabled lights are drawn at reduced strength together with their label and do not react.

A **bare light** with no label (our `widgets::dot`) is used where space is tight: mixer track enable, playlist track mute, controller on/off. Its meaning comes from the hint bar.

### F. Push buttons and toggle buttons

1. **Icon button** (toolbar, window headers, tools): a square raised tile with our vector glyph centred (`icons::button`).
2. **Text button**: a raised tile with a short label ("+ Insert", "Clip", "Save…", "Open…").
3. **Toggle button**: the same tile. When on, it fills with the accent and its glyph or text switches to the dark tone (`icons::button` with `active`). Window show/hide toggles are quieter: when on, they get only an accent glyph and an accent underline (`icons::toggle`; toolbar section).
4. **Segmented switch**: two to four adjoining text segments in one recessed frame, exactly one lit (`icons::segmented`, for example `Pattern | Song`). Mutually exclusive mode tabs ("Low-pass | Band-pass | High-pass") use the same widget.
5. **Small "ON" tile** (mixer track enable on wide strips): a tiny text tile lit when on (mixer section).

Interaction: a press shows the pressed tone, release over the button triggers it, release outside cancels. Toggles flip on release. Right-click on a toggle bound to an automatable parameter opens the control menu; otherwise it opens the button's own options if it has any (for example the recording options on the record button). Buttons with a drop-down of their own show a **tiny down chevron** (two short strokes) at their bottom-right corner, the same mark the toolbar section uses.

### G. Step buttons

Covered in the channel rack section. As a reusable control:

1. A tall rounded tile, taller than wide.
2. Off: the beat-group tone, alternating between `STEP_A` and `STEP_B` every beat.
3. On: the channel colour. Loud steps (high velocity) are full strength; quiet steps are blended toward the off tone.
4. Playing step: a thin bright edge on the step under the play position while the pattern plays.

Interaction: left-click toggles, left-drag paints, right-drag erases (existing). Hover lifts an off step one step. **Proposed (needs `channel-rack.md` updated first)**, optional: the wheel over a lit step changes its velocity.

### H. Drop-down fields

From left to right:

1. **Field**: a recessed rectangle one row tall, a little darker than the panel.
2. **Current choice**: body text, left-aligned, shortened with an ellipsis if too long.
3. **Opener mark**: a small down chevron at the right end, dim until hovered.

Interaction:

- Left-click opens the list as a menu (I) directly under the field, at least as wide as the field, with the current choice checked.
- **Proposed (needs `controls.md` updated first)**: the wheel over the closed field steps to the previous or next choice without opening the list, and the hint bar names the new choice.
- Right-click: the control menu if the choice is automatable (filter type, waveform); otherwise nothing.

**Preset selector** (instrument and effect window headers; plugin-windows section, which places it): a drop-down field showing the preset name, with a **previous** button to its left and a **next** button to its right. Both are small square icon buttons with left and right chevrons. Clicking them steps through presets in browser order, wrapping at the ends. Right-click on either opens the full preset list (the same as clicking the field).

### I. Menus (menu-bar menus, window menus, drop-down lists, right-click menus)

A menu is a floating panel with a thin edge, a step lighter than the panel it opens over, and a faint shadow. Each **row** is a little less tall than a button row. Columns, left to right:

1. **Check gutter**: a narrow column holding a check mark for items that are on and for the current choice in a list. Otherwise it is empty, so labels line up whether or not they carry a check.
2. **Label**: body text in our own words. Items that open a dialog end with "…".
3. **Shortcut text**: right-aligned, dim, in the form "Ctrl+Shift+S". It comes from the keymap and is never hard-coded (as `toolbar.rs` already does with `shortcut_text`).
4. **Submenu chevron**: a small right chevron at the far right on items that open a submenu. The submenu opens to the right on hover after a short delay, its top row level with the parent row.

Row kinds:

- **Normal item**: as above.
- **Check item**: the check shows in the gutter when on. A left-click toggles it and closes the menu. **Proposed (needs `controls.md` updated first)**: a right-click toggles it **without closing the menu**, so the user can try an option and see the effect.
- **Radio item**: like a check item, but only one in its group is checked.
- **Section heading**: a full-width row on a slightly darker band, in small dim text (for example "Control from", "Value"). Not clickable. It groups the items below it until the next heading or separator. Long menus use headings instead of nested submenus.
- **Separator**: a thin horizontal line inset from both edges.
- **Disabled item**: label and shortcut drawn dim, no highlight on hover.
- **Draggable item**, **Proposed (needs `controls.md` updated first)**: an item that produces a file or object (for example "Save preset as…", "Export as audio…") carries a small **drag grip** in the check gutter. Dragging the row out of the menu drops the result on the browser, playlist or another window instead of clicking it (N).

Interaction: hovering highlights the whole row edge to edge and puts the item's hint in the hint bar. Up and Down move the highlight, Right opens a submenu, Left or Esc closes one level, Enter activates, and a typed letter jumps to the next item starting with it. A click outside closes every level. Very long menus (preset lists) scroll inside the panel with small up/down arrow rows at the ends, never with a scroll bar. Menus are single-column; quick values that belong to one control (the tempo display's) are a headed group inside the same list.

**Window menus.** Every window header starts with the window menu button at its top-left corner (three short stacked bars; main-window section). It opens that window's menu, which looks like every other menu.

### J. Text fields

1. **Field**: a recessed rectangle one row tall, the same tone as the drop-down field.
2. **Text**: body text, left-aligned, with a thin accent caret while focused. When empty, a dim placeholder says what goes there ("Search…").
3. **Clear button** (search fields only): a tiny cross at the right end, shown only when there is text.

Interaction: click to focus, which draws the focus outline. Enter commits, Esc reverts and leaves the field, focus loss commits (as `controllers.md` does for the formula field). A multi-line field (formula, notes) grows downward up to a few rows and then scrolls. Keyboard shortcuts are suspended while a field has focus.

**Rename in place**, **Proposed (needs `channel-rack.md`, `mixer.md` and `playlist.md` updated first)**: a double-click on a name (channel, insert, pattern, playlist track) turns it into a text field of the same size in place, with the whole name selected. Enter or focus loss commits; Esc cancels. Where double-click already has a meaning on that name, that meaning wins and renaming stays in the menu. Our "Rename…" dialogs stay for menu use.

### K. Type-in value box (from "Enter exact value…" in the control menu)

A small floating box opened just under the control. **Proposed (needs `controls.md` updated first)**, since only dragging exists today. Top to bottom:

1. **Value line**: the current value in large bright digits, as an editable text field with all its text selected.
2. **Unit switch**: at the right end of the value line, a pair of tiny toggle tiles choosing how the value is entered and shown for this control: the control's **own units** (dB for volume, Hz for a frequency, semitones for pitch) or **plain percent**. Controls with no units of their own show only percent and no switch.
3. **Fine slider** (own design; one box instead of a separate larger control window): a full-width horizontal slider (C) bound to the same value. Its drag is much finer than the knob's, for precise setting with the mouse.
4. **Footer row**: a confirm button (check glyph) and a cancel button (cross glyph) at the left, then a dim caption naming the target ("Insert 3 · Volume").

Enter or the check commits as one undo step. Esc or the cross cancels. A click outside commits. Values outside the range are clamped, and the clamp shows as a brief flash of the edge.

**Value units, app-wide.** A setting "Show values as" (own units / plain percent; own units by default) chooses how every caption, hint and read-out formats values. Its two choices are the same two the unit switch offers per control. Both use the shared formatters in Labels.

### L. The control menu (right-click on any automatable control)

The same menu for every knob, fader, slider, LED switch, number box, automatable drop-down and the tempo well. Groups, top to bottom (our wording; the grouping is our own):

1. *Mode items, for displays with modes only (radio items).*
2. **Automation** group:
   - "Create automation clip" (exists today): creates the automation and drops a clip on a free playlist track.
   - "Open its automation…": opens the existing clip in the automation editor. Disabled when there is none.
   - "Show recorded moves…": opens the editor for moves recorded on this target. Disabled when there are none.
   - "Use as the value at song start": stores the current value as the target's starting value. Hidden for targets with no starting value.
   - "Clear song-start value": disabled when nothing is stored.
3. Heading **"Control from"**:
   - "Learn from hardware…": waits for the next hardware knob movement and links it. A dialog then shows the link, its range and its mapping curve (`controllers.md`).
   - "Add controller ▸" (exists today as "Add controller"): a submenu of our kinds, with today's labels: "LFO", "Macro knob", "Envelope (first channel's notes)", "Keyboard (first channel's key)", "Peak (insert 1's level)", "Formula".
   - "Remove link": shown only when the control is linked or driven by a controller.
4. Heading **"Value"**:
   - "Copy" and "Paste" (Paste is disabled when the clipboard holds no value).
   - "Enter exact value…": opens the type-in box (K).
   - "Reset to default": the default follows in dim text, for example "Reset to default (0.0 dB)".

Each item's hint says what it will do to this particular target ("Creates an automation clip for Insert 3 · Volume"). Everything beyond the two existing items is **Proposed (needs `controls.md` and `controllers.md` updated first)**.

### M. How automated and linked controls are marked (own design)

The manual explains linking and automation but gives no visual mark. Ours:

| Situation | Mark |
|---|---|
| An automation clip exists for the control | The status dot (knob, slider handle, top edge of a fader cap, top right of an LED ring, a corner of a number box) in the automation tone |
| Linked to a hardware controller | The status dot drawn as a hollow ring in the automation tone |
| Driven by a project controller (LFO, macro and so on) | A filled dot plus a dim band along the track arc showing the controller's output range |
| Playing, with the value written by automation or a controller | The value arc and pointer take the automation tone instead of the accent and move live. A manual drag during playback still works; when released, the arc returns to the automation tone. |
| Recording automation (record armed, user dragging) | The value arc takes the recording tone for the length of the drag |

The hint bar appends the source to the value: "Insert 3 · Volume: −4.2 dB · automated (clip 'Master fade')" or "… · linked (MIDI ch 1, CC 49)".

### N. Drag handles and drag sources

- **Menu drag grip**: see I, "Draggable item". The row shows a grab pointer over the grip.
- **Marker handles** (sample editor loop start and end, playlist and piano-roll markers): a small flag hanging from the top edge, or standing on the bottom edge, of the marker line, coloured by the marker's role. Dragging moves the marker in time. Double-click renames it where it has a name. Right-click opens the marker's menu.
- **Divider handles** (panel splitters): three small dots across the divider. The pointer becomes a resize arrow. Double-click restores the default split.
- **Drag sources** (browser rows, pattern list rows, channel name buttons, effect slots): the whole row is the handle, with no separate glyph. While dragging, the drag ghost follows the pointer (main-window section) and drop targets get the focus outline.
- **Waveform displays** (sample editor, a sampler's sample page, the browser's preview strip), **Proposed (needs `sample-editor.md` updated first)**: a left-drag that starts on the waveform drags the sample out, to drop on a sampler channel or a plugin that takes audio files, the same as dragging its browser row. Where the waveform already uses left-drag (selection in the sample editor), the drag starts from a small grip in the display's corner instead.
- *Option*: "Animate drags" in settings. When off, the ghost is replaced by a plain pointer change.

### O. Scroll bars (with zoom)

The main-window section owns the full scroll-bar rules, including thickness, placement and which windows get which kind. In short:

- **Full bars** (playlist, piano roll), horizontal, left to right: a left chevron button (click steps, holding repeats), the recessed **rail**, the **thumb** (its length is the visible fraction) with a **zoom grip** at each end, a right chevron button, then the **vertical zoom box**, a small square at the far right end of the row, above the vertical bar. The vertical bar has an up chevron, rail, thumb and down chevron.
  - Drag the thumb's body: scrolls.
  - Drag a zoom grip: moves that edge of the visible range, zooming while the opposite end stays put.
  - Drag the zoom box up or down: changes the row height. Its glyph shows the level.
  - Proposed in the main-window section (needs `playlist.md` and `piano-roll.md` updated first): right-drag on the thumb scrolls both ways at once; middle-drag on empty content pans both ways.
  - Own addition, under the same proposal: double-click the thumb to fit everything, and double-click the zoom box to reset it.
- **Simplified bars** (channel rack, mixer, browser): rail, thumb and end chevrons only.
- **Menus** scroll with arrow rows (I). Dialog lists may use a thumb-only bar.
- Setting "Thin scroll bars" halves the thickness in all editors.

### P. Hint bar and tooltips

- The **hint bar** is the main way controls explain themselves (toolbar section; `hint.rs`). Hovering any control writes one line there. Controls show **no tooltips while the hint bar is visible**.
- With the hint bar hidden, the same text appears as a tooltip after a short hover delay, as `hint.rs` does today: a small floating box near the pointer.
- The **floating hint panel** (optional, View menu; toolbar section) shows the same hint in larger text in a small movable box. Its move grip and progress ring use the toolbar section's glyph.
- While any value control is being dragged, the hint bar shows the live value until the drag ends, even if the pointer leaves the control.

### Q. Colour choice

Our colours are picks from the 16-entry playlist palette (`theme::track_color`: the eight clip colours, then darker versions of them; the `set_color … <1-16|none>` commands).

**Colour popover** (from a "Colour ▸" item in channel, pattern, insert and track menus; a plain swatch grid exists today in the playlist track header menu):

1. **Swatch grid**: two rows of eight square swatches, the clip colours on top and their darker versions below. The current colour has the focus outline.
2. **Recent row**: the last few colours used, as smaller swatches, right below.
3. **Action row**: a "No colour" text button, a **random** icon button (picks a palette colour not yet used nearby), and a **sequence** icon button that colours the selected items one after another along the palette (own design; for groups of channels or inserts).

Click a swatch: apply and close. Hover: the hint names the swatch ("Colour 5") and the target previews it live. **Proposed (needs `playlist.md` updated first)**: right-click a swatch to apply it without closing, to compare.

Free colours are not supported. If they are ever added, a colour dialog will be designed then as our own layout; until then colour choice is this popover.

### R. Meters, scope and load bar (read-only)

Covered by the toolbar and mixer sections. Here only: they are **read-only wells** in the LCD style (`icons::scope`, `icons::load_bar`, `widgets::meter`). They take no drag. Click behaviour is defined where they are used, and right-click gives their own options (scope mode, meter range).

### S. Group box (how controls are grouped in editors)

Matches the plugin-windows section's group-box rule (`instrument::section`):

- A group box is a rounded panel a step darker than the page, with its title in small dim capitals inside its top-left corner. There is no tab or band on the edge.
- Optional, own design: a bypass light just before the title for groups that can be switched off, and a small fold chevron at the right end of the title line.
- Inside a box the order is: a row of mode controls (segmented switch or drop-down), then one or two rows of medium knobs, then an optional small graph (envelope shape, filter curve) beside or under the knobs.

## Icons

| Where | What it does | Our glyph |
|---|---|---|
| LCD well, number box: right edge | Step the value up or down | Two tiny chevrons stacked, up above down, each two short strokes |
| Drop-down field: right end | Opens the list | A small down chevron, two short strokes meeting at a point |
| Menu row: right end | Opens a submenu | A small right chevron |
| Menu row: check gutter | Item is on or chosen | A check mark: a short stroke down-right, then a longer stroke up-right |
| Menu row: check gutter (draggable items) | Drag the result out of the menu | A grip: two columns of three small dots |
| Preset selector | Previous / next preset | Left and right chevrons, two strokes meeting at a point (as in the plugin-windows section), each on its own small square tile |
| Type-in box footer | Confirm / cancel | A check mark; a cross of two strokes |
| Type-in box value line | Switch unit | Text tiles ("dB", "%"), not glyphs |
| Search field: right end | Clear the text | A small cross |
| Scroll bar ends | Scroll one step | Small chevrons pointing left, right, up or down (main-window section) |
| Scroll thumb ends | Zoom grips | Two very short parallel ticks inset at each end of the thumb (main-window section) |
| Vertical zoom box | Vertical zoom | Two horizontal lines in the small square, close together at minimum zoom and at the square's top and bottom edges at maximum (main-window section) |
| Divider handles | Resize panels | Three small filled dots in a line across the divider |
| Group box title (optional) | Fold the group | A small chevron, pointing right when folded and down when open |
| Colour popover | Random colour | A die: a rounded square with three dots on its diagonal |
| Colour popover | Colour the selection in sequence | Three small squares in a row, stepping up from low to high |
| Floating hint panel | Move grip and progress | The toolbar section's glyph: an arc ring that fills clockwise around a small four-way arrow cross |
| Button with its own options | Has a drop-down | A tiny down chevron (two strokes) in the bottom-right corner |
| Knob / control status | Automated or driven; linked | A small filled dot; a small hollow ring |

All glyphs are drawn in the style of `crates/musiccraft-ui/src/icons.rs`: a few strokes of even weight, square-ish proportions, and no fills except small dots.

## Colour roles

Tokens live in `crates/musiccraft-editors/src/theme.rs` and are named here by token only. Names marked **new** are proposals. Tokens proposed by other sections are reused: `LCD_TEXT` and `SEPARATOR` (toolbar), `SCROLL_RAIL`, `SCROLL_THUMB` and `WINDOW_EDGE` (main-window), `SELECTION_FILL` (piano roll).

| Element | Token(s) | Relative tone and state changes |
|---|---|---|
| Knob well | `PANEL_DARK` on a `PANEL` surface; `LCD_BG` inside a group box | Always one step darker than the surface it sits on |
| Knob track arc (unlit) | new `KNOB_TRACK`: the same tone as `LCD_EDGE` | Just visible; must read on `PANEL`, `PANEL_DARK` and `RAISED` |
| Knob value arc | `ACCENT` | Disabled: `ACCENT_DIM`. Automated or driven while playing: `AUTOMATION`. Recording a drag: `DANGER`. |
| Knob body | `RAISED` | Hover and drag: `RAISED_HOVER` |
| Knob pointer | `TEXT` | Disabled: `TEXT_DIM` |
| Default tick, detent light | `TEXT_DIM`; lit `HIGHLIGHT` | Lit only while the value is held at the detent |
| Knob caption | `TEXT_DIM` | Hovered or dragged: the value in `LCD_TEXT` (today it borrows `PLAYHEAD`) |
| Status dot / ring | `AUTOMATION` | Hollow ring for hardware links, filled for clips and controllers |
| Controller range band | `AUTOMATION` at low strength | Only for controls driven by a project controller |
| Fader groove | `METER_BG`, edge `LCD_EDGE` | — |
| Fader cap | `RAISED`, edge `TEXT_DIM`, reading line `ACCENT` | Hover and drag: `RAISED_HOVER`. Automated: reading line `AUTOMATION`. |
| Fader unity tick, scale ticks | `TEXT_DIM` | — |
| Slider track / fill / handle | Track `PANEL_DARK`, fill `ACCENT`, handle `RAISED` with a `TEXT` reading line | Hover: handle `RAISED_HOVER` |
| LCD well | `LCD_BG` fill, `LCD_EDGE` edge | Invalid entry: edge flashes `DANGER` |
| LCD digits | `LCD_TEXT` (toolbar section) | The brightest text in its region. The small group uses the same token, a little dimmer for tempo decimals (toolbar section). |
| LCD separators, caption, step chevrons | `TEXT_DIM` | Hovered chevron: `TEXT`. Pressed: `ACCENT`. |
| LCD while typing | Digits `TEXT`, caret `ACCENT` | Selected text: `SELECTION_FILL` |
| Number box "nothing" dash | `TEXT_DIM` | — |
| LED off | `PANEL_DARK` fill, `TEXT_DIM` ring | Hover: ring `TEXT` |
| LED on | `ACCENT` fill | Mute and solo lights use the role colour their own section names. Disabled: `ACCENT_DIM`. |
| Icon button, off | `RAISED` fill, glyph `TEXT` (toolbar section) | Hover: `RAISED_HOVER`. Pressed: `ACCENT_DIM`. |
| Toggle button, on | Accent fill, glyph `BACKGROUND` | — |
| Window toggle | No fill, glyph `TEXT_DIM` | Hover: `RAISED_HOVER` fill. On: glyph `ACCENT` with an `ACCENT` underline. |
| Segmented switch | Frame `PANEL_DARK`; lit segment `HIGHLIGHT` with `BACKGROUND` text | Unlit text `TEXT_DIM`; `TEXT` on hover |
| Step button | `STEP_A` / `STEP_B` off; on: the channel colour (`theme::palette`) | Hovered off step: `RAISED`. Quiet steps: the channel colour blended toward the off tone. |
| Drop-down field, text field | new `FIELD`: the same tone as `PANEL_DARK` | Hover: edge `LCD_EDGE`. Focused: edge `ACCENT`. |
| Field text / placeholder | `TEXT` / `TEXT_DIM` | Disabled: everything `TEXT_DIM` |
| Menu panel | new `MENU_BG`: the same tone as `RAISED`, edge `WINDOW_EDGE` | — |
| Menu row hover | `ACCENT_DIM` | Text stays `TEXT` |
| Menu section heading | new `MENU_HEADING`: the same tone as `PANEL`, text `TEXT_DIM` | One step darker than the menu panel |
| Menu separator | `SEPARATOR` | — |
| Menu shortcut text, chevrons | `TEXT_DIM` | On the hovered row: `TEXT` |
| Menu check mark | `ACCENT` | On the hovered row: `TEXT` |
| Disabled menu item | `TEXT_DIM` at reduced strength | No hover fill |
| Type-in box | `MENU_BG`; value digits `LCD_TEXT` on `LCD_BG` | Unit tile on: `ACCENT` fill |
| Scroll rail / thumb / zoom grips / zoom box | As the main-window section: `SCROLL_RAIL`, `SCROLL_THUMB`, grips `TEXT_DIM`, zoom box `LCD_BG` with `LCD_EDGE` | Thumb hover `RAISED_HOVER`, dragging `ACCENT_DIM`. Grips and zoom-box lines `ACCENT` while dragged. |
| Focus outline (keyboard focus, drop target) | `ACCENT` | Drop target: outline plus `SELECTION_FILL` |
| Tooltip | `MENU_BG`, text `TEXT` | — |
| Group box | `PANEL_DARK` body, title `TEXT_DIM` (as `instrument::section` today) | Bypassed group: contents drawn at reduced strength |
| Colour swatches | `theme::track_color(n)` | Current: `ACCENT` outline. Hover: `TEXT` outline. |

## Labels and wording

- **Knob captions** are one short word, two at most, in sentence case ("Cutoff", "Reso", "Env amt"), as the editors already do. The value replaces the caption while the knob is hovered or dragged.
- **Group box titles** are short and in capitals ("SUB", "NOISE", "ROUTING", "ARPEGGIATOR"), as today.
- **Hint bar text** for a value control: `<Target>: <value> <unit>`, then a dim middle dot and the gestures when there is room. Examples:
  - "Insert 3 · Volume: −4.2 dB · drag, Shift fine, double-click resets"
  - "Kick · Pan: 25% left"
  - "Lead · Cutoff: 2.40 kHz"
  - For buttons: "<What it does> (<shortcut>)", for example "Metronome (Ctrl+M)".
- **Value formats.** One formatter per kind, shared by captions, hints, the type-in box and the "Reset to default (…)" text:

| Kind | Format | Examples |
|---|---|---|
| Level | dB, one decimal, true minus sign, "−∞ dB" at silence | "−4.2 dB", "+1.5 dB", "−∞ dB" |
| Pan, balance | percent with a side word, "Centre" at zero | "25% left", "Centre", "100% right" |
| Generic 0–1 | percent, no decimals | "64%" |
| Bipolar generic | signed percent | "+12%", "−40%" |
| Frequency | Hz below 1000, kHz with two decimals above | "120 Hz", "2.40 kHz" |
| Time | ms below one second, s with two decimals above; note lengths when tempo-synced | "35 ms", "1.20 s", "1/8 beat" |
| Pitch | semitones and cents | "+7 st", "−12 cents" |
| Tempo | three decimals, whole part large (toolbar section) | "124.000" |
| Position | `bar.beat.step`, or `m:ss.cc` in time mode (toolbar section) | "12.3.4", "2:05.47" |

- With "Show values as" set to plain percent, every kind above except tempo and position is shown on a 0–100% scale.
- **Menu wording** follows L: verbs, sentence case, "…" on items that open a dialog.
- **LCD captions** stay as they are: `BPM`, `BAR.BEAT.STEP`, `MIN:SEC` (toolbar section).
- No labels inside knobs, and no units in captions.

## States and feedback

| State | Knob / slider / fader | LCD / number box | LED / buttons | Fields and menus |
|---|---|---|---|---|
| **Idle** | Body `RAISED`, accent arc | Dim chevrons | Raised tile / dark LED | Recessed field |
| **Hover** | Body one step lighter; value in the hint bar; caption shows the value | Chevrons brighten; edge zones change the pointer | Tile one step lighter; LED ring brightens | Field edge appears; menu row filled |
| **Pressed** | — | Chevron lit | Tile fill drops to the dim accent | — |
| **Dragging** | Body stays lifted; live value in caption and hint; proposed: pointer hidden, detent light at the default | Only the dragged group changes | — | — |
| **On / active** | — | — | Accent fill (buttons), lit dot (LED), underline (window toggles) | Check in the gutter |
| **Selected** | Selection belongs to the containing strip or row, not to the control | — | — | Current choice checked |
| **Focused (keyboard)** | Thin accent ring just outside the track | Accent edge, caret while typing | Accent ring | Accent edge |
| **Disabled / inert** | Whole control at reduced strength; arc in the dim accent | Digits dim | Tile and glyph dim | Dim text, no hover |
| **Owner off** (muted channel, bypassed effect) | Controls stay live and are not dimmed (plugin-windows section); a bypassed group box dims its contents | Same | Same | — |
| **Automated / driven** | Status dot; arc in the automation tone while playing | Corner dot | Dot on the ring | — |
| **Linked** | Hollow status ring | Same | Same | — |
| **Recording** (record armed and dragging) | Arc in the danger tone | — | — | — |
| **Invalid entry** | — | Edge flashes in the danger tone, old value kept | — | Field edge in the danger tone until fixed |
| **Playing** | Values written by automation move live | The time well counts | Play button lit | — |

Feedback timing: hover changes are immediate, with no fades. Detent lights and invalid-entry flashes last a fraction of a second. Tooltips appear after a short delay, and only with the hint bar hidden. Submenus open after a short delay.

## MusicCraft today: gaps

Ordered by visual impact.

1. **Driven controls show only the status dot.** Knobs and faders driven by an automation clip or controller get the dot and a hint suffix (`automation::mark`, `widgets::status_dot`). Still missing: the automation-tone arc and fader reading line while playing, the controller range band, and the hollow ring for hardware links (no hardware links exist yet). Files: `crates/musiccraft-editors/src/widgets.rs`, `crates/musiccraft-ui/src/automation.rs`.
2. **The control menu lacks recorded moves, song-start values, hardware learn and exact entry.** It now has open-automation, the "Control from" and "Value" headings, remove controller, copy, paste and reset (`automation::menu`, `control.set`, `control.reset`). Reset does not yet show the default in dim text, and the toolbar master knob and tempo well do not use this menu. Files: `crates/musiccraft-ui/src/automation.rs`, `crates/musiccraft-ui/src/toolbar.rs`.
3. **No type-in value box, horizontal slider or "Show values as" setting.** Values can only be dragged. File: `crates/musiccraft-editors/src/widgets.rs` (no `slider` or type-in widget).
4. **The tiny knob and the medium knob behave differently.** `widgets::knob` draws only the value arc (a knob at minimum, or a bipolar knob at centre, is a bare disc), does not lift on hover, and ignores the wheel, against `docs/spec/controls.md` ("Scroll over a knob or fader nudges it"). Its drag speeds also differ from `labeled_knob`'s: 160 pixels per full range (1000 with Shift) against 150 (600 with Shift), so the same gesture feels different on tiny and medium knobs. One shared drag rule is needed. File: `crates/musiccraft-editors/src/widgets.rs` (`knob`, `labeled_knob`).
5. **LCD wells are one string in one size**, with no per-group drag, no edge chevrons, no type-in and no right-click menu. The tempo well drags whole and fractional parts together. No compact number box exists; small integers use egui drag values or text. Files: `crates/musiccraft-ui/src/icons.rs` (`lcd`), `crates/musiccraft-ui/src/toolbar.rs` (tempo).
6. **Bare lights and "ON" tiles remain.** Option checkboxes are now LED switches (`widgets::led_switch`, a round light with its label to the right; clicking either toggles). Knob-row lights with a caption under them, and the mixer's "ON" tile, are still `widgets::dot` and text tiles. Menu check items stay egui checkboxes. Files: `crates/musiccraft-editors/src/widgets.rs` (`dot`), `crates/musiccraft-ui/src/mixer.rs`.
7. **Menus and drop-downs are egui defaults.** No heading rows, no right-click toggling of check items, no draggable items. Drop-downs use egui `ComboBox` with its own triangle. Shortcut text already shows in the menu bar (`toolbar.rs`). Files: `crates/musiccraft-ui/src/toolbar.rs`, `ComboBox` and `menu_button` calls across `crates/musiccraft-ui/src`.
8. **Value text is shared only for levels and pans.** `musiccraft_editors::format` gives dB levels with a true minus and "25% left" pans in the mixer, channel rack and toolbar; frequency, time, pitch and generic percent still use per-widget `format!` calls.
9. **Scroll bars have no chevrons, zoom grips or vertical zoom box** (main-window section, gap list). Views use egui `ScrollArea`; zoom is by keys and wheel only. Files: `crates/musiccraft-ui/src/playlist.rs`, `piano_roll.rs`.
10. **Preset arrows do nothing on right-click.** They are now chevron tiles (`widgets::chevron_button`), but right-click on them does not open the preset list. Files: `crates/musiccraft-editors/src/instrument.rs`, `effect.rs`, `antiserum.rs`.
11. **Colour choice exists only in the playlist track header menu**, as a plain swatch grid with no recent row, random or sequence colouring, and no live preview. Channels, patterns and inserts have no colour popover even where commands exist (`pattern.set_color`). File: `crates/musiccraft-ui/src/playlist.rs`.
12. **No default detent, keyboard focus, pointer hiding or Alt-click reset** on knobs and faders (all proposals above). File: `crates/musiccraft-editors/src/widgets.rs`.
13. **Knob track tones.** Knob value text now uses `theme::LCD_TEXT`. The knob track uses `PANEL` in `labeled_knob` and is absent in `knob`. Files: `crates/musiccraft-editors/src/widgets.rs`, `crates/musiccraft-editors/src/theme.rs`.
14. **Group boxes have no fold chevron or bypass light.** `instrument::section` already draws the darker rounded box with its title inside, as S asks. Files: `crates/musiccraft-editors/src/instrument.rs`, `effect.rs`.
15. **No rename in place.** Names change through dialogs only. File: `crates/musiccraft-ui/src/dialogs.rs`.
16. **File dialogs are the system's own**, so there is no favourite-folders control of ours; nothing is planned here (dialogs section).

## Sources

**Manual pages compared** (base `https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/`), for which controls exist, their order, grouping, proportions and behaviour only. All wording here is our own; no colours were sampled and nothing was measured or traced.

- `basics_interface.htm`:
  - "Hint Panel": hints for whatever is under the pointer; the optional larger floating hint panel.
  - "Knobs & sliders": vertical drag; the right-click type-in and the larger "Set" control (our fine slider inside the type-in box); values in percent, 0–1 or the control's own units, chosen when typing (our unit switch and "Show values as"); fine adjustment with a modifier or both mouse buttons (we use Shift, and leave out the two-button drag); the pause at the default with a modifier to skip it (our detent); reset by Alt-click or middle-click; linking from the right-click menu.
  - "Number Displays": vertical drag, step zones at the top and bottom edges with a changed pointer, press-and-hold then type.
  - "Window menus": a menu button at each window's top-left.
  - "Menu items": dragging a saved result out of a menu; right-clicking check items to try them without closing; the option to turn off drag animations.
  - "Horizontal Scroll-Zoom Bars & Vertical Zoom", with its screenshot `basics_interface_scrollbars.png`: right-drag two-way scrolling, middle-drag panning, the vertical zoom box, half-thickness scroll bars.
  - "Presets": previous and next arrows at the top right of instrument and effect windows; right-click for the preset list.
  - "Wave displays": dragging a displayed sample onto a sampler channel or another plugin.
  - "Color Selector": looked at, and deliberately not reproduced; we keep the palette popover.
  - "Favorite folders": looked at; not applicable, since we use system file dialogs.
- `basics_controllers.htm`, "Controller Hardware" and "Knob and Slider Controllers": linking a control to hardware from its right-click menu.
- `automation_internal.htm`, "Linking a software interface control to an internal controller" and "Last tweaked control": that controls can be driven by internal controllers and links. The manual gives no visual mark for this.
- `chansettings_main.htm`, "Main Channel Settings" (with its screenshot): the on light with its caption under it, in line with the knob captions.
- Screenshots looked at for layout, order and proportion only: `basics_interface_knobnslider.png`, `basics_interface_lcd.png`, `basics_interface_lcd_red.png`, `basics_interface_menu.png`, `basics_interface_right_click_menus.png`, `basics_interface_dragmenus.png`, `basics_interface_presets.png`, `basics_interface_scrollbars.png`, `basics_interface_color.png`, `basics_knobnslider_values_contextaware.png`, `panel_transport_tempo.png`, `panel_Extended_Hintbar.png`, `panel_vol.png`, `toolbar_multilink.png`, `automation_linkgeneric.png`, `chansettings_sampler.png`, `chansettings_main.png`.

**Repository files:** `docs/spec/controls.md`, `docs/spec/controllers.md`, `docs/spec/window-layout.md`, `docs/spec/keymap.md` and `crates/musiccraft-ui/keymap/default.keymap` (key clashes), `crates/musiccraft-editors/src/widgets.rs`, `crates/musiccraft-editors/src/hint.rs`, `crates/musiccraft-editors/src/theme.rs`, `crates/musiccraft-editors/src/instrument.rs` (`section`, preset row), `crates/musiccraft-ui/src/icons.rs` (`button`, `toggle`, `lcd`, `segmented`, `scope`, `load_bar`), `crates/musiccraft-ui/src/automation.rs`, `crates/musiccraft-ui/src/toolbar.rs`, `crates/musiccraft-ui/src/mixer.rs`, `crates/musiccraft-ui/src/channel_rack.rs`, `crates/musiccraft-ui/src/playlist.rs`, `docs/screenshots/synth-and-eq.png`, `docs/screenshots/playlist-mixer.png`. Other guide sections referred to: main-window (window header, scroll bars, zoom glyphs, drag ghost), toolbar (time and tempo displays, button tones, `LCD_TEXT`, `SEPARATOR`, floating hint glyph), plugin-windows (group boxes, on light, preset chevrons), mixer (fader, strips), channel rack (step buttons), piano roll (`SELECTION_FILL`).

**Own design:**

- Every drawing: knob well, track, arc, pointer and default tick; the fader, slider, LED, field and menu looks; every glyph; every colour role and the proposed tokens `KNOB_TRACK`, `FIELD`, `MENU_BG` and `MENU_HEADING`.
- The knob size classes and their names; the two LED label variants as a general rule.
- The status dot, ring and range band for automated, linked and controller-driven controls, and the automation-tone arc during playback.
- Per-group drag on LCD wells, the clamp and invalid-entry flashes.
- The type-in box layout, its fine slider, the unit tiles and the "Show values as" setting name.
- The control menu's grouping, headings and wording.
- Keyboard focus on knobs and its key set; menu letter-jump.
- The colour popover (palette grid, recent row, random, sequence colouring).
- Rename in place, the group box fold chevron and bypass light, the value formats table, double-click to fit on the scroll thumb.
- The modifier choice: Shift for fine (as in `controls.md`) and Ctrl to skip the detent, which differs from the manual's modifiers.
