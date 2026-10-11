# Spec: plugin hosting (CLAP)

## Behavior

- MusicCraft hosts **CLAP** plugins as channel instruments and as mixer insert
  effects. A plugin is referenced by its CLAP plugin id (for example
  `com.example.synth`); the file it comes from is found by scanning, so projects
  move between machines.
- **Scanning** looks for `*.clap` files in the standard CLAP locations
  (`$HOME/.clap`, `/usr/lib/clap`, `/usr/local/lib/clap` on Linux;
  `~/Library/Audio/Plug-Ins/CLAP` and `/Library/Audio/Plug-Ins/CLAP` on macOS;
  `%COMMONPROGRAMFILES%\CLAP` and `%LOCALAPPDATA%\Programs\Common\CLAP` on
  Windows) and in every directory listed in `CLAP_PATH`. Each file can hold
  several plugins. A plugin whose features include `instrument` is offered as an
  instrument; one with `audio-effect` as an effect.
- **Instruments**: `channel.add_plugin <plugin-id> [name]` adds a channel whose
  sound comes from the plugin. Notes, step-sequencer hits, live MIDI and
  previews reach the plugin as CLAP note events (velocity 0–1, note pan as a
  pan note expression). Channel volume, pan, mute and mixer routing apply to the
  plugin's stereo output like any other channel. The built-in instrument knobs
  (envelope, filter…) do not apply.
- **Effects**: `mixer.add_plugin <track> <plugin-id> [name]` appends a plugin
  effect slot. It takes the slot's place in the chain, stereo in and out.
  Bypass, removal, moving and undo work as for built-in effects. Plugin effects
  have no sidechain input.
- **Parameters**: a plugin's parameters are identified by their CLAP parameter
  id and use the plugin's own ("plain") values.
  `channel.set_plugin_param <channel> <param-id> <value>` and
  `mixer.set_plugin_param <track> <slot> <param-id> <value>` set one; both are
  undoable and coalesce while dragging. Values set this way are stored in the
  project and win over the saved state.
- **Plugin editor**: a plugin's own window can be opened from its channel or
  slot. When a parameter is moved in that window, MusicCraft records it through
  the same parameter commands, so it is undoable and saved.
- **State**: the plugin's opaque state (CLAP `state` extension) is stored in the
  project as base64 (`channel.set_plugin_state`, `mixer.set_plugin_state`;
  not undoable — they mirror what the plugin already holds). The app refreshes
  it when the plugin reports a change and before saving or exporting. On load,
  the state is restored first, then the stored parameter values are applied.
- **Missing plugins**: a project that names a plugin that is not installed
  still loads; the channel is silent and the effect passes audio through, with a
  warning in the status bar. The stored state is kept untouched.
- **Export** renders plugins offline with fresh instances restored from the
  project, so exports sound like playback.
- `plugin.list` lists the scanned plugins (id, name, kind, file).

## Real-time rules

- Plugin instances are created, activated, deactivated and destroyed on the
  main thread. The audio thread only calls `start_processing`, `process` and
  `stop_processing`.
- Instances live in a fixed table of engine slots that survives graph
  rebuilds, so adding a channel does not reset a plugin.
- Notes and parameter changes reach a plugin through preallocated, fixed-size
  event buffers; nothing allocates on the audio thread. Parameter changes the
  plugin makes in its own window come back through a lock-free queue.
- A plugin that is removed is handed back to the main thread before it is
  deactivated and destroyed.

## Safety boundary

- Loading a plugin means running code from a shared library through the C ABI,
  which requires `unsafe`. All of it lives in one crate, `musiccraft-clap`; every
  other crate keeps `unsafe_code = "forbid"`. The rest of MusicCraft talks to
  plugins only through the safe `External` trait (engine) and the
  `PluginFactory` trait (render).
- A crashing plugin can take the app down; there is no out-of-process
  sandboxing yet. Scanning loads every `.clap` file found (that runs its
  code); files that fail to load are skipped and reported.

## Plugin windows

- Plugins that can open a floating window of their own get one. On Linux,
  plugins that only embed are placed in an X11 window MusicCraft creates
  (through XWayland on Wayland desktops), marked as a fixed-size dialog unless
  the plugin can resize. Window resizes go through the plugin's
  `adjust_size` before `set_size`.
- While a window is open, the app polls the plugin's timers and file
  descriptors (`timer-support`, `posix-fd-support`) every frame.
- Closing the window with the title-bar button works like the panel's close
  button.

## Limits

- Notes go to the plugin's first note port, as CLAP note events, or as MIDI
  for plugins that only accept MIDI. Only the main audio port's first two
  channels are used; other ports get silent buffers.
- A plugin asking to be restarted (latency or port changes) keeps running
  with its current setup.

## Sources

- CLAP — open plugin standard, MIT licensed: headers and documentation at
  github.com/free-audio/clap (plugin entry and factory, `process`, events,
  `params`, `state`, `gui`, `timer-support`, `posix-fd-support`, search paths in
  `entry.h`).
- Placement of plugins as channel instruments and mixer insert effects follows
  the publicly documented workflow of pattern-based DAWs; everything else is our
  own design.
