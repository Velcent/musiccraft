# Style guide: Mixer

Part of the [interface style guide](README.md). Read it with the shared rules in [design language](design-language.md), [controls](controls.md) and the window header in [main window](main-window.md).

The mixer is where every sound in the project ends up. A row of tall, narrow strips (the master and the numbered inserts) sits side by side like a hardware desk, and a fixed-width panel beside them holds the selected track's effect slots, its EQ and its input and output. Aim for a dense, dark wall of identical, repeated strips. The meters and lit switches are the brightest things on it, the strips' own controls are small and quiet, and routing is drawn right on the strips, so the user reads the mix at a glance without opening anything. Selecting a strip and then clicking things on other strips (route arrows, send knobs) is the core gesture. It should feel immediate, with no dialogs.

Layout, order and proportions below are the target. Behaviour that differs from our specs (a removed or replaced control, a different mouse button, a new gesture or shortcut) is marked **Proposed**: each one needs `docs/spec/mixer.md` (and `docs/spec/keymap.md` for keys) updated first, and until then today's behaviour stays the default.

## Placement and proportions

**Kind of window.** The mixer is an editor window floating in the workspace, like the channel rack and piano roll. It has no OS title bar. Its first row is the compact header defined in the main-window section and in `docs/spec/window-layout.md`. F9, the toolbar's mixer toggle and the View menu show and hide it (toggle rules in the main-window section).

**Default slot.** In the default arrangement the mixer sits at the top of the workspace, to the right of the channel rack, and reaches the right edge. It takes about two thirds of the workspace width and a little under half its height (see the main-window section's default-layout table). The first time it opens it takes that slot. After that it reopens where the user left it.

**Inside the window, left to right:**

| Region | Relative size | Notes |
| --- | --- | --- |
| Strip area | Everything except the slot panel, about three quarters of the default window width | Holds the three docks (left, middle, right). Only the middle dock scrolls. |
| Separator | A hairline | Not draggable. The slot panel keeps its own width. |
| Slot panel | A fixed width of about ten Slim strips. Roughly a quarter of the default window, and nearer half in a narrow window. | It can move to the left side of the strip area (options menu) or be hidden (header toggle; Proposed: Ctrl+Enter). Hiding it gives its width to the strips. |

**Inside the window, top to bottom:** the one-row header, then the strip area and slot panel side by side, filling the rest of the height. There is no footer. The strip area's scroll bar is its own bottom row (see Anatomy).

**Strip proportions.** A strip is very tall and narrow. At the default height it is roughly twenty times taller than wide, and even a short mixer keeps it more than ten times taller than wide. The width comes from the strip layout (see Anatomy, "Strip layouts"). Slim is our existing strip width (see `window-layout.md`, "Mixer"). The others are multiples of it. Strips always fill the full height of the strip area. Growing the window vertically lengthens the faders and reveals sections that were hidden for lack of room. Growing it horizontally shows more strips.

**Vertical budget of a Slim strip at the default height** (own design, compared by eye with the manual's layout, not measured):

| Section | Share of the strip's height |
| --- | --- |
| Number cell (and icon cell when shown) | One small text row each |
| Upper block: name and meter | About a quarter |
| Enable light and pan knob | About a tenth |
| Fader | About a third |
| Track switches, stacked (phase, swap, separation, effects, arm) | About a sixth |
| Route row | About a fifteenth |
| Cable lane | About a fifteenth |
| Scroll bar (middle dock only) | A thin rail |

**Resizing.** The window resizes from all edges and corners (main-window rules). When the mixer gets shorter, the fader gives up travel first, down to a minimum of about three cap heights. After that, sections hide in this order: the icon cell, the phase and swap rows with the separation knob, the arm switch, then the effects switch. The number cell, name, meter, enable light, pan, fader and route row never hide. The slot panel shrinks in its own order: first the EQ knob rows, then the EQ graph, then the slot list scrolls inside the panel. The input and output rows always stay visible.

**Docking and detaching.** The window collapses to its header, maximizes and detaches like every editor window (main-window section). The options menu has "Detach from main window" (shared item). Inside the window, **docks** split the strip area into three parts (Proposed; see Anatomy, B). The left and right docks hold still, and the middle dock scrolls between them.

## Anatomy

### A. Header row, left to right

The same slot order as every editor window (main-window section, "The header row, left to right"). Only the mixer's own tools, its title and its right-hand placement are listed here. The window menu button (our glyph: three short stacked bars) and the collapse, maximize/restore and close buttons at the far right are the shared ones.

The header reads as two parts that line up with the body below: over the strip area, the menu button, the tools and the layout selector; over the slot panel, the title, then the window buttons.

| # | Element | What it is | Click | Right-click | Other input | States |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Window menu button** | The shared menu button at the top-left corner | Opens the mixer options menu (A1 below), followed by the shared window items | Same | Hover: hint | Shared header button states |
| 2 | **Strip details toggle** | Small icon button | Shows or hides the track switches block on every strip (phase, swap, separation, effects, arm) | — | Hover: hint | On: glyph in the accent, with an underline (`icons::toggle` style) |
| 3 | **Slot panel toggle** | Small icon button | Shows or hides the slot panel (Proposed: also Ctrl+Enter while the pointer is over the mixer) | — | Hover: hint | On, as above. On by default. |
| 4 | **Add insert** | Small plus button (replaces today's "+ Insert" text button; Proposed label change) | Adds one insert directly after the selected track, then selects the new track | Proposed: a small menu with "Add one", "Add several…" (asks for a count), "Add from saved track…" (later; see gaps) | Hover: hint | — |
| 5 | **Layout selector** | Small text showing the current strip layout's name, followed by a small right chevron. No box around it, just text. | Opens the list of eight layouts (see "Strip layouts") | — | Proposed: mouse wheel steps through the layouts | Hover: brighter text |
| 6 | **Drag area** | The empty stretch between the layout selector and the title | Raises and focuses the window. Drag moves it. | — | Double-click: maximize (shared proposal) | — |
| 7 | **Title** (right-hand window item, shared slot 6) | `Mixer — <track name>` in small text. It starts at the slot panel's left edge, so it sits over the panel, and it runs up to the window buttons (cut with an ellipsis if needed). When the panel is hidden or moved to the left, the title stays at the right end, right-aligned against the window buttons (own design, keeping the shared order). It names the selected track (the one whose slots the panel shows). | — (not a selector) | — | — | Focused window: normal text. Unfocused: dim. |
| 8–10 | **Collapse, maximize/restore, close** | The shared window buttons | Shared | — | — | Shared |

The header carries no instruction sentence. Help text goes to the hint bar.

**A1. Options menu** (our wording, grouped by separators):

1. **View**
   - "Draw routes" (tick, on by default; the cables of D)
   - "Put names under the meters" (tick; switches every layout to its names-below variant, the same as picking that variant in the selector)
   - "Only loaded slots" (tick; see E2)
   - "Effects panel on the left" (tick)
   - "Hairlines between strips" (tick)
   - "Strip colour" ▸ Off / Faint / Medium / Strong (how strongly each track's colour tints its whole strip; default Faint)
2. **Tracks**
   - "Rename…" (existing label; Proposed key F2)
   - "Colour…"
   - "Icon…"
   - "Reset selected tracks" (removes their effects, sends and EQ, and restores the default name and colour, as one undo step)
   - "Delete selected tracks"
   - "Clear out idle inserts" (deletes inserts that nothing plays through: no channel, no incoming send, no effects, default settings)
3. **Window** (the shared items from the main-window section, after a separator): "Detach from main window", "Maximize" or "Restore", "Collapse to header", and the rest of that shared list.

Right-clicking a tick item toggles it without closing the menu (shared rule).

### B. Strip area

From left to right: the **left dock**, a **dock divider**, the **middle dock**, a **dock divider**, the **right dock**. Strips sit edge to edge, separated by a narrow gap of the well colour (a hairline when "Hairlines between strips" is on). Docks, the dock dividers and the height handle are all Proposed (today everything scrolls together; see gaps).

1. **Left dock** (fixed, never scrolls)
   1. **Level strip** (optional; own design). A narrow column showing the selected track's stereo level next to a dB scale ladder (tick marks with small numbers from the top of the meter's range down to its floor). Its number cell reads "dB". It has no controls. Hovering over it shows the selected track's peak level in the hint bar. It is hidden in the Slim layout and shown in wider ones.
   2. **Master strip**. Same anatomy as an insert (C below) with these differences:
      - the number cell reads "M";
      - no solo (solo never silences the master);
      - no arm switch;
      - its route row shows only the "feeds master" switch for the selected insert (see C12).
   3. Any insert the user docked left, in track-number order.
2. **Dock divider**. A gap about a quarter of a strip wide, darker than the strips, running the full height. A **height handle** sits on it, level with the pan knobs, just above the top of the faders: three small dots stacked vertically. Dragging the handle up or down moves the meter/fader split on every strip at once (the upper block, enable light and pan row move together), trading meter height against fader travel. Double-click restores the default. The pointer becomes a vertical resize arrow over the handle. The trade is saved per user.
3. **Middle dock** (scrolls horizontally): every insert that is not docked left or right, in number order.
   - A new project shows the master plus sixteen inserts, so at the default size the middle dock overflows. That is expected.
   - **Scroll bar**: a thin rail across the bottom of the middle dock only, under the cable lane. The thumb shows the visible share of strips. Chevrons at both ends step one strip at a time.
   - Proposed: the mouse wheel over any strip's number, name or meter scrolls the middle dock by whole strips. Over a knob or fader, the wheel adjusts that control instead.
4. **Dock divider** (as 2).
5. **Right dock** (fixed): inserts the user docked right (typically buses and effect returns), in number order. Empty and zero-width until something is docked there. The divider then appears.

Docking is per track: right-click › "Pin" › "Left side" / "Scrolling area" / "Right side". A track behaves the same in every dock, and only its position changes. Track numbers do not change when tracks are docked.

### C. A strip, top to bottom (Slim layout, names above meters)

| # | Element | What it shows | Click | Right-click | Other input | States |
| --- | --- | --- | --- | --- | --- | --- |
| 1 | **Number cell** | The track number ("M" for the master), centred in small bold text on a cell tinted with the track's colour. The full strip width, one text row tall. | Selects this track only (its slots appear in the panel and the title follows) | Track menu (C-menu below) | Proposed: Ctrl+Shift+click adds or removes the track from the selection; Ctrl+Shift+drag across numbers selects a range; Shift+wheel moves the selected tracks left or right; double-click renames; a browser item can be dropped on it (see F). | Selected / menu target: see States |
| 2 | **Icon cell** (Normal and wider) | The track's optional icon, a small glyph from our icon set, centred. Empty by default. | As 1 | As 1 | — | Hidden in Slim and when the mixer is short |
| 3 | **Upper block: name and meter**, side by side | **Name** at the left (about half the strip width), written bottom to top in small text, anchored at the bottom of the block, and cut with an ellipsis when too long. **Meter** at the right: two thin bars (left and right) in a recessed well, rising from the bottom of the block, with a 0 dB tick and a short peak-hold line above each bar that falls back slowly. | Name: as 1. Meter: as 1 | Track menu (today the name's right-click menu) | Proposed: middle-click or double-click on the name renames; the wheel scrolls the middle dock; Shift+wheel moves the track; Ctrl+Shift+click or drag multi-selects (as 1). Hover: the hint shows the name, number, current peak and routing summary. | Name text dim, bright when selected. The meter well is outlined in the accent when the track is selected. A clip mark (a small bar at the very top) lights when a peak passed 0 dB and stays lit until the meter is clicked (master and device outputs only; see States). |
| 4 | **Enable light** | A small round light, centred, in place of today's "ON" text button. Lit means the track is heard. | Toggles mute (as today's "ON") | Proposed: toggles solo, and today's separate "S" solo button goes. Until `mixer.md` changes, the "S" button stays as a tiny round switch directly under the light. | Proposed: Ctrl+click solos. Alt+click solos this track together with every track it feeds and every track feeding it (the same set our solo rule already keeps audible). Shift+click locks the light (a padlock replaces it, and solo then leaves it alone). Ctrl+right-click un-mutes every track. | On, off, soloed, silenced by solo, locked (see States). Proposed: with several tracks selected, a click changes all of them. |
| 5 | **Pan knob** | A small bipolar knob (our existing knob drawing), centred | Selects the track | Control menu (automation, controller, reset; `controls.md`) | Drag vertically to change (Shift for fine). Wheel nudges. Double-click or middle-click centres it. Hover: the hint shows the value. | Proposed: ganged across the selection (see "Multi-selection") |
| 6 | **Fader** | A long vertical groove with a flat rectangular cap as wide as the strip allows, and a unity tick at three quarters of the travel (existing fader curve) | Click on the cap selects the track. Proposed: a click above or below the cap glides it toward the pointer instead of jumping. | Control menu | Drag the cap (Shift for fine). Wheel nudges. Double-click or middle-click returns to unity. Hover and drag: the hint shows the level in dB. | Cap: normal, hover, dragging, selected, armed (see States). Proposed: ganged across the selection. |
| 7 | **Invert phase switch** | A tiny square switch on its own row, centred, showing a glyph | Toggles the option for this track (Proposed: for all selected tracks). Today the option lives only in the name's right-click menu, which keeps it. | — | Hover: hint with the current state | Off: dim glyph. On: glyph in the highlight tone on a raised square. |
| 8 | **Swap left and right switch** | The same tiny switch on the next row down, centred | As 7 | — | As 7 | As 7 |
| 9 | **Stereo separation knob** | A small bipolar knob, centred, on the row under the swap switch. Turning it left narrows the image to mono, centre leaves it as is, turning it right widens it. The direction follows our existing `mixer.set_separation` (−1 mono … +1 wider). | Selects | Control menu | Drag, wheel, double-click or middle-click resets to centre | Arc hidden at centre, so an untouched knob looks quiet |
| 10 | **Effects switch** | Small text "fx" with the count of active effects in front ("2 fx", today's "N fx" label). Centred, under the separation knob. | Proposed: bypasses or restores all of the track's effects at once (all selected tracks when several are selected). Today the count is a label only. | Proposed: opens or closes the windows of all its effects | Hover: hint lists the effects | Lit (effects active), bypassed (dim and struck through), none (disabled, unresponsive) |
| 11 | **Arm switch** | A small round dot, under the effects switch | Proposed: arms or disarms the track for audio recording (needs a decision; see gaps) | Proposed: menu with "Arm", "Arm and name the take…" | Hover: hint | Off: hollow. Armed: filled in the danger tone. While recording: it pulses. Absent on the master. |
| 12 | **Route row** | The routing control for the *selected* track's output, drawn on every strip, with clearly more room above it than between the switches (see C12) | See C12 | See C12 | See C12 | See C12 |
| 13 | **Cable lane** | A shared band across each dock, under the route rows, where routing cables are drawn (see D) | Click on empty lane: nothing | — | — | — |

Between rows 5 and 6 there is no visible break. The fader starts directly under the pan knob. The track switches (7–11) sit in one tight block with no labels, stacked one per row in this order: invert phase, swap, separation knob, effects, arm. Then comes a wider gap, the route row (12) and the cable lane (13).

**C12. Route row, every case.** One control per strip, centred, about as tall as a small knob. What it shows depends on the selected track (the "source"):

| This strip is… | It shows | Click | Right-click | Other |
| --- | --- | --- | --- | --- |
| The source itself | A short **down arrow**, lit in the accent: "routing from here" | — | — | Hover: hint with the routing summary ("Sends to: Master, 5 Reverb") |
| A possible destination not yet fed (the master, or any higher-numbered insert, under our "sends go up" rule) | An **up arrow**, dim | Creates the route: for the master `mixer.route_to_master <src> on`, for an insert `mixer.set_send <src> <this> 1` | Menu: "Send here", "Send here only (stop feeding the master)" | Hover: arrow brightens, and the hint says "Send Insert 3 here" |
| A destination already fed by the source (insert) | A **send knob** (small knob, arc in the accent) with a tiny **remove chevron** just above it | Chevron: removes the send (`mixer.set_send … 0`) | Knob: control menu. Chevron: menu with "Send here only" and "Remove send". | Drag the knob: send level. Double-click or middle-click: unity. Wheel nudges. Hover: hint with the level in dB. |
| The master, when the source feeds it | The **up arrow lit** in the accent (the master route is on/off, so there is no knob) | Turns the master route off | Menu as above | — |
| A strip that cannot receive from the source (an insert numbered lower than the source) | An **outlined, inert up arrow** | Nothing. The hint explains why: "Sends only go to higher-numbered inserts". | — | — |
| Any strip that receives from tracks other than the source | **Small stacked chevrons** above whatever it shows | — | — | Hover: hint lists who feeds it |
| The master selected as source | Every strip shows nothing in its route row. The master's own row shows the down arrow. | — | — | — |

Today the dim up arrow already toggles a route, so that click is not a change. Proposed: the lit source arrow (replacing today's text summary), the send knobs and remove chevrons on strips (replacing the panel's send sliders), the right-click menus, the inert outlined arrows and the incoming chevrons.

Proposed: with several tracks selected, the last one clicked is the source for the route row. Ctrl+Shift+right-click on a target strip offers routing for all selected tracks at once (C-menu, Routing).

**C-menu. Track menu** (right-click on number, icon, name or meter; today the name's right-click menu, which holds "Rename…", the stereo options and "Delete"; the rest is Proposed). It applies to every selected track, and the right-clicked strip is "this track". Its first line is a disabled heading: "Insert 3 · Bass (2 selected)".

1. "Rename…" (Proposed key F2), "Colour…", "Icon…", "Random colour"
2. **Routing**
   - "Play selected channels through this track" (Proposed key Ctrl+L, mixer-local), "Spread selected channels across tracks from here" (Proposed key Ctrl+Shift+L, mixer-local: the first selected channel goes to this track, the next to the following track, and so on)
   - **Feed** ▸
     - "From the selection into this track"
     - "From the selection into this track (and off the master)"
     - "From this track into the selection"
     - "From this track into the selection (and off the master)"
   - "Make a bus from selected…" (adds a new insert after the highest selected track, sends the selection into it, turns off their master routes)
   - **Highlight** ▸ "Its channels" (Proposed key Alt+L, mixer-local; selects in the channel rack the channels that play through this track), "Its destinations", "Its sources" (select those tracks in the mixer)
   - "Reset routing" (back to master only)
3. "Solo" (Proposed key S, mixer-local), "Solo with connected tracks" (Proposed key Alt+S, mixer-local)
4. "Invert phase", "Swap left and right" (ticks, mirroring C7 and C8), "Stereo separation" with an inline number field (as today)
5. **Layout**: "Gap before this track" (tick; a visual separator), "Group selected" (gaps on both sides of the selection), "Pin" ▸ "Left side" / "Scrolling area" / "Right side", "Shift left" (Proposed key Alt+Left), "Shift right" (Proposed key Alt+Right)
6. "Insert track before", "Delete" (danger text; not on the master)

**Keys.** All mixer keys above are Proposed (needs `keymap.md` updated first). They are keymap entries, not code, and they act only while the pointer is over the mixer and no text field has focus. Two clash with existing bindings in `default.keymap` / `keymap.md` and are therefore mixer-local only: S (globally `ui.toggle_slide`) and Ctrl+L (`channel.route_free` over the channel rack). If a window-local override of a global key is not wanted, S stays unbound in the mixer and solo is reached by the menu and the enable light. Ctrl+A (select all tracks), F2, Ctrl+Enter, Alt+S, Alt+L and Alt+Left/Right are not bound today.

**Multi-selection** (Proposed; today one track is selected at a time).

- A plain click on a number, name or meter selects only that track. Ctrl+Shift+click toggles a track in or out of the selection. Ctrl+Shift+drag across the number, name and meter area selects a range. Ctrl+A selects all.
- A click on a fader or knob of an unselected track selects that track alone. A click on one that is already selected keeps the selection.
- Faders, pan and separation knobs, and the enable, phase, swap, effects and arm switches are **ganged** across the selection. Dragging one selected fader moves every selected fader by the same amount of travel. Each move is clamped to its own range, and the whole gesture is one undo step.
- The last track clicked is the **primary** selection: its slots show in the panel, its name is in the title, and it is the routing source.

### D. Routing cables

- Drawn only while "Draw routes" is on (default on), and only for the routes of the selected tracks (all of them when several are selected).
- Each route is a smooth curve in the cable lane. It leaves the bottom of the source's down arrow, sags into the lane, and rises into the bottom of the destination's send knob or lit master arrow. Several cables from one source fan out from the same point.
- When the destination is in another dock (most often the master in the left dock), the cable does not cross the dock divider. It drops straight out of the bottom of the lane at the source end, and a short stub rises into the destination from the bottom edge. The two ends read as connected "below" the mixer.
- A send at very low level and a **sidechain link** (a compressor or vocoder on the destination keyed by the source) are drawn faint and dashed (own extension: our sidechains live in effect parameters).
- Cables are not interactive. They are a picture of the routing. Hovering over the lane does nothing.

### E. Slot panel, top to bottom

1. **Input row** (one row, the full panel width):
   - an **input glyph** at the left, in place of today's "In" label (Proposed label change). It is purely decorative: not a button, hover hint "Audio input". It occupies the spot where a recording pickup-point switch could go later (left out; see gaps);
   - a **device dropdown** filling the rest: the input device recorded from ("System default" first, then the devices from Settings › Audio). Click: the list. Wheel: step through devices. Right-click: "Reset to default".
   - The row is shown for every track. It reflects the single global input until per-track inputs exist (see gaps).
2. **Effect slots**: ten rows of equal height, numbered 1 to 10 top to bottom. Together they take about half the panel's height at the default size. Opening a window by clicking the name and the mix knob work today. Proposed: the slot chevron and its menu (replacing the inline move and remove buttons), the wheel and drag reordering, right-click rename, Ctrl+click solo and drops. Each row, left to right:
   1. **Slot chevron**: a small right-pointing chevron. Click: the slot menu (E2a). Right-click: the same menu opened on the plugin effects list.
   2. **Name**: the effect's label in small text, filling the row.
      - Empty slot: a placeholder "Slot 3" in dim italic.
      - Click on a loaded name opens or closes the effect's window. Click on an empty name opens the add list.
      - Right-click on the name: "Rename…" (slot label).
      - Wheel over a loaded slot moves the effect up or down one slot (`mixer.move_effect`).
      - Drag a loaded slot onto another row to move it (an accent insertion line shows where). Ctrl+drag copies it.
      - Drop a browser effect onto a row to load it there.
   3. **Mix knob**: a small knob at the right end of the name, 0–100 % wet. Double-click or middle-click: 100 %. Hidden on empty slots, and shown dim and inert on send and band-split slots (they ignore it).
   4. **Slot light**: a small round light at the far right edge. Click: bypass or restore this effect. Ctrl+click: solo it (bypass the others on this track until clicked again). Hidden on empty slots.

   **E2a. Slot menu** (our wording):
   - Empty slot: the built-in effects by category, then "Plugin effects" ▸.
   - Loaded slot: "Replace" ▸ (the same lists), "Remove", "Move up", "Move down", "Open window", "Bypass", "Rename…", "Presets" ▸ (previous, next, the list, "Save preset…"), "Move into an effect rack" (wraps the effect in our `rack` effect).

   **Only loaded slots** (options menu): only loaded slots show, packed together, followed by a single empty row that adds after the last effect. The row height stays the same and the freed space goes to the EQ.
3. **EQ section** (the track's built-in three-band EQ). A small dim label "EQ" sits at its lower-left corner. The graph, band faders and knob rows replace today's three gain knobs; their gestures are Proposed.
   1. **Graph** (about two thirds of the panel width, as tall as about four slot rows): the response curve over a log frequency axis with faint grid lines. It has three round band handles (low shelf, peak, high shelf).
      - Drag a handle sideways for frequency and up or down for gain.
      - Right-drag sideways for bandwidth.
      - Double-click a handle to reset that band.
      - Hover: a small readout near the handle ("1.2 kHz −3.5 dB").
      - A flat EQ draws a straight line.
   2. **Band faders**: three short vertical gain faders side by side, filling the rest of the width to the right of the graph, in band order. Their drawing and gestures match the strip fader, with 0 dB at the centre of travel. A tiny chevron at the top of the fader group hides them ("Hide band faders") so the graph takes the full width.
   3. **Knob rows**, under the graph: a row of three frequency knobs, with a small dim "Hz" label at the left, and a row of three bandwidth knobs with a dim "Q" label. They are in the same order as the handles. They hide first when the mixer is short.
4. **Output row** (one row, at the very bottom):
   - an **output glyph** at the left, in place of today's "Out" label (Proposed label change);
   - a **dropdown** (for inserts, Proposed: it replaces today's "To master" and "+ Send" routing editor at the bottom of the panel):
     - Master: the audio output device ("System default" first).
     - Insert: where it goes. The text is the routing summary ("Master, 5 Reverb", or "Nowhere"). The list shows tick items: "Master", then every higher-numbered insert as "5 Reverb". Ticking or unticking one creates or removes that route, the same as the route row.

The panel has no heading of its own. The header title names the track.

### F. Drag and drop onto the mixer

All Proposed (nothing can be dropped on the mixer today).

| Dragged | Dropped on | Result |
| --- | --- | --- |
| A built-in or plugin effect from the browser | A strip's number, name or meter | Loads it into the track's first free slot. The strip is outlined in the accent while hovered. |
| The same | A slot row | Loads it into that slot (replacing a loaded effect asks first). An accent outline marks the row. |
| An instrument from the browser | A strip | Adds a channel routed to that track. Names the track after the instrument if it still has its default name. |
| A loaded slot | Another slot row (same track) | Moves it. Ctrl copies it. |
| A strip (drag by its number cell) | Between two strips | Moves the track (same as "Shift left" and "Shift right"). An accent insertion line shows the target gap. |

### G. Strip layouts

The layout selector offers eight layouts: four widths, each with names above or below the meters. Width multiples are relative to Slim. Exact sizes are our own and go into `window-layout.md`.

| Layout | Width | Differences from the Slim anatomy above |
| --- | --- | --- |
| **Slim** (default) | 1× (our current strip) | As described in C. No icon cell, no level strip. |
| **Normal** | About 1.25× | Icon cell shown. The meter bars are a little wider. Knobs a step larger. The level strip is shown in the left dock. |
| **Wide** | About 1.6× | The upper block keeps its split. The phase and swap switches and the separation knob move into a narrow column **beside the fader** (to its left), which saves a row. A small level readout (dB) appears under the fader. |
| **Full** | About 3.5× | As Wide, plus a short **effect list** between the upper block and the enable light: one line per loaded effect, in slot order, in tiny text. Click an entry to open its window. Right-click an empty line to add an effect. The track name can be written horizontally above the meter here. |
| **… names below** (one variant of each) | Same | In the upper block, the meter sits on top at full strip width and the vertical name sits under it. |

Changing the layout never changes the project. It is a per-user view setting, like the panel toggles.

## Icons

All glyphs are simple vector drawings in the style of `crates/musiccraft-ui/src/icons.rs`: one stroke weight, drawn inside an inset box, coloured by state. Shared header glyphs (menu, collapse, maximize, close) are defined in the main-window section and are not repeated here.

| Where | What it does | Our glyph |
| --- | --- | --- |
| Header, strip details toggle | Shows or hides the track switches block on strips | A tall narrow rectangle outline (a strip) with its lower third filled |
| Header, slot panel toggle | Shows or hides the slot panel | A wide rectangle outline with a vertical line a third of the way in from the right and that right third filled |
| Header, add insert | Adds an insert after the selected track | A plus: two crossing strokes of equal length |
| Header, layout selector | Opens the layout list | Text, then a small right chevron (two short strokes meeting at a point) |
| Dock divider handle | Trades meter height against fader travel | Three small filled dots stacked vertically |
| Number cell, level strip | Labels the level strip | Text "dB", no glyph |
| Icon cell | User's choice per track | One of our existing glyphs (`Icon::*`) or a small set of new plain ones (drum, wave, microphone outline, note): drawn small and dim |
| Enable light | Mute / solo | The existing `widgets::dot`: a filled circle with a thin ring |
| Solo switch (only until the enable-light proposal is accepted) | Solo | Today's "S" text, kept, on a tiny round switch directly under the enable light |
| Enable light, locked | Locked mute | A padlock: a small rounded rectangle under an arch (the same as the channel rack's locked light) |
| Pan, separation, send, mix, EQ knobs | Values | Our existing knob drawing (`widgets::knob`): body, arc, pointer line |
| Fader | Level | Our existing fader drawing (`widgets::fader`): groove, unity tick, flat cap with a centre line |
| Phase switch | Invert polarity | A small circle with a diagonal stroke through it, extending just beyond the circle at both ends |
| Swap switch | Swap left and right | Two short horizontal arrows one above the other, the top one pointing right and the bottom one pointing left |
| Effects switch | Bypass all effects | Text "fx" with the count. When bypassed, a single horizontal stroke through the text. |
| Arm switch | Arm for recording | A small filled circle (the record symbol, `Icon::Record` at tiny size). A hollow ring when off. |
| Route row, source | Marks the routing source | A short vertical stroke ending in a filled downward triangle |
| Route row, available | Create a route here | A small filled upward triangle |
| Route row, unavailable | No route possible | The same upward triangle as an outline only |
| Route row, remove chevron | Remove this send | A tiny upward chevron (two strokes), above the send knob |
| Route row, fed from elsewhere | Shows incoming routes | Two tiny upward chevrons stacked |
| Routing cable | Shows a route | A smooth curve of one stroke, ending on the destination control. Dashed when faint. |
| Scroll bar ends | Step one strip | Left and right chevrons, single strokes |
| Slot chevron | Opens the slot menu | A small right-pointing chevron |
| Slot light | Bypass this effect | `widgets::dot`, smaller |
| Input row glyph | Marks the input | A small circle (a socket) with a short arrow coming in from the left and touching it |
| Output row glyph | Marks the output | The same circle with a short arrow leaving it to the right |
| EQ band handles | Drag a band | Small filled circles, the same drawing as the handles in our EQ effect window |
| Band faders hide chevron | Hides the band faders | A tiny right chevron (hidden) or down chevron (shown) |
| Clip mark | Peak passed 0 dB | A short solid bar across the top of the meter well |

## Colour roles

Tokens are in `crates/musiccraft-editors/src/theme.rs`. Names marked "new" are proposals. Shared header tokens (`HEADER`, `HEADER_ACTIVE`, `WINDOW_EDGE`, `SCROLL_RAIL`, `SCROLL_THUMB`) are the ones proposed in the main-window section.

| Element | Token(s) | Relative tone and state changes |
| --- | --- | --- |
| Window body, slot panel background | `theme::PANEL` | Base tone |
| Strip area well (gaps between strips) | `theme::PANEL_DARK` | One step darker than the strips, so each strip reads as a separate slab |
| Strip fill | new `theme::STRIP`, the same as `PANEL` | With "Strip colour" on: new `theme::strip_tint(colour, strength)`, the track colour blended into `STRIP` at faint, medium or strong strength. Faint must stay quieter than the meters. |
| Strip fill, selected | new `theme::STRIP_SELECTED`, the same as `RAISED` | The primary selection also gets a thin `ACCENT` edge around the whole strip |
| Strip fill, menu target ("this track") | `theme::RAISED_HOVER` | Only while its menu is open |
| Hairlines between strips (option) | `theme::GRID_LINE` | — |
| Dock divider | `theme::BACKGROUND` | Darker than the well. Handle dots in `TEXT_DIM`, `TEXT` on hover, `ACCENT` while dragging. |
| Number cell | new `theme::number_cell(colour)`: the track colour darkened halfway toward `PANEL_DARK`. Number text `TEXT`. | Master default: `HIGHLIGHT` darkened the same way. Selected: unchanged (selection shows in the strip fill). |
| Track colour | Proposed stored per-track colour from the `theme::track_color` palette (default `theme::palette(n)` at creation) | Stays with the track when it moves |
| Name text | `theme::TEXT_DIM` | Selected: `theme::TEXT` |
| Meter well and bars | `theme::METER_BG`, edge `theme::LCD_EDGE`, bars `ACCENT` → `HIGHLIGHT` → `DANGER` by level (existing `widgets::meter`) | Selected: well outline in `ACCENT`. Peak-hold line: `TEXT`. 0 dB tick: `TEXT_DIM`. |
| Clip mark | `theme::DANGER` | Off: invisible |
| Level strip scale | Tick marks and numbers in `theme::TEXT_DIM` on `theme::PANEL_DARK` | — |
| Enable light | On: `theme::ACCENT`. Off: `theme::PANEL_DARK` with a `TEXT_DIM` ring (existing `dot`) | Soloed: lit, with an outer ring in `HIGHLIGHT`. Silenced by another track's solo: `ACCENT_DIM`. Locked: padlock in `TEXT_DIM`. |
| Knobs | Body `theme::RAISED`, arc `theme::ACCENT`, pointer `theme::TEXT` (existing) | Hover: body `RAISED_HOVER` |
| Fader groove | `theme::METER_BG` with `LCD_EDGE` edge (existing) | — |
| Fader cap | `theme::RAISED`, edge `TEXT_DIM`, centre line `ACCENT` (existing) | Hover and drag: `RAISED_HOVER`. Selected: new `theme::FADER_SELECTED`, `ACCENT_DIM` fill with an `ACCENT` edge. Armed: new `theme::FADER_ARMED`, `DANGER` blended halfway into `RAISED`. |
| Phase and swap switches | Off: glyph `TEXT_DIM`, no fill | On: glyph `HIGHLIGHT` on `RAISED`. Hover: fill `RAISED_HOVER`. |
| Effects switch | Active: text `ACCENT` | Bypassed: `TEXT_DIM` with strike. No effects: `GRID_BAR`. |
| Arm switch | Off: ring `TEXT_DIM` | Armed: fill `DANGER`. Recording: pulses between `DANGER` and its dimmed form. |
| Route arrows | Available: `TEXT_DIM` | Hover: `TEXT`. Lit (master fed, source arrow): `ACCENT`. Unavailable: `GRID_BAR` outline. Incoming chevrons: `TEXT_DIM`. |
| Send knob | Knob tokens, arc `ACCENT` | Remove chevron `TEXT_DIM`, `DANGER` on hover |
| Routing cables | new `theme::CABLE`, the same as `ACCENT` | Faint and sidechain: new `theme::CABLE_FAINT`, the same as `ACCENT_DIM`, dashed |
| Scroll bar | `SCROLL_RAIL`, `SCROLL_THUMB` (main-window proposals) | Hover `RAISED_HOVER`. Dragging `ACCENT_DIM`. |
| Slot row, loaded | new `theme::SLOT_LOADED`, the same as `RAISED`. Name `TEXT`. | Bypassed: name `TEXT_DIM`. Window open: a thin `ACCENT` bar at the row's left edge. Hover: `RAISED_HOVER`. |
| Slot row, empty | new `theme::SLOT_EMPTY`, the same as `PANEL_DARK`. Placeholder `TEXT_DIM` italic. | Hover: one step lighter (`RAISED`) |
| Slot light | On `ACCENT`. Off `PANEL_DARK` with `TEXT_DIM` ring. | Soloed slot: `HIGHLIGHT` ring |
| Slot chevron | `TEXT_DIM` | Hover `TEXT` |
| Input and output dropdowns | Fill `PANEL_DARK`, text `TEXT`, edge `LCD_EDGE` | "System default" and "Nowhere" in `TEXT_DIM`. Hover: edge `RAISED_HOVER`. |
| Input and output glyphs | `TEXT_DIM` | — |
| EQ graph | Well `theme::LCD_BG`, edge `LCD_EDGE`, grid `GRID_LINE`, curve `ACCENT`, area under the curve `ACCENT_DIM` at low strength | Handles: `HIGHLIGHT`. Dragged handle: `PLAYHEAD`. Readout: `TEXT`. |
| EQ labels ("EQ", "Hz", "Q") | `TEXT_DIM` | — |
| Drop target outline, insertion lines | `theme::ACCENT` | Only while dragging |
| "Delete" menu items | `theme::DANGER` text | — |

## Labels and wording

- **Window title:** `Mixer — <track name>`, and `Mixer — Master` for the master (existing format).
- **Default track names:** "Master", "Insert 1", "Insert 2", … (existing).
- **Number cells:** "M" for the master, the insert number with no prefix, "dB" on the level strip.
- **Layout names:** "Slim", "Normal", "Wide", "Full", and the variants "Slim, names below", "Normal, names below", and so on.
- **Strip text:** strips carry no word labels except the name, the number and "fx" (and today's "S" until the solo proposal is accepted). Every other control is a glyph, knob or light, named in the hint bar.
- **Proposed renames of existing labels** (each needs `mixer.md` / `window-layout.md` updated first; until then the current label stays):
  - "ON" text button → the round enable light (no text);
  - "+ Insert" → a plus glyph, hint "Add an insert after the selected track";
  - "empty" slot text → "Slot 1" … "Slot 10"; "+ add effect" → click on any empty slot;
  - "In" / "Out" → input and output glyphs;
  - today's signed "Pan …" readout → "Pan: 30% right" style; "-inf" → "−∞ dB";
  - "→ M, 3" source summary → the lit down arrow, with the summary moved to its hint and to the output dropdown;
  - "To master" / "+ Send" → the output dropdown's tick items.
- **Slot placeholders:** "Slot 1" … "Slot 10".
- **EQ labels:** "EQ", "Hz", "Q".
- **Input/output defaults:** "System default". An insert routed nowhere shows "Nowhere".
- **Hint style** (the same as the other sections): sentence case. Start with the control's name and value, then a short "how to change it". Use a colon, full stop or middle dot between parts, and no trailing full stop. Hints go to the hint bar, or into a tooltip when the hint bar is hidden (existing `Hint` trait). Examples:
  - Number and name: "Insert 3 · Bass: click to select, Ctrl+Shift+click to add, right-click for options"
  - Meter: "Insert 3 · peak −6.2 dB · sends to Master, 5 Reverb"
  - Enable light: "Enabled: click to mute" / "Muted: click to enable"; once right-click solo is accepted, "Enabled: click to mute, right-click to solo" / "Soloed: right-click to un-solo"
  - Pan: "Pan: centred" / "Pan: 30% left"
  - Fader: "Level −3.5 dB: drag, or double-click for 0 dB"
  - Separation: "Stereo separation: as is" / "40% narrower" / "40% wider"
  - Phase: "Invert phase: off" · Swap: "Swap left and right: off"
  - Effects switch: "2 effects on (Reverb, Delay): click to bypass all, right-click to open their windows"
  - Arm: "Arm for audio recording"
  - Route arrows: "Send Insert 3 here" / "Insert 3 feeds the master: click to stop" / "Sends only go to higher-numbered inserts"
  - Send knob: "Send from Insert 3: −4.0 dB. Chevron removes it"
  - Slot: "Slot 2 · Delay: click to open, scroll to move, right-click to rename" / "Slot 4: empty, click to add an effect"
  - Mix knob: "Mix 100%"
  - EQ handle: "Mid: 1.2 kHz, −3.5 dB, Q 0.7: drag for frequency and gain, right-drag for width"
  - Header: "Window menu" (shared), "Show strip details", "Show effect slots (Ctrl+Enter)", "Add an insert after the selected track (right-click: more)", "Strip layout: Normal. Click for the list, scroll to change"
- **Number formats:**
  - Level and send level: signed dB with one decimal ("+1.5 dB", "−6.0 dB", "−∞ dB"). The Wide and Full readout drops the unit ("−6.0"), as today.
  - Pan: whole percent with "left" or "right", or "centred".
  - Separation: whole percent with "narrower" or "wider", or "as is".
  - Mix: whole percent.
  - EQ frequency: "120 Hz" below 1 kHz, otherwise "1.2 kHz" with one decimal. Gain: signed dB with one decimal. Bandwidth: "Q 0.7".
  - Track numbers: plain integers.

## States and feedback

- **Hover:** every interactive element lightens one step (knob bodies, fader caps, slot rows, arrows) and the hint bar updates. Strips themselves do not highlight on hover, so sweeping across the mixer stays calm.
- **Pressed:** small switches darken briefly to `ACCENT_DIM`. Knobs and faders have no press state: the moving arc or cap is the feedback.
- **On or active:** lit enable light, lit slot light, accent "fx" text, highlighted phase or swap glyph, lit master arrow, accent arcs on send knobs.
- **Selected:** the strip fill lifts to `RAISED`, the meter well gets an accent outline, the fader cap turns to the selected look, and the name text brightens. Several strips can be selected at once and all look the same. The primary selection adds a thin accent edge round the whole strip.
- **Menu target:** the right-clicked strip lifts one step further while its menu is open, so the user sees which track "this track" means.
- **Muted:** the enable light goes dark. Nothing else dims, so the strip stays readable. The meter falls silent by itself.
- **Soloed:** the soloed track's light gains a ring in the highlight tone. Tracks silenced by the solo show their light in the dim accent (enabled but not heard).
- **Locked:** a padlock replaces the light.
- **Bypassed effects:** the "fx" text is struck through and dim, and the panel's slot names are dim.
- **Disabled:** arrows on strips that cannot receive draw as outlines and ignore clicks. The "fx" switch with no effects is in the faintest tone and ignores clicks. Mix knobs on send slots are dim and inert.
- **Playing:** only the meters move (and the level strip). Peak-hold lines fall back slowly. The clip mark stays lit until clicked.
- **Armed and recording:** the arm dot is filled in the danger tone and the fader cap takes the armed look. While the transport records, the arm dot pulses.
- **Dragging:**
  - Fader and knob drags move live, and ganged controls move together.
  - The hint bar shows the value throughout.
  - Dragging the dock handle moves the meter/fader boundary on all strips live.
  - Moving a track or slot shows an accent insertion line.
  - Browser drops outline the target strip or slot in the accent.
- **Focus:** the mixer's header follows the shared focused/unfocused look. The proposed mixer-local keys (see "Keys" under C) act only while the pointer is over the mixer and no text field has focus.
- **Collapsed:** only the header row shows (shared).

## MusicCraft today: gaps

Ordered by visual impact. File references are where the change lands. `docs/spec/mixer.md` ("Layout") and `docs/spec/window-layout.md` ("Mixer") describe today's strip order and must be updated first (spec before code).

1. **Strip anatomy.** The strip now runs: tinted number cell, name beside the meter, round enable light with a tiny S light under it, pan, fader, then phase and swap switches, the separation knob and the effect count (the dB level is in the fader's hint). The light solos on Ctrl+click and from its right-click menu, and the effect count bypasses or enables all effects. Still missing: the arm switch. Files: `crates/musiccraft-ui/src/mixer.rs` (`strip`, `details`).
2. **Routing picture.**
   - Arrows now show three states: filled (routed), outlined (can be routed) and a faint inert outline; the selected strip shows an upward source arrow with its destinations in the hint.
   - There are no incoming-route chevrons.
   - Send levels are sliders in the side panel, not knobs on the destination strips.
   - There are no cables at all.
   `crates/musiccraft-ui/src/mixer.rs` (`route_arrow`, `route_summary`, `routing`).
3. **Slot panel.** The heading and hint sentence are gone, and slot rows have the chevron (slot menu with Open window, Bypass, Move up/down and Remove), the name or a dim "Slot n" that opens the add list, the mix knob and a round light. Slots can be dragged to reorder, and the input and output rows use glyphs. Still missing: wheel reordering, Replace, Rename, Presets and rack items in the slot menu, "Only loaded slots", and the output dropdown that replaces the "To master" and "+ Send" editor. `crates/musiccraft-ui/src/mixer.rs` (`effects`, `slot_row`, `routing`, `output_row`).
4. **Track EQ.** Today it is three small gain knobs labelled "low", "mid", "high", with the frequency only in a right-click number field. There is no response graph, no band faders and no frequency or bandwidth knobs. The model has no per-band bandwidth (the mid band's Q is fixed), so the "Q" row needs `mixer.set_eq` to take a width first. `crates/musiccraft-ui/src/mixer.rs` (`eq_row`); `musiccraft-project` `TrackEq`; `docs/spec/mixer.md` ("Track EQ and slot mix").
5. **Header.** The strip-details and slot-panel switches and the add-insert plus are in place, with View and Tracks in the window menu. Still missing: the layout selector, the add-insert right-click menu, and the options menu's other View and Tracks items (colour, icon, reset, clear out idle inserts). `crates/musiccraft-ui/src/mixer.rs` (`header_tools`, `window_menu`).
6. **Docks.** The master is separated from the inserts only by a small extra gap, and everything scrolls together (the master scrolls out of view). There is no fixed left dock, no right dock, no divider and no height handle. Docking needs a stored per-track dock (a new undoable command, e.g. `mixer.set_dock <track> left|middle|right`). `crates/musiccraft-ui/src/mixer.rs` (`show`); `crates/musiccraft-project/src/model.rs` (`MixerTrack`).
7. **Selection.** Only one track can be selected (`UiState::selected_track`). There is no Ctrl+Shift multi-selection, no ganged faders, no Ctrl+A, and no "menu target" highlight. `crates/musiccraft-ui/src/lib.rs`, `mixer.rs`.
8. **Track colour and icon.** The number colour is computed from the index (`theme::palette(index + 3)`, master `HIGHLIGHT`) and the number is drawn as coloured text, not as a tinted cell. There is no stored colour, no icon, and no strip tint option. `crates/musiccraft-ui/src/mixer.rs` (`strip`); `MixerTrack` needs `color` and `icon` fields with undoable commands.
9. **Layouts.** The strip width is fixed (`STRIP_WIDTH`), with no layout selector, no names-below variant and no Wide or Full arrangements. `crates/musiccraft-ui/src/mixer.rs`; sizes go into `docs/spec/window-layout.md`.
10. **Fader states.** Holding the rail glides the cap toward the pointer, but the cap has no selected or armed look. `crates/musiccraft-editors/src/widgets.rs` (`fader`).
11. **Meter details.** Meters hold their peak as a thin line and show a clip mark for a few seconds, but there is no selected outline and no level strip with a scale. `crates/musiccraft-editors/src/widgets.rs` (`meter`), `mixer.rs`.
12. **Missing commands behind the controls.**
    - move a track left or right ("Shift left" / "Shift right", Shift+wheel);
    - gaps and groups;
    - "Play selected channels through this track" from the mixer side (proposed mixer-local Ctrl+L; today Ctrl+L is `channel.route_free` over the rack, `keymap.md`);
    - the "Feed" submenu's many-to-one and one-to-many sends, and "Make a bus from selected…";
    - lock mute;
    - mixer-track arming. Today audio recording arms *playlist* rows (`playlist.toggle_track_arm`, `docs/spec/playlist.md`, "Audio recording"), so the strip's arm switch needs the owner's decision before it is specified.
    `crates/musiccraft-commands`.
13. **Drag and drop.** Slots can be dragged onto each other to reorder, and browser effects can be dropped on a strip or the slot panel. Still missing: dropping onto a particular slot (it fills the first free one), and dropping sounds or generators on strips. `crates/musiccraft-ui/src/mixer.rs`.
14. **Scrolling.** The strips and the slot panel have the simplified bars (`scrollbars::plain_area`) and the wheel over a strip's name steps whole strips, and so does the wheel over a number cell; the wheel over empty strip space does nothing yet. `crates/musiccraft-ui/src/mixer.rs`.
15. **Input row.** The input picker changes the one global input device (Settings › Audio) from whichever track is selected, and its hint now says so. The input glyph is decorative only. `crates/musiccraft-ui/src/mixer.rs` (`input_row`).
16. **Specs to change first.** `docs/spec/mixer.md` ("Layout", "Routing") and `docs/spec/window-layout.md` ("Window header", "Mixer") for every layout and order change above; `docs/spec/mixer.md` for each item marked Proposed; `docs/spec/keymap.md` for the mixer-local keys. The main-window section's mixer-header gap ("only icons, the layout selector, the title and the window buttons") is consistent with this section: the add, strip-details and slot-panel buttons are icons.

Deliberately left out (no matching MusicCraft feature): a waveform meter view, per-track latency offsets (including the strip's latency row between the effects switch and the arm switch) and input latency, a recording pickup point selector (before or after the effects) on the input row, a multi-touch toggle, and a utility track that processes whatever track is selected. The level strip keeps only that track's meter role.

## Sources

- **Public online manual pages compared** (text and screenshots, under the owner's 2026-10-21 "layout plus general feel" decision; nothing measured, traced or colour-sampled):
  - `mixer.htm`:
    - "Mixer Functions": track kinds, docks, stretching, scrolling, selection.
    - "Mixer Controls": header icons and their order; master, insert and current tracks; the layout menu (eight view modes, the alternative views with names under the meters, the per-track effect list in the largest view); the fader/meter relative-height handle (three dots on the track dividers); track control icons (invert phase, swap channels, effects master switch, track delay, record arm, send switch); peak meters; mute switch (left-click mute, right-click solo, Ctrl, Alt and Shift variants); panning; level faders (click above or below the cap); stereo separation; audio input and output; effects slots (load arrow, name opens, wheel reorders, disable light); effect slot menu; track properties and scroll.
    - "Routing Instrument Channels to Mixer Tracks" and "Drag and Drop Routing".
    - "External Audio Inputs & Outputs".
    - "Internal Mixer Track Routing & Sidechaining", with "Routing Iconography", "Post-fader Sends (default)", "Sidechain Sends", "Multiple Track Routing" and "Mixer Docks".
    - Screenshots: `img_shot/mixer_main.png` (re-checked for this final pass: the stacked order of the switch block, the height handle level with the pan row, the title starting over the slot panel, the input row's left-hand switch), `mixer_routing.png`, `mixer_routingicons.png`, `mixer_sidechain.png`, `mixer_scrollbar.png`, `mixer_externalaudio.png`, `mixer_mixerrouting_menu.png`, `effects_pluginbank.png`.
  - `mixer_mixermenu.htm`: "Options" (Disk recording, View, Detached), "Mixer Track", "Routing", "Group", "Layout", "CPU". Screenshot `img_shot/mixer_mixermenu.png`. Only the functions were taken; every menu item here is reworded and regrouped in our own terms (View options, the "Feed", "Highlight" and "Pin" submenus, "Shift left/right").
  - `mixer_trackprops.htm`: "Mixer Track Properties" and "Panel Controls" (A. Parametric EQ: three fixed bands, left-drag for frequency and gain, right-drag for bandwidth; B. delay panel; C. input delay compensation; D. recording pickup). Screenshots `img_shot/mixer_trackproperties.png`, `mixer_trackproperties_output.png` (`mixer_audioinputcompensation.png` was viewed but is not about layout).
  - `effects_plugins.htm`: "Working with Effect Plugins" and "How to add an effect" (slot menu, drag from the browser, wheel reordering, name click opens, mix level). Screenshot `img_shot/mixer_plugin.png`.
  - `basics_shortcuts.htm`: "Mixer" (S, Alt+S, Alt+L, Alt+Left/Right, Ctrl+A, Ctrl+L, Ctrl+Shift+L, Ctrl+Shift+click, Ctrl+Enter, F2, Shift+wheel).
  - `basics_interface.htm`: the overview screenshot `img_shot/basics_interface.png` (the mixer's place and share of the workspace, the title at the header's right end over the slot panel).
- **Repository files:**
  - `crates/musiccraft-ui/src/mixer.rs`, `crates/musiccraft-ui/src/chrome.rs`, `crates/musiccraft-ui/src/icons.rs`, `crates/musiccraft-ui/src/lib.rs`;
  - `crates/musiccraft-editors/src/theme.rs`, `crates/musiccraft-editors/src/widgets.rs`;
  - `crates/musiccraft-project/src/model.rs` (`MixerTrack`);
  - `docs/spec/mixer.md`, `docs/spec/window-layout.md`, `docs/spec/controls.md`, `docs/spec/keymap.md`, `docs/spec/playlist.md` ("Audio recording"), `docs/spec/recording.md`, `docs/clean-room.md`;
  - `crates/musiccraft-ui/keymap/default.keymap` (clash check for the proposed keys), `crates/musiccraft-ui/src/channel_rack.rs` (Ctrl+L);
  - `docs/screenshots/playlist-mixer.png`, `docs/screenshots/synth-and-eq.png`;
  - shared conventions from the main-window and channel-rack sections of this guide (header slot order, default layout, window items).
- **Own design:**
  - all glyphs, colour roles and labels, and all menu wording (options menu, track menu, slot menu);
  - the title's fallback placement when the slot panel is hidden or on the left;
  - keeping the "S" solo switch under the enable light until the solo proposal is accepted, and the mixer-local treatment of clashing keys;
  - the layout names and the four-widths-by-two-variants scheme, and the width multiples;
  - the level strip (replacing a utility track);
  - the vertical budget and the order in which sections hide when short;
  - the master's on/off route switch (our model has no master send level);
  - inert outlined arrows following our "sends only go up" rule;
  - cables to other docks drawn as two stubs;
  - faint dashed cables for compressor and vocoder sidechains;
  - the "Make a bus from selected…" wording;
  - the primary-selection edge and menu-target lift;
  - the Wide layout's switch column beside the fader;
  - the Full layout's effect list position;
  - the EQ band-fader and knob-row arrangement in our drawing;
  - the output dropdown doubling as a routing editor for inserts;
  - drag and drop of slots and strips;
  - every hint text and number format.
