# Style guide: Channel settings, plugin and effect windows

Part of the [interface style guide](README.md). Read it with the shared rules in [design language](design-language.md), [controls](controls.md) and the window header in [main window](main-window.md).

These are the windows where a user shapes one sound at a time: a channel's instrument (built-in, Antiserum or a hosted CLAP/VST3 plugin) and each mixer insert effect. Aim for a small, dense floating panel. The chrome is two thin bands: the shared one-row **window header** (defined once in the main-window section; here it carries the name and the presets) and a **control strip** (page tabs at the left, the channel's mixing controls at the right). Below them is a page that is all content. The chrome is quiet and flat; the strip is a recessed band between header and page, and the values on the page (lit arcs, curves, digits) are the brightest things in the window. The page's group boxes tile edge to edge like a control surface, with no spare rows, so a whole instrument page reads at a glance. Every instrument and every effect gets the same frame, so the user learns it once.

## Placement and proportions

**Kinds of window.** These are the main-window section's "secondary windows". All of them float inside the workspace, above the playlist, rack, piano roll and mixer, and stay inside the workspace as they do today.

| Window | Opened from | Frame |
|---|---|---|
| Channel window (our "Channel settings") | Clicking a channel's name button in the channel rack, or the rack row menu → Channel settings | Header, then the channel strip with tabs, then the page |
| Effect window | Clicking an effect's name in a mixer slot | Header, then a slim effect strip (own design), then the effect page |
| The plugin's own window (a hosted CLAP/VST3 editor) | The open-window button on the plugin's Sound page (and, as proposed, automatically with its channel or effect window) | The plugin's own desktop window. Our header and strip stay in the companion channel or effect window |

**Proportions** (relative only; our own sizes live in `docs/spec/window-layout.md`):

- **Header:** one row of small tools, the same row as every other window's header (main-window section, "The header row, left to right"; "Window header" in window-layout.md). It is the thinnest band in the window.
- **Channel strip:** about twice the header's height, which is room for exactly one knob with its caption under it.
  - The tabs take the left part, roughly the left third. Each tab is a square cell as tall as the strip.
  - The mixing controls sit flush right.
  - The mixer-track box is set apart from the knobs by a gap of about one knob width, at the far right end.
- **Page:** everything else. In a typical instrument window the header and strip together take less than a fifth of the height.
- **Width:**
  - A channel window is about as wide as the channel rack showing a 16-step pattern. Placed side by side, the two read as siblings.
  - It never gets narrower than the strip needs: every tab plus On, Pan, Volume, Pitch, Range and Track.
  - Wide pages (Antiserum, long plugin parameter names) widen it to at most about half the workspace. The strip then stretches: tabs stay left, controls stay right, and the gap in the middle grows.
- **Height:** follows the page's content, up to the workspace's height minus a small margin. Only the page scrolls. The header and strip never scroll away.
- **Effect window:** narrower than a channel window. It is the width of the effect's graph plus margins, about two-thirds of a channel window. Its height is header + slim strip + graph + one or two rows of knobs, and normally it does not scroll.

**Opening, closing, how many**

- **Channel windows**
  - Clicking a channel's name opens its channel window, and clicking the same name again closes it (today's behaviour).
  - By default there is one channel window, and it follows the clicked channel while keeping the same tab (today).
  - **Proposed (needs `channel-rack.md` updated first):** Alt+click on a channel's name opens an additional channel window for that channel and leaves the others open. Plain click keeps today's behaviour.
  - **Proposed (needs `settings.md` updated first):** a setting **Keep instrument windows open** (Settings → General). When on, every name click opens its own window instead of reusing the current one. Off by default.
- **Effect windows**
  - There is one window per mixer slot, and any number can be open. Clicking the slot's name toggles its window (today).
  - Removing the slot closes its window (today). **Proposed (needs `mixer.md` updated first):** moving the slot up or down keeps the window open, with an updated title.
  - **Proposed (needs `mixer.md` updated first):** right-clicking a mixer strip's effect count ("2 fx") opens all of that track's effect windows, or closes them if any are open.
- **Placement of new windows**
  - New windows cascade. Each additional window opens one header-height lower and one header-height further right than the previous one.
  - Channel windows start near the workspace's top right and effect windows near its top left (both as today).
- **Limits:** there is no limit on MusicCraft's own windows. A limit on hosted plugins' own windows is an open question.
- **Focus:** the window clicked last comes to the front. A click anywhere in it, including on the page, raises it.

**Moving, collapsing, detaching, sizing**

- **Move:** drag the header's drag area or the title (main-window section).
- **Collapse:** the shared collapse button in the header's right-hand window-button group rolls the window up to its header row and keeps its position and size. Presets still step from the header while the window is collapsed. **Proposed (needs `window-layout.md` updated first):** because these windows are fixed-size and have no maximize, double-clicking the title or the drag area collapses or expands them.
- **Strip toggle:** a header tool hides or shows the channel strip, leaving the header and page. The choice is remembered per channel while the app runs. It is UI state and is not saved in the project.
- **Sizing:** built-in windows cannot be resized by their edges, because their pages are laid out in fixed grids of group boxes. Height adapts by itself. They therefore leave out the maximize button (main-window section: fixed-size windows omit it).
- **Scale:** the interface scale in Settings → General (75–200 %) scales these windows with everything else. There is no per-window scale for our own windows.
- **Hosted editors:**
  - They keep the plugin's own size and can be resized only if the plugin allows it (`docs/spec/plugins.md`, "Plugin windows").
  - Scaling is the plugin's own today. **Proposed (future, needs `plugins.md` updated first):** a per-plugin editor scale choice in the Plugin tab: follow MusicCraft's interface scale, follow the system, or a custom factor. It is offered only where the plugin format lets a host set the editor's scale; otherwise the plugin's own scaling stays.
- **Detach (future):** the window menu's shared "Detach from main window" item (main-window section, "Detaching and multiple screens") moves the window into its own desktop window, for a second monitor, keeping header, strip and page. Leave the item out until detaching exists.

## Anatomy

### A. Window header (one row)

The header is the shared row from the main-window section (slots 1–9). Here only this window's own contents are listed; slots not listed (window play button, maximize) are left out.

| Slot | Element | What it shows | Click | Right-click | Scroll | Double-click | Hover / states |
|---|---|---|---|---|---|---|---|
| 1 | **Window menu button** | The shared menu glyph (main-window section) | Opens the window menu (below) under the button | Same as click | — | — | As main-window: hover lightens it; while the menu is open it looks pressed with the glyph in the accent |
| 2 | **Strip toggle** (the window's only tool) | Our strip glyph | Hides or shows the channel strip | — | — | — | Lit while the strip is shown |
| 4a | **Channel colour mark** | A small square in the channel's colour, just before the title (channel windows only) | Same as clicking the title | — | — | — | Dimmed when the channel is muted |
| 4 | **Title** | Main-window's title format: name, an em dash, then the context. Channel window: the channel's name, then the mixer track it plays to ("Lead — Insert 3"). Effect window: the effect's name, then the track's name ("Equaliser — Lead"). Long names are cut with an ellipsis before anything to their right is squeezed. Not a selector, so no chevron | Focuses the window. Dragging moves it | Opens a small menu with "Rename…" (channel windows) | — | Proposed: collapses or expands (see Placement) | Hint gives the full title and how to move or rename it. Focused: full text tone; unfocused: dimmed |
| 5 | **Drag area** | Header background | Raises and focuses; drag to move | Window menu | — | Proposed: collapses or expands | — |
| 6 | **Keys-to-plugin toggle** *(reserved)* | Our keyboard glyph. Drawn only for a hosted plugin whose editor is embedded in our window (future) **and** that asks to receive typed keys, so it is absent today | Toggles whether typed keys go to the plugin first or to MusicCraft's shortcuts first | — | — | — | Lit when the plugin gets keys first |
| 6 | **Preset name box** | The current preset's name, or "(custom)" when no preset matches. A small down-chevron sits at its right end. Shown only for instruments and effects that have presets and while "Show preset name in header" is on | Opens the preset list: category headings and, for Antiserum, the search field at the top. Choosing a preset is one undo step | Opens the window menu's PRESETS group | Proposed (own): steps to the previous or next preset | — | Inset like a small display. "(custom)" is dimmer than a real name |
| 6 | **Previous / next preset** | Two small chevrons, left and right, tight together, just left of the window buttons | Steps through the list and wraps at the ends. From "(custom)", "next" goes to the first preset and "previous" to the last | Opens the preset list (as clicking the box does) | — | — | Hint: "Previous preset" / "Next preset". Hidden when there are no presets |
| 7 | **Collapse** | The shared collapse glyph | Collapses to the header or expands | — | — | — | Hint "Collapse to header" / "Expand"; glyph flips when collapsed |
| 9 | **Close** | The shared close cross (`chrome.rs`) | Closes the window. For a channel window this is the same as clicking the channel's name again | — | — | — | Hint "Close"; on hover the glyph turns the danger tone (main-window section) |

The right-hand items (slot 6) and window buttons (7, 9) are laid out from the right end, so a long title gives way to them, never the other way round. Slot 8 (maximize) is left out because these windows are fixed-size.

**Window menu, channel window** (our wording; groups are separated by small dimmed headings; the shared common window items come last, after a separator):

- Top
  - Open in piano roll (existing rack item)
  - Replace instrument… *(new command)*
  - Delete (in `theme::DANGER`, as in the rack menu)
- PRESETS
  - Previous preset / Next preset
  - Save preset… / Open preset… (Antiserum has these today; built-ins get them when user presets exist). **Proposed (needs `controls.md` updated first):** the "Save preset…" row can also be dragged, like a file, onto the browser or onto another channel of the same instrument, to save or apply the preset there.
  - Show presets in the browser *(new)*: opens the browser at this instrument's or effect's presets
  - Randomize settings *(future, own)*: sets the instrument's parameters to random values, one undo step
  - Show preset name in header (tick)
- COMPARE *(new)*
  - Store as B: keeps a copy of the current settings
  - Swap A and B: exchanges the current settings with the stored copy. One undo step
- PARAMETERS
  - Show parameter list (hosted plugins: jumps to the Sound page's list)
  - Add controller ▸ (the existing controller submenu)
- PROCESSING *(hosted plugins, future)*: deliberately absent today, because MusicCraft has no per-plugin processing options yet. When one exists (for example letting a plugin sleep while it is silent), it goes in this group and in the Plugin tab.
- CHANNEL
  - Rename…
  - Colour… *(new command)*
  - Clone (Alt+C)
  - Route to a free mixer track (Ctrl+L, `channel.route_free`)
  - Group…
- Common window items (main-window section): Detach from main window *(once detaching exists)*, Collapse to header, Reset position. Then Close.

**Window menu, effect window:**

- Top
  - Replace effect… *(new)*
  - Remove effect
  - Move up / Move down (`mixer.move_effect`)
  - Bypass (tick)
- PRESETS: as for channel windows
- COMPARE: as for channel windows
- PARAMETERS: as for channel windows
- PROCESSING: as for channel windows (hosted plugin effects, future)
- Common window items, then Close: as for channel windows

### B. Channel strip (one band, about twice the header's height)

**Left group: page tabs.** Square icon cells, touching, in this order. A tab exists only when its page applies. There is no separate sample tab: a sample-based instrument's sample, playback and waveform controls are on its Sound page (C2).

1. **Sound** (every channel): the instrument's own controls. For a hosted plugin it is the plugin page.
2. **Plugin** (hosted plugins only, *new*): details and hosting options.
3. **Envelope** (instruments that use the channel envelope: `instrument::has_envelope`).
4. **Functions** (existing label; every channel except MIDI output and channel stack channels): arpeggiator, swing, timing, echo, voices. See Labels for a proposed rename.

How the tabs behave:

- Click selects the tab. The window keeps the last tab while it stays open (today).
- Hovering shows the tab's name and purpose in the hint bar.
- The active cell takes the page's background, so it reads as joined to the page below. Inactive cells sit on the strip with a dimmed glyph.
- When the selected tab does not exist for a newly shown channel, fall back to Sound (today).

**Right group: mixing controls**, flush right, in this order. Knobs follow the controls section and `docs/spec/controls.md`: left-drag up or down (Shift for fine), scroll nudges, double-click or middle-click resets, right-click opens the shared control menu (controls section: Create automation clip, Add controller ▸, Reset to default, and the rest). The caption under a knob swaps to its value while it is hovered or dragged.

1. **On light**
   - A small round light, the same widget as the channel rack row's mute light (`widgets::dot`), with the caption "On" under it, on the same line as the knobs' captions, so it lines up with them.
   - Click toggles mute (`channel.toggle_mute`).
   - **Proposed (needs `channel-rack.md` updated first):** right-click opens a menu: Solo (`channel.solo`), Mute others.
   - Lit = the channel plays.
2. **Pan knob**
   - Bipolar: the arc grows from the top centre.
   - Mirrors the rack row's pan knob (`channel.set_pan`).
   - Value text: the shared pan format from the controls section ("25% left", "Centre", "25% right").
3. **Volume knob**
   - Mirrors the rack row's volume (`channel.set_volume`).
   - Value text in percent.
   - Shown a little larger, or with its value arc a step brighter, than its neighbours, because it is the main control.
4. **Pitch knob**
   - Bipolar, in semitones (`pitch`).
   - For hosted plugins the slot stays, so the strip never shifts. The knob is drawn disabled until plugins can follow it.
5. **Range box** *(planned)*
   - A compact number box (controls section) joined to the Pitch knob's right side: the most the Pitch knob bends, in semitones.
   - Drag up or down to change it.
   - Leave it out until a range parameter exists.
6. *(gap of about one knob width)*
7. **Track box**
   - A compact number box (controls section), with the caption "Track" under it.
   - It shows "M" for the master or the insert number, like the rack row's box.
   - Drag up or down, or type, to route (`channel.route`) (today).
   - **Proposed (needs `channel-rack.md` updated first):** click the **caption** to route to a free mixer track (`channel.route_free`). The caption brightens on hover to show it can be clicked.
   - **Proposed (needs `channel-rack.md` updated first):** double-click the box to open the mixer with that track selected.
   - Hovering names the track in the hint bar.
   - Hidden for channels that make no audio (MIDI output, channel stack).

**Variants**

- **MIDI output channel:** the strip shows On and, in the Track box's place, a "MIDI ch" box (1–16, `channel.set_midi_channel`). Pan, Volume and Pitch are left out. The only tab is Sound (the MIDI page).
- **Channel stack channel:** the strip shows On only. The only tab is Sound (the list of stacked channels).

### C. Pages (the area under the strip)

**Shared group-box rule.** Every page is a grid of **group boxes** (`instrument::section`):

- Each box is a darker rounded panel with its title in small capitals inside its top-left corner.
- Boxes in one row share their height. Their widths follow their content: a box with two knobs is narrow, a box with a graph is wide.
- Two to four boxes sit side by side per row. Rows stack with a thin, even gap, the same gap as between boxes.
- A box never spans the full width just to hold one row of knobs if another box can share the row.
- Full-width boxes are kept for graphs, waveforms and the keyboard strip.

**C1. Sound, built-in synths** (Synth, Drum synth, Pluck, FM, Keys, Additive):

- The instrument's own sections, tiled by the rule above. This guide does not cover what is inside them.
- The preset row that sits at the top of these pages today moves up into the header (A, slot 6).

**C2. Sound, sample-based instruments** (Sampler, Granular, Resynth):

- **Two columns** of boxes, about equal width.
  - Left column, top to bottom: SAMPLE, a single-row box, then the playback options box (one-shot, reverse, fades), then slices.
  - Right column: pitch/root and stretch-related boxes, then the instrument's own controls.
  - The SAMPLE box holds the file name, which is truncated; **proposed (own)**, clicking it reveals the file in the browser. At its right end are three small icon buttons: load (folder), show in browser, remove sample.
- **Waveform**, full width at the bottom, about a third of the page's height:
  - It draws the channel colour on a dark well.
  - Start, end and loop markers are thin vertical lines with grab handles at the top edge. Drag a handle to move its marker.
  - Left-click previews the sample.
  - Scroll zooms around the pointer.
  - Right-click opens the sample menu: Change sample… (existing rack item), Reload, Show in browser, Open in sample editor.
- **Zoom/scroll bar** under the waveform: a thin full-width bar whose thumb is the visible part. Drag the thumb's ends to zoom and its middle to scroll (as the main-window section's scroll bars).

**C3. Sound, a hosted plugin: generic parameter view.** This page also serves plugins that have no editor of their own. Top to bottom:

1. **Tool row**
   - At the left, the **open-window button**: our open-window glyph plus "Plugin window". It is shown only when the plugin has an editor, and it is lit while that window is open. Click opens or closes it.
   - Then the **filter field**, with a magnifier glyph inside it at the left and "Filter parameters" as its placeholder.
   - At the right end, a dimmed count: "24 of 120".
2. **Parameter list**
   - Fills the rest of the page and is the page's own scroll area, not a second, nested one.
   - Parameters are grouped under small dimmed headings by the plugin's module path, in the plugin's order.
   - Each row has three columns:
     - **Name**, about two-fifths of the width. Brighter when MusicCraft stores a value for it, dimmed when it is untouched.
     - **Bar**, about two-fifths. A horizontal value bar: left-drag sets the value, Shift for fine, scroll nudges, double-click or middle-click resets.
     - **Value**, the rest. Right-aligned, small monospace, as the plugin formats it.
   - Rows are striped. Read-only parameters are drawn disabled.
   - Right-click on a row opens the control menu (controls section) plus "Forget stored value" (only when one is stored).
   - Hovering a row shows the parameter's id and module in the hint bar.
3. **Empty list:** a single dimmed line, "This plugin exposes no parameters."

**C4. Plugin (hosted plugins only, *new*).** Two columns of boxes:

- **DETAILS** (left): name, maker, format (CLAP or VST3), id, file, latency. These are label/value rows, with labels dimmed and values in text tone.
- **PLUGIN** (right): buttons Reload plugin, Rescan plugin folders, Open plugin window. Later also the editor-scale choice (see Placement) and any processing options.
- **NOTES** (left, under DETAILS): what reaches the plugin, in plain sentences. For example, "Notes go to the first note input", or "Only the main stereo output is used" (`docs/spec/plugins.md`, "Limits").

**C5. Envelope.**

- **Target row:** a row of small text tabs at the top, one per thing the channel envelope can shape. Leave the row out while only the amplitude envelope exists, as today.
- **ENVELOPE box:** about half the width, or the full width while it is the only box.
  - The **graph** sits on top, at least as tall as two knob rows, on a dark well. It is drawn in the channel colour and has a round handle at each stage's corner. Drag a handle to change that stage; one drag is one undo step.
  - Below the graph is a row of knobs: Attack, Decay, Sustain, Release.
- **Other boxes:** when the channel gains a per-channel modulator or filter (not today), each gets its own box to the right of ENVELOPE in the same row. A modulator box is about a third of the width. A filter box is a narrow column about a sixth of the width, with two stacked knobs and a type drop-down.
- **Keyboard strip:** across the full width at the bottom (C7).

**C6. Functions.** Tiled boxes, then the keyboard strip. Captions are listed under Labels and wording (existing):

- **Row 1:** VOICES (Max, Poly/Mono, the plugin's own voice controls) | TIMING (Delay and Length, plus the Swing knob and its Delay/Shorten choice) | a narrow box for any controls the instrument adds.
- **Row 2:** ARPEGGIATOR across the full width (direction choice row, then rate, Octaves, Gate). Its five-way direction row does not fit a third of the window, so it gets its own row (own design).
- **Row 3:** CHOKE (Group, Silenced by, plus a small "Self-choke" button that sets both to the same group) | ECHO (Repeats, Time, Feedback, Pitch step; wraps onto a second line when narrow).
- **Bottom:** the keyboard strip (C7).
- When Antiserum hides the arpeggiator (it has its own), row 2 is left out.

**C7. Keyboard strip** (bottom of Envelope and Functions; *new*):

- A full-width box about a fifth of the page's height.
- **Top line:** the box title "KEYBOARD" with the current root key ("Root C5"), then a Reset button and a fine-tune knob at the right.
- **Below:** a horizontal piano.
  - A thin ruler sits above the keys with octave labels at each C, using `key_name`, where C5 = MIDI 60.
  - The root key is filled in the accent tone.
  - Left-click a key to preview it. Right-click a key to set the root key (`channel.set_param … root_key`).
  - Dragging on the ruler marks a key range: the keys outside it are dimmed and the range's ends can be dragged. This needs a new key-range parameter.
- **Under the keys:** a thin zoom/scroll bar, as under the waveform.

**C8. Missing or failed plugin.** In place of the plugin page, show a dark message box centred in the page with a warning glyph and one sentence in the warning tone. Use today's three texts from `plugins.rs`: not available in this build, failed to start, not installed. Under it, a "Rescan plugin folders" button when that can help.

### D. Effect window

- **Header:** as A, with the effect menu.
- **Effect strip** (own design; the reference keeps bypass and mix on the mixer slot only): a slim band.
  - For built-in effects there is no tab group. Plugin effects get the Sound and Plugin tabs at the left.
  - At the right: the **On light** (bypass, `mixer.toggle_effect`; lit = processing) and the **Mix knob** (`mixer.set_effect_mix`, the same value as the slot's mix knob).
  - Bypass moves here from today's dot in the body.
- **Page**, top to bottom:
  1. Any routing or source row the effect needs, such as the reverb's impulse picker or a sidechain source. These are one-line rows of label + drop-down + button.
  2. The **graph**: full width, on a dark well, with draggable points, as today. Special effects show the **tuner** or the **rack view** in the graph's place.
  3. The effect's **knob row**, wrapping onto a second row when needed.
- **Hosted plugin effects:** the page is the generic parameter view (C3).

### E. The plugin's own window (hosted editor)

- A separate desktop window, sized by the plugin (today).
- Title: "<channel or track name> — <plugin name>", so several open editors can be told apart in the desktop's window list.
- Where the platform lets us draw inside it, it also gets our compact header (window menu, preset arrows if any, title, collapse, close), as the main-window section says for hosted plugin windows. Where it does not, the desktop's own title bar is used.
- Closing it with its title-bar button works like the open-window button (today). The companion channel or effect window stays open.
- **Proposed (needs `plugins.md` updated first):** opening a plugin channel's window by clicking its name also opens the plugin's own window, and closing the channel window closes it.

## Icons

All glyphs are our own vector drawings in the style of `crates/musiccraft-ui/src/icons.rs`: thin even strokes, a few points, drawn inside the cell's inner square, coloured by the state colour passed in. New ones become `Icon` variants. The shared header glyphs (window menu, collapse, close, detach/return) are owned by the main-window section and only referenced here.

| Where | What it does | Our glyph |
|---|---|---|
| Header slot 1 | Opens the window menu | The shared **menu** glyph: three short horizontal bars, evenly stacked (main-window section) |
| Header slot 2 | Shows or hides the channel strip | **Strip:** three horizontal lines across the cell, each with a small filled circle at a different point along it (left, right, middle) |
| Header slot 6, reserved | Typed keys to the plugin first | The existing **Keyboard** glyph |
| Header slot 6, preset box end | Opens the preset list | **Down-chevron:** two short strokes meeting in a point at the bottom |
| Header slot 6, preset step | Previous / next preset | **Left / right chevrons:** two strokes meeting in a point, pointing left or right. Not the font glyphs used today |
| Header slot 7 | Collapse to header / expand | The shared **collapse** glyph: an upward chevron over a short bar; a downward chevron over the bar while collapsed (main-window section) |
| Header slot 9 | Close | The existing **close cross** in `chrome.rs` |
| Window menu → Detach from main window (future) | Moves the window to its own desktop window | The main-window section's detach/return glyphs |
| Strip tab: Sound | Instrument's own controls | **Sine:** one smooth full cycle of a sine wave across the cell, centred vertically |
| Strip tab: Plugin | Plugin details and hosting | **Info:** a circle outline with a short vertical bar and a dot above it, centred |
| Strip tab: Envelope | Channel envelope | **Envelope:** a filled silhouette over a baseline: a straight rise to the top, a short fall, a flat shelf, then a fall back to the baseline. No dots at the corners |
| Strip tab: Functions | Arpeggiator, swing, timing, echo, voices | **Steps:** three small filled squares climbing from bottom-left to top-right, like notes of a rising arpeggio |
| Generic view: open-window button | Opens or closes the plugin's own window | **Open window:** a square outline open at its top-right corner, with a short diagonal arrow leaving through the gap |
| Generic view: filter field | Filters parameters | The existing **Zoom** (magnifier) glyph, small, inside the field's left end |
| SAMPLE box: load | Loads a sample | **Folder:** a folder outline with a tab on its top-left edge (distinct from the Browser glyph: add a small plus at the lower right) |
| SAMPLE box: show in browser | Shows the file in the browser | The existing **Browser** glyph |
| SAMPLE box: remove | Removes the sample from the channel | A small **cross**, as the close cross |
| Missing-plugin message | Warning | **Warning:** a triangle outline with a short vertical bar and a dot inside |
| On light, Track box, knobs | — | No glyph. The On light is the rack's round light with an "On" caption, the Track box carries digits, and the knobs are `widgets::labeled_knob` |

## Colour roles

Colour is given only as tokens from `crates/musiccraft-editors/src/theme.rs`, plus relative tone. The existing surface tokens run from darkest to lightest as `PANEL_DARK`, `BACKGROUND`, `PANEL`, `RAISED`, `RAISED_HOVER`. Tokens marked "proposed" are defined in the section named.

| Element | Token(s) | Relative tone and state changes |
|---|---|---|
| Page background (window body) | `theme::PANEL` | Base tone of the window, as every window body (main-window section) |
| Window header row | Proposed `theme::HEADER` (= `PANEL`), focused: proposed `theme::HEADER_ACTIVE` (main-window section) | Lifts one step when the window is focused, like every other header |
| Channel / effect strip | `theme::BACKGROUND` | One step darker than both the header and the page, so the three bands stay distinct and the strip reads as a recessed control band |
| Window outline | Proposed `theme::WINDOW_EDGE` (main-window section: one step lighter than `RAISED`). Focused: `theme::ACCENT_DIM` | Only the front window gets the accent outline |
| Title text | `theme::TEXT` (front window), `theme::TEXT_DIM` (windows behind) | The em dash and the context part are always `theme::TEXT_DIM` |
| Channel colour mark | `theme::palette(index)` | Dimmed (lower opacity) when the channel is muted |
| Header and strip glyph buttons | Glyph `theme::TEXT_DIM`. Hover: `TEXT` on a `RAISED_HOVER` square. Open or lit: glyph `theme::ACCENT` | Pressed: fill `theme::ACCENT_DIM`. Close glyph on hover: `theme::DANGER` (main-window section) |
| Preset name box | Fill `theme::LCD_BG`, edge `theme::LCD_EDGE`, text `theme::TEXT` | "(custom)" in `theme::TEXT_DIM` |
| Tab cell, inactive | No fill (strip shows through), glyph `theme::TEXT_DIM` | Hover: fill `theme::RAISED_HOVER`, glyph `theme::TEXT` |
| Tab cell, active | Fill `theme::PANEL` (same as page), glyph `theme::ACCENT` | Reads as joined to the page; no line between them |
| On light | On: lit `theme::ACCENT`. Off: the dark recessed fill of the rack's mute light. Caption `theme::TEXT_DIM` | Same widget and tones as the channel rack's mute light |
| Knobs | Rim `theme::PANEL_DARK`, face `theme::RAISED`, track `theme::PANEL`, value arc `theme::ACCENT`, pointer `theme::TEXT` | Disabled: arc `theme::ACCENT_DIM`, pointer `theme::TEXT_DIM` |
| Knob caption | `theme::TEXT_DIM` | Hovered or dragged: shows the value in proposed `theme::LCD_TEXT` (toolbar section; equal to `theme::PLAYHEAD`, which `labeled_knob` uses today) |
| Automated knob mark | `theme::AUTOMATION` | A small dot under the caption when an automation clip drives the control |
| Track, Range, MIDI-ch boxes | The controls section's compact number box: fill `theme::LCD_BG`, edge `theme::LCD_EDGE`, digits proposed `theme::LCD_TEXT`, caption `theme::TEXT_DIM`. The channel rack's mixer-track box should use the same look, so the two boxes match | Clickable caption hover: `theme::TEXT` |
| Group boxes | Fill `theme::PANEL_DARK`, title `theme::TEXT_DIM` | Darker than the page |
| Graph and waveform wells | `theme::LCD_BG` | Curves in the channel colour (channel windows) or `theme::ACCENT` (effects). A grabbed handle is `theme::HIGHLIGHT` |
| Choice buttons inside pages | Unselected `theme::RAISED`; selected `theme::ACCENT_DIM` with `theme::TEXT` | As the theme's selection visuals |
| Parameter list rows | Stripes alternate `theme::PANEL` and `theme::PANEL_DARK`. Names: stored `theme::TEXT`, untouched `theme::TEXT_DIM`. Bar fill `theme::ACCENT` on a `theme::RAISED` track | Read-only: bar `theme::ACCENT_DIM`, name dimmed |
| Group headings in lists and menus | `theme::TEXT_DIM`, small capitals | — |
| Keyboard strip | Proposed `theme::KEY_WHITE` and `theme::KEY_BLACK`, as the piano-roll section defines them (which owns them): `KEY_WHITE` one step lighter than `theme::TEXT`, the lightest surface in the app; `KEY_BLACK` between `theme::PANEL_DARK` and `theme::BACKGROUND`. The piano roll uses literals today. Root key `theme::ACCENT`. Keys outside the range are dimmed | Pressed key: `theme::ACCENT` |
| Warning message (missing plugin) | Text and glyph `theme::HIGHLIGHT` on `theme::PANEL_DARK` | Errors (failed to start): `theme::DANGER` |
| Delete / remove menu items | `theme::DANGER` | As the rack menu's "Delete" |

## Labels and wording

**Window names**

- Channel window: "Channel settings" (menus and hints, existing). Its header title is the channel's name, an em dash, and the track's name: "Lead — Insert 3". The master is shown as "Master".
- Effect window: "Equaliser — Lead". The effect's label comes first and the track's name second, which reverses today's "Lead › Equaliser".
- Plugin's own window: "Lead — <plugin name>".

**Header**

- Hints: "Window menu — presets, rename, routing and more", "Show or hide the channel strip", "Previous preset", "Next preset", "Collapse to header" / "Expand", "Close".
- "(custom)" when no preset matches (existing).

**Tabs**

- "Sound", "Envelope", "Functions" (existing) and "Plugin" (new).
- **Proposed rename:** "Functions" → "Playing", our own word for the page of note-playing options, so the tab does not echo the reference's name for the same tab.
- Tab hints:
  - "Sound — the instrument's own controls"
  - "Plugin — details and hosting options"
  - "Envelope — how each note's level rises and falls"
  - "Functions — arpeggiator, swing, timing, echo and voices"

**Channel kinds:** this section uses "MIDI output" and "Channel stack", the renames the browser section proposes for today's "MIDI Out" and "Layer".

**Strip captions** (sentence case, as our knobs are): "On", "Pan", "Volume", "Pitch", "Range", "Track", "MIDI ch".

**Group titles** (small capitals, existing style)

- Existing: SAMPLE, GRAINS, RESYNTHESIS, PADS, LAYER, MIDI OUT, ENVELOPE, ARPEGGIATOR, VOICES.
- Existing since the Functions split: CHOKE, ECHO, TIMING.
- New: KEYBOARD, DETAILS, PLUGIN, NOTES.
- Proposed renames: LAYER → CHANNEL STACK, MIDI OUT → MIDI OUTPUT (with the browser section).

**Functions-page captions** (existing, in `instrument.rs`; our own words, not the reference's):

| Box | Captions |
|---|---|
| VOICES | Max, Poly / Mono |
| TIMING | Delay, Length, Swing, and the swing choice Delay / Shorten |
| CHOKE | Group, Silenced by, and the Self-choke button |
| ECHO | Repeats, Time, Feedback, Pitch step |

**Menu items:** our words, as listed under A ("Route to a free mixer track", "Show presets in the browser", "Store as B", "Swap A and B", "Replace instrument…", "Detach from main window"). Existing rack labels are kept: "Open in piano roll", "Rename…", "Clone", "Group…", "Delete", "Change sample…". Items that open a dialog end with "…".

**Generic view:** "Plugin window", "Filter parameters", "24 of 120", "Forget stored value", "This plugin exposes no parameters."

**Hint-bar style**

- Sentence case, one line, no full stop.
- State or value first, then an em dash, then what the gestures do. For example:
  - "On — click to mute this channel"
  - "Volume 80% — drag to change, double-click to reset"
  - "Plays to insert 3 (Lead) — drag to change" (plus ", click Track for a free mixer track, double-click to show it in the mixer" once those proposals land)
- With the hint bar hidden the same text appears as a tooltip (`hint.rs`).

**Number formats** (from `ParamSpec::format` and the controls section's shared formatters)

| Kind | Format |
|---|---|
| Volume and other 0–1 amounts | Whole percent ("80%") |
| Pan | As the controls section: "25% left", "Centre", "25% right" |
| Pitch | Semitones with one decimal and sign ("+2.0 st") |
| Times | "250 ms" below one second, "1.20 s" above |
| Frequencies | "880 Hz", "2.50 kHz" |
| Track box | "M" or the insert number |
| MIDI channel | 1–16 |
| Key names | "C5" = MIDI 60 (`key_name`) |
| Plugin values | As the plugin formats them |

## States and feedback

| State | Look (relative) |
|---|---|
| Hover | Buttons and tab cells get the `RAISED_HOVER` fill and their glyph brightens to `TEXT`; the close glyph turns `DANGER`. A knob caption shows its value. A clickable Track caption brightens. A parameter row's name brightens. The hint bar shows the hint |
| Pressed | Fill one step deeper (`ACCENT_DIM`) for the press's duration |
| Active / on | Active tab: page-coloured cell with an accent glyph. On light lit. Strip toggle and open-window button glyphs lit in accent. An open menu's button looks pressed with an accent glyph |
| Selected | The current preset is highlighted in the preset list with the theme's selection fill. The chosen value of a choice row is filled `ACCENT_DIM` |
| Muted (channel) / bypassed (effect) | On light dark. The header's colour mark and, for effects, the title dim. The effect's graph curve drops to `ACCENT_DIM`. Knobs stay editable and do not dim |
| Disabled | Controls that do not apply (Pitch on a hosted plugin, read-only plugin parameters): dim arc and caption, no hover change, a hint saying why |
| Playing | No change to the window frame. Graphs that show live data (tuner, meters inside effects) keep updating. *(Optional, own design:* the colour mark brightens briefly when the channel starts a note) |
| Recording | No special look in these windows. A control written by automation recording shows the automation dot |
| Dragging | The knob's caption shows the value. The hint bar shows the name and value. A dragged graph handle turns `HIGHLIGHT`. Dragging the header moves the window. One drag is one undo step |
| Focus | The front window's header lifts to `HEADER_ACTIVE`, its outline is `ACCENT_DIM` and its title is full brightness. The windows behind have the plain header and outline and a dimmed title. Page content keeps full brightness |
| Collapsed | Header row only, with the collapse glyph flipped. Preset box and arrows still work |
| Missing plugin / error | Warning tone message box (C8). The header and strip still work, so the channel can be muted, re-routed or deleted |

## MusicCraft today: gaps

Ordered by visual impact.

1. **Pages stack full-width sections instead of tiling boxes.** `crates/musiccraft-editors/src/instrument.rs` (`section`, `envelope`) still gives the Envelope page and the instruments' Sound pages full-width sections, one under another. The guide tiles two to four boxes per row (C). The Functions page now tiles (`functions_with`): VOICES | TIMING, then ARPEGGIATOR across the full width, then CHOKE | ECHO. The sampler page in `channel_settings.rs` (`sampler`) has the waveform in the middle of its section, not full width at the bottom (C2).
2. **Antiserum's preset bar lives in the page body.** The built-in instruments' presets are now in the channel window header (`InstrumentEditor::preset_picker`). Antiserum's ◀ / list / ▶ / "Save…" / "Open…" row in `crates/musiccraft-editors/src/antiserum.rs` (`preset_bar`) should move there too, with Save and Open in the window menu and the search kept inside the preset list. There is no way to show presets in the browser and no draggable save item.
3. **The hosted-plugin page repeats chrome and nests scrolling.** In `crates/musiccraft-ui/src/plugins.rs` (`panel`):
   - The plugin's name (accent) and id are drawn again in the body.
   - "Open plugin window" is a text button.
   - The parameter grid is a scroll area with a fixed maximum height inside the page's own scroll area.
   - There are no module headings and no parameter count.
   - The plugin's own window is titled with the plugin name only (`crates/musiccraft-app/src/plugins.rs`, `set_editor`).
4. **There is no Plugin (details) tab** for hosted plugins (C4). Format, maker, file and the rescan/reload buttons are not shown in the channel window. There is no editor-scale choice for hosted editors and no processing options; `docs/spec/plugins.md` says nothing about either.
5. **There is no keyboard strip** (root key, preview, key range) at the bottom of the Envelope and Functions pages (C7). The root key is only reachable on some instruments' own pages.
6. **Only one channel window can exist.** `crates/musiccraft-ui/src/lib.rs` holds a single `windows.channel_settings` flag plus `selected_channel`. There is no Alt+click to open more, and no "Keep instrument windows open" setting.
7. **There is no detaching** to a desktop window (future item, main-window section).

## Sources

**Public online manual** (https://www.image-line.com/fl-studio-learning/fl-studio-online-manual/html/). The text and screenshots of these pages were compared for layout, proportions, density and behaviour only. Nothing was traced, measured or colour-sampled, and all wording here is our own.

- `plugins/wrapper.htm`, "Plugin Wrapper" (text and its wrapper screenshot):
  - The numbered header controls (menu at the far left, settings toggle, name with mixer track, keyboard-priority toggle shown only for plugins that take keys, preset arrows, minimise/close at the right, the tab row with the channel controls).
  - "Plugin Menu": the kinds of commands grouped in the menu (presets, an A/B buffer, parameters, processing, channel naming and routing). Its presets group gave the ideas of browsing presets in the browser, a draggable save item, randomizing and showing the preset name; its processing group gave the reserved PROCESSING group.
  - "Plugin Wrapper Settings" (settings / processing / troubleshooting tabs), used only as the idea of a plugin details tab.
  - Scaling (host / system / custom factor for editors) and detaching notes.
- `chansettings.htm`, "Channel Settings": which tabs exist and that they depend on the instrument; the window's menu.
- `chansettings_main.htm`, "Main Channel Settings" (text and its screenshot): the order of the top strip (tabs at the left, including a separate sample tab that we fold into Sound; on light with its caption, pan, volume, pitch with range, mixer track box at the right). Also the on light's right-click solo, and the track box's drag, double-click-to-mixer and click-the-caption-for-a-free-track behaviour.
- `chansettings_sampler.htm`, "Sampler Channel Settings": File, Content, Playback, Time stretching, Precomputed processes, Sample View / Mouse Controls. This gave the two-column grouping with the waveform and zoom bar at the bottom.
- `chansettings_ins.htm`, "Envelope / Instrument Settings": target tabs, the envelope box with its graph above the knobs, the modulator and filter boxes side by side, and the keyboard at the bottom.
- `chansettings_misc.htm`, "Miscellaneous Channel Settings": the tiling of groups (polyphony, time, cut groups, arpeggiator, echo) and the preview keyboard with root key and key range. Its captions were deliberately not reused.
- `mixer_plugin.htm`, "Effect slots 1 to 10": "Using Effects" (click a slot's name to open or close its window, mix level on the slot), "Effect Slot Menu".
- `mixer.htm`, "Mixer Explained": effects slots and the track's effects switch (right-click opens or closes all of a track's effect windows).
- `channelrack.htm`, "Channel Rack": the channel button's click to open and close, and the modifier-click to keep several windows open.
- `envsettings_general.htm`, "System Settings - General":
  - "Miscellaneous": whether opening one plugin closes others, hiding the wrapper toolbar, detaching new plugins, the optional limit on open editors.
  - "Display": interface scaling.
- `basics_interface.htm`, "FL Studio User Interfaces": "Presets" (preset arrows at the top right, right-click for the list), "Knobs & sliders", "Number Displays", "Window menus" (menu at the top left), "Hint Panel", "Multi-Screen Support".
- Consulted, not used: `basics_externalplugins.htm` (plugin scaling notes), `mixer_trackprops.htm`.

**Repository**

- `crates/musiccraft-ui/src/channel_settings.rs`, `effect_window.rs`, `plugins.rs`, `chrome.rs`, `icons.rs`, `mixer.rs`, `channel_rack.rs`, `piano_roll.rs`, `antiserum.rs`, `lib.rs`, `browser.rs`
- `crates/musiccraft-editors/src/effect.rs`, `instrument.rs`, `antiserum.rs` (outer frame only), `widgets.rs`, `theme.rs`, `hint.rs`
- `crates/musiccraft-params/src/lib.rs` (effect titles), `crates/musiccraft-app/src/plugins.rs`, `crates/musiccraft-clap/src/gui.rs`
- `docs/spec/plugins.md`, `channel-rack.md`, `window-layout.md`, `controls.md`, `mixer.md`, `settings.md`, `keymap.md`, `docs/clean-room.md`
- `docs/screenshots/synth-and-eq.png`, `antiserum.png`, `playlist-mixer.png`
- Other sections of this guide referred to: main-window (shared header, its glyphs and tokens, detaching), controls (knobs, number boxes, control menu, pan format), toolbar (`LCD_TEXT`), piano-roll (`KEY_WHITE`, `KEY_BLACK`), browser ("MIDI output", "Channel stack").

**Own design**

- Every glyph and every colour-role mapping, including the use of the proposed tokens `WINDOW_EDGE`, `HEADER`, `HEADER_ACTIVE`, `LCD_TEXT`, `KEY_WHITE` and `KEY_BLACK`.
- All labels, hints, captions and menu wording, including the proposed "Playing" tab name and the Functions-page captions (Group, Silenced by, Self-choke, Delay, Length, Shorten, Repeats, Feedback, Pitch step).
- The channel colour mark, the strip toggle's placement as the header's only tool, and double-click on the title or drag area to collapse.
- The effect window's slim strip with the On light and Mix knob.
- Alt+click for extra windows and the "Keep instrument windows open" setting.
- Cascading placement, and keeping effect windows open when their slot moves.
- The Plugin tab's contents, the generic parameter view, the "Self-choke" button, scrolling over the preset box, and the editor-scale choice's placement in the Plugin tab.
- The C6 arrangement of our own Functions groups, the keyboard strip's details, and the colour-mark-on-note option.
- Group titles drawn inside the box rather than as a tab on its edge.
