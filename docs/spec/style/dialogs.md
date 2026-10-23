# Style guide: Automation, event editor, settings and other dialogs

Part of the [interface style guide](README.md). Read it with the shared rules in [design language](design-language.md), [controls](controls.md) and the window header in [main window](main-window.md).

The secondary windows are where a user fixes one detail and goes back to the music: the event editor (one parameter's recorded movements inside a pattern), the automation clip editor (one curve of points), the settings window, the export dialog, the project info page, the sample editor frame, and the small message, confirmation and progress boxes. They should feel like tools that belong to the same compact, dark workspace as the main editors. They have a narrow title row, groups of controls packed into darker boxes with a small heading in their top-left corner, a control followed by its label on the same short row, and a lot of options in little space without looking crowded. The content (curves, waveforms, values being edited) is always brighter than the chrome around it. Nothing pops up unless it has to, and a box that does pop up names its action on its buttons.

## Placement and proportions

**General rules for every secondary window**

- All of them float above the workspace (the area between the browser and the right edge, under the toolbar). None of them is docked into the main layout. They keep the position and size they had last time they were open in this session (own design: per window kind, not saved in the project).
- There are two weights of window:
  - **Editor windows**: the event editor, the automation clip editor and the sample editor. They use the shared one-row window **header** defined in the main-window section ("The header row, left to right": window menu button at the far left, the window's tool groups, an optional play button, the title, the drag area, optional right-hand items, then collapse, maximize/restore and close at the far right). This section lists only each window's own tools, title and right-hand items. They are freely resizable and can be moved anywhere. Several can be open at once, and they stack with the other editor windows.
  - **Dialogs**: settings, export, project info, message and confirmation boxes. They have a plain title row (title text at the left, close cross at the right, no tools). They open centred on the workspace and are not resizable, except that settings may grow taller. Settings and export are not modal: the rest of the app keeps working behind them. Message and confirmation boxes are modal: they dim nothing, but the app ignores clicks outside them until they are answered.
- **File dialogs are the operating system's own.** Open, Save as, Export (picking the destination), Add folder and similar choices use the native file chooser (the desktop portal on Linux, the standard panels on macOS and Windows). MusicCraft draws no file browser of its own for these. Our in-app browser stays the way to find samples and presets. Because the chooser is native, the reference's own favourite-folder shortcuts in its file dialogs have no counterpart here; the operating system's bookmarks and recent places fill that role. This is deliberate, not an omission. The native dialog opens on the project's folder (or the user's home folder for an unsaved project), with a file-type filter that matches the action. The suggested file name comes from the project name.
  - The dialog is attached to the main window and does not freeze it: audio keeps playing and the window keeps drawing while it is open (on macOS the system panel is modal, as usual there). Only one file dialog is open at a time; asking for a second one while the first is open puts "a file dialog is already open" in the hint bar.
  - Which actions use it, and how: Open (one project file), Save as and Save new version of an unsaved project (a project file, named after the project), Export audio (a WAV, FLAC, MP3 or Ogg file; the format prompt follows, see the export dialog), Export MIDI (a .mid file), Export stems (a folder; the stems are written into it), Add channels from files (one or more WAV files, one sampler channel each), Add folder to the browser and Add a plugin folder (a folder), Save and Open Antiserum patch (a .json file in the user presets folder), the convolution reverb's Load file… (a WAV file), and in Settings → Folders the FL Studio data folders and a drive mapping's local folder (folders; the Windows side of a drive mapping is still typed, since it names a path on another system).
  - Text prompts remain only for names and short values: rename, marker name, channel group, time signature.
- **Sample slots take a sound from the browser or from the file dialog.** A sample slot is anything that holds one sound: the sampler's (and granular and resynthesis) sample, a drum kit's pads, a zone sampler's Add zone, and Antiserum's sample, granular and spectral oscillators and its multisample zones. Each slot's button (Change…, Sample…, Add zone…; right-click on a drum pad) opens a menu:
  1. **Browse files…**: the system's file dialog, opening on the folder of the slot's current file (or the project's folder), filtered to the sound files the app can play (WAV).
  2. A separator, then **Sound packs** ▸ the built-in sounds, and one submenu per browser folder (Settings → Folders), nested like the browser's tree: subfolders first, then sounds. The folders are read from disk again each time the menu opens.
  3. **Clear**, after a separator, for slots that may be empty (drum pads, Antiserum's oscillator sample).

  A sound dragged from the browser onto a slot fills it: onto the sample row, a pad, the zone list (adds a zone) or an Antiserum oscillator's sample row or zones. The slot is outlined in the accent tone while the sound is over it, as other browser drop targets are. The channel rack's own drops (onto a sampler row, or the rack to add a channel) are unchanged. The exported instrument plugins use the same menu, with the user's music and sample folders in place of the browser's, and no drag (there is no browser in a plugin host).

**Event editor**

- A floating editor window that is clearly **wider than tall**, about two to two and a half times as wide as it is high. It opens at about the width of the playlist area and about a third of the workspace height, low on the workspace so it does not cover the toolbar.
- Inside, from top to bottom:
  1. the header (one row);
  2. a thin **overview strip** that is both horizontal scroll bar and zoom control, about the height of the header or a little less;
  3. a thin **time ruler**, about half a header tall;
  4. the **value area**, which takes all the rest, roughly four fifths of the window's height.
- A narrow vertical scroll bar runs down the right edge beside the ruler and value area. It is about as wide as the overview strip is tall. A small square **vertical zoom box** sits at the right end of the overview strip, above the vertical scroll bar.
- It can be resized from any edge or corner. Making it taller stretches only the value area. It can be maximised from the window menu ("Maximize" fills the workspace). Moving it to its own desktop window is a later addition and should use the same "pop out" choice as the other windows (see the plugin windows section).

**Automation clip editor**

- A floating editor window, **wider than tall**, at about three times as wide as high by default. It opens just above the bottom of the workspace, spanning most of the workspace's width, so the clip being edited in the playlist stays visible above it.
- Inside, top to bottom:
  1. the header;
  2. a **settings band** of a few small knobs (range and speed), about one and a half header rows tall. This is a later addition (see gaps);
  3. a **graph section**: a section heading ("Curve") with the LFO switch beside it, then the curve graph with a horizontal scroll bar under it and a vertical one at its right, then one thin **graph footer row** of small tools;
  4. a **targets section**: a heading ("Targets") with a row of small tool buttons beside it, then a short list (three or four rows visible) of the parameters this clip drives.
- The graph takes about three fifths of the window's height. The targets section is about a fifth, and the rest is header, band and footer. The targets section can be folded to its heading row with a chevron at its right end, and the graph then grows into the space.
- Double-clicking an automation clip in the playlist opens this window. It scrolls the graph so that the place that was double-clicked is in view.

**Settings window**

- A dialog **about as tall as it is wide, slightly taller**. It opens centred on the workspace and is wide enough for two columns of option rows side by side. Pages that are longer than the window (General, Folders) scroll vertically inside the page area. The tab strip and title row stay fixed.
- Under the title row comes a **tab strip along the top** (not down the side), one row of tabs, about as tall as the title row. Under that, the **page area** fills the rest, with an even inner margin all round. The page is a vertical stack of **section boxes** with a small gap between them.
- The window is one fixed width for all pages, so switching tabs never makes it jump sideways. The height may differ per page, up to a cap of about four fifths of the workspace height. Beyond that the page scrolls.

**Export dialog**

- A dialog that is **tall and narrow**, nearly twice as tall as it is wide. It opens centred and is not resizable.
- From top to bottom: title row, then a stack of section boxes (Length, Format, Quality, Options), then at the very bottom a **progress bar** across the full width, and below it a **button row**.
- Every section except Length can be folded to its heading row with a chevron at the heading's right end. A folded section takes one thin row, and the dialog shrinks to fit. The folded state is remembered for the session.

**Project info page**

- One page (tab) of the settings window, so it has the same size and frame. It is a short page that does not scroll: four labelled fields, a larger comments box, a link field, and two or three lines of statistics at the bottom left with one button at the bottom right.

**Sample editor frame**

- Today the sample editor is a folding section inside a channel window (see `docs/spec/sample-editor.md`). The frame described here keeps that placement. It sits **inside the channel window's page**, the full width of the page, below the sample's own settings. It is about a third as tall as it is wide when open.
- The frame has, top to bottom: a **tool row** of small icon buttons in groups, a thin **info row** (rate, length, selection), the **waveform** filling most of the height, a thin **overview strip** that zooms and scrolls, and a **footer row** of value controls (stretch ratio, pitch, key and scale) with their apply buttons.
- A later "open in its own window" button in the tool row can move the same frame into a floating editor window. That window is wider than tall at about two and a half to one, with the same layout.

**Message, confirmation and progress boxes**

- **Message box**: small, wider than tall (about two to one), centred on the workspace slightly above the middle. Its width fits the text up to a cap of about a third of the workspace, and longer text wraps.
- **Confirmation box**: the same size and position as the message box, with two or three buttons in a row at the bottom right.
- **Progress** for long jobs is shown in one of two places:
  - inside the dialog that started the job, as the export dialog's bar does;
  - for jobs started without a dialog (stem export from the menu, plugin rescan, sample edits that take more than a moment), as a **thin bar inside the hint bar**. A thin bar runs along the hint bar's bottom edge with the job's name as the hint text, and a small cross at the right end cancels.
- No separate progress window is ever opened.

## Anatomy

### Event editor window

Our event editor shows and edits one **pattern lane**: the recorded automation a pattern holds for one target (`docs/spec/recording.md`, "Automation inside patterns"). The window does not exist yet, so everything below is **Proposed (needs `recording.md` updated first)**: the spec section and its commands come before any code.

All keys named for this window are **window-local** (active while the pointer is over the event editor), so they never take over the global keymap (`docs/spec/keymap.md`). Two of them, Ctrl+M and Ctrl+I, are global keys elsewhere (metronome, wait for input); over this window they mean the window's own action, and everywhere else they keep their global meaning. If the owner prefers no overlap at all, leave those two unbound and reach the actions through the menu.

**A. Header** (one row, left to right; window menu at the far left and collapse, maximize and close at the far right come from the shared header in the main-window section)

| # | Element | What it shows | Click / drag / right-click / wheel | States |
|---|---|---|---|---|
| A1 | Menu button | Our three-bar menu glyph | Click opens the window menu (see A-menu below). | Hover: one step lighter. Open: lit in the accent. |
| A2 | Tools button | A small sliders glyph (see Icons) | Click opens the Tools part of the menu directly (Scale values, Shape generator). | As A1 |
| A3 | Snap button | A square icon button the size of the tools, with the shared snap glyph (`Icon::Snap`, toolbar section). No text in the header; the current value is in its hint ("Snap: Beat") and ticked in its list | Click opens the snap list: Global (follow the toolbar's snap), None, 1/4 step, 1/2 step, Step, Beat, Bar, the same list as the toolbar and playlist. The wheel steps through it. Snap applies only to the Draw tool. Holding Alt while drawing bypasses snap. | Hover: one step lighter. List open: pressed look. |
| — | Small gap | — | — | — |
| A4 | Tool buttons, in this order: **Draw** (P), **Paint** (B), **Erase** (D), **Smooth** (I), **Select** (E), **Zoom** (Z) | Square icon buttons of equal size, touching or nearly so | Click picks the tool, and exactly one is always active. The keys are the shared tool keys from the keymap (P, B, D, E, Z, as in the piano roll and playlist), plus I for Smooth, window-local. Holding right-click while rolling the wheel over the group steps through the tools. | Active: accent fill, dark glyph. Inactive: `RAISED` fill, `TEXT` glyph (the one button rule set in the design-language section). Hover: one step lighter. |
| — | Small gap | — | — | — |
| A5 | Lane play button | A small round button with the play triangle | Click plays the pattern from the selection start, or from its start. Click again stops. | Lit in the accent while this pattern is playing |
| A6 | Target title (selector) | "Events — <pattern> — <target name>", e.g. "Events — Drums — Kick volume", with a small right chevron at the end | Click opens the list of lanes this pattern holds, one row per target, plus "Edit another parameter…", which picks any automatable control by its last tweak. The wheel steps through the lanes. | The chevron points down while the list is open. If the pattern has no lanes the title reads "Events — <pattern> — (no lane)". |
| A7 | Flexible gap | — | Dragging here moves the window | — |
| A8 | Collapse button | The shared collapse glyph (main-window section: an upward chevron over a short bar; flipped while collapsed) | Collapses the window to its header and back | — |
| A9 | Maximize / restore | The shared corner-brackets glyph | Fills the workspace, or restores | — |
| A10 | Close | The shared close cross | Closes the window. Nothing is lost: every edit was already a command. | Hover: the cross in the danger tone |

**A-menu: window menu** (A1). The groups are listed in order. Every item is a command with its key shown at the right (all keys window-local, see above):

- **File**: Import from MIDI file… (Ctrl+M: reads one controller lane from a .mid file into this lane), Export lane… (writes it as a one-controller .mid file). Both use the native file dialog.
- **Edit**: Cut, Copy, Paste, Delete (the selection, or the whole lane when nothing is selected), Select all, Select previous range (Ctrl+Left), Select next range (Ctrl+Right), Insert space (Ctrl+Ins), Remove space (Ctrl+Del), Write current value (Ctrl+I: puts the target's current value at the selection start, or at the lane start), Convert to automation clip… (opens the small Convert box, see E below).
- **Tools**: Scale values… (multiply, offset, invert, curve the selection's values), Shape generator… (Ctrl+L: fills the selection with a repeating sine, triangle, saw or square shape of a chosen period and depth).
- **View**: Background waveform (Alt+N: shows a dropped sample behind the values), Time shading (none, 1 bar, 2 bars, 4 bars), Follow playhead, Invert grid shading.
- **Snap**: the same list as A3.
- **Zoom**: Zoom in (PgUp), Zoom out (PgDn), Zoom to selection (Shift+5), Zoom out fully (Shift+4).
- **Options**: Shift following on cut and paste (when on, pasting inserts and cutting closes the gap; when off, pasting overwrites and cutting leaves a flat gap), Smooth after drawing (runs Smooth on what was just drawn when the mouse is released).
- **Window**: Maximize, Collapse, Close.

**B. Overview strip** (one thin row)

1. **Left chevron button** at the left end: scrolls one step left per click and repeats while held.
2. **Rail** across most of the width. It shows a faint miniature of the whole lane's values, drawn as a thin outline.
3. **Thumb** on the rail: a raised bar covering the visible part.
   - Drag the middle to scroll.
   - Drag either end to zoom: a shorter thumb means zoomed in.
   - Double-click to zoom to fit.
   - While the lane plays, the thumb shows the play position as a thin line.
   - A label in small text, centred on the thumb, names the current tool action ("Draw", "Select", …) when the thumb is wide enough. It is hidden otherwise.
4. **Right chevron button**: like 1, to the right.
5. **Vertical zoom box**: a small square at the far right. Drag up or down to zoom the value axis. Double-click resets it to the full 0–100 % range.

**C. Time ruler** (thin row)

- Bar numbers in small text at each bar line, with beat ticks between them. The density thins out when zoomed out (own rule: never closer than about three digit-widths).
- A **playhead marker** sits on the ruler: a small downward triangle at the play position. Dragging it moves the song position.
- Double-click or Ctrl+drag on the ruler selects a time range. The range shows as a band on the ruler and a faint shade over the value area. Click without dragging clears the selection.
- Right-click on the ruler: Zoom to selection, Clear selection.

**D. Value area**

- **Background**: the grid. Vertical bar lines are the strongest, beat lines are fainter and step lines faintest. Every few bars (per "Time shading") alternate between two very close tones so groups of bars read at a glance. Horizontal lines at 25 %, 50 % and 75 % of the value range are faint. The 50 % line is slightly stronger when the target is centred (pan, pitch).
- **Values**: the lane is drawn as a **filled area** from the bottom up to the value at every tick. Its top edge is a crisp line and the fill under it is a lighter, translucent version of the same tone. Recorded data is dense, so it reads as a smooth landscape. Hand-drawn steps read as flat plateaus. Before the first event and after the last, there is no fill (the lane does not drive the parameter there; see recording.md). The area is not one colour block: the top edge is the strongest part.
- **Behaviour by tool**:
  - **Draw**: left-drag draws freehand, writing one value per snap step under the pointer. Right-drag draws a straight line from the press point to the release point. Shift locks the value (draws flat). Ctrl+drag selects a time range instead. Alt bypasses snap.
  - **Paint**: like Draw but ignores snap, so it writes every tick. Right-drag also draws a straight line.
  - **Erase**: left-drag marks a range in the danger tone, and on release resets it to the value at the range's start (flat). Ctrl+drag selects a range.
  - **Smooth**: left-drag marks a range in the accent tone, and on release redraws it as straight lines between its ends. Right-drag does the same with a smooth curve instead of straight lines.
  - **Select**: left-drag selects a time range (as the ruler does).
  - **Zoom**: left-drag a range to zoom to it. Right-click zooms out one step.
- **Wheel**: scrolls horizontally. Ctrl+wheel zooms horizontally around the pointer. Alt+wheel zooms vertically. The window's scroll bar scrolls vertically when zoomed in on values.
- **Hover**: a thin vertical hairline follows the pointer through the value area. The hint bar reads the value at that tick in the target's own units ("Kick volume: −6.0 dB at 3.2.1"). The pointer becomes a pencil cross for Draw and Paint, and the tool's own cursor for the others.
- **Playhead**: a thin vertical line across the whole value area in the playhead tone, moving while playing.
- **Background waveform** (View → Background waveform, after a sample was dropped on the area): the sample's waveform is drawn behind the grid in a dim tone, centred vertically, so values can be lined up with audio. Toggled with Alt+N.
- **Empty lane**: the area shows one line of dim centred text: "Draw to add values, or move the control while recording in pattern mode."

**E. Convert box** (Edit → Convert to automation clip…)

- A small floating box, wider than tall, anchored over the top-right corner of the value area. It has:
  - a title row ("Convert to automation clip") with a close cross;
  - one recessed section holding a single **Detail** slider (fewer points to more points), with the label to its left;
  - a button row: **Reset** at the left and **Convert** at the right.
- While the slider moves, the value area previews the points the clip would get. The points are drawn as small dots on a thin curve over the dimmed lane. When the preview takes too long, it updates on release instead of while dragging. The wheel over the slider steps it.
- **Convert** runs one command that creates the automation clip, places it in the playlist and opens it in the automation clip editor.

### Automation clip editor window

**A. Header** (left to right)

1. **Menu button** (three bars). Its menu, grouped:
   - **State**: Copy curve, Paste curve (from another clip), Open curve file…, Save curve file… (native dialog).
   - **Tools**: Flip vertically, Scale values…, Normalize (stretch so the highest point reaches the top and the lowest the bottom), Thin out points… (a Detail slider box like E above), Smooth…, Curve from audio file… (follows a sample's loudness; native dialog).
   - **View**: Invert grid shading, Background gradient under the curve.
   - **Window**: Maximize, Collapse, Close.
2. **Title**: "Automation — <clip name>", ending in a small right chevron. Click lists all automation clips in the project so the window can switch clip. The wheel steps through them.
3. **Flexible gap**.
4. **Collapse**, **Maximize**, **Close**, the shared buttons from the main-window section.

**B. Settings band** (later addition; its own spec first). It is one recessed row:

| Order | Element | Behaviour |
|---|---|---|
| 1 | **On** pill switch | Turns the clip's output on or off without deleting it. Same widget as the mixer strip's On switch. |
| 2 | **Low** knob | The value the curve's bottom maps to (0–100 % of the target). Its caption shows the value while dragged. |
| 3 | **High** knob | The value the curve's top maps to. Low above High inverts. |
| 4 | **Speed** knob plus a small LCD **×** box beside it | Plays the curve faster or slower. The box is a multiplier (×1, ×2, ×4 …) set by drag or wheel. |
| 5 | Flexible gap | — |
| 6 | **Link** button at the far right | Shows or hides the targets section. Its glyph: two small circles joined by a line. |

Every knob: drag up or down to change it, double-click to reset, Ctrl+drag for fine steps, right-click for the control menu (type in value, reset, create automation, add controller).

**C. Graph section**

1. **Heading row**: the small heading "Curve" inside the section box's top-left corner (the shared section-box rule, see Settings C). Directly to its right is an **LFO** round toggle with the label to its right (later addition). When the LFO is on, the graph shows the LFO's repeating shape and the footer gains its knobs (see 5).
2. **Graph**: the curve on a grid. The grid tones and lines are as in the event editor, with time running left to right in bars and the value from bottom (0 %) to top (100 %).
   - **Curve**: a smooth line (not a filled area) of medium weight. A faint fill under it, a much lighter wash of the curve tone, helps it read; this is the "Background gradient" view option and is on by default.
   - **Points**: small filled circles with a thin dark rim, at every point. The first and last points are the same size. A **selected point** is drawn larger with a ring in the highlight tone.
   - **Tension handles**: a small **hollow** circle at the middle of every segment, sitting on the curve. It is half the size of a point and drawn in the curve tone with no fill. Segments that are straight lines at zero tension still show the handle.
   - **Behaviour** (**Proposed (needs `playlist.md` updated first)**). Today's editor, as `playlist.md` specifies it, adds a point on click, deletes it on right-click, picks a segment's shape by right-clicking the segment and bends it with Alt+drag. That stays the default until the spec changes. The proposed gestures are:
     - Left-click empty grid: deselects all.
     - Right-click empty grid: adds a point there, and keeps the button held to place it. Shift+right-click adds a point on the curve at that time without changing its shape.
     - Left-drag a point: moves it. Shift locks its value, Ctrl locks its time, Alt bypasses snap. The point cannot pass its neighbours.
     - Left-drag a tension handle up or down: bends the segment. Ctrl gives fine steps. Right-click a handle: resets the tension to zero.
     - Right-click a point: opens the **point menu** (see D).
     - Ctrl+drag on the grid: rubber-band selects points. Shift+Ctrl+drag adds to the selection. Middle-click a point: removes it from the selection. Dragging any selected point moves them all.
     - Delete: removes the selected points, but never the last one.
     - Alt+left-click a point deletes it (own choice of modifier).
     - Drag the last point to the right: makes the clip longer.
     - **Step mode** (footer toggle, see 4): left-drag draws freehand, laying down a new point each time the pointer passes a snap line. Shift draws only straight horizontal and vertical lines (pulses).
     - **Slide mode** (footer toggle): dragging a point also moves every point after it by the same time, so the shape after it is kept.
   - **Hover**: a point or handle under the pointer grows slightly and its value shows in the hint bar ("Point 3 of 7 · 62 % · bar 2.3"; for handles, "Tension +0.40"). While dragging, the same text also floats beside the pointer in small monospace text.
   - **Wheel**: scrolls horizontally. Ctrl+wheel zooms around the pointer. Home zooms out fully.
   - **Playhead**: a thin line in the playhead tone while the clip plays.
3. **Scroll bars**: horizontal along the graph's bottom and vertical along its right side. They are thin, with chevrons at the ends, in the shared style.
4. **Graph footer row** (one thin row under the horizontal scroll bar):
   - at the left: the **options chevron** (a small down chevron; opens the same menu as A1), then the **Zoom out fully** button (magnifier);
   - a flexible gap;
   - at the right, three small toggle buttons in this order: **Step drawing** (pencil), **Snap** (the shared `Icon::Snap`), **Slide following points** (a two-headed horizontal arrow). They are lit in the accent when on.
5. **LFO footer** (replaces the left part of the footer while the LFO is on): five small knobs in a row with tiny captions: **Speed**, **Shape** (morphs pulse → sine → triangle), **Skew**, **Width**, **Depth** (centre is zero, left inverts). Then a tiny round **Multiply** toggle: when it is on, the LFO scales the curve; when it is off, the LFO is added to the curve.

**D. Point menu** (right-click a point). Small text, grouped:

- A heading line in dim text: "Point 3 of 7".
- **Delete**.
- A heading "Shape of the segment before this point", then radio items, one per shape, in our wording and order: **Curve** (one bend, set by the handle), **S-curve** (two mirrored bends), **Hold** (stays flat, then jumps), **Steps** (equal steps; the handle sets how many). Further shapes may be added later; each gets its own spec entry and a name in our own words before it appears here.
- **Copy value**, **Paste value**, **Type value…**: opens a tiny inline field over the point. The value is a percentage (0–100), or the target's own units when it has them. Enter applies.

**E. Targets section** (later addition, after one target becomes many in the spec)

1. **Heading row**: the heading "Targets", then a row of small icon buttons in this order:
   - **Add**: a plus. While it is lit, every control the user moves is added as a target. Click again to stop.
   - **Remove**: a cross. Removes the selected row.
   - **Edit link**: a gear. Opens the link settings: range mapping and smoothing.
   - **Show**: an arrow into a box. Opens the window holding the target control.
   - **Wiggle**: a small wave. Briefly moves the target control so it can be found.
   - **Convert to pattern lane**: a dotted outline of a lane.
   - **Select in playlist**: a dashed box.
   
   A fold chevron sits at the far right of the heading row.
2. **List**: one row per target, in small text: "<owner> — <parameter>", e.g. "Lead — Filter cutoff". Click selects a row, and the selected row uses the selection fill. Double-click does the same as Show. Right-click gives Show, Edit link and Remove. Empty list: one dim line, "No targets. Press + and move a control."

### Automation clip in the playlist (how the curve looks inside a clip)

- The clip's body is **tinted with the clip colour** across its full height, darker than a pattern clip's body so the curve stands out. The curve is drawn on that tint as a bright line with a lighter translucent fill under it, down to the clip's bottom.
- Points show as small circles and tension handles as smaller hollow circles, **only while the playlist's automation focus is on** or the pointer is over the clip. Otherwise only the curve and fill show, which keeps the playlist calm.
- The clip's title bar shows a small curve glyph (three points joined by a line) before the name, then the name.
- The same point gestures work directly in the playlist clip as in the editor (right-click adds, drag moves, handle drag bends, right-click a point for the point menu). Double-click opens the editor.

### Settings window

**A. Title row**

- At the left: a small gear glyph, then the title "Settings — <page caption>". The page caption is a plain description of the current tab:
  - "Audio devices" (Audio)
  - "MIDI devices" (MIDI)
  - "Interface and startup" (General)
  - "Folders and plugins" (Folders)
  - "Current project" (Project)
  - "Project info" (Info, proposed)
  - "About MusicCraft" (About, proposed)
- At the right: the close cross. Esc closes the window.

**B. Tab strip** (one row along the top, left to right, in three groups separated by a gap about the width of one tab):

1. User settings: **MIDI**, **Audio**, **General**, **Folders**, (later **Theme**).
2. This project: **Project**, **Info** (proposed; today there is no such tab).
3. App: **About** (proposed; today there is no such tab).

The tabs:

- Each tab is a label in normal-size text with roomy side padding. **Each tab is as wide as its own label plus that padding**, so tabs differ in width; they are not stretched to equal widths (unlike the browser's equal-width tab strip). Within a group, tabs touch each other with only a hairline divider between them. Plain rectangular tab ends of our own; no slanted or shaped corners.
- **Active tab**: filled with the page tone and joined to the page with no line under it. The label is in the accent tone and bold.
- **Inactive tabs**: one step darker, label in the dim text tone.
- **Hover**: label to full text tone.
- Click selects the tab. **Proposed (needs `settings.md` updated first):** Ctrl+Tab and Ctrl+Shift+Tab step through the tabs while the settings window has focus (window-local; neither key is bound in the global keymap). The window remembers the last tab, and Options menu items open it at a given tab (as today).
- If the window is too narrow for every tab, a small right chevron appears at the far right of the strip and opens a list of the hidden tabs.

**C. Page area: the common building blocks**

- **Section box**: a darker box (one step darker than the page) with a hairline edge. Its **heading** is small bold text **inside** the box's top-left corner, on the box's own fill, not a separate tab. This is the one section-box rule shared with the controls and plugin-windows sections (their group boxes); the design-language section owns it, and this section follows whatever it settles. No shaped or slanted heading ends.
  - A heading may carry a dim note after the name in brackets, e.g. "Display (applies after restart)".
  - Optional sections have a small up chevron at the box's top-right corner. It folds the box to its heading row, and the chevron then points down.
- **Option row**: one compact row. **The control comes first and its label second**, to the right of the control, in normal text. Examples are a drop-down box followed by "Sample rate", or a round toggle followed by "Send MIDI clock". This control-then-label order is the area's most recognisable layout habit. It keeps all controls aligned on one left edge, and the labels read like a list.
- **Two columns**: inside a section, rows can sit in two columns. The left column starts at the box's inner margin and the right column at the box's middle. Short toggles fill both columns, top to bottom then the right column. Wide controls take the full width.
- **Drop-down box**: a raised dark box, wide enough for its longest choice, with the current choice in normal text at the left and a small right chevron at its right end. Click opens the list under it. The wheel steps through the choices without opening it. The box is sized per row, not stretched.
- **Round toggle**: a small circle (an LED-like dot) with its label to the right.
  - Off: a dark dot with a lighter rim.
  - On: filled in the accent tone with a faint glow ring.
  - Disabled: dimmed together with its label.
  - Clicking the label toggles it too.
- **Number box**: a small LCD-style box (dark well, digits in the display tone) used for counts and channels: Maximum backups, port numbers, MIDI channels. Drag up or down or use the wheel to change it, double-click to type, right-click for a short list of common values. An unset value shows a dim dash.
- **Status text**: small text lines under a control. The first word is in a status tone (accent for "Running", highlight for "Fallback", danger for "Failed") and the rest is in dim text.
- **Info text**: small dim text, wrapped to the box width, kept to one or two sentences. It is used sparingly, only where a choice has a consequence (e.g. "Changing these restarts audio").
- **Wide button**: a raised flat rectangle with centred text, at least a third of a column wide. It sits at the bottom of the section it acts on.
- **Folder rows**: a thin striped list. Each row has a folder glyph button at the left (click: native folder chooser to replace that row's path), the path in normal text, and an optional display name in a right-hand column (editable by click). Empty spare rows show only the folder glyph, so adding a folder is "click the next empty row". The list scrolls if longer than about eight rows.

**D. Pages** (sections in order, top to bottom; rows in order within each)

1. **MIDI**
   - **Output** section:
     - A striped device list three to four rows tall with columns Name | Kind | On marker | Port. The selected row uses the selection fill.
     - Under it, left to right: a round toggle "Send clock and start/stop", then a number box "Port" at the right end.
   - **Input** section:
     - A device list like Output, with an on marker (a small power glyph) in each enabled row.
     - Under it: a round toggle "Enable" for the selected device, a drop-down "Controller type" (generic only for now), and a number box "Port" at the right.
     - Then two columns. Left: drop-down "Note velocity follows" (Velocity / Fixed) and round toggles "Wait for the knob to reach the value before following it", "Use a new controller as soon as it is found", "Sustain pedal holds notes". Right: number boxes for "Preview MIDI channel" and similar channel rows (later additions).
   - **Typing keyboard** section: a round toggle "Typing keyboard plays notes (Z–M, Q–P rows)" with its info text.
   - Bottom of the page: a wide button **Refresh devices** at the left.
2. **Audio**
   - **Input and output** section:
     - Drop-down (wide, about two thirds of the box) "Output device".
     - Drop-down "Input device".
     - Drop-down (narrow) "Sample rate".
     - Status lines: "Running at 48000 Hz · 2 outputs, 2 inputs", then "Latency: 256 frames (5.3 ms)".
   - **Buffer** band: a second, slightly lighter recessed band inside the same section. At its left is a raised tile showing "Buffer 256 frames (5.3 ms)". Click opens the buffer list, and the wheel steps it. Info text sits to its right.
   - **Processing** section: drop-down "Live resampling quality" (later), number box "Metronome track", round toggle "Clear plugin tails when playback stops" (later).
   - Bottom: wide buttons **Restart audio** and **Refresh devices**.
3. **General**
   - **Display** section, with the heading note "(scale applies after release)": a slider row "Interface scale" with the percentage beside it, a round toggle "Show the hint bar", and a drop-down "Animations" (later).
   - **Startup** section: round toggles "Start with an empty project" and "Reopen the last project" (later).
   - **Backup** section: a round toggle "Autosave a recovery copy every minute".
   - **Undo** section (later): number box "Undo levels".
   - **Input** section (later): a wide button "Keyboard shortcuts…" at the right column, which opens the shortcut list.
4. **Folders**
   - **Browser folders** section: folder rows (see C) with a Name column.
   - **Plugin folders** section: folder rows, an info line naming the standard locations, then a status line "123 plugins found" and a wide button **Rescan plugins**. If any failed, a dim fold line "3 files failed to load" in the highlight tone opens to list them.
   - **Samples from Windows projects** section: two path rows ("Factory data", "User data") and a two-column "Drive mappings" list (Windows prefix | folder here) with an Add row at the bottom.
5. **Project**
   - **Time** section: number box "Tempo (BPM)", drop-down "Time signature", number box "Master pitch (cents)" with "A4 = 440.00 Hz" in dim text after it.
   - **Mixing** section: drop-down "Pan law".
   - **File** section: the path in normal text (or "Not saved yet" in dim text), and a wide button **Show in file manager**.
6. **Info** (proposed; see "Project info page" below).
7. **About** (proposed): the app name in large text, the version, licence line "GPL-3.0-or-later", and a short credits list in a scrolling box.

### Project info page

Top to bottom, in one column with a right-hand extra in places:

1. Label "Title" (small text above the field), then a wide single-line field, about two thirds of the width. In the right third, on the same row: a round toggle "Pop up these notes when the project loads".
2. Label "Author" above a field of the same width. In the right third: label "Genre" above a drop-down field you can also type into.
3. Label "Comments" above a multi-line box, the full width and about four lines tall. It scrolls when longer.
4. Label "Web link" above a full-width single-line field.
5. Bottom block:
   - Left: three lines of small text: "Started on 2026-10-21 14:05.", "Time spent on it: 1 h 58 min.", "12 channels · 8 patterns · 15 mixer inserts · 34 playlist clips · 2 automations · 6 plugins."
   - Right: one wide button **Reset working time**.
- Fields apply their text when they lose focus or Enter is pressed, each as one undoable command.
- "Pop up these notes when the project loads" makes a small message box with the title, comments and link appear when the project is loaded.

### Export dialog

**A. Title row**: "Export to <name>.wav" (the extensions of every chosen format are listed, e.g. "<name>.wav / .flac"). The file name is in the highlight tone and the rest in the text tone. The close cross is at the right.

**B. Length section** (heading "Length", not foldable):

- Row 1: drop-down "Mode" (Song / Pattern), then drop-down "Tail" (Keep tail, Wrap tail to start, Cut at end).
- Row 2, in small dim text spread across the width: "Length: 16 bars", "Duration: 0:32", "Size: about 11.2 MB".

**C. Format section** (foldable). The one exception to the top-left heading: "Format" sits at the right end of the box's top row, so the format toggles can start at the box's left edge on that same row.

- Top row: a row of round toggles, one per format: **WAV**, **FLAC**, **MP3**, **OGG**, **MIDI**. More than one can be on, and each one writes its own file.
- Then a two-column grid of the chosen formats' settings. Settings of unchosen formats are dimmed, not hidden, so the layout never jumps:
  - Left: drop-down "WAV depth" (16-bit, 24-bit, 32-bit float) and drop-down "Channels" (Stereo, Mono mix, Left only, Right only).
  - Left: slider "MP3 rate", with its value ("320 kbps") in small text under the slider. Right: slider "OGG quality" with its value under it.
  - Left: drop-down "FLAC depth". Right: slider "FLAC compression" with "level 5" under it.

**D. Quality section** (foldable): drop-down "Resampling", then two columns of round toggles: "Highest quality for plugins", "Ignore voice limits", "Dither 16-bit output".

**E. Options section** (foldable): two columns of round toggles.

- Left column: "Write marker positions", "Write loop points", "Write tempo".
- Right column: "One file per mixer track (stems)". Under it, indented, one small round toggle "Also export the master" (MusicCraft has no "current track" utility insert, so there is no second choice). Then "Keep track effects on" and "Keep master effects on". The latter is disabled unless "Also export the master" is on.

**F. Progress bar** (full width, a little taller than a row):

- Idle: an empty dark well.
- While rendering, the well fills from the left as a plain rectangle in the accent role. No shaped or pointed end.
- One readout in small text, centred in the well: "Bar 64 of 116".
- When done, the bar fills completely for a moment, then the well empties and the dialog stays open on its settings, ready for another export (own choice: closing it is one click or Esc, and the hint bar names the written files).

**G. Button row** (bottom):

- Left: a round toggle "Show the file when done".
- Right: two wide buttons, **Export in background** and **Export**. While rendering, **Export** reads **Stop**: clicking it stops and deletes the partial file.
- Enter = Export, Esc = Close (or Stop while rendering).
- The output path is chosen with the native save dialog. It opens **before** the export dialog when the project has not been exported before, and from a small **Change…** link after the file name in the title row. The format the save dialog's file ends in picks the export format; changing the format afterwards changes the extension, and a file name without one gets the format's.

### Sample editor frame

1. **Tool row** (one row of small icon buttons in groups, left to right):
   - Group 1, playback: play selection (play triangle), stop (stop square), loop toggle (two arrows chasing round a circle).
   - Group 2, clipboard: cut (scissors), copy (two overlapped rectangles), paste (a clipboard).
   - Group 3, range edits: Trim to selection (two inward chevrons around a bar), Delete selection (a bar with a cross), Silence (a flat line), Fade in (a ramp rising left to right), Fade out (a ramp falling), Reverse (a left arrow over a right arrow), Normalize (a bar touching a top line).
   - Group 4, level: Gain −3 dB and +3 dB as two small buttons with "−3" and "+3" in text.
   - Flexible gap.
   - Right end: Zoom to selection (magnifier with a bracket), Show whole sample (magnifier with a minus), and later Open in its own window (the pop out glyph).
2. **Info row** (thin, small text, left to right in display tone on the dark well): "48000 Hz", "32-bit", "Length 2.415 s". To the right: "Selection 0.512–1.200 s". When nothing is selected: "No selection: edits apply to the whole sample".
3. **Waveform** (fills the height):
   - Peaks drawn mirrored around a centre line in the channel colour. A selected range is shaded with a translucent highlight across the full height and has thin edge lines.
   - Behaviour:
     - Left-drag selects. Shift+drag extends the selection.
     - Double-click selects all.
     - Click clears the selection and sets the play start (a thin line).
     - Right-click gives the edit menu (the same actions as the tool row).
     - Wheel scrolls, Ctrl+wheel zooms around the pointer.
   - A playhead line in the playhead tone moves while the sample plays.
   - Hover shows the time in the hint bar.
4. **Overview strip**: a thin rail showing the whole sample in miniature, with a thumb for the visible part. It works like the event editor's overview strip.
5. **Footer row**: "Stretch ×1.50" (drag box) with **Stretch**, then a gap, "Pitch +2.0 st" with **Pitch**, then a gap, key and scale drop-downs with **Tune**. Buttons are small, the same height as the drag boxes.
6. When the project is unsaved, the waveform area shows a centred dim line: "Save the project first: edited audio is written beside it." The tools are disabled.

### Message and confirmation boxes

- **Title row**: a short title naming the subject, not the app: "Unsaved changes", "Missing samples", "Remove mixer track". The close cross at the right acts like the cancel choice.
- **Body**, top to bottom:
  - a small glyph at the top-left (information: a circle with a short bar and a dot; warning: a triangle with a bar and dot; question: a circle with a question mark), and the message to its right in normal text, wrapped. The first sentence states the situation and the second the consequence;
  - optional details: a dim fold line "Details" that opens a scrolling box (e.g. the list of missing files);
  - optional round toggle at the bottom-left: "Don't ask again" (for questions the user may opt out of; the opt-outs are listed and can be reset on the General page, later).
- **Button row** (bottom right, right-aligned):
  - Buttons are named by their action, never "Yes/No": "Save", "Don't save", "Cancel"; "Remove", "Keep".
  - The default (safe) button is the rightmost and has the accent edge. A destructive choice, if there is one, is leftmost in the danger tone.
  - Enter presses the default, Esc presses Cancel.
- **Hint-bar notices** take the place of message boxes for anything that does not need an answer: "Exported song.wav", "3 plugins failed to load (see Settings → Folders)". A message box is used only when the user must decide something or acknowledge data loss.

## Icons

All glyphs are our own simple vector drawings in the style of `crates/musiccraft-ui/src/icons.rs`: thin even strokes, few points, a single colour passed in by state. The existing glyphs (Pencil, Brush, Eraser, Select, Zoom, Play, Stop, Record) are reused as they are.

| Where | What it does | Our glyph |
|---|---|---|
| Every editor window header, far left | Window menu | Three short horizontal bars, evenly stacked (shared with the other windows) |
| Event editor header, A2 | Tools menu | **Sliders**: two horizontal lines, each with a small filled square on it at a different point |
| Event editor A3, clip footer | Snap | The shared `Icon::Snap` from the toolbar section: two short lines meeting at a right angle (a grid corner) with a filled dot sitting in the corner |
| Event editor tools | Draw / Paint / Erase / Select / Zoom | Existing `Icon::Pencil`, `Icon::Brush`, `Icon::Eraser`, `Icon::Select`, `Icon::Zoom` |
| Event editor tools | Smooth | **Smooth**: three dots joined by one gentle curve that passes through them |
| Event editor A5 | Play the lane's pattern | Play triangle in a small circle; two bars while playing |
| Titles that open lists | List | Small right chevron; points down while the list is open |
| Window collapse, maximize, close | — | The shared glyphs from the main-window section (an upward chevron over a short bar, flipped while collapsed; four outward corner brackets, turned inward to restore; the cross) |
| Overview and scroll bar ends | Step scroll | Single-stroke chevrons, left/right/up/down |
| Vertical zoom box | Zoom values | The shared zoom-box glyph (main-window section): two horizontal lines inside the small square, their gap growing with the zoom |
| Ruler | Playhead marker | Small filled downward triangle |
| Clip editor footer | Options | Small down chevron |
| Clip editor footer | Zoom out fully | Magnifier with a short minus inside the lens |
| Clip editor footer | Step drawing | Existing pencil, small |
| Clip editor footer | Slide following points | A horizontal line with an arrowhead at each end |
| Clip editor settings band | Show targets | Two small circles joined by a straight line |
| Targets row | Add / Remove | Plus / cross |
| Targets row | Edit link | Gear: a circle with six short teeth |
| Targets row | Show target | A small square with an arrow entering it from the left |
| Targets row | Wiggle target | One cycle of a small wave with a dot at its peak |
| Targets row | Convert to pattern lane | A rounded rectangle outline drawn with dashes |
| Targets row | Select in playlist | Existing `Icon::Select` |
| Playlist automation clip title | Marks an automation clip | Three small dots joined by a two-segment line, rising then falling |
| Settings title | Window identity | Gear |
| Device lists | Device on | A power symbol: an open circle with a short vertical bar through its gap |
| Folder rows | Pick folder | Folder outline with a tab on its top-left edge |
| Folder rows | Remove | Small cross |
| Sample editor tool row | Play / stop / loop | Play triangle / stop square / two curved arrows chasing round a circle |
| Sample editor | Cut / copy / paste | Scissors: two small loops with crossing blades / two overlapped rectangle outlines / a rectangle with a small clip tab at its top centre |
| Sample editor | Trim | Two inward-pointing chevrons either side of a short vertical bar |
| Sample editor | Delete selection | A short bar with a small cross over it |
| Sample editor | Silence | One flat horizontal line on a centre axis |
| Sample editor | Fade in / fade out | A straight ramp rising left to right / falling left to right, filled under |
| Sample editor | Reverse | Two horizontal arrows, one above the other, pointing opposite ways |
| Sample editor | Normalize | A short vertical bar whose top touches a horizontal line above it |
| Sample editor | Zoom to selection / whole | Magnifier with two brackets inside / magnifier with a minus |
| Sample editor | Own window (later) | Pop out: a square open at its top-right corner with a diagonal arrow leaving through the gap (shared) |
| Message boxes | Info / warning / question | Circle with bar and dot / triangle with bar and dot / circle with a question mark |
| Export progress bar | — | No glyph: a plain rectangular fill |

## Colour roles

Colours are only tokens from `crates/musiccraft-editors/src/theme.rs`, with relative tone. The surfaces run from darkest to lightest: `PANEL_DARK`, `BACKGROUND`, `PANEL`, `RAISED`, `RAISED_HOVER`. Shared chrome tokens proposed in other sections (`WINDOW_EDGE`, `HEADER`, `HEADER_ACTIVE`, `SCROLL_RAIL`, `SCROLL_THUMB`) are reused here.

| Element | Token(s) | Relative tone and state changes |
|---|---|---|
| Dialog and window body | `theme::PANEL` | Base tone |
| Title row / editor header | `HEADER` / `HEADER_ACTIVE` (proposed in the main window section) | The focused window's header lifts one step. The edge is `WINDOW_EDGE`, and `ACCENT_DIM` when focused. |
| Tab strip background, inactive tabs | `theme::PANEL_DARK` | One step darker than the page. Labels `TEXT_DIM`, hover `TEXT`. |
| Active tab | Fill `theme::PANEL`, label `theme::ACCENT` | Joined to the page with no line under it |
| Section box | Fill `theme::PANEL_DARK`, the same as the plugin-windows group boxes (one shared rule, owned by the design-language section) | Darker than the page. Hairline edge `theme::GRID_LINE` |
| Section heading | Small bold text `theme::TEXT_DIM` on the box's own fill, inside its top-left corner | Heading notes in `TEXT_DIM`, not bold |
| Inner band (buffer band) | `theme::BACKGROUND` | One step lighter than the section box, still darker than the page |
| Drop-down box | Fill `theme::RAISED`, text `theme::TEXT`, chevron `TEXT_DIM` | Hover fill `RAISED_HOVER`. Open: edge `ACCENT_DIM`. Disabled: text `TEXT_DIM`. |
| Round toggle | Off: dot `theme::PANEL_DARK` with rim `theme::RAISED_HOVER`. On: dot `theme::ACCENT` with a faint ring in `ACCENT_DIM` | Disabled: both dimmed with the label |
| Number box | Fill `theme::LCD_BG`, edge `theme::LCD_EDGE`, digits proposed `theme::LCD_TEXT` (toolbar section; equal to `PLAYHEAD` today) — the controls section's compact number box | Unset dash `TEXT_DIM`. Editing: edge `ACCENT`. |
| Option labels | `theme::TEXT` | Disabled `TEXT_DIM` |
| Info text | `theme::TEXT_DIM`, small | — |
| Status word | `theme::ACCENT` (running), `theme::HIGHLIGHT` (fallback), `theme::DANGER` (failed) | Rest of the line `TEXT_DIM` |
| Wide buttons | Fill `theme::RAISED`, text `theme::TEXT` | Hover `RAISED_HOVER`. Pressed `ACCENT_DIM`. Default button: thin edge `ACCENT`. |
| Lists (devices, folders, targets) | Stripes alternate `theme::PANEL_DARK` and `theme::BACKGROUND` | Selected row: proposed `SELECTION_FILL` (shared with the browser; today `ACCENT_DIM`), text `TEXT` |
| Text fields | Fill `theme::PANEL_DARK` with a hairline edge `theme::GRID_LINE`, text `theme::TEXT` | Focus: edge `ACCENT`. Hint text `TEXT_DIM`. |
| Event editor grid | Well `theme::LCD_BG`. Lines `theme::GRID_LINE` (steps), `theme::GRID_BEAT`, `theme::GRID_BAR` | Alternate bar groups: proposed `theme::BAR_GROUP` (piano-roll section, also used by the playlist): a very faint lightening wash |
| Event lane values | Top edge `theme::AUTOMATION`. Fill: new `theme::AUTOMATION_FILL`, `AUTOMATION` at low opacity | Erase marking: `DANGER` wash. Smooth marking: `ACCENT` wash. Selection: `SELECTION_FILL` wash over the area and the ruler. |
| Ruler | Fill `theme::PANEL_DARK`, numbers `theme::TEXT_DIM` | Selection band `ACCENT_DIM` |
| Playhead | `theme::PLAYHEAD` | Hover hairline: `TEXT_DIM` |
| Background waveform | `theme::TEXT_DIM` at low opacity | Behind the grid lines |
| Automation curve (editor) | Line `theme::AUTOMATION`. Fill `AUTOMATION_FILL` | — |
| Points | Fill `theme::AUTOMATION`, rim `theme::PANEL_DARK` | Hover: slightly larger. Selected: larger with a `HIGHLIGHT` ring. |
| Tension handles | Outline `theme::AUTOMATION`, no fill | Dragged: outline `HIGHLIGHT` |
| Automation clip body in the playlist | The clip colour darkened (proposed `theme::CLIP_BODY`, as in the playlist section, darker for automation clips) | Curve in the clip colour lifted towards `TEXT`. Fill: the clip colour at low opacity. |
| Footer toggles | Glyph `TEXT` at rest (the shared button rule, design-language section) | On: glyph `ACCENT`. Hover: fill `RAISED_HOVER`. |
| Export progress well | `theme::METER_BG` | Fill `theme::ACCENT`, plain rectangle. Readout `theme::TEXT`, centred |
| Hint-bar progress | Line `theme::ACCENT` along the hint bar's bottom edge | Failed: `DANGER` |
| Sample waveform | `theme::palette(channel)` on `theme::LCD_BG` | Selection: `HIGHLIGHT` at low opacity with edges at full `HIGHLIGHT` |
| Message glyphs | Info `ACCENT`, warning `HIGHLIGHT`, error `DANGER` | — |
| Destructive button | Text `theme::DANGER` on `RAISED` | Hover fill `RAISED_HOVER` |

## Labels and wording

**Window and dialog titles**

- "Events — <pattern> — <target>"
- "Automation — <clip name>"
- "Settings — <page caption>", with the page captions listed in the anatomy
- "Export to <file>"
- "Convert to automation clip"
- Message boxes take the subject as their title ("Unsaved changes").
- The sample editor's fold heading stays "Edit sample".

**Tabs**: today Audio, MIDI, General, Folders, Project (`settings.rs`, `Tab`). Proposed additions: Info and About (later Theme). Keep today's "Folders" rather than "Files".

**Section headings** (small bold text inside each box's top-left corner):

- MIDI: Output, Input, Typing keyboard
- Audio: Input and output, Buffer, Processing
- General: Display, Startup, Backup, Undo, Input
- Folders: Browser folders, Plugin folders, Samples from Windows projects
- Project: Time, Mixing, File
- Export: Length, Format, Quality, Options
- Clip editor: Curve, Targets

**Segment shapes**: our own plain words: **Curve**, **S-curve**, **Hold**, **Steps**. Any later shape gets a name of our own when it is specified. **Proposed rename:** today's menu says "Single curve" and "Double curve"; rename them to "Curve" and "S-curve" in the menu only, and keep the command names `single` and `double`.

**Proposed option labels** (own wording, replacing nothing that exists today): MIDI input "Wait for the knob to reach the value before following it" and "Use a new controller as soon as it is found"; Audio "Clear plugin tails when playback stops"; Project info "Pop up these notes when the project loads"; Export "One file per mixer track (stems)", "Also export the master", "Keep track effects on", "Keep master effects on", "Highest quality for plugins".

**Tool names (event editor)**: Draw, Paint, Erase, Smooth, Select, Zoom. The keys come from the keymap.

**Hint bar text**: a sentence-case verb phrase that names the gestures, separated by middle dots, with no final full stop. For example:

- "Drag to draw · right-drag for a straight line · Alt ignores snap"
- "Right-click to add a point · drag a hollow handle to bend"
- "Point 3 of 7 · 62 % · bar 2.3"

**Number formats**

- Values: percent with no decimals ("62 %"), or the target's own units ("−6.0 dB", "25% left", "1.20 kHz"), following the controls section's value-format table (pan as "25% left" / "Centre" / "25% right"; frequency with two decimals above 1 kHz).
- Times: bar.beat.step as on the toolbar ("2.3.1").
- Durations: "0:32" (minutes:seconds), and "1:02:05" past an hour.
- Sample times: three decimals with "s" ("1.200 s").
- Rates: "48000 Hz".
- Buffer: "256 frames (5.3 ms)".
- Sizes: "about 11.2 MB".
- Bit rates: "320 kbps".
- Stretch: "×1.50". Pitch: "+2.0 st".
- Counts: "12 channels · 8 patterns".

**Buttons** are named by their action: Export, Stop, Convert, Reset, Rescan plugins, Restart audio, Refresh devices, Save, Don't save, Cancel. Never use OK or Yes/No.

**Info texts** are one or two short sentences in our own words. They state the consequence ("Changing these restarts audio"), not a tutorial.

## States and feedback

- **Hover**:
  - Buttons, tabs and drop-downs lift one step (`RAISED_HOVER`) and labels go from dim to full text.
  - Points and handles grow slightly.
  - Graph areas show the hairline and hint bar readout.
- **Pressed**: buttons fill with `ACCENT_DIM` while held.
- **Active / on**:
  - Tools: accent fill and dark glyph.
  - Round toggles: an accent dot.
  - Footer toggles: an accent glyph.
  - Active tab: accent label joined to the page.
- **Selected**:
  - List rows and lane ranges use the selection fill.
  - Points are larger with a highlight ring.
  - Sample ranges get the highlight wash.
- **Disabled**: control and label dimmed together (`TEXT_DIM`, flat fill, no hover). Export settings for unchosen formats stay visible but dimmed.
- **Muted / off**: an automation clip that is switched off shows its curve dimmed by about half in both the editor and the playlist, and the editor's On pill is off.
- **Playing**: a playhead line in the playhead tone in the event editor, clip editor and sample waveform. The lane play button is lit.
- **Recording**: while a lane records (pattern mode with recording armed), the event editor's title gets a small record dot in the danger tone before "Events". The values being written appear live at the playhead.
- **Dragging**:
  - A point being dragged keeps its highlight ring, and its value floats beside the pointer.
  - A rubber band is a thin dashed accent outline with a faint fill.
  - Scroll thumbs fill with `ACCENT_DIM`.
- **Focus**: the focused window's header lifts and its edge takes `ACCENT_DIM`. A focused text field or number box has an `ACCENT` edge. Keyboard focus on buttons and tabs shows as a thin accent outline only after Tab is used, not after mouse clicks.
- **Busy**: during export the settings sections are disabled (dimmed). The progress fill moves and Export becomes Stop. The hint bar mirrors "Exporting… 64 of 116 bars".
- **Error**:
  - Failed actions put a danger-toned status word in the relevant section (Audio "Failed: device busy").
  - Failed actions also put a hint bar notice.
  - A failed export shows a warning box with "Details".

## MusicCraft today: gaps

Ordered by visual impact.

1. **The export "dialog" is a small prompt with the chosen file and a row of format chips**, shown after the native save dialog (`crates/musiccraft-ui/src/dialogs.rs`, `Purpose::Export`). It needs the following, plus a spec in `docs/spec/` before the code:
   - the tall sectioned layout (Length, Format, Quality, Options);
   - several formats at once;
   - the mode and tail choices;
   - length, duration and size readouts;
   - a progress bar, Stop, and export in the background.

   Stems and MIDI are separate menu items today and would become toggles here. MP3 and OGG exist only as single fixed choices ("MP3 320k", "Ogg q8" chips): the guide's bit-rate and quality sliders, and the FLAC depth and compression settings, have no setting in the model or the export commands behind them yet.
2. **The settings window is a plain tab row over loose widgets** (`crates/musiccraft-ui/src/settings.rs`, `show`, `row`, `note`). It has:
   - `selectable_value` tabs, not joined tabs in groups;
   - no section boxes with headings;
   - labels **before** controls (a fixed-width dim label column), not control-then-label;
   - check boxes instead of round toggles;
   - MIDI input and output as radio lists, not device tables;
   - a default size wider than tall;
   - only five tabs (Audio, MIDI, General, Folders, Project): there is no Info tab and no About tab. "About MusicCraft" in the help menu only writes a status message (`crates/musiccraft-ui/src/toolbar.rs`, `help_menu`).

   The tab order is also Audio first, where the guide puts MIDI first. That order is a small preference and can stay if the owner prefers. `docs/spec/settings.md` lists the tabs and must be updated first for the new tabs, the grouping and the control-then-label layout.
3. **No event editor** for pattern lanes. Lanes can be recorded and cleared (`crates/musiccraft-ui/src/playlist.rs`, "Clear automation (n lanes)"), but there is no window to see or draw them, and no convert-to-clip command. A spec section and commands come first (`docs/spec/recording.md`).
4. **Automation clip editor: what is still missing** (`crates/musiccraft-ui/src/automation.rs`, `curve.rs`). The header menu, clip switcher, settings band (On, Low, High), ruler, curve fill, points, tension handles, selection, point menu with Type value, step drawing, slide, snap toggle, zoom, horizontal scroll bar and playhead are in, with the gestures `docs/spec/playlist.md` now specifies. Still missing:
   - the switcher is a chevron button in the header's tool group, not a chevron at the end of the title;
   - no vertical scroll bar or value zoom (the value axis always shows 0–100 %);
   - the window menu has no curve files, Scale values, Thin out, Smooth, Curve from audio file or the View options;
   - Middle-click to remove a point from the selection is not bound (Shift+click toggles instead).
5. **Automation clips in the playlist**: done. The body is darker than a pattern clip's, the curve has a fill, points and handles show while the pointer is over the body, and the point gestures work in place (`crates/musiccraft-ui/src/playlist.rs`, `clip_curve`). There is no separate "automation focus" switch: points show on hover only.
6. **No Speed, LFO mode or multiple targets on automation clips**. `Automation` now has On, Low and High (`crates/musiccraft-project/src/model.rs`, `automation.set_enabled`, `automation.set_range`). Speed, LFO and targets need spec and model work first. The separate controllers in the browser (`crates/musiccraft-ui/src/controllers.rs`, `docs/spec/controllers.md`) already cover LFOs and could share widgets with the LFO footer.
7. **No project info page**. The Project tab shows name, tempo, master pitch, pan law, path and counts (`crates/musiccraft-ui/src/settings.rs`, `project`). Author, genre, comments, link, start date and working time do not exist in the project model.
8. **Sample editor frame is a waveform plus wrapping text buttons** (`crates/musiccraft-ui/src/sample_editor.rs`, `strip` and `show`). It has:
   - no tool row with grouped icons;
   - no info row in a display well;
   - no zoom or overview strip (the waveform always shows the whole sample in a short fixed-height strip);
   - no play, playhead or clipboard;
   - no right-click menu.
9. **Confirmation boxes**: only "Unsaved changes" exists (`crates/musiccraft-app/src/main.rs`, the unsaved-changes window). Its buttons are already named by action ("Save", "Don't save", "Cancel"), which matches. Save has the default-button edge, Enter saves and Esc cancels; it still lacks the glyph (there is no warning icon yet). Destructive actions elsewhere (remove track, delete pattern) run at once and rely on undo, which is acceptable. Only add boxes where undo cannot help.
10. **No progress display anywhere**. Exports and rescans block silently. The hint bar has no progress line (`crates/musiccraft-editors/src/hint.rs`; the hint bar itself is drawn in `crates/musiccraft-ui/src/toolbar.rs`).
11. **Prompts and canvases carry hard-coded sizes** (`set_min_width` in `dialogs.rs`, `default_size` in `settings.rs`, the `GRAPH_HEIGHT` constant in `automation.rs`). New sizes should come from `docs/spec/window-layout.md` once its secondary-window entries are written.
12. **No shared section-box rule yet.** This section, the controls section and the plugin-windows section must draw section and group boxes the same way (darker box, small heading inside its top-left corner). The design-language section settles the one rule; until then nothing in code changes for it.

## Sources

- **Manual pages compared** (text and screenshots, layout and behaviour only; nothing copied):
  - `automation_eventeditor.htm`: "Event Editor": "Controls" (toolbar icons, target), "Event Editor Facts", "Waveform Helper View", "Working with Event Automation data", "Event Editor Menu" (File, Edit, Tools, View, Snap, Zoom, Target control, Relocate Events, Auto Smoothing). Also the screenshot of the convert box.
  - `playlist_automationclip.htm`: "Automation Clips", "Automation Clip Controls", "Working with Automation Clips" (points, tension handles, curve types, step and slide), "Automation Clip Channel Settings", "Options Menu", "Target Links Panel", "Settings" (min, max, time, LFO).
  - `envsettings_midi.htm`: "System Settings - MIDI" (output, input, miscellaneous MIDI options, external sync).
  - `envsettings_audio.htm`: "System Settings - Audio" (input/output, buffer, audio processing thread, playback tracking, CPU, mixer).
  - `envsettings_general.htm`: "System Settings - General" (Display, Language, GUI Input, Undo History, Warnings).
  - `envsettings_files.htm`: "System Settings - Files" (Backup, Plugins, Browser extra search folders, User data folder, External tools, Troubleshooting).
  - `envsettings_themes.htm`, `envsettings_about.htm`, `envsettings_debug.htm`: tab set and page purpose only.
  - `songsettings_settings.htm`: "Project General Settings" (data folder, time settings, advanced, playlist).
  - `songsettings_songinfo.htm`: "Project Info".
  - `fformats_save_export.htm`: "Export Project Dialog" (project type, output format, quality, miscellaneous options, rendering buttons).
  - `menu_file.htm`, `menu_options.htm`, `menu_tools.htm`: where the windows are opened from.
  - `basics_interface.htm`: overall feel; the favourite-folders control of the file dialogs (noted only to say native dialogs have no counterpart).
  - (`eventeditor.htm` returns "not found"; the event editor page is `automation_eventeditor.htm`.)
- **Screenshots viewed**: `automation_eventeditor.png` (header order with snap as an icon button before the tools, overview strip with chevrons and a vertical zoom box at its right end, thin ruler with a playhead marker, values as a filled area, vertical scroll bar), `eventeditor_convert.png`, `chansettings_autoclip.png`, `playlist_automation_edit.png`, `settings_system_audio_asio.png` (title with page caption, grouped tab row with tabs sized to their labels, section boxes with top-left headings, control-then-label rows, status lines, the lighter buffer band), `settings_system_midi.png`, `settings_system_general.png`, `settings_system_files.png`, `settings_song_project.png`, `settings_song_projectinfo.png`, `fformats_wavmidimp3.png` (export dialog section order, format toggles row, two-column options, progress bar and button row placement), `edison2.png` (sample editor frame: tool rows, info row, waveform, overview strip, footer). We used them only for placement, order, grouping, proportions and density; nothing was measured, traced or colour-sampled, and no drawing (tab ends, progress-bar ends, icons) is reproduced.
- **Repository**:
  - `crates/musiccraft-ui/src/settings.rs` (`Tab`, `show`, `row`, `note`, `project`), `dialogs.rs` (`Dialog`, `Purpose::Export`, format chips, `FilePick`), `sample_slot.rs`; `crates/musiccraft-editors/src/sounds.rs` (the sample slot menu); `crates/musiccraft-file-dialog`; `automation.rs` (`show`, `canvas`, `HEIGHT`, shape menu), `controllers.rs`, `sample_editor.rs` (`strip`, `show`), `playlist.rs`, `icons.rs`, `chrome.rs` (window header), `toolbar.rs` (`help_menu`, hint bar);
  - `crates/musiccraft-ui/keymap/default.keymap` and `docs/spec/keymap.md` (clash check: Ctrl+M and Ctrl+I are global, so the event editor's keys are window-local);
  - `crates/musiccraft-app/src/main.rs` (unsaved-changes box);
  - `crates/musiccraft-project/src/model.rs` (`Automation`, `Curve`);
  - `crates/musiccraft-editors/src/theme.rs`, `hint.rs`;
  - `docs/spec/settings.md`, `recording.md`, `controllers.md`, `sample-editor.md`, `window-layout.md`, `playlist.md` (automation editor gestures);
  - `docs/screenshots/playlist-mixer.png`.
- **Other sections of this guide referred to**: main-window (shared header, its glyphs, the zoom-box glyph, `HEADER`, `HEADER_ACTIVE`, `WINDOW_EDGE`), toolbar (`Icon::Snap`, `LCD_TEXT`, snap list), controls (number box, value formats, control menu), plugin-windows (group boxes), playlist and piano-roll (`BAR_GROUP`, `SELECTION_FILL`, `CLIP_BODY`, snap list), design-language (the button rest rule and the section-box rule, which it owns).
- **Own design**:
  - all colour roles and the proposed token `AUTOMATION_FILL`;
  - all glyphs and all labels and wording (tab captions, section names, shape names Curve, S-curve, Hold, Steps, tool name "Smooth", button names, every option label including the MIDI, audio, project-info and export toggles);
  - the plain rectangular progress fill with one centred readout, and the export dialog staying open when done;
  - the single "Also export the master" choice under stems;
  - window-local keys for the event editor;
  - the native file dialogs;
  - the sample slot menu (Browse files…, the built-in sounds and browser folders as submenus, Clear) and dropping browser sounds on slots;
  - the hint-bar progress line;
  - the rule that message boxes are only for decisions;
  - action-named buttons;
  - the dimmed-not-hidden format settings;
  - the export dialog's opening order with the native save dialog;
  - the event editor's empty-lane text;
  - placing the sample editor frame inside the channel window;
  - the settings window's tab grouping and our tab set (no account, debug or theme pages for now);
  - all number formats.
