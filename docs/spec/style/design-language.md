# Style guide: Design language: colour, type, icons, shape and names

Part of the [interface style guide](README.md). Read it with the shared rules in [design language](design-language.md), [controls](controls.md) and the window header in [main window](main-window.md).

This section is the shared vocabulary every other section points to: the tones, type, shapes, glyphs, words and small motions that make MusicCraft look like one program. Where another section disagrees with a rule here, this section wins and the other section follows it. The feel to aim for is a dense, dark studio desk. The chrome is quiet, flat and low in contrast, so the music stands out: clips, notes, lit steps and meters are brighter and more saturated than anything around them. One teal accent says "on / active / this value", one amber highlight says "current mode / singled out / look here", and nothing else competes. Every colour, glyph, face and word below is MusicCraft's own. Only the overall impression (dark, compact, content brighter than chrome, a theme the user can shift as a whole) was compared with the public manual.

## Placement and proportions

The design language has no window of its own. It lives in code and shows everywhere:

- **Tokens** live in `crates/musiccraft-editors/src/theme.rs` (`theme::*` constants, `theme::palette`, `theme::track_color`, `theme::fonts`, `theme::apply`, `theme::watermark`). `musiccraft-ui` re-exports the module as `crate::theme`. **No other file may write a colour literal.** Every colour a widget paints is a token, or a token passed through one of the derivations named in this section (darkened, dimmed, at low strength).
- **Glyphs** live in `crates/musiccraft-ui/src/icons.rs` (the `Icon` enum and `icons::paint`). The header marks that are drawn in `chrome.rs` today (the collapse arrow and the close cross) move into `Icon` (see Gaps).
- **Control drawings** (knob, labelled knob, fader, dot, meter) live in `crates/musiccraft-editors/src/widgets.rs`. The toolbar's square button, window toggle, LCD, segmented switch, scope and load bar live in `icons.rs`. Window headers live in `crates/musiccraft-ui/src/chrome.rs`; their slot order belongs to the main-window section.
- **Proportions the language fixes everywhere** (our own sizes, set in `docs/spec/window-layout.md` and the widget code, never read off a screenshot):
  - A tool or transport button is a little wider than tall. Window toggles are one step larger and quieter (no fill when off).
  - An LCD display is about one and a half buttons tall. It has a large value line and a caption line about half its height.
  - A small knob is about the height of a tool button. A labelled knob is about two and a half buttons tall, caption included.
  - Step buttons are taller than wide. Mixer strips are narrow, about two tool buttons wide.
  - Text never sets a row's height. Rows are sized by their controls, and labels sit in them with a small margin above and below.
- **Density rule.** Controls within a group sit close together (about an eighth of a button width apart). Groups are separated by a gap of about one button width or by a separator line, never by an extra box. Windows have small inner margins. The workspace, not the chrome, takes the room.
- **Theme adjustment (future, own design).** The manual describes theme settings that shift the whole interface in a few general ways (overall tint, colour strength, brightness, contrast, text brightness) plus a handful of role colours. MusicCraft should offer the same kind of whole-theme adjustment as a **Theme** page in Settings. It would be built as transforms over the token groups below: chrome tones, the accent, the highlight, the danger tone, meter tones, step tones and the content palette. It must never be a per-widget colour editor. The defaults are the tokens as they stand. Until that page exists the theme is fixed.

## Anatomy

This section's anatomy is the set of design tokens and rules, in the order an implementer needs them.

### 1. Tonal hierarchy (chrome)

The chrome uses tones from one cool slate family, from darkest to lightest. Each level has one job. A new surface takes the level that matches its job and never invents an in-between tone.

| Level (darkest first) | Token | Job |
|---|---|---|
| 1 | `theme::METER_BG` | The deepest wells: meter channels and fader grooves. Nothing is darker. |
| 2 | `theme::LCD_BG` | Display wells: position, tempo, CPU, the hint bar, scopes, the output visualiser and the vertical zoom box. These are the "glass" of the desk. |
| 3 | `theme::PANEL_DARK` | Inset trays inside a panel: the segmented-switch tray, the menu/hint field in the toolbar, the ring around a labelled knob, the mini roll in the rack, off dots and lights, group boxes in editor pages, and egui's `extreme_bg_color` (text fields, scroll areas). |
| 4 | `theme::BACKGROUND` | The empty workspace, with `theme::watermark` faint in its corner. It is also the "ink" drawn on top of any lit fill: a glyph on an active button, text on a lit LED, text on a lit segment. |
| 5 | `theme::BLACK_KEY_ROW`, `theme::WHITE_KEY_ROW`; proposed `theme::LANE` | Editor canvases. Piano-roll rows alternate by key (`WHITE_KEY_ROW` / `BLACK_KEY_ROW`). Playlist lanes all share the proposed `theme::LANE`, the same tone as `WHITE_KEY_ROW`. These sit between the workspace and the panel, so a canvas reads as a recess in its window. |
| 6 | `theme::PANEL` | Window bodies, docked panels, the toolbar band (in both its one-band and two-band layouts) and window header rows (egui `panel_fill` and `window_fill`). Proposed `theme::HEADER` names this tone for headers. |
| 6½ | proposed `theme::HEADER_ACTIVE` | The focused window's header: `PANEL` lifted about halfway toward `RAISED`. A just-visible lift, the only in-between tone allowed. |
| 7 | `theme::STEP_B` / `theme::LCD_EDGE` / `theme::GRID_LINE` | The thinnest chrome lines: the edges of wells, the finest grid lines, separators, and the darker step group. One small step above the panel. |
| 8 | `theme::RAISED`; proposed `theme::MENU_BG` and `theme::STRIP_SELECTED` at this tone | Every pressable surface at rest: button fills, knob bodies, fader caps, unlit LEDs, egui `inactive` fills and `faint_bg_color`. Menus, popups and tooltips use `MENU_BG` (controls section), one step above `PANEL`, so they lift off the window they open over. The selected mixer strip uses `STRIP_SELECTED` (mixer section). |
| 9 | `theme::GRID_BEAT` | Beat lines in grids. |
| 10 | `theme::RAISED_HOVER`; proposed `theme::WINDOW_EDGE` at this tone | A pressable surface under the pointer. `WINDOW_EDGE` is the outer edge of a floating window, one step lighter than `RAISED` (it names the literal in `theme::apply` today). |
| 11 | `theme::STEP_A` | The lighter step group. The brightest chrome fill, so unlit steps still read as pads in a dark rack. |
| 12 | `theme::GRID_BAR` | Bar lines: the brightest line in a grid. |
| Text, dim | `theme::TEXT_DIM` | Captions, units, unfocused titles, glyphs of quiet toggles at rest (window buttons, header marks), menu and group headings. |
| Text | `theme::TEXT` | Titles of the focused window, names, values in ordinary fields, glyphs of tool and transport buttons at rest and under hover. |
| Text, display | `theme::PLAYHEAD` (proposed alias `theme::LCD_TEXT`) | Digits in LCDs and the live value under a dragged knob. The brightest text, used only for numbers that change. |

Rules:

- **Recess for showing, raise for pressing.** Anything that shows a value (display, meter, canvas, well) is darker than the panel around it. Anything that can be pressed is lighter than it. A surface that does both (the tempo LCD can be dragged) stays dark and says so in its hint.
- **One step only.** Hover lifts a surface by one level: `RAISED` becomes `RAISED_HOVER`, and an off step becomes the proposed `theme::STEP_HOVER`, one step lighter than its own group (never darker). Press and selection never jump more than two levels.
- **Lines are tones, not black.** Separators and edges use `theme::LCD_EDGE` (proposed role alias `theme::SEPARATOR`). A floating window's outer edge uses `WINDOW_EDGE`; the focused window's edge is `theme::ACCENT_DIM`.

### 2. Content tones (brighter than chrome)

- **The content palette** is `theme::PALETTE`: eight saturated, light colours, chosen by `theme::palette(index)` (it cycles). It is used for channel colours (lit steps, channel name buttons), audio clips, mixer strip colours and channel kinds in the browser.
- **The track palette** is `theme::track_color(0..16)`: the eight palette colours, then the same eight darkened. It is used for playlist track colours and pattern colours.
- **How users pick a colour.** Everywhere a colour is chosen (track, pattern, channel, note group), one shared picker shows the 16 track-palette colours as a grid of two rows of eight, with "No colour" under it. The current swatch carries an `ACCENT` ring; a hovered swatch a `TEXT` ring. A small dice button (`Icon::Dice`) picks a random swatch (own design).
- **Content is always brighter and more saturated than chrome.** Even the darkened row of the track palette stays brighter than `theme::STEP_A`. A content colour is never used for chrome. Chrome tokens are never used for content, except `theme::AUTOMATION` for automation content.
- **Content derivations** use the content colour itself, never a new hue:
  - **Clip title strip**: the clip colour at full strength.
  - **Clip body**: proposed `theme::CLIP_BODY`, the clip colour mixed most of the way toward `LANE`, so grid lines show through.
  - **Clip content** (waveform, note miniature): proposed `theme::CLIP_CONTENT`, the clip colour mixed toward `TEXT`, the brightest tone in the clip (playlist section).
  - **Uncoloured clip**: proposed `theme::CLIP_NEUTRAL`, between `RAISED_HOVER` and `TEXT_DIM` in tone. It names the `UNCOLOURED_CLIP` literal in `crates/musiccraft-ui/src/playlist.rs`.
  - **Selected clip or note**: proposed `theme::CLIP_SELECTED` / `theme::NOTE_SELECTED` (one tone), near `TEXT` and tilted faintly toward `ACCENT`. It is distinct from every palette colour.
  - **Muted content**: its colour at low strength, with the outline kept at partial strength.
  - **Ghost notes** (other channels in the piano roll): proposed `theme::GHOST_NOTE`, the note colour at very low strength.
- **Note colours** (piano-roll colour groups 1–16) should come from the same 16-entry track palette through `theme::track_color`, so a pattern, its channel and its notes can share a colour. Today they come from a separate hue wheel (see Gaps).

### 3. Signal colours (one meaning each)

| Token | Means | Reserved for |
|---|---|---|
| `theme::ACCENT` (teal) | On, active, live value | The play button while playing. The active tool. The open window's toggle glyph and underline. Knob value arcs. The fader cap's centre line. The normal range of meters. Enable, mute and slot lights when on (the ON LED of a mixer strip today). The selection outline, the keyboard focus ring and the primary selected mixer strip's edge. Links. The scope trace. The channel rack's selection switch. |
| `theme::ACCENT_DIM` | Chosen / pressed background; on but not heard | egui's selection fill and pressed (`active`) state, a toggled text tool ("Slide", "Porta"), the focused window's edge, and lights of tracks or channels silenced by another's solo (enabled but not heard). |
| `theme::HIGHLIGHT` (amber) | Current mode, singled out, caution | The lit segment of the pattern/song switch. The current pattern's name in a header. The STEP edit badge. Solo (today the lit "S" switch; target: a ring around the soloed light). The master strip's colour. The middle (hot) range of meters and the load bar. Loop and time-range shading at low strength. Settings notices. The save reminder. Never "on". |
| `theme::DANGER` | Recording, destruction, overload, error | The record button and armed dots. COUNT-IN and WAITING badges. Clip in meters. The load bar over its limit. "Delete" and "Clear…" menu items. Error text in the hint bar. The close glyph under hover. Pause markers. |
| `theme::AUTOMATION` | Automation | Automation clips and curves, automation entries in the playlist's clip list (with their glyph), controller names, ordinary markers. At reduced strength it draws automation previews inside pattern clips. |
| `theme::PLAYHEAD` | Time and live numbers | The playhead line and its ruler triangle. LCD digits. The live value caption of a labelled knob. At low strength, the playing-step column. |

Two strict rules:

- **Teal and amber never swap.** If a control shows "this one of several is selected or singled out", use amber. If it shows "this thing is switched on / running", use teal.
- **Red is only for record, delete, clip and error.** Mute is never red. A muted thing is dimmed, not coloured.

The palette's teal and amber are a step lighter than `ACCENT` and `HIGHLIGHT`, so a lit step never reads as a signal (a test in `theme.rs` keeps content and signal colours apart).

### 4. Disabled and dim

- A disabled control keeps its shape and place. Its text and glyph are drawn in the proposed `theme::TEXT_FAINT` (`TEXT_DIM` moved toward `PANEL`), with no hover change. Its fill stays `RAISED`. It is never hidden, so layouts do not jump. (A window button that cannot apply, such as maximize on a fixed-size window, is left out rather than greyed; main-window section.)
- "Dim" content (a muted track's name, an inaudible row, an empty slot's "empty") uses `TEXT_DIM`. "Off" content (a muted note or clip) uses its own colour at low strength. Never grey content out to `TEXT_DIM`: it would lose its identity.

### 5. Typography

- **Faces.** Only egui's bundled faces are used (no font files of our own and none from any other product). The proportional face is used for all ordinary text, the monospace face for changing numbers. `theme::fonts()` appends the monospace face as the last fallback for proportional text, so arrows and small triangles in labels draw. A test (`crates/musiccraft-ui/src/glyph_tests.rs`) checks that every non-ASCII character in a UI string exists in the bundled faces. Rule for new text: if a symbol is needed as an icon, draw it in `icons.rs` rather than relying on a font glyph.
- **Size classes.** Our own sizes, in egui points, as named constants in `theme`; no UI code uses a size literal:

| Class | Proposed name | Used for |
|---|---|---|
| Display | `theme::FONT_DISPLAY` (mono, the largest UI text) | LCD value line (position, tempo whole digits) |
| Title | `theme::FONT_TITLE` | Window header titles, the browser's tree roots, a channel window's name line |
| Body | egui body (default) | Menu items, buttons, combo boxes, list rows, track names |
| Label | `theme::FONT_LABEL` | Knob captions, track header names, mixer strip names (written vertically), piano-roll note names |
| Small | egui `.small()` | Main menu titles, the hint bar, group-box titles, menu headings |
| Caption | `theme::FONT_CAPTION` (the smallest) | LCD captions (BAR.BEAT.STEP, BPM, CPU), "empty" in mixer slots, unit marks |
| Mono small | `theme::FONT_VALUE` (mono) | The CPU percentage, the tempo's decimals, step and tick numbers, the time signature button, mixer level readouts |
| Readout | `theme::FONT_READOUT` | The tuner's note name only |
| Watermark | `theme::FONT_WATERMARK` (large, near-transparent) | `theme::watermark` only |

- **When to use mono:** any number that changes while you watch or drag it (positions, tempo, levels, percentages, the time signature, counts in number boxes), so the digits do not jitter. Names, words and static numbers (track numbers, ruler bar numbers) use the proportional face.
- **Weight.** Strong (bold) text is used only where weight is the cue:
  - main-menu titles, group-box titles and menu or dialog section headings;
  - mode badges (COUNT-IN, WAITING, STEP) and the lit segment of the pattern/song switch;
  - the active tab in a tabbed window (Settings);
  - the "current one" in a list where weight is the only mark: names of open folders in the browser, the current channel in the piano roll's ghost-channel panel.
  
  Window titles are not bold: the focused window's title goes from `TEXT_DIM` to `TEXT` instead (main-window section). Nothing else is bold.
- **Capitalisation:**
  - **ALL CAPS, small and bold:** LCD captions (BAR.BEAT.STEP, MIN:SEC, BPM, CPU), mode badges (COUNT-IN, WAITING, STEP, TAP), one-letter switches (S), the text LED ON (until the mixer replaces it with a light) and group-box titles inside editor pages (OSCILLATORS, FILTER, LFO, ENVELOPE, SAMPLE, PLUGIN).
  - **Main-menu titles** are in capitals today (FILE … HELP). Proposed (toolbar section): sentence case, like every other title.
  - **Sentence case:** everything else. That covers every window title, the browser's included ("Channel rack", "Piano roll — Lead", "Browser"), buttons ("+ New pattern", "+ Insert"), menu items ("Rename…", "Clone"), menu headings ("Control from"), knob captions ("Low gain", "Env amt"), hints and track names.
  - Lists do not use capitalised section headings. The playlist's clip list drops today's PATTERNS / AUDIO / AUTOMATION headings and marks each item's kind with its glyph (playlist section).
  - Lower case is not a style. User-given and preset names are shown exactly as typed.

### 6. Shape

- **Corner radii (our own values, three classes plus square):**
  - **Control**: tool and transport buttons, window toggles, LCDs, the segmented tray, scopes, clip bodies, group boxes.
  - **Cell**: step buttons, the hint bar frame, mixer strip frames, fader caps, notes, LEDs.
  - **Tiny**: swatches, meter wells, the fader groove, note corners at small zoom, menus, popups and tooltips.
  - **Windows** are square or take the Tiny radius, so tiled windows read as one surface and not as a stack of cards (main-window section).
  - Nothing else is fully square except grid cells and lane rows. Nothing is a pill except round dots, lights and knobs.
- **Borders:** one-point lines only. Wells (LCD, meter, groove, hint bar, zoom box) carry an `LCD_EDGE` inner edge, so a silent meter or an empty display is still visible. Raised controls carry **no** border; their tone alone separates them. The fader cap is the exception: it has a `TEXT_DIM` edge so it reads as a grip. Menus and popups carry a `WINDOW_EDGE` hairline.
- **No bevels, no gradients, no gloss, no textures.** Depth comes only from the tone hierarchy.
- **Shadows:** tiled and floating editor windows cast none. Menus, popups, tooltips and modal dialogs cast one faint shadow. Docked panels, buttons and clips cast none; the playlist's optional clip-shadow view setting (playlist section) is the one exception, off by default.
- **Group boxes** (how controls are grouped in editor pages; controls section): a box one step darker than the page (`PANEL_DARK`), Control radius, with its title in small, dim, bold capitals inside its top-left corner. There is no heading tab or band on the box edge. Every section uses this one style.
- **Menu headings** (controls section): a full-width row on a band one step darker than the menu (proposed `theme::MENU_HEADING`, the tone of `PANEL`), in small dim sentence-case text, not clickable.
- **Line weights** (our own, relative):
  - **Hairline**: grid lines, separators, edges, unity and 0 dB ticks.
  - **Icon stroke**: the standard glyph stroke.
  - **Heavy stroke**: handles inside glyphs (magnifier handle, slice blade, brush handle), knob arcs, the fader centre line, automation curves, the window-toggle underline.
  - **Playhead**: between hairline and heavy, in `PLAYHEAD`.
- **Grids** use three tiers plus row shading:
  - **Step / snap lines**: `theme::GRID_LINE`.
  - **Beat lines**: `theme::GRID_BEAT`.
  - **Bar lines**: `theme::GRID_BAR`.
  - **Rows**: `WHITE_KEY_ROW` and `BLACK_KEY_ROW` in the piano roll (by key). In the playlist all lanes share `LANE` and do not alternate; track separators are `GRID_LINE` hairlines.
  - **Optional alternate-bar shading** (proposed `theme::BAR_GROUP`): the lane tone lifted very slightly every other bar group, so long arrangements read in groups.
  - Lines are drawn under content, never over it. The playhead is drawn over everything.
- **Step buttons** alternate `STEP_A` and `STEP_B` by beat of the pattern's signature, so a 16-step row reads as four groups.

### 7. Icons (rules)

- **Drawing box:** each glyph is drawn into the button's rectangle inset by just under a fifth of its short side. Coordinates are fractions of that inset box (the `p(r, x, y)` helper), so a glyph scales to any button.
- **Single colour:** the glyph is painted in one colour passed by the caller. The state chooses the colour; the glyph never carries its own colour.
- **Stroke:** the standard icon stroke for outlines and the heavy stroke for at most one "handle" element. The finest stroke is reserved for interior detail (keyboard key seams, the rack glyph's unlit cells).
- **Filled vs outline:** solid objects and transport symbols are filled (play, stop, record, notes in the piano-roll glyph, the eraser's rubber half). Containers and tools are outlined (folder, select box, metronome body, magnifier lens). A glyph may combine outline and fill, but never more than two fill areas plus one outline.
- **One rule for button colour** (every section cites this):
  - **Rest:** square tool, option and transport buttons have a `RAISED` fill and a `TEXT` glyph. The record button is the one exception: its glyph stays `DANGER` at rest so it is always recognisable (toolbar section). Quiet toggles (window buttons) and header marks (menu, collapse, maximize, close) have no fill and a `TEXT_DIM` glyph.
  - **Hover:** a `RAISED_HOVER` fill appears and the glyph turns `TEXT`.
  - **Pressed:** `ACCENT_DIM` fill.
  - **Active:** in an `icons::button` the whole button fills with its role colour at slightly reduced strength (`ACCENT` for tools and play, `DANGER` for record) and the glyph turns `BACKGROUND`. In an `icons::toggle` (window buttons) the glyph turns `ACCENT` and a heavy `ACCENT` underline appears; a window that is open but behind others shows the underline only.
  - A button set to "nothing" (snap set to none, no scale chosen) keeps a `TEXT_DIM` glyph.
  - Glyph shapes never change with state, except a symbol that changes meaning: play becomes pause while playing (proposed), and the collapse chevron flips when collapsed.
- **Pickers in window headers** are square icon buttons the size of the tools (snap, scale, chord, placing source). The current value is named in the button's hint ("Snap: Beat") and ticked in its list. When the header has room, a short value text may follow the glyph; it is the first thing to fold away. The toolbar's global snap panel (glyph plus a dropdown showing the value) is the one place a value is always shown.
- **One concept, one glyph.** Every section uses the glyph named here for a shared idea (menu, snap, fold, zoom box, plugins, chevrons). A section may not draw its own variant.
- **Never:** text characters as icons (♪ ⟳ ▪ × ◀ ▶ ▼ ☰ in labels today: see Gaps), emoji, bitmaps, or any glyph drawn to resemble another product's artwork.

### 8. Names (glossary and style)

**Panels and windows (our names, from code; renames are proposals, marked):**

| Thing | MusicCraft name | Where in code |
|---|---|---|
| Top band | toolbar (menus, hint bar, transport, displays, window buttons) | `toolbar.rs` |
| One-line help under the menus | hint bar (shows hints, falls back to the status line) | `toolbar.rs::hint_bar` |
| Song arranger | Playlist (title); proposed "Playlist — *arrangement*" once arrangements exist | `playlist.rs` |
| Left column of the playlist | pattern list today, with sections PATTERNS, AUDIO, AUTOMATION and the filter tabs All · Pat · Aud · Aut; proposed name "Clip list", without headings (playlist section) | `playlist.rs` |
| Step sequencer window | Channel rack | `channel_rack.rs` |
| Note editor | Piano roll — *channel* | `piano_roll.rs` |
| Mixer window | Mixer — *track* (Mixer — Master when the master is selected) | `mixer.rs` |
| Mixer columns | insert tracks (strips); the first is Master; default names "Insert *n*" | `mixer.rs` |
| Effect list beside the strips | slots ("empty", "+ add effect") | `mixer.rs` |
| File and plugin tree | Browser (proposed title "Browser" in place of today's capitalised caption). Roots today: Current project, Controllers (*n*) and a plugin-tree root. Proposed (browser section): This project, Controllers (*n*), Instruments & effects, Presets, then user folders | `browser.rs`, `controllers.rs` |
| Channel window | "Channel settings" in menus and hints; title "*channel* — *track*" ("Lead — Insert 3") | `channel_settings.rs` |
| Effect window | title "*track* › *effect*" today, proposed "*effect* — *track*" ("Equaliser — Lead") | `effect_window.rs` |
| Automation window | Automation — *name* | `automation.rs` |
| Settings | Settings | `settings.rs` |
| Shortcut list | Keyboard shortcuts | `dialogs.rs` |
| Analyser | Spectrum analyser | `spectrum.rs` |
| Built-in wavetable synth | Antiserum (owner-chosen name) | `antiserum.rs` |
| Song/pattern switch | Pattern / Song (proposed, to replace today's PAT / SONG; "Ptn" where too narrow) | `toolbar.rs` |
| Position display | caption BAR.BEAT.STEP or MIN:SEC | `toolbar.rs` |
| Tempo display | caption BPM; TAP button beside it | `toolbar.rs` |
| Load display | CPU | `icons.rs::load_bar` |
| Playlist tools | Draw, Paint, Delete, Mute, Slice, Select, Zoom, Play (hints name each with its key, e.g. "Play (Y): click to play from there") | `playlist.rs` |
| Grid setting | Snap (values None, 1/4 step, 1/2 step, Step, Beat, Bar; proposed Global first) | `playlist.rs`, `piano_roll.rs`, `geometry.rs` |
| Rack header | group filter, pattern selector, swing, pattern length, graph editor toggle (labelled "Graph" today) | `channel_rack.rs` |
| Piano roll header | Slide, Porta, "Colour: channel" / "Colour *n*", scale ("No scale"), Velocity, event lane picker | `piano_roll.rs` |
| Mixer strip switches | ON (enable), S (solo), "*n* fx", route arrow | `mixer.rs` |
| Controller wording | "Controller learn" (toggle), "Add controller…" (menus) | `docs/spec/controls.md`, `docs/spec/controllers.md` |
| Default track names | "Track *n*" (playlist), "Insert *n*" (mixer) | — |

**Wording style:**

- **Labels** are one or two plain words in sentence case: nouns for things ("Snap", "Velocity"), verbs for actions ("Rename…", "Clone").
- **An ellipsis (…)** marks an item that opens a dialog or asks for more input.
- **"+ "** before a label marks an add button ("+ Insert", "+ New pattern").
- **Hints** (main-window section, following the existing hints in `playlist.rs`): the control's name, then its shortcut in parentheses, then a colon and what click, drag and right-click do, with middle dots between gestures ("Scroll bar: drag to scroll · drag an end to zoom"). Sentence case, one line, no full stop at the end. While dragging, the hint bar shows the live value in place of the description. The idle text is a short readiness line with the two most useful shortcuts.
- **Window titles:** the window's name, an em dash, then the context ("Piano roll — Lead", "Equaliser — Lead"). The "›" form is retired.
- **Number formats** in displays:

| Display | Format |
|---|---|
| Position | `bar.beat.step` (1-based) or `m:ss.ss` |
| Tempo | Whole BPM in the large size, then "." and three decimals in the small size (`128.000`). Today: two decimals in one size (see Gaps) |
| Load | Whole percent with "%" |
| Mixer levels | Signed dB with one decimal and no unit (the strip says dB once) |
| Swing, volume and other 0–1 amounts | Whole percent ("35%") |
| Pan | Whole percent with "left" / "right", or "centred" |
| Pattern length | Integer steps |
| Time signature | `n/d` |
| Pattern selector | "*number*  *name*" |
| Note names | Letter, then sharp, then octave (C5); C4 is marked on the keyboard |
| Track numbers | 1-based. The master is "M" in its number cell and "Master" in its name. |
| Zoom while dragging | "Bars 5–12" (1-based, en dash); "Row height 140%" |

**Names we never use** (kinds, not a list of the names themselves):

1. The original product's name, abbreviation or logo-word, and its maker's name, anywhere in the UI, in file extensions we write, in default project or preset names, or in hints. The only exception is the import format's plain file extension in the import dialog's file filter, which names a file type and is not branding.
2. The names of its bundled instruments, effects, samplers, tools and preset packs, including near-spellings. Our instruments get plain descriptive names ("Synth", "Drum synth: kick", "Zone sampler", "Equaliser") or owner-chosen dictionary words (Antiserum).
3. Its coined labels for interface parts and features: invented compound words, branded names for its hint area, its plugin wrapper, its plugin list, its channel filters, its routing or recording modes, its mode-switch captions and its theme presets. Where a generic DAW term exists we use it (playlist, piano roll, mixer, channel rack, browser, step sequencer, hint bar, pattern, clip, track, insert, send, master, transport, snap, tempo, metronome). Where none exists we describe it plainly ("group filter", "graph editor", "Instruments & effects", "Controller learn").
4. Its colour names and theme preset names.
5. Its menu wording copied verbatim. Our menu items are our own phrasing of what the command does.

### 9. Motion and feedback

- **Hover:**
  - Raised surfaces lift one level (`RAISED` to `RAISED_HOVER`). Glyphs of quiet toggles and header marks go from `TEXT_DIM` to `TEXT`; the close mark goes to `DANGER`.
  - Hovering a control writes its hint to the hint bar at once (no delay). egui tooltips are not used for hints.
  - A labelled knob swaps its caption for its formatted value in `PLAYHEAD` while hovered or dragged.
- **Press:**
  - Buttons show `ACCENT_DIM` while held.
  - Custom buttons act on release. Steps act on press and paint as the pointer drags across them.
- **Drag:**
  - The dragged control keeps its hover tone for the whole drag, even when the pointer leaves it.
  - The value shows live in the control's own display (knob caption, LCD digits) and in the hint bar.
  - A drag is one undo step.
- **Record** (toolbar section; own design):
  - **Armed:** the record button is steadily filled `DANGER`.
  - **Recording** (armed and playing): the fill pulses slowly between `DANGER` and `DANGER` dimmed, about once a second. Armed track dots in the playlist and mixer pulse the same way.
  - **Count-in or waiting for the first note:** the record button pulses, and the time display shows a dim COUNT-IN or WAITING in place of its digits.
  - Today the button is steadily lit whenever armed, so recording looks the same as armed (see Gaps).
- **Save reminder** (toolbar section, own design): the save button's glyph pulses slowly in `HIGHLIGHT` when the project has gone unsaved for a while.
- **Playing step** (proposed, own design): while the transport plays a pattern, the channel rack lays a column of the proposed `theme::STEP_PLAYING` (`PLAYHEAD` at low strength) over the current step. Lit steps in that column lift one step brighter. Today there is no marker.
- **Playhead:**
  - It moves continuously, with no step-by-step jumps.
  - Its ruler triangle is filled `PLAYHEAD`.
  - When following playback is on, the view pages ahead of it.
- **Meters:**
  - Levels rise instantly and fall smoothly at a fixed rate (about 24 dB a second; `UiState::update_meters` in `crates/musiccraft-ui/src/lib.rs`).
  - Colour by level: `ACCENT` in the normal range, `HIGHLIGHT` in the hot range and `DANGER` at or above full scale.
  - Proposed: a thin peak-hold line that holds for about a second, then falls. A clip mark stays lit at the top of the well until clicked.
- **Selection changes** are instant (no fades). Windows open and close instantly; collapse, maximize and auto-hide may slide very briefly (main-window section) unless animations are off. The only other animations are the pulses, the meter fall and the playhead.

## Icons

**Existing icons in `crates/musiccraft-ui/src/icons.rs`** (17 `Icon` variants, all our own drawings), plus the two header marks drawn in `chrome.rs`:

| Where | What it does | Our glyph |
|---|---|---|
| Transport | Play / stop (`Icon::Play`) | A filled triangle pointing right, its left edge slightly in from the box edge |
| Transport | Stop and rewind (`Icon::Stop`) | A filled square with slightly rounded corners, inset in the box |
| Transport | Record (`Icon::Record`) | A filled circle a little smaller than the box |
| Toolbar option (planned) | Metronome (`Icon::Metronome`) | An outlined trapezoid, narrow at the top, with one slanted arm line from low centre to the upper right |
| Toolbar option | Typing keyboard as piano (`Icon::Keyboard`) | A rounded rectangle outline with four thin vertical key seams and three short filled black keys in the upper half |
| Window buttons | Playlist (`Icon::Playlist`) | Three filled rounded bars in three rows, staggered left and right like clips on tracks |
| Window buttons | Channel rack (`Icon::ChannelRack`) | A 4 × 3 grid of small cells: a diagonal run of them filled, the rest thin outlines |
| Window buttons | Piano roll (`Icon::PianoRoll`) | A filled tall bar at the left (the keyboard) and three filled short rounded bars at different heights to its right |
| Window buttons | Mixer (`Icon::Mixer`) | Three thin vertical lines, each with a small filled cap at a different height |
| Window buttons | Browser (`Icon::Browser`) | A folder outline: a body with a raised tab on its upper left |
| Editor tools | Draw (`Icon::Pencil`) | A slanted outlined bar from upper right to lower left ending in a small filled tip |
| Editor tools | Paint (`Icon::Brush`) | A heavy slanted handle line ending in a filled wedge of bristles at the lower left |
| Editor tools | Delete (`Icon::Eraser`) | A tilted square (diamond) outline, its lower-left half filled, resting above a baseline |
| Editor tools | Select (`Icon::Select`) | A dashed rectangle outline |
| Editor tools | Mute (`Icon::Mute`) | A filled speaker (box plus cone) with a small cross to its right. Its doc comment says "slash"; the drawing is a cross (see Gaps) |
| Editor tools | Slice (`Icon::Slice`) | A wide outlined bar crossed by a heavy slanted blade line |
| Editor tools | Zoom (`Icon::Zoom`) | A circle outline (lens) with a heavy handle to the lower right |
| Window header, left (in `chrome.rs`, today) | Collapse / expand | A small filled triangle pointing down (expanded) or right (collapsed). Replaced by `Icon::Fold` at the right (below) |
| Window header (in `chrome.rs`) | Close | Two crossing diagonal strokes, slightly lighter than the icon stroke |

**Icon slots other sections need that do not exist yet.** Each becomes a new `Icon` variant. The names are proposed, and every glyph is our own:

| Where | What it does | Our glyph |
|---|---|---|
| Every window header, far left | Window menu | `Icon::Menu`: three equal short horizontal bars, evenly stacked. The single menu glyph for every window (replaces the ☰ character) |
| Header, after the main tool group | Play this window's content | Existing `Icon::Play`, small; `Icon::Pause` while playing |
| Transport | Pause (play while playing) | `Icon::Pause`: two upright filled bars |
| Lists, combo boxes, submenus, selector titles, scroll-bar ends | Opens a list / submenu; steps a scroll | `Icon::ChevronDown`, `Icon::ChevronRight`, `Icon::ChevronLeft`, `Icon::ChevronUp`: two short strokes meeting in a point |
| Header, when tools do not fit | Shows the folded tools | `Icon::More`: two small right chevrons side by side |
| Header, right-hand window buttons | Collapse to header / expand | `Icon::Fold`: an upward chevron over a short horizontal bar (expanded); a downward chevron over the bar (collapsed). It replaces the filled triangle at the left of `chrome.rs` |
| Header, right-hand window buttons | Maximize / restore | `Icon::Maximize`: four small L-shaped corner brackets opening outward; `Icon::Restore`: the same brackets turned inward |
| Header, far right | Close | `Icon::Close`: the current cross moved from `chrome.rs` |
| Header (future) | Detach to its own desktop window | `Icon::PopOut`: a square outline open at the upper right, with a short diagonal arrow leaving through the gap |
| Header of a detached window | Return to the main window | `Icon::DockBack`: a square outline with a short arrow entering from the upper-right corner, pointing at the centre |
| Scroll bars (playlist, piano roll, event editor) | Vertical zoom box | `Icon::ZoomBox`: two horizontal lines in the small square; the gap between them grows with the zoom, almost touching at the minimum and at the top and bottom edges at the maximum |
| Scroll-bar thumb ends | Zoom grips | Two very short parallel vertical ticks inset at each end of the thumb (drawn by the scroll bar) |
| Playlist / piano roll / event editor header, toolbar | Snap | `Icon::Snap`: two short lines meeting at a right angle (a grid corner) with a filled dot in the corner. The one snap glyph for every section |
| Transport / sample editor | Loop on / off | `Icon::Loop`: two curved arrows chasing each other round a circle |
| Toolbar option | Loop recording | `Icon::LoopRecord`: a three-quarter arc ending in an arrowhead around a small filled dot |
| Toolbar option | Wait for input before recording | `Icon::Wait`: an hourglass, two filled triangles meeting tip to tip between short top and bottom bars |
| Toolbar option | Count-in | `Icon::CountIn`: three short vertical ticks rising left to right, then a small filled play triangle |
| Toolbar option | Blend (overdub) recording | `Icon::Blend`: two rounded rectangles offset diagonally, the back one outlined and the front one filled |
| Toolbar option | Follow playback | `Icon::Follow`: a vertical line capped by a small down triangle, with two short motion lines to its left |
| Toolbar option | Step edit | `Icon::StepEdit`: a short filled note bar followed by a right chevron |
| Toolbar option (future) | Controller learn | `Icon::Link`: a small knob (circle with a pointer line) joined by a short line to a small filled square |
| Toolbar utility | Undo / redo | `Icon::Undo`: a half-circle arc ending in a left-pointing arrowhead; `Icon::Redo`: mirrored |
| Toolbar utility | Save | `Icon::Save`: a down arrow ending above an open tray (a U-shaped line) |
| Toolbar utility | Export audio | `Icon::Export`: three waveform bars followed by a right arrow |
| Toolbar utility | Record audio | `Icon::RecordAudio`: three waveform bars with a small filled circle at the lower right |
| Toolbar utility, sample rows, browser audio files | Sample editor / audio | `Icon::Wave`: a centre line crossed by five vertical bars of different heights (replaces the ♪ character) |
| Toolbar utility | Help | `Icon::Help`: a circle outline with a question mark inside |
| Window buttons, browser plugin rows | Plugins | `Icon::Plugins`: a 2 × 2 grid of rounded squares, three filled and one outlined. The one glyph for "plugins" everywhere |
| Window buttons | Tap tempo | `Icon::Tap`: a filled rounded square (pad) with two concentric arcs above it |
| Window buttons | Window layout menu | `Icon::Layout`: a rectangle split into a narrow left column and two stacked right panes |
| Toolbar corner | Open the spectrum analyser | `Icon::Expand`: a short diagonal arrow to the upper right |
| Settings | Settings | `Icon::Gear`: a circle with six short teeth and a hollow centre |
| Browser header | Fold all folders | `Icon::FoldAll`: an upward chevron over two short bars |
| Browser header | Refresh | `Icon::Refresh`: about three quarters of a ring ending in a small arrowhead (replaces the ⟳ character) |
| Browser tabs and rows | Favourite | `Icon::Star`: a five-pointed star outline; filled when starred |
| Browser rows | Instrument / effect / preset | `Icon::Keyboard` (small); `Icon::Effect`: a square outline with an S-curve from corner to corner; `Icon::Preset`: a circle outline with a pointer line to the upper right (a knob) |
| Browser, clip list | Pattern | `Icon::Pattern`: a small square outline with three short dashes inside at different heights |
| Clip list, playlist | Automation | `Icon::Automation`: a smooth curve rising to the upper right, ending in a small filled diamond (replaces "~") |
| Browser rows | Recent / history | `Icon::Clock`: a circle with two hands |
| Rack, mixer, playlist lights | Locked state | `Icon::Lock`: a padlock, a rounded rectangle body under an arch |
| Rack, plugin windows | Missing sample or plugin | `Icon::Warning`: a triangle outline with a short bar and a dot inside |
| Rack header | Graph editor toggle | `Icon::Graph`: a baseline with four thin stems of different heights, each topped by a small dot |
| Rack, playlist, mixer | Add | `Icon::Plus`: two crossing strokes of equal length |
| Number boxes | Step up / down | `Icon::Spin`: a tiny up chevron over a tiny down chevron |
| Preset selector | Previous / next preset | `Icon::ChevronLeft` / `Icon::ChevronRight` in small tiles (replace the ◀ ▶ characters) |
| Mixer header | Strip details / slot panel toggles | `Icon::Strip`: a tall narrow outline with its lower third filled; `Icon::Slots`: a wide outline with its right third filled |
| Mixer strip | Invert polarity | `Icon::Phase`: a small circle with a diagonal stroke through it, extending past the circle |
| Mixer strip | Swap left and right | `Icon::Swap`: two short horizontal arrows one above the other, pointing opposite ways |
| Mixer input / output | Input / output | `Icon::In` (planned) / `Icon::Out` (existing): a small circle with a short arrow entering from the left / leaving to the right, drawn before the slot panel's Out row and each send row |
| Piano roll, playlist tools | Audition / play from here | `Icon::Audition`: a play triangle with one short sound arc to its right |
| Playlist tools | Slip content inside a clip | `Icon::Slip`: a wide outlined rectangle holding a short double-headed horizontal arrow |
| Piano roll header | Ghost notes | `Icon::Ghost`: two horizontal bars offset diagonally, the rear one dashed and the front one solid |
| Piano roll header | Fit to scale | `Icon::Scale`: a staircase of three steps rising left to right |
| Channel window tabs | Sound / Envelope / Functions / Info | `Icon::Sine` (one sine cycle), `Icon::Envelope` (a filled rise–fall–shelf–fall silhouette), `Icon::Steps` (three small squares climbing), `Icon::Info` (a circle with a bar and a dot) |
| Plugin windows, sample box | Load a file | `Icon::FolderAdd`: the `Icon::Browser` folder with a small plus at the lower right |
| Browser preview strip | Preview volume | `Icon::Speaker`: the speaker body of `Icon::Mute` without the cross, with one small arc in front |
| Ruler | Playhead marker | A small filled down triangle (drawn by the ruler, not an `Icon`) |
| Grips and resize corners | Drag to resize | `Icon::Grip`: three small dots in a line; `Icon::ResizeCorner`: three short diagonal strokes shortening toward the corner |
| Colour picker | Random colour | `Icon::Dice`: a rounded square with three dots on its diagonal |
| Device lists | Device on | `Icon::Power`: an open circle with a short vertical bar through its gap |
| Sample editor | Cut / copy / paste / reverse / fade | `Icon::Cut` (two small loops with crossing blades), `Icon::Copy` (two overlapped rectangle outlines), `Icon::Paste` (a rectangle with a clip tab), `Icon::Reverse` (two opposite horizontal arrows), `Icon::FadeIn` / `Icon::FadeOut` (a filled ramp rising / falling) |

## Colour roles

| Element | Token(s) from theme.rs (or proposed new token name) | Relative tone and state changes |
|---|---|---|
| Empty workspace | `theme::BACKGROUND`; watermark proposed `theme::WATERMARK` (`TEXT` almost transparent) | Darker than every panel |
| Toolbar band, window bodies | `theme::PANEL` | Base chrome level, in both toolbar layouts |
| Window header row | proposed `theme::HEADER` (= `PANEL`) / `theme::HEADER_ACTIVE` (`PANEL` lifted halfway to `RAISED`) | Lifts slightly on the focused window; title `TEXT_DIM` → `TEXT` |
| Floating window edge | proposed `theme::WINDOW_EDGE` (one step lighter than `RAISED`) | Focused window: `ACCENT_DIM` |
| Menus, popups, tooltips | proposed `theme::MENU_BG` (the tone of `RAISED`), edge `WINDOW_EDGE`; headings proposed `theme::MENU_HEADING` (the tone of `PANEL`) with `TEXT_DIM` text | One step above the window they open over; faint shadow |
| Separators | proposed `theme::SEPARATOR` (= `LCD_EDGE`) | One step above `PANEL` |
| Inset trays, text fields, group boxes | `theme::PANEL_DARK` | One step below `PANEL` |
| LCD wells, hint bar, scope, zoom box | `theme::LCD_BG`, edge `theme::LCD_EDGE` | Two steps below `PANEL`; zoom-box lines `TEXT_DIM`, `ACCENT` while dragged |
| LCD digits | `theme::PLAYHEAD` (proposed alias `theme::LCD_TEXT`) | Brightest text; small digits (tempo decimals) one step dimmer |
| LCD captions | `theme::TEXT_DIM` | — |
| Meter well, fader groove | `theme::METER_BG`, edge `theme::LCD_EDGE` | Darkest surface |
| Meter fill | `theme::ACCENT` → `theme::HIGHLIGHT` → `theme::DANGER` by level | Proposed peak line: `TEXT`; clip mark: `DANGER` |
| Tool / option / transport button at rest | `theme::RAISED` fill, glyph `theme::TEXT` (record: glyph `DANGER`) | Hover: `RAISED_HOVER`, glyph `TEXT`. Pressed: `ACCENT_DIM` |
| Tool / transport button active | Role colour at slightly reduced strength (`ACCENT` for play and tools, `DANGER` for record); glyph `theme::BACKGROUND` | Recording: the record fill pulses |
| Window toggle | Closed: glyph `TEXT_DIM`, no fill. Open: glyph and underline `theme::ACCENT` | Open but behind: underline only. Hover: `RAISED_HOVER` fill |
| Header marks (menu, collapse, maximize, close) | Glyph `TEXT_DIM`, no fill | Hover: `TEXT` on `RAISED_HOVER`; close under hover: `DANGER`. Menu open: pressed, glyph `ACCENT` |
| Segmented switch (pattern/song) | Tray `PANEL_DARK`; lit segment `theme::HIGHLIGHT` with bold `BACKGROUND` text | Unlit segment: `TEXT_DIM` text, no fill |
| LED with text (ON, today) | On: `ACCENT` with `BACKGROUND` text; off: `RAISED` with `TEXT_DIM` text | — |
| Round light (mute / enable / slot) | On: `ACCENT`; off: `PANEL_DARK` with a `TEXT_DIM` ring | Soloed: an outer `HIGHLIGHT` ring. Silenced by another solo: `ACCENT_DIM`. Locked: a padlock or bar in `TEXT_DIM` |
| Solo "S" switch (today) | Off: `TEXT_DIM`; soloed: `HIGHLIGHT` | Stays until the solo-on-the-light proposals in the mixer and playlist sections are accepted |
| Knob | Body `RAISED`; arc `ACCENT`; pointer `TEXT`; track `PANEL` (labelled knob) | Hover/drag: caption becomes the value in `PLAYHEAD` |
| Fader | Groove `METER_BG`; cap `RAISED` with a `TEXT_DIM` edge and an `ACCENT` centre line; unity tick `TEXT_DIM` | Hover/drag: cap `RAISED_HOVER` |
| Mixer strip | Rest: `PANEL_DARK` frame. Selected: proposed `theme::STRIP_SELECTED` (= `RAISED`); the primary selection adds a thin `ACCENT` edge | Master strip colour: `HIGHLIGHT` |
| Step off | `theme::STEP_A` / `theme::STEP_B` by beat | Hover: proposed `theme::STEP_HOVER` (one step lighter than its group) |
| Step on | `theme::palette(channel)` | In the playing column: one step brighter |
| Playing step column | proposed `theme::STEP_PLAYING` (`PLAYHEAD` at low strength) | — |
| Grid lines | `theme::GRID_LINE` / `GRID_BEAT` / `GRID_BAR` | Bar line is brightest |
| Piano-roll rows | `theme::WHITE_KEY_ROW` / `theme::BLACK_KEY_ROW` | Proposed scale rows: `theme::SCALE_ROW` (white row lifted slightly) |
| Piano keyboard | proposed `theme::KEY_WHITE` (near `TEXT`), `theme::KEY_BLACK` (near `BACKGROUND`), `theme::KEY_SEAM` | Replaces literals in `piano_roll.rs` |
| Playlist lane | proposed `theme::LANE` (= `WHITE_KEY_ROW`), optional `theme::BAR_GROUP`; separators `GRID_LINE` | Lanes do not alternate |
| Clip title strip | `theme::track_color(n)` / `theme::palette(n)` / proposed `theme::CLIP_NEUTRAL` | Hover: one step lighter |
| Clip body / content | proposed `theme::CLIP_BODY` (clip colour toward `LANE`), `theme::CLIP_CONTENT` (clip colour toward `TEXT`) | Muted: every part at low strength |
| Note | Proposed: note colour from `theme::track_color(group)` | Selected: proposed `theme::NOTE_SELECTED`; muted: low strength; ghost: proposed `theme::GHOST_NOTE` |
| Selected clip | proposed `theme::CLIP_SELECTED` (same tone as `NOTE_SELECTED`), strip and outline | The body keeps its colour |
| Rubber-band selection | proposed `theme::SELECTION_FILL` (`ACCENT` at very low strength), outline `ACCENT` | — |
| Text selection, focus ring | `theme::ACCENT_DIM` fill, `theme::ACCENT` stroke | — |
| Loop / time range | `theme::HIGHLIGHT` at low strength (fill) and partial strength (edges) | — |
| Playhead | `theme::PLAYHEAD` | — |
| Automation clip, curve, list items | `theme::AUTOMATION`; preview inside patterns at reduced strength; proposed `theme::AUTOMATION_FILL` under curves | — |
| Record, armed, errors, delete items | `theme::DANGER` | Armed dots and the record fill pulse while recording |
| Current pattern name, STEP badge, save reminder | `theme::HIGHLIGHT` | Save reminder pulses slowly |
| Track / pattern / channel colour picker | 16 swatches of `theme::track_color(0..16)` in two rows of eight, "No colour" below | Hover swatch: `TEXT` one-point ring; current: `ACCENT` ring |
| Disabled text and glyphs | proposed `theme::TEXT_FAINT` (`TEXT_DIM` toward `PANEL`) | No hover change |
| Scroll bars | proposed `theme::SCROLL_RAIL` (`PANEL_DARK`), `theme::SCROLL_THUMB` (`RAISED`) | Hover `RAISED_HOVER`; drag `ACCENT_DIM` |
| Past the song end | proposed `theme::PAST_END` (lane darkened) | — |

## Labels and wording

- **Main menus:** today FILE, EDIT, ADD, PATTERNS, VIEW, OPTIONS, TOOLS, HELP in small bold capitals (`toolbar.rs::menus`). Proposed (toolbar section): sentence case, "File · Edit · Add · Patterns · View · Options · Tools · Help". Menu items are sentence case and our own phrasing. "Undo *action*" names the step it undoes.
- **Group-box titles** in editor pages: small bold capitals in `TEXT_DIM` inside the box's top-left corner (OSCILLATORS, FILTER, LFO, ENVELOPE in the synth; SAMPLE, PLUGIN in channel windows).
- **Menu headings:** small dim sentence-case text on a heading band ("Control from", "Value").
- **List headings:** none. Today's PATTERNS / AUDIO / AUTOMATION headings in the playlist's left column are dropped (playlist section).
- **Window titles**, existing: "Channel rack", "Playlist" (with "Arrangement" in `TEXT_DIM`), "Piano roll — *channel*", "Mixer — *track*", "*channel* — *track*", "*effect* — *track*", "Automation — *name*", "Settings", "Keyboard shortcuts", "Spectrum analyser". Proposed renames: the browser's capitalised caption → "Browser"; "Playlist" → "Playlist — *arrangement*" once arrangements exist.
- **Display captions:** BAR.BEAT.STEP, MIN:SEC, BPM, CPU.
- **Switches:** today PAT / SONG, TAP, ON, S, "Graph", "Slide", "Porta", "Clip", "Snap". Proposed renames: PAT / SONG → "Pattern" / "Song" (toolbar section); "Graph" becomes an icon toggle named "Graph editor" in its hint (channel-rack section); ON becomes a round light (mixer section).
- **Group filter:** "Group filter" in hints, "No group" for the unfiltered choice (channel-rack section). Today's hint uses a coined term (see Gaps).
- **Controllers:** "Controller learn" for the toolbar toggle and "Add controller…" in menus.
- **Add buttons:** "+ Insert", "+ New pattern", "+ add effect" (proposed "+ Add effect", see Gaps), "+ Add folder…", "+ WAV file…".
- **Placeholders:** "Search…" in the search field, "empty" for an empty slot, "(custom)" for unsaved presets, "—" for no value.
- **Hint style:**
  - Name, shortcut in parentheses, colon, then what each gesture does, separated by middle dots. One line, sentence case, no closing full stop.
  - Use "click", "drag", "right-click", "double-click", "scroll", "Shift", "Ctrl", "Alt" exactly. Name other windows by our names.
  - Errors in the hint bar are plain statements of what to do ("select a time range first: Shift+drag the playlist ruler"), in `DANGER`.
  - The idle text is "Ready." followed by the most useful shortcuts.
- **Units:**
  - Shown in the caption or once per strip, never repeated beside every value.
  - Signed values always show their sign (+0.0).
  - Use the en dash for ranges (F5–F9, 1–16) and the true minus sign in prose.
- **Spelling:** British English in labels ("Colour", "Favourites", "analyser"), matching the existing "Colour" labels. The owner settled the two exceptions on 2026-10-22: "Spectrum analyser" and "Equaliser".

## States and feedback

| State | How it looks |
|---|---|
| Rest | Tool, option and transport buttons: `RAISED` fill, `TEXT` glyph (record: `DANGER` glyph). Quiet toggles and header marks: no fill, `TEXT_DIM` glyph |
| Hover | One tone lighter (`RAISED_HOVER`, `STEP_HOVER`); glyphs `TEXT`; the close mark `DANGER`; the hint bar shows the hint at once |
| Pressed | `ACCENT_DIM` fill; custom buttons act on release |
| Active / on | Tool or transport: filled in its role colour with a `BACKGROUND` glyph. Window toggle: `ACCENT` glyph and underline. Light or LED: lit `ACCENT`. Text toggle: `ACCENT_DIM` fill |
| Current mode (one of several) | `HIGHLIGHT` fill with bold `BACKGROUND` text (the pattern/song switch), or `HIGHLIGHT` text (current pattern) |
| Selected (content) | Notes and clips: `NOTE_SELECTED` / `CLIP_SELECTED`. List rows: `ACCENT_DIM`. Rack channel: filled selection switch and lit name button. Mixer strip: lifted to `STRIP_SELECTED` with a thin `ACCENT` edge for the primary selection (mixer section) |
| Muted | Content at low strength; track names `TEXT_DIM`; the enable or mute light dark with a `TEXT_DIM` ring. Never red |
| Solo | Today: the "S" switch lit `HIGHLIGHT`. Target (proposed in the mixer, playlist and channel-rack sections): the soloed light gains a `HIGHLIGHT` ring, and lights silenced by the solo show `ACCENT_DIM`, distinct from muted (dark) |
| Disabled | `TEXT_FAINT` text and glyph, no hover, fill unchanged, never hidden |
| Playing | Play button lit `ACCENT` (proposed: shows the pause glyph); playhead moves; playing step column marked; meters live |
| Armed | Record button filled `DANGER`; armed track dots filled `DANGER` |
| Recording | Record fill and armed dots pulse; count-in or waiting: record button pulses and COUNT-IN / WAITING replaces the time digits; STEP badge in `HIGHLIGHT` |
| Dragging | The dragged control keeps its hover tone; its value shows live in `PLAYHEAD` and in the hint bar; dragged clips and notes are drawn at their target with the source left in place at low strength until release |
| Focus (keyboard) | egui focus stroke in `ACCENT`; the focused window's edge `ACCENT_DIM`, header `HEADER_ACTIVE`, title `TEXT` |
| Error | Hint bar text `DANGER`; the field that failed keeps a one-point `DANGER` edge until edited (proposed) |
| Overload | Meter top `DANGER`; load bar `DANGER` |

## MusicCraft today: gaps

Ordered by visual impact. Where a gap changes layout or behaviour fixed in a spec, that spec is updated first.

1. **Many icon slots do not exist.** `Icon` now has the header glyphs (menu, fold, unfold, maximize, restore, close, more) and refresh. Chevrons, zoom box, pause, snap, loop, gear, star, lock, warning, plus and the rest listed above are still missing.
2. **Toolbar band tone.** The one-band toolbar fills its band with `PANEL_DARK`, darker than window headers, and the two-band fallback uses two tones. Target: one `PANEL` band in both layouts, with only the menu/hint field and the display wells recessed (toolbar section).
3. **Disabled controls fade the egui way.** `TEXT_FAINT` exists in `theme` but disabled widgets still rely on egui's default fading; draw their text and glyphs in `TEXT_FAINT` and keep fills unchanged.
4. **Other sections to align with this one.** The dialogs section's heading tabs on section boxes should become the title-inside-the-box group style; its magnet snap glyph and double-arrow zoom-box glyph should become `Icon::Snap` and `Icon::ZoomBox`.
5. **No theme adjustment.** The theme is fixed in `theme.rs`. A future Settings → Theme page (own design) would shift token groups as a whole.

## Sources

- **Manual, compared for general feel only** (text read and screenshots looked at; nothing sampled, traced or measured, no wording copied):
  - `basics_interface.htm`, "FL Studio User Interfaces": the overview and its main screenshot `img_shot/basics_interface.png`, used for the overall impression only (a dark, compact desk; content such as clips, steps and notes brighter than the window chrome; a hint area under the menus; window-menu buttons at the top left of windows). Its sections "Hint Panel" (hints shown at once in one place), "Window menus" (every window has a menu button), "Horizontal Scroll-Zoom Bars & Vertical Zoom" with `img_shot/basics_interface_scrollbars.png` (that a vertical zoom box and zoom grips exist; our glyphs follow the main-window section), "Themes" and "Color Selector" with `img_shot/basics_interface_color.png` (that colours are picked from a picker offering a random choice and a reset).
  - `envsettings_themes.htm`, "System Settings - Themes", "Options": only the general idea that a theme is adjusted as a whole (tint, colour strength, brightness, contrast, text brightness) plus a few role colours; `img_shot/system_settings_themes.png` looked at for the layout of such a page.
  - The icon-slot list gathers the slots the other sections found in the manual pages they name in their own Sources (toolbar, main window, browser, channel rack, piano roll, playlist, mixer, plugin windows, event editor). Only the slot (what the button does and where it sits) was taken; every glyph here is ours.
- **Repository:**
  - `crates/musiccraft-editors/src/theme.rs` (all tokens and their order, palette, fonts, `apply`, watermark);
  - `crates/musiccraft-ui/src/icons.rs` (all 17 icons and the button, toggle, LCD, segmented, scope and load-bar widgets);
  - `crates/musiccraft-editors/src/widgets.rs` (knobs, fader, dot, meter);
  - `crates/musiccraft-ui/src/chrome.rs` (header marks);
  - `crates/musiccraft-ui/src/toolbar.rs`, `playlist.rs`, `piano_roll.rs`, `channel_rack.rs`, `mixer.rs`, `browser.rs`, `controllers.rs`, `lib.rs` (meter fall), `geometry.rs` (snap labels), `settings.rs`, `glyph_tests.rs` and `crates/musiccraft-editors/src/antiserum.rs`;
  - `docs/clean-room.md`, `docs/spec/window-layout.md`, `docs/spec/controls.md` and `docs/spec/controllers.md`;
  - `docs/screenshots/playlist-mixer.png`, `channel-rack.png`, `piano-roll.png`, `synth-and-eq.png` and `antiserum.png`;
  - the other sections of this guide (main window, toolbar, controls, browser, channel rack, piano roll, playlist, mixer, plugin windows, dialogs), for shared token, glyph and label decisions.
- **Own design:**
  - the tonal-hierarchy order and its rules;
  - the meanings reserved for teal, amber, red, automation and playhead, and the solo and silenced-by-solo roles;
  - all proposed token names and their relations to existing tokens;
  - the size-class names, the weight rule and the capitalisation rules;
  - the radius classes, shadow rule, group-box and menu-heading styles, line-weight tiers and grid tiers;
  - every icon drawing, the icon rules and the single button-colour rule;
  - the record pulse, save reminder, playing-step column, peak line and clip mark;
  - the shared colour picker and the Theme page proposal;
  - the glossary's proposed renames, the number formats and all wording.
