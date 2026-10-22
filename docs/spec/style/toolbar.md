# Style guide: Toolbar, main menu, transport and hint bar

Part of the [interface style guide](README.md). Read it with the shared rules in [design language](design-language.md), [controls](controls.md) and the window header in [main window](main-window.md).

The top band is the cockpit of the whole program. Everything global lives there: the menus, the hint bar that explains whatever the pointer is over, the transport, the tempo and position readouts, the recording switches, the output and load monitors, the pattern selector and the buttons that open the main windows. It should feel like an instrument panel, not a desktop toolbar: dark chrome packed into two thin rows of same-size small buttons, small recessed "display" wells whose bright digits are the brightest thing in the band, almost no padding, and every control explaining itself in the hint bar on hover. The reference manual's default arrangement is our target for **layout, order, proportions and density**. Every colour, glyph, font, button drawing and word is MusicCraft's own. Where this section changes behaviour we already ship (a different mouse button, a removed or renamed control, a new gesture or shortcut), the change is marked **Proposed (needs `<spec>.md` updated first)**, and today's behaviour stays the default until that spec changes. Additions that replace nothing (new panels, new menu items, new right-click menus) are targets, and they too go into the specs before the code (see the gaps list).

## Placement and proportions

- **Position.** The band runs across the full width of the main window at its very top. It sits above both the browser column and the workspace, and nothing sits beside it. It is docked and cannot be detached. It never floats over content.
- **Height.** The band is **two control rows** ("racks") plus a thin margin above and below. That is about a twelfth of the height of a laptop-size window, roughly twice as tall as MusicCraft's band today. The height is fixed: when the window grows, only the width changes. Each row is exactly one toolbar button tall. The button size is ours, see `docs/spec/window-layout.md` ("Density").
- **Left column.** At the far left, a column holds the **menu bar** in the upper row over the **hint field** in the lower row. Both are the same width, about a fifth of the band at a typical window width. Our `MENU_COLUMN` constant already gives the right proportion. The column reads as one recessed block split into two lines.
- **Two racks.** Right of the left column, the band is two independent left-to-right strips.
  - **Upper rack:** option toggles, then the time display, the output visualiser, the performance panel and the utility shortcuts.
  - **Lower rack:** master knobs, then the transport, tempo, global snap, the pattern panel and the window buttons.
  - Items pack to the left. A **flexible space** at the right end of each rack takes up the spare width. Anything placed after that space (a user layout may put window buttons there) stays anchored to the right edge as the window is resized.
- **Relative widths.** One unit is one toolbar button.

  | Item | Width |
  | --- | --- |
  | Each option toggle, transport button, window button and master knob | 1 unit |
  | Mode switch (two labels stacked) | 1 unit |
  | Tempo display | about 2 units |
  | Global snap (glyph plus dropdown) | about 2½ units |
  | Time display | about 4 units |
  | Performance panel | about 4 units |
  | Output visualiser | about 5 units (scope about half, peak bars about half) |
  | Pattern panel | about 5 units (chevron ½, name field 4, plus ½) |
- **Spacing.** Buttons within a group nearly touch, with only a sliver of a gap. Between groups the gap is slightly larger and holds an optional thin vertical **divider** line (on by default; it can be turned off from the toolbar context menu). There are no labels above or below buttons. The only captions are the tiny ones inside the display wells.
- **Hiding.** Ctrl+F11, or View → Toolbar → *Hide toolbar*, removes the whole band so the workspace gets the room. The same chord brings it back (a new shortcut, see gap 18).
- **Floating hint panel.** This optional overlay (View → Toolbar → *Show floating hints*) sits in the lower-left corner of the main window, over the workspace. It is wider than the docked hint field, about two text lines tall, and uses larger text. Its position is remembered. See Anatomy H.
- **Customise tray.** While the layout is being customised, a full-width tray opens directly under the band, over the top of the workspace and about three button rows tall. It closes again when customising ends. See Anatomy G.
- **Reflow at narrow widths.** This is our own design and extends `docs/spec/window-layout.md`. As the window narrows, space is given up in this order:
  1. The flexible spaces at the right ends of the racks shrink to nothing.
  2. Fixed spaces between groups shrink to the minimum sliver.
  3. Resizable panels shrink to their minimum widths:
     - The hint field shrinks to about three quarters of its width.
     - The output visualiser drops its scope and keeps only the peak bars.
     - The pattern name field shortens and ends long names with an ellipsis.
     - The performance panel shows only the CPU figure.
  4. Low-priority items hide, in this order: optional items, then the utility shortcuts, then the performance panel, then the output visualiser. Finally the option toggles fold behind a **"more" chevron** at the end of their group. The chevron opens a dropdown listing them as check items.
  5. Below the one-band width in `docs/spec/window-layout.md`, the existing stacked fallback applies. The first band holds the menus, the hint field and the window buttons, and the second band holds the transport row.

  Never hidden at any width: the menus, the hint field (at its minimum), the mode switch, play, stop, record, tempo, the time display, the pattern panel and the window buttons.

## Anatomy

### A. Band as a whole
- **Background:** one flat chrome surface. Display wells and the menu/hint field are recessed into it.
- **Right-click on empty band area** (the background, a gap or a divider) opens the toolbar context menu (G).
- **Dragging onto a window button:** while dragging something from the browser (a sample, a preset), hovering it over a window button for a moment brings that window to the front so the drop can continue there.
- **Rack grips:** each rack starts with a narrow grip, visible only while customising. Dragging a grip swaps the two racks or moves a rack's start.

### B. Left column

**B1. Menu bar (upper row).** Eight top-level menus, set close together inside one recessed field the width of the hint field, in this order: file, edit, add, patterns, view, options, tools, help. Today their titles are `FILE · EDIT · ADD · PATTERNS · VIEW · OPTIONS · TOOLS · HELP` in capitals; a rename to sentence case is proposed in Labels and wording.
- **Click** a title to open its dropdown. While a menu is open, **hovering** another title switches to it.
- **Esc** or a click outside closes the menu.
- **Items:** each shows its shortcut right-aligned and dimmed. Toggles show a check mark. Groups are separated by thin lines, and a group may start with a small dim heading (own touch).
- **Right-clicking a check item** flips it without closing the menu, so several switches can be set in one visit (own design).
- Full menu contents are in F.

**B2. Hint field (lower row).** A recessed field the same width as the menu bar, holding two short text lines.
- **Line 1 (dim):** the project name, with a small unsaved-changes marker after it when there are unsaved edits.
- **Line 2 (normal):** one of the following, in priority order:
  1. While a control is being **adjusted**, its live value, for example `Tempo: 128.500 BPM` or `Master volume: -3.2 dB`.
  2. While a control or meter is **hovered**, what it does (or a meter's reading at the pointer).
  3. Otherwise, the last status message. Errors are shown in the danger role.
- **Right-click hint:** when the hovered control has a right-click menu, line 2 ends with the text `· right-click for options`. There is no glyph for this.
- **Status markers (optional, at the right of line 2):** a small glyph naming what kind of thing the hint is about. The planned kinds are: can be automated, can be linked to a controller, success, failure, warning, long job running, seeking, and grouped items. Each uses one of our own glyphs (see Icons). Only the set of kinds is taken from the reference; the glyphs and roles are ours.
- **MIDI input LED:** a small round LED at the right end of the field. It blinks when MIDI arrives from a controller or MIDI input and shows which of four cases happened:
  - nothing received (idle);
  - the input was taken by a link stored in this project;
  - the input was taken by a link stored globally (for every project);
  - the input arrived but nothing used it.

  The token for each case is in Colour roles.
- **MIDI clock LED (future, with *Send MIDI clock*):** a second small LED beside the first. It blinks on every beat and flashes in a stronger role on the first beat of each bar. It only runs while MIDI clock output is on, and is dim otherwise.
- **Right-click** on the field: *Copy text* (copies the value or hint shown).
- **Long text** ends with an ellipsis and never wraps.
- **Hints off:** with View → Toolbar → *Show hint bar* unchecked, line 2 shows only status messages, and hover text becomes ordinary tooltips (as `hint.rs` already does).

### C. Upper rack (left to right)

**C1. Option toggles.** A row of latching toggle buttons in a single group. Each is lit while on.
- **Click** toggles the option.
- **Hover** shows its name, state and shortcut in the hint bar.
- **Right-click** opens the option's small menu, where it has one.

The order is:
1. **Metronome** (Ctrl+M). Right-click: click sound (built-in choices) and click volume.
2. **Wait for input** (Ctrl+I). Playback waits for the first played note. Right-click: *Only while recording is armed*, and *Only for the kinds of input being recorded* (follows "What to record", D2).
3. **Count-in** (Ctrl+P). Right-click: number of bars (1, 2, 4).
4. **Blend recording**. On: recorded notes are added to what is there (blend). Off: they replace it (replace). This maps to our `record.set_mode`.
5. **Loop recording**. Recording keeps looping the pattern or the loop region.
6. **Typing keyboard** (Ctrl+T). Right-click: octave up and down, and *Follow the selected channel's root note*.
7. **Auto-scroll**. Editors keep the playhead in view. Right-click: *Keep playhead centred*, where the view moves under a fixed playhead.
8. **Grouping**. Edits act on whole note and clip groups. Add this only when grouping exists. Until then the slot is absent and nothing else moves into it.
9. **Step edit** (Ctrl+E).
10. **Controller learn** (future). Arms "move a control, then move a hardware knob, to connect them". It carries a tiny dropdown chevron on its right edge for its menu: *Learn*, *Create automation clip*, *Cancel*.

**C2. Time display** (song position). A recessed display well about four units wide and one row tall.
- **Digits:** large monospace digits. The least-significant field is drawn smaller than the rest, so the eye reads bars first. A tiny mode caption sits in the top-right corner.
- **What it shows:** the playhead of what is playing, meaning the pattern position in pattern mode and the song position in song mode. While stopped, it shows where Play will start.
- **Modes:**
  - **Bars:** `bar.beat.step`, the default. The reference's bar mode has a switch for whether its middle field counts beats or steps; ours already shows both as separate fields, so no switch is needed.
  - **Time:** `minutes:seconds.hundredths`.
  - **Bars with ticks** (optional): `bar.beat.tick` for fine work.
- **Click** cycles bars and time, as today. **Right-click** opens a radio list of the modes.
- **Hover:** the hint shows the position in the other mode. No drag; the display never seeks by accident (own design). Seeking from the band is the job of the song position slider (E).

**C3. Output visualiser.** One recessed well about five units wide, split into two parts.
- **Left half: scope.** A live trace of the master output. With *Show both channels* on, left and right are drawn as two overlaid traces of different tone.
- **Right half: peak bars.** Two thin **horizontal** peak bars stacked one above the other (left channel on top). They fill from the left. The clip zone at the far right lights and holds after an over until clicked.
- **Analyser button:** a tiny button under the bars, at the well's bottom-right corner, opens the full spectrum analyser window.
- **Right-click on the scope:** mode radio (*Scope*, *Spectrum*), then the check items *Show both channels* and *Slow sweep* (a scrolling rather than stationary trace).
- **Hover over the bars:** the hint shows left and right peaks in dB.
- **Resizing:** while customising (G), drag its right edge. Outside customising it does not resize.

**C4. Performance panel.** A recessed well about four units wide with two lines.
- **Upper line:** a short CPU load bar at the left with the load percentage right-aligned after it, then memory in use (MB) right-aligned at the far end.
- **Lower line:** a small scrolling CPU-history graph at the left and the **voice count** (sounding voices) at the right with a tiny caption.
- **Double-click** opens the plugin load monitor window, which lists each plugin's share of the processing time.
- **Right-click:** refresh speed (fast, normal, slow), *Show frame rate instead of voices*, and *Copy diagnostics* (audio device, sample rate, buffer, MIDI ports).
- **Hover:** the hint shows load, device, buffer and MIDI ports, as today.

**C5. Utility shortcuts.** A group of non-latching buttons:
1. **Undo** (Ctrl+Z). Right-click opens the undo history list.
2. **Sample editor**. Opens our sample editor on the selected channel. Right-click: open it empty.
3. **Record audio**. One step: arms audio input and starts recording into the playlist.
4. **Help** (F1). Right-click: *About MusicCraft*.
5. **Save** (Ctrl+S). Right-click: *Save as…* and *Save new version*. The button gently pulses in the attention role when the project has unsaved changes and has not been saved for a while (own choice: ten minutes).
6. **Export audio** (Ctrl+R). Right-click: *Export stems…* and *Export MIDI…*.

**C6. Flexible space** to the right end. MusicCraft keeps the system title bar today, so no minimise, maximise or close buttons are drawn here. If we ever draw our own, they sit at this end as the main-window section describes.

### D. Lower rack (left to right)

**D1. Master knobs.** Two small knobs, one unit each, drawn with our own knob widget:
- **Listening volume.** Affects only what you hear. It is never part of an export and cannot be automated, but it is saved per project. **Proposed (needs `transport.md` and `window-layout.md` updated first):** today the toolbar knob sets the master insert volume, which is exported; that stays the default until the spec changes.
- **Master pitch.** Our `project.set_master_pitch`. Per `transport.md` it retunes every built-in instrument, while plugins and MIDI output channels are not retuned. It snaps to whole semitones while dragging, and Shift drags freely. Right-click: *Enter A4 frequency…* (our `project.set_a4`). Open question, our own decision to make in `transport.md`: whether audio clips follow master pitch and whether a channel can opt out. The reference retunes audio clips too and asks each channel to opt in.

Both follow `docs/spec/controls.md`: left-drag, scroll, double-click or middle-click to reset, right-click for the menu. While adjusting, the hint shows `Listening volume: -6.0 dB` or `Master pitch: +12 cents (A4 = 443.0 Hz)`.

**D2. Transport group.** These sit close together as one group.
1. **Mode switch.** One unit wide, with its two labels **stacked vertically** (pattern on top, song below). The selected label is lit and the other is dim. Today the two labels sit side by side, two buttons wide (gap 11).
   - **Click** toggles the mode (L).
   - **Right-click the pattern half** opens or focuses the channel rack. **Right-click the song half** opens or focuses the playlist.
2. **Play.** Shows a play triangle while stopped. While playing it is lit and shows two upright bars.
   - **Click** runs `transport.toggle`: play, or stop and rewind. If recording is armed, Play starts the take.
   - **Ctrl+click** pauses (`transport.play_pause`).
3. **Stop.** Play and stop are joined edge to edge with no gap, reading as one pair.
   - **Click** stops and rewinds.
   - **Double-click** is panic: all notes off (`transport.panic`).
   - **Right-click:** check item *Restart from the set position*. When on, after Stop, Play starts again from the position the user last set (for example by clicking the playlist's timeline) instead of from the start.
4. **Record.** Stands a little apart from the pair. It is lit while armed and pulses while a take is actually being recorded.
   - **Click** arms or disarms (R).
   - **Alt+click while disarmed** opens a small "What to record" popup and arms recording.
   - **Right-click** opens the recording menu. Its sections:
     - **What to record:** notes, automation (knob movements), audio, and clips launched live (future, once live clip launching exists).
     - **Options:** *Start playing when recording starts*, *Turn recording off when stopped*, *Record only between the record markers* (future: the playlist's start and end recording markers bound the take), *Count-in*, *Wait for input*, *Loop recording*, *Step edit*.
     - **Quantize while recording:** *Note starts*, *Note ends*, *Keep note lengths* (snapped to the global snap), *Notes as steps* (played notes land in the step sequencer as steps), *Automation*.
     - **Mode:** *Blend*, *Replace* (as today).

Button shapes are our own rounded-rectangle buttons. Grouping and order are what we take from the manual, not its button outlines.

**D3. Tempo display.** A recessed well about two units wide.
- **Digits:** the whole-BPM digits are large. The fractional part, to three decimals, is smaller and slightly dimmer to their right. A tiny pair of stacked up and down chevrons sits at the right edge.
- **Range:** 10–999 BPM, as in `transport.md` (our own range).
- **Left-drag up or down:**
  - on the whole digits: steps of 1 BPM;
  - on the decimals: steps of 0.001 BPM;
  - with Shift on either: fine steps.
- **Scroll** steps 1 BPM, or fine steps with Shift.
- **Click the upper or lower chevron:** +1 or −1 (or ±0.001 when the decimals were last dragged).
- **Double-click** opens an inline type-in field. Enter applies, Esc cancels.
- **Right-click:** *Enter a tempo…*, *Tap tempo* (opens the tap tempo window), a short list of round tempos (own list: 60, 90, 120, 140, 174), *Play at half speed (preview only)* (a temporary listening aid; the display does not change), *Create automation clip*, *Add controller…*, *Copy*, *Paste*, *Reset to 130*.
- **Hover** shows `Tempo 128.000 BPM — drag whole digits or decimals; double-click to type`.
- **Our additions:** right after the well come two small buttons, **TAP** and the **time signature** (for example `4/4`, opening our signature picker). Both behave as today.

**D4. Global snap.** A small snap glyph followed by a dropdown about two units wide, showing the current value.
- **Values:** our `Snap` list today (None, 1/4 step, 1/2 step, Step, Beat, Bar). The reference's list is longer (snap to the nearest grid line, snap to the start of the grid cell, step and beat fractions down to sixths, including thirds for triplets); the missing values are gap 9.
- **Effect:** it sets the snap of every editor whose own snap is set to **Global**. A new *Global* entry at the top of each editor's snap list is the default there. It also sets the quantize grid for recording.
- **Click** opens the list. **Scroll** steps through values. **Right-click:** *Use as default length for new notes*.

**D5. Pattern panel.** Left to right:
1. **Options chevron.** A narrow button with a small down chevron. Click opens the pattern options menu, which has the same items as the patterns menu (F).
2. **Name field.** About four units wide.
   - **Shows:** the pattern's number and name in the pattern's own colour (`pattern_text`), plus a tiny stacked up/down chevron pair at its right edge (own design: the reference uses a separate up/down selector for this).
   - **Click** opens the list of all patterns, as the combo box does today. Choosing one selects it.
   - **Proposed (needs `window-layout.md`, or a new `toolbar.md`, updated first):** follow the reference's assignment instead: click selects the pattern and reveals it in the playlist, and right-click opens the list of all patterns. Until then, right-click also opens the list, and Ctrl+click selects and reveals.
   - **Scroll** or the chevrons step to the next or previous pattern (keyboard Plus and Minus).
   - **Double-click** renames the pattern (F2).
3. **Plus button.** Finds the next empty pattern and names it (F4, as today). Right-click: *First empty pattern* (Shift+F4).

**D6. Window buttons.** Latching buttons that show or hide the main windows. Left to right:
1. **Playlist** (F5). Right-click: open maximised.
2. **Channel rack** (F6).
3. **Piano roll** (F7). Right-click: open maximised.
4. **Browser** (Alt+F8). Right-click: open the plugins window.
5. **Mixer** (F9).
6. **Plugins** (F8). Opens a window listing every instrument and effect, which can be dragged into the project.
7. **Tap tempo**. Opens the tap tempo window: a large tap pad with a bar that fills as enough taps are counted; two hold-to-nudge buttons that briefly speed up or slow down playback to line up with another source and return to the set tempo on release; a switch that makes each tap reset the downbeat and realign audio clips that are locked to the grid (off by default); and a switch that silences the project when tapping starts and fades it back in over the next few taps. Today's TAP rule (`transport.md`) stays as the tap logic. The window's full layout belongs to the dialogs section.
8. **Window layout**. Carries a tiny down chevron. Click opens the layout menu (the same as View → *Windows layout*).

Behaviour of every window button:
- **Click** shows the window, or hides it if it is already open and in front. Clicking a window that is open but behind others brings it to the front.
- **Open state:** lit glyph plus underline. **Hover:** fill.
- **Dragging onto a button** brings that window forward (A).

**D7. Flexible space** to the right end. The reference uses this slot for online content, which MusicCraft leaves empty.

### E. Optional items

These are hidden by default and added while customising:
- **Song position slider:** a thin horizontal slider across the pattern or song length. Drag to seek. The reference treats it as a standard panel; hiding it by default is our own choice, since the time display and the editors' timelines already show and set the position.
- **Chord display:** names the chord being played live or selected in the piano roll. Right-click sets how long it lingers: none, 1, 3 or 5 seconds, or until the next chord.
- **Add button:** a text button reading `Add ▸` that opens the add menu.
- **Modifier keys** Ctrl, Alt and Shift as latching text buttons, for touch screens.
- **Copy and paste** buttons.
- **Last moved control:** opens a large knob for the control moved last.
- **Project info** (F11), **Settings** (F10), **Panic**, and **Spectrum analyser**.

### F. Main menu contents

Grouped by function and worded in our own terms. ✓ means it exists today. Online store, account, cloud, news and tutorial-video items have no place here. Shortcuts that are not in `crates/musiccraft-ui/keymap/default.keymap` today are proposed new bindings (gap 18); none of them clashes with an existing binding.

| Menu | Items (groups separated by "—") |
| --- | --- |
| **File** | New ✓ · New from template ▸ · Open… (Ctrl+O) ✓ · Open recent ▸ (last ten, *More* ▸ up to fifty; Alt+1…Alt+0 open the first ten) · Save (Ctrl+S) ✓ · Save as… (Ctrl+Shift+S) ✓ · Save new version (Ctrl+N) ✓ · Save as template… — Import ▸ (MIDI file…, project from another program…) · Export ▸ (Audio, WAV/FLAC… (Ctrl+R) ✓, Stems: one file per playlist track… ✓, MIDI file… (Ctrl+Shift+M) ✓, Project and samples as a zip…, Collect samples into the project folder) — Recover unsaved work ✓ · Revert to last backup — Exit ✓ |
| **Edit** | Undo *‹action›* (Ctrl+Z) ✓ · Redo (Ctrl+Shift+Z, Ctrl+Y) ✓ · Undo history… — Cut (Ctrl+X) · Copy (Ctrl+C) · Paste (Ctrl+V). These act on the selected channels in the channel rack, or on the focused editor's selection. |
| **Add** | Channel ▸ (built-in instruments) ✓ · Plugin instrument ▸ ✓ · Effect ▸ (into the selected mixer insert's next free slot) — Plugins window (F8) · Show plugins in the browser · Rescan plugins — Automation for the last moved control — Pattern ✓ · Mixer insert ✓ |
| **Patterns** | First empty pattern (Shift+F4) · Next empty pattern (F4) ✓ · Next empty pattern, keep its name — Reveal in playlist · Rename and colour… (F2; pressing F2 again cycles the colour) ✓ (rename only) · Random colour · Time signature… · Transpose… — Insert empty pattern before · Clone ✓ · Delete ✓ · Move up · Move down — One pattern per channel · Render to audio clip ▸ (Quick, With settings…, Render and replace clips) — Next pattern (Plus) ✓ · Previous pattern (Minus) ✓ — numbered list of all patterns ✓ |
| **View** | **Windows:** Playlist (F5) ✓ · Piano roll (F7) ✓ · Channel rack (F6) ✓ · Mixer (F9) ✓ · Browser (Alt+F8) ✓ · Plugins (F8) · Tap tempo · Spectrum analyser ✓ · Plugin load monitor · Keyboard shortcuts ✓ — **Toolbar ▸** Customise toolbar… · Show dividers · Show hint bar ✓ (today `Hint bar`) · Show floating hints · Hide toolbar (Ctrl+F11) — **Windows layout ▸** Hide every window (F12) · Hide effect and plugin windows (Alt+F12) · Hide all but the focused window (Ctrl+F12) · Line up instrument windows (Shift+F12) · Reset to the default layout (Ctrl+Shift+H) · Bring every window into view · Restore my layout · Save my layout |
| **Options** | **Settings:** Audio settings (F10) ✓ · MIDI settings ✓ · General settings ✓ · Folders and plugins ✓ — **Project:** Project info (F11) ✓ · Project settings (tempo, signature, master pitch) — **MIDI:** Enable controller input · Enable MIDI output · Send MIDI clock — **Switches:** Play notes from the computer keyboard (Ctrl+T) ✓ (today `Typing keyboard to piano`) · Metronome (Ctrl+M) ✓ · Count-in (Ctrl+P) · Wait for input (Ctrl+I) · Blend recorded notes · Step edit (Ctrl+E) · Loop recording · Record audio input (moved here from File) · Auto-scroll · Grouping (future) · Controller learn (future) |
| **Tools** | Find in browser (Ctrl+F) · Record audio into the playlist — Panic ▸ (All notes off (Ctrl+H), Cancel recording) — Clean up ▸ (Select channels with no notes, Remove unused audio clips, Reset empty playlist tracks, Remove unused files and save, Send channels to MIDI output (for MIDI export)) — Quantize selected channel ✓ · Rescan plugins ✓ · Reload samples ✓ — Last moved control ▸ (Create automation clip, Add controller…, Copy, Paste) — Played-notes log ▸ (Put the last 2 / 5 / 10 / 20 / 30 minutes of played notes into the selected pattern, Clear log) |
| **Help** | User guide (F1) · Keyboard shortcuts ✓ · What's new — Diagnostics ▸ (Reset settings to defaults, Reset interface scale, Copy system info) — About MusicCraft ✓ |

Every item runs a command or a `view.*` / `ui.*` invocation, and every shortcut lives in the keymap (`docs/spec/keymap.md`). The *Windows layout* name and item list here are the ones the main-window section should use too (gap 19).

### G. Toolbar context menu and customising
- **Right-click** an empty part of the band to open the context menu (our items and order):
  - *Customise toolbar…* (check item while the tray is open)
  - *Show dividers* (check item)
  - *Plain buttons (no shading)* (check item; drops the subtle raised shading)
  - *Show hint bar* (check item)
  - *Show floating hints* (check item)
  - *Toolbar layouts* ▸, with our own layout names:
    - *Default (two rows)*
    - *Compact (one row)*
    - *Transport only*
    - user layouts, plus *Last used* (saved automatically)
  - *Hide toolbar* (Ctrl+F11)
- **Customising:**
  - **Tray:** a tray opens under the band. It holds every available item as a captioned tile in a grid: panels, buttons, the optional items, and a **Space** item. Items already in the band are shown dimmed in the tray.
  - **Add, move, remove:** drag a tile into the band to add it, drag within the band to move it, and drag an item out of the band (anywhere into the tray) to remove it.
  - **Spaces and resizing:** a Space has a handle on its right edge to set its width. Resizable panels (hint field, output visualiser, pattern name) also show a right-edge handle. Edges snap to whole units, and Shift disables snapping.
  - **Flexible space:** the right-most Space in a rack is the flexible one.
  - **Tray footer:** a one-line hint ("Drag items between the toolbar and this tray"), a *Toolbar layouts* menu (*Save layout as…*, *Delete layout*, the layout list) at its right, and a close cross.
  - **Leaving:** closing the tray or unchecking *Customise toolbar…* ends customising. The layout is remembered per user, not per project.

### H. Floating hint panel
- **Off by default.** Turn it on from View → Toolbar or the toolbar context menu (*Show floating hints*).
- **Content:** it shows the same text as hint line 2, in larger type.
- **Click-through:** the mouse passes through it everywhere except its handle.
- **Handle:** at its right end, a ring fills clockwise to show the progress of long jobs (rendering, scanning, loading). A move handle sits in the ring's centre.
- **Moving:** hover the centre and the pointer becomes a move cursor; drag to move the panel. The position is remembered.

## Icons

All glyphs are vector drawings in the style of `crates/musiccraft-ui/src/icons.rs`: drawn in a box inset by about a sixth, with a single thin stroke and simple fills. "Existing" means the `Icon` variant is already drawn. None is modelled on the reference's button drawings.

| Where | What it does | Our glyph |
| --- | --- | --- |
| Option toggle 1 | Metronome | Existing `Icon::Metronome`: an outlined trapezoid body with a slanted arm line from near the base to the upper right. |
| Option toggle 2 | Wait for input | New `Icon::Wait`: an hourglass, two filled triangles meeting tip to tip between a short top bar and a short bottom bar. |
| Option toggle 3 | Count-in | New `Icon::CountIn`: three short vertical ticks rising in height left to right, followed by a small filled play triangle. |
| Option toggle 4 | Blend recording | New `Icon::Blend`: two rounded rectangles offset diagonally, the back one outlined and the front one filled. |
| Option toggle 5 | Loop recording | New `Icon::LoopRecord`: a three-quarter circular arc ending in an arrowhead, around a small filled dot. |
| Option toggle 6 | Typing keyboard | Existing `Icon::Keyboard`. |
| Option toggle 7 | Auto-scroll | New `Icon::Follow`: a vertical playhead line with a small downward triangle cap, and two short horizontal motion lines to its left. |
| Option toggle 8 | Grouping (future) | New `Icon::Group`: two small filled squares inside one dashed rounded outline. |
| Option toggle 9 | Step edit | New `Icon::StepEdit`: a short filled note bar followed by a right-pointing chevron. |
| Option toggle 10 | Controller learn (future) | New `Icon::Link`: a small knob (circle outline with a pointer line) joined by a short line to a small filled square. A tiny down chevron (two short strokes meeting at a point) sits at the button's right edge. |
| Output visualiser corner | Open the spectrum analyser | New `Icon::Expand`: a short diagonal arrow pointing to the upper right. |
| Utility 1 | Undo (right-click: history) | New `Icon::Undo`: a half-circle arc ending in an arrowhead that points left. |
| Utility 2 | Sample editor | New `Icon::Wave`: a horizontal centre line crossed by five vertical bars of varying height. |
| Utility 3 | Record audio | New `Icon::RecordAudio`: three waveform bars with a small filled circle at the lower right. |
| Utility 4 | Help | New `Icon::Help`: a circle outline with a question mark inside. |
| Utility 5 | Save | New `Icon::Save`: a downward arrow ending above an open tray (a U-shaped line). |
| Utility 6 | Export audio | New `Icon::Export`: three waveform bars followed by a right-pointing arrow. |
| Transport | Play / playing | Existing `Icon::Play` (filled triangle). While playing it becomes new `Icon::Pause`: two upright filled bars. |
| Transport | Stop | Existing `Icon::Stop` (filled square). |
| Transport | Record | Existing `Icon::Record` (filled circle). |
| Tempo / pattern name | Step up / down | A tiny pair of stacked chevrons (stroke), one pointing up and one down. |
| Global snap | Snap value | New `Icon::Snap`: two short lines meeting at a right angle (a grid corner) with a filled dot sitting in the corner. |
| Pattern panel | Pattern options | A small down chevron: two short strokes meeting at a point. |
| Pattern panel | Next empty pattern | A plus. |
| Window buttons | Playlist, channel rack, piano roll, browser, mixer | Existing `Icon::Playlist`, `Icon::ChannelRack`, `Icon::PianoRoll`, `Icon::Browser`, `Icon::Mixer`. |
| Window buttons | Plugins window | New `Icon::Plugins`: a 2 × 2 grid of rounded squares, three filled and one outlined. |
| Window buttons | Tap tempo window | New `Icon::Tap`: a filled rounded square (a pad) with two concentric arcs above it. |
| Window buttons | Windows layout menu | New `Icon::Layout`: a rectangle outline split into a narrow left column and two stacked right panes, with a tiny down chevron beside it. |
| Hint field | MIDI input LED, MIDI clock LED | A small filled circle each. |
| Hint field (optional) | Status markers | Can be automated: a short rising curve ending in a filled dot. Can be linked: a tiny `Icon::Link`. Success: a check mark. Failure: a cross. Warning: an exclamation mark in a triangle outline. Long job: a circle outline with two clock hands. Seeking: a pair of chevrons pointing in the seek direction. Grouped: a tiny `Icon::Group`. |
| Floating hint panel | Progress and move handle | An arc ring that fills clockwise, with a small four-way arrow cross in its centre. |
| Rack start (customising) | Grip | Three short stacked horizontal lines. |
| Optional items | Settings, Project info, Panic, Spectrum | A gear; a circle with a lowercase "i"; a circle outline with an exclamation mark; four vertical bars falling in height. |

## Colour roles

| Element | Token(s) | Relative tone and state changes |
| --- | --- | --- |
| Band background | `theme::PANEL` | One step lighter than the workspace (`theme::BACKGROUND`). It matches window headers and the design-language section, so the band reads as the same chrome. (The main-window draft says `theme::PANEL_DARK`; see gap 17.) |
| Menu/hint recessed field | `theme::PANEL_DARK` fill, `theme::LCD_EDGE` outline | One step darker than the band. |
| Display wells (time, tempo, output, performance) | `theme::LCD_BG` fill, `theme::LCD_EDGE` outline | The darkest chrome in the band. Content inside is the brightest. |
| Display digits | Proposed `theme::LCD_TEXT` | The brightest text tone, one step above `theme::TEXT`. Start it equal to `theme::PLAYHEAD`, which the LCDs use today, so nothing changes visually; only the role gets its own name. |
| Small digits (tempo decimals, least-significant time field) | `theme::TEXT` | One step dimmer than the large digits. |
| Display captions, chevrons | `theme::TEXT_DIM` | Dim. They brighten to `theme::TEXT` on hover. |
| Menu titles | `theme::TEXT` | The open menu's title gets a `theme::RAISED_HOVER` fill. |
| Menu item shortcuts, group headings | `theme::TEXT_DIM` | Dim. |
| Hint line 1 (project name) | `theme::TEXT_DIM` | The unsaved marker uses `theme::HIGHLIGHT`. |
| Hint line 2 | `theme::TEXT` | Live values use `theme::LCD_TEXT`; errors use `theme::DANGER`. |
| Status markers | Success `theme::ACCENT`, failure `theme::DANGER`, warning `theme::HIGHLIGHT`, all others `theme::TEXT_DIM` | Shown only while the hint they belong to is shown. |
| MIDI input LED | Idle `theme::RAISED`; taken by a project link `theme::ACCENT`; taken by a global link `theme::HIGHLIGHT`; received but unused `theme::TEXT_DIM` | Flashes briefly, then fades back to idle. |
| MIDI clock LED | Idle `theme::RAISED`; each beat `theme::TEXT_DIM`; first beat of a bar `theme::ACCENT` | Stays idle while MIDI clock output is off. |
| Buttons, off | `theme::RAISED` fill, glyph `theme::TEXT` | Hover: `theme::RAISED_HOVER` fill. Pressed: `theme::ACCENT_DIM` fill. |
| Option toggles, on | `theme::ACCENT` fill, glyph `theme::BACKGROUND` | All ten toggles use the same on-state. |
| Mode switch | Selected label: `theme::HIGHLIGHT` fill with `theme::BACKGROUND` text. Other label: `theme::TEXT_DIM` on `theme::PANEL_DARK`. | Hover on the dim label raises its text to `theme::TEXT`. |
| Play, playing | `theme::ACCENT` fill, glyph `theme::BACKGROUND` | — |
| Record | Armed: `theme::DANGER` fill. Recording: the fill pulses between `theme::DANGER` and `theme::DANGER` dimmed. | Disarmed: the glyph is `theme::DANGER` on `theme::RAISED`. |
| Stop | `theme::RAISED`, glyph `theme::TEXT` | No on state. |
| Window buttons | Closed: glyph `theme::TEXT_DIM`, no fill. Open: glyph `theme::ACCENT` with a `theme::ACCENT` underline. | Hover: `theme::RAISED_HOVER` fill. A window that is open but behind others gets the underline only. |
| Peak bars | `theme::METER_BG` track; level `theme::ACCENT` → `theme::HIGHLIGHT` → `theme::DANGER` | The clip zone holds `theme::DANGER` until clicked. |
| Scope traces | Left: `theme::ACCENT`. Right: `theme::ACCENT_DIM`. Centre line: `theme::LCD_EDGE`. | When silent, only the centre line shows, so the well never looks like a hole. |
| CPU bar | `theme::ACCENT` / `theme::HIGHLIGHT` / `theme::DANGER` at rising load, as today | The history graph uses `theme::ACCENT_DIM`. |
| Pattern name | The pattern's own colour (`pattern_text`), or `theme::TEXT` | — |
| Dividers | Proposed `theme::SEPARATOR` | The same tone as `theme::LCD_EDGE`: one step lighter than `theme::PANEL`. |
| Save reminder pulse | `theme::HIGHLIGHT` glyph | Pulses slowly. |
| Customise tray | `theme::PANEL_DARK`; tiles `theme::RAISED` | Tiles already in the band are drawn at reduced opacity. |
| Floating hint panel | `theme::PANEL_DARK` at partial opacity; ring track `theme::RAISED`, ring fill `theme::ACCENT` | — |
| Disabled items | `theme::TEXT_DIM` at reduced opacity | — |
| Keyboard focus | A thin `theme::ACCENT` outline | — |

## Labels and wording

- **Menu titles** (existing): `FILE`, `EDIT`, `ADD`, `PATTERNS`, `VIEW`, `OPTIONS`, `TOOLS`, `HELP`. **Proposed rename** (clean-room: the full set in capitals matches the reference's menu bar): sentence case, `File · Edit · Add · Patterns · View · Options · Tools · Help`, matching the sentence-case rule used everywhere else.
- **Mode switch** (existing): `PAT` and `SONG`. **Proposed rename** (clean-room: this caption pair is the reference's): `Pattern` over `Song`, shortened to `Ptn` over `Song` where the stacked one-unit button is too narrow.
- **Typing keyboard** (existing menu label `Typing keyboard to piano`). **Proposed rename** (clean-room): menu `Play notes from the computer keyboard (Ctrl+T)`; short hint label `Computer keyboard as piano`.
- **Hint bar toggle** (existing View-menu label `Hint bar`). **Proposed rename:** `Show hint bar`, to match the other context-menu items.
- **Display captions** (existing, kept): `BAR.BEAT.STEP` and `MIN:SEC` for the time display, `BPM` for tempo, `CPU`. New: `VOICES`, `MB`. In the new corner position, the time caption may shorten to `BARS` / `TIME` (own wording).
- **Small text buttons** (existing): `TAP`, the signature (`4/4`), `+`. New: `Add ▸` (optional item).
- **Window names** for hints and menus: Playlist, Channel rack, Piano roll, Browser, Mixer, Plugins, Tap tempo, Spectrum analyser, Plugin load monitor, Keyboard shortcuts.
- **Recording wording** (existing): Count-in, Wait for input, Loop recording, Step edit, Blend, Replace. New (own wording): "What to record", "Start playing when recording starts", "Turn recording off when stopped", "Record only between the record markers", "Notes as steps", "Restart from the set position".
- **Toolbar menu wording** (new, own): Customise toolbar…, Show dividers, Plain buttons (no shading), Show floating hints, Toolbar layouts, Hide toolbar.
- **Controller wording:** "Controller learn" for the toggle and "Add controller…" in menus, matching `docs/spec/controls.md` and `docs/spec/controllers.md`.
- **Hint style:**
  - Sentence case and an imperative or descriptive phrase, with the shortcut in parentheses at the end, for example `Metronome (Ctrl+M)` or `Next empty pattern (F4)`.
  - When a control has a right-click menu, the hint ends with `· right-click for options`.
  - A latching toggle's hint names its state, for example `Count-in: on (Ctrl+P)`.
  - A value readout is `<control>: <value> <unit>`.
- **Number formats:**
  - Tempo: whole digits, then `.` and three decimals in the small size (`128.000`).
  - Time display, bar mode: `bar.beat.step`, unpadded (`12.3.4`).
  - Time display, time mode: `m:ss.cc` (`2:05.47`).
  - Volume: dB with one decimal and a sign (`-3.2 dB`).
  - Pitch: signed cents (`+12 cents`).
  - CPU: whole percent (`37%`). Memory: whole MB. Voices: a whole number.
  - Pattern field: number, two spaces, name (`3  Lead`).

## States and feedback

- **Hover:** the button fill rises one step (`theme::RAISED_HOVER`) and the glyph brightens to `theme::TEXT`. Hint line 2 updates at once. Hovering a window button or toggle never changes its latched look.
- **Pressed:** while the mouse is down, the fill drops to `theme::ACCENT_DIM`. A button acts on release.
- **On (latched):** solid fill in the role colour with the glyph knocked out to `theme::BACKGROUND`. All option toggles share one on-look. Window buttons instead use a lit glyph plus an underline, which is quieter than a toggle.
- **Selected mode:** the lit half of the mode switch. Exactly one half is always lit.
- **Playing:** the play button is lit and shows the pause glyph. The time display counts. The scope and peak bars move. The MIDI LEDs may flash.
- **Paused:** the play button is not lit and shows the play glyph. The time display holds its position, and its digits dim one step while paused.
- **Count-in or waiting:** the record button pulses. The time display's digits are replaced by a dim `COUNT-IN` or `WAITING`. These words sit beside the record button today and move into the display.
- **Armed:** the record button is filled with `theme::DANGER`. **Recording** (armed and playing): the fill pulses slowly.
- **Step edit on:** the step-edit toggle is lit, and the hint shows `Step edit: notes go in at the playhead`. This replaces today's `STEP` word beside the record button.
- **Muted or silent output:** the peak bars are empty and the scope shows only its centre line.
- **Clipping:** the clip zone of the affected peak bar latches in `theme::DANGER` until clicked. Hovering it shows the peak level.
- **Disabled** (for example Redo with nothing to redo, or Delete pattern with one pattern): `theme::TEXT_DIM` at reduced opacity, with no hover change.
- **Dragging** (a display value, a knob, the hint panel): the pointer becomes a vertical-resize cursor on displays and knobs, and a move cursor on the floating hint handle. The hint shows the live value. One drag is one undo step (gestures are coalesced as today).
- **Drag-over a window button** while dragging content: the button takes the hover fill, then the window comes forward after a short delay.
- **Focus:** a menu opened from the keyboard shows the highlighted item with the `theme::ACCENT_DIM` selection fill. A focused display's type-in field gets a thin `theme::ACCENT` outline.
- **Unsaved changes:** a marker after the project name in hint line 1, and the save button's slow pulse after the reminder interval.
- **Customising:** every band item shows a faint outline. Resizable items show a right-edge handle. Items already in the band are dimmed in the tray.

## MusicCraft today: gaps

Ordered by visual impact. Per `CLAUDE.md`, update the specs (`docs/spec/window-layout.md`, `transport.md`, `recording.md`, `keymap.md`, or a new `docs/spec/toolbar.md`) before the code.

1. **The recording menu is thin.** The option toggles are on the band, but there is no auto-scroll toggle, and the recording menu has no clips kind, no record-marker option, no notes-as-steps option and no automation quantize. Files: `crates/musiccraft-ui/src/toolbar.rs` (`record_options`), `docs/spec/recording.md`.
2. **The window-button set is incomplete.** The buttons sit at the end of the lower rack, followed by the Layout button (default layout and maximizing), and right-click on Playlist and Piano roll opens them maximized. There are no Plugins or Tap tempo buttons (the tap tempo window opens from View, the tempo menu and a right-click on TAP), and the Browser button has no right-click. File: `crates/musiccraft-ui/src/toolbar.rs` (`window_buttons`).
3. **The hint field is partial.** It shows the project name over the hint, with an unsaved dot and right-click Copy, but has no MIDI LEDs, no status markers, no live value while adjusting, no floating hint panel and no progress ring. Files: `crates/musiccraft-ui/src/toolbar.rs` (`hint_field`), `crates/musiccraft-editors/src/hint.rs`.
4. **The output monitor lacks details.** Scope and horizontal peak bars share one well, and the bars hold their peaks and show a clip mark, but there is no right-click mode menu and no separate analyser button (the whole well opens the analyser). File: `crates/musiccraft-ui/src/icons.rs` (`output_monitor`).
5. **Tempo display extras.** Whole digits and decimals drag separately, each wheel notch is 1 BPM, double-click types a tempo, and the right-click menu has Enter a tempo…, round tempos, Copy and Reset. The step chevrons, Paste and Tap tempo are in. Still missing: the half-speed preview and the automation items. The tap tempo window has the pad, the count bar and the readout, but no nudge buttons, realign switch or mute-while-tapping switch. File: `crates/musiccraft-ui/src/toolbar.rs` (`tempo`).
6. **No global snap.** Only the playlist and piano roll have their own snap lists. There is no Global choice and no toolbar snap panel. The `Snap` list has triplet and beat-fraction values (`piano-roll.md`, "Snap values"), but no snap to the nearest grid line and no snap to the start of the grid cell. Files: `crates/musiccraft-ui/src/geometry.rs` (`Snap`), `crates/musiccraft-ui/src/playlist.rs`, `crates/musiccraft-ui/src/piano_roll.rs`.
7. **The pattern panel.** The options chevron, the field with its step arrows, wheel stepping, double-click rename, right-click list and Ctrl+click reveal are in. Still missing: the proposed click-to-reveal assignment and the plus button's right-click (first empty pattern, Shift+F4). File: `crates/musiccraft-ui/src/toolbar.rs` (`pattern_panel`).
8. **Transport details are missing.**
    - Stop panics on double-click, but has no "restart from the set position" option.
    - Record has no Alt+click "What to record" popup and no what-to-record choices or record-quantize options.
    - Files: `crates/musiccraft-ui/src/toolbar.rs`, `docs/spec/transport.md`, `docs/spec/recording.md`.
9. **The toolbar knob changes the master insert volume, which is exported.** The target is a listening-only volume plus a master pitch knob (Proposed, D1). Master pitch is reachable only from Settings → Project today, and `transport.md` does not say whether audio clips follow it. File: `crates/musiccraft-ui/src/toolbar.rs` (master `knob`).
10. **The performance panel is partial.** It shows CPU and the voice count, but there is no history graph, no memory figure and no double-click load monitor. File: `crates/musiccraft-ui/src/icons.rs` (`performance`).
11. **Menu contents are thin or misplaced.**
    - Patterns lacks first empty, reveal, colour, transpose, move, one-pattern-per-channel and render.
    - View lacks the Toolbar and Windows layout submenus and the Plugins window.
    - Options lacks the recording switches.
    - Tools lacks panic, clean-up, last-moved-control and the played-notes log.
    - Help lacks the user guide and What's new.
    - File: `crates/musiccraft-ui/src/toolbar.rs` (`file_menu` … `help_menu`).
12. **No toolbar context menu or customising.** There are no dividers toggle, layouts, hide (Ctrl+F11) or narrow-width reflow beyond the one-band/two-band switch. Files: `crates/musiccraft-ui/src/toolbar.rs`, `docs/spec/window-layout.md`.
13. **Keymap gaps.** The default keymap has no F2, F8, F11, Shift+F4, Ctrl+F11, Shift+F12, Ctrl+X, Ctrl+C, Ctrl+V, Ctrl+F or Alt+1…Alt+0. All are proposed new bindings; none clashes with an existing one. (F12, Alt+F12, Ctrl+F12 and Ctrl+Shift+H are now bound, main-window section.) Files: `crates/musiccraft-ui/keymap/default.keymap`, `docs/spec/keymap.md`.
14. **A shared submenu to agree on.** Menu titles are sentence case, the mode switch reads Ptn over Song, and the typing-keyboard and hint-bar items use our wording. The main-window draft calls the layout submenu "Arrange windows" with different items; both sections should use this section's *Windows layout* name and item list. File: `crates/musiccraft-ui/src/toolbar.rs` (`view_menu`).

## Sources

- **Reference manual** (FL Studio online manual, `https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/`). Compared for layout, order, proportions, density and behaviour only, under the owner's 2026-10-21 "layout plus general feel" decision:
  - `toolbar_panels.htm`, sections:
    - toolbar right-click menu (which toolbar options exist; our items, wording and order differ)
    - Main menu bar (which menus exist and their order)
    - Title & Hint bar: hint behaviour, copying a value, the hint icon display (the set of status kinds only), the sync indicator (the four MIDI input cases and the MIDI clock beat/bar blink), and the Extended Hint Panel
    - Transport & recording controls: Pat/Song (right-click opens the channel rack or playlist), Play, Stop (double-click panic; the remember-the-start-position option), Record (Alt+click popup and the Recording Filter: kinds, options, quantize), Tempo (whole and fine digits, the right-click automation and type-in items, the half-speed preview idea), Tap tempo (pad, nudge, realign, mute while tapping)
    - Song position
    - Monitor volume and main pitch (listening-only volume; master pitch snapping, Shift, A4 entry, its reach to audio clips and per-channel opt-in)
    - Global snap panel (local snap following a global value; the list of snap values; default note length on right-click; record quantize)
    - Add panel
    - Multilink panel (the learn-then-move flow and its menu functions)
    - Time panel (bar and time modes; the steps/beats switch)
    - Pattern panel (options menu functions; click and right-click on the name; up/down selector)
    - Output panel (scope modes, stereo and scrolling options, peak meter, resizing only in toolbar edit mode)
    - CPU & memory panel (double-click monitor, refresh speeds, voices or frame rate)
    - Chord panel
    - Toolbar shortcut icons (which buttons exist and what they do; right-click alternates)

    The online and communications panels were read only to leave them out.
  - `toolbar_edit.htm`: toolbar editing (edit mode, spaces, two racks, presets, and the order in which space is given up at narrow widths).
  - `menu_file.htm`, `menu_edit.htm`, `menu_add.htm`, `menu_patterns.htm`, `menu_view.htm`, `menu_options.htm`, `menu_tools.htm`, `menu_help.htm`: the functions in each top-level menu, regrouped and reworded. Store, account, cloud, news and video items were dropped.
  - `basics_interface.htm`: the interface overview (toolbar at the top over the browser and workspace), the hint panel, and value displays.
  - `recording.htm`: the record button and the recording filter.
  - Screenshots viewed (not traced, measured or colour-sampled): the overall toolbar, its edit mode and context menu, and the time and output panels. The toolbar icon set was viewed only to identify which button is which in the default order. None of its drawings is reproduced.
- **Repository:**
  - `crates/musiccraft-ui/src/toolbar.rs`, `crates/musiccraft-ui/src/icons.rs`, `crates/musiccraft-ui/src/chrome.rs`, `crates/musiccraft-ui/src/geometry.rs`, `crates/musiccraft-ui/src/lib.rs`
  - `crates/musiccraft-editors/src/hint.rs`, `crates/musiccraft-editors/src/theme.rs`, `crates/musiccraft-editors/src/widgets.rs`
  - `crates/musiccraft-ui/keymap/default.keymap`
  - `docs/spec/transport.md`, `docs/spec/recording.md`, `docs/spec/keymap.md`, `docs/spec/window-layout.md`, `docs/spec/controls.md`, `docs/spec/controllers.md`, `docs/parity.md`
  - `docs/screenshots/playlist-mixer.png`, `docs/screenshots/channel-rack.png`, `docs/screenshots/piano-roll.png`
- **Own design:**
  - every glyph in the Icons table, including the status markers, and every colour role and token (including the proposed `theme::LCD_TEXT` and `theme::SEPARATOR`, and the token chosen for each MIDI LED case)
  - all labels, captions and hint wording (including the `· right-click for options` suffix, the context-menu items, the recording-menu items and the proposed renames), and the number formats
  - menu groupings and wording
  - the reflow order beyond the manual's general rule, and the stacked fallback from `window-layout.md`
  - layout names (*Default (two rows)*, *Compact (one row)*, *Transport only*)
  - the TAP and time-signature buttons next to the tempo display, the round-tempo list, and the 10–999 BPM range (the reference's range is narrower; ours is set in `transport.md`)
  - the time display's no-drag rule and the song position slider being hidden by default
  - keeping both beat and step fields in bar mode instead of a steps/beats switch
  - the pattern name's step chevrons and the `+` button finding the next empty pattern (instead of a separate up/down selector), and keeping click-to-list as today's default
  - the save-reminder interval
  - the Global snap entry in editor snap lists
  - master pitch's scope (built-in instruments only, per `transport.md`)
  - keeping the system title bar instead of drawn window buttons
  - Shift for fine adjustment (from `controls.md`)
  - right-clicking a menu check item to flip it without closing the menu
