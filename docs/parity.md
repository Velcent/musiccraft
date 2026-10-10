# Feature parity checklist

What a pattern-based DAW user expects, and whether MusicCraft has it. Checked by
`cargo xtask parity`, which verifies each requirement mechanically:

- `cmd:<name>` — the command is registered.
- `key:<chord>=<invocation>` — the default keymap binds the chord to it.
- `spec:<file>` — `docs/spec/<file>` exists.
- `fx:<kind>` — the mixer offers this effect kind.
- `done` — implemented in the UI or app; verified by tests and review, not by
  the checker.
- `todo` — not implemented yet.

Features are described in our own words at the level of a public manual's table
of contents; see `docs/clean-room.md`.

| Area | Feature | Requires |
| --- | --- | --- |
| Transport | Play / stop | `cmd:transport.toggle` `key:Space=transport.toggle` |
| Transport | Pattern / song mode | `cmd:transport.set_mode` `key:L=transport.toggle_mode` |
| Transport | Tempo | `cmd:project.set_tempo` `spec:transport.md` |
| Transport | Metronome | `cmd:transport.toggle_metronome` `key:Ctrl+M=transport.toggle_metronome` |
| Transport | Loop a time range of the song | `cmd:transport.set_loop` `cmd:transport.clear_loop` |
| Transport | Record audio | `todo` |
| Channel rack | Sampler channels with built-in sounds | `cmd:channel.add_sampler` `spec:channel-rack.md` |
| Channel rack | WAV samples | `cmd:channel.set_sample` |
| Channel rack | Step sequencer | `cmd:step.toggle` |
| Channel rack | Channel volume / pan | `cmd:channel.set_volume` `cmd:channel.set_pan` |
| Channel rack | Mute / solo | `cmd:channel.toggle_mute` `cmd:channel.solo` |
| Channel rack | Route to mixer insert | `cmd:channel.route` |
| Channel rack | Rename / clone / reorder / delete | `cmd:channel.rename` `cmd:channel.clone` `cmd:channel.move` `cmd:channel.remove` |
| Channel rack | Instrument parameters | `cmd:channel.set_param` |
| Channel rack | Swing | `cmd:pattern.set_swing` |
| Channel rack | Channel settings window (envelope, filter, sample view) | `done` |
| Channel rack | Melodic patterns previewed in the rack | `done` |
| Piano roll | Velocity editor lane | `cmd:note.set_velocity` `done` |
| Piano roll | Select, copy, paste, duplicate, nudge | `cmd:edit.batch` `done` |
| Channel rack | Per-note pan and pitch graph editor | `todo` |
| Piano roll | Draw, move, resize, delete notes | `cmd:note.add` `cmd:note.move` `cmd:note.resize` `cmd:note.remove` `spec:piano-roll.md` |
| Piano roll | Velocity | `cmd:note.set_velocity` |
| Piano roll | Quantize | `cmd:note.quantize` |
| Piano roll | Transpose | `cmd:note.transpose` |
| Piano roll | Snap to grid | `done` |
| Piano roll | Ghost notes of other channels | `done` |
| Piano roll | Audition notes from the keyboard | `done` |
| Piano roll | Note slides / portamento | `todo` |
| Piano roll | Chord and scale helpers | `cmd:note.add_chord` `done` |
| Piano roll | Import MIDI files | `cmd:note.import_midi` |
| Patterns | New pattern | `cmd:pattern.add` `key:F4=pattern.add` |
| Patterns | Next / previous pattern | `key:Plus=pattern.next` `key:Minus=pattern.prev` |
| Patterns | Clone / rename / delete / length | `cmd:pattern.clone` `cmd:pattern.rename` `cmd:pattern.remove` `cmd:pattern.set_length` |
| Playlist | Place, move, resize, delete clips | `cmd:playlist.add_clip` `cmd:playlist.move_clip` `cmd:playlist.resize_clip` `cmd:playlist.remove_clip` `spec:playlist.md` |
| Playlist | Track mute | `cmd:playlist.toggle_track_mute` |
| Playlist | Select, duplicate, group move | `cmd:edit.batch` `done` |
| Playlist | Click the ruler to move the playhead; follow playback | `done` |
| Playlist | Audio clips (drop samples on the playlist) | `cmd:playlist.add_audio_clip` `done` |
| Playlist | Automation clips | `cmd:playlist.add_automation_clip` `cmd:automation.create` `cmd:automation.add_point` |
| Playlist | Markers | `cmd:playlist.add_marker` `cmd:playlist.remove_marker` |
| Mixer | Insert tracks | `cmd:mixer.add_track` `cmd:mixer.remove_track` `spec:mixer.md` |
| Mixer | Volume / pan / mute / solo | `cmd:mixer.set_volume` `cmd:mixer.set_pan` `cmd:mixer.toggle_mute` `cmd:mixer.toggle_solo` |
| Mixer | Effect slots: add, remove, reorder, bypass | `cmd:mixer.add_effect` `cmd:mixer.remove_effect` `cmd:mixer.move_effect` `cmd:mixer.toggle_effect` |
| Mixer | Effect parameters | `cmd:mixer.set_effect_param` |
| Mixer | Peak meters | `done` |
| Mixer | Sends between inserts and buses | `cmd:mixer.set_send` `cmd:mixer.route_to_master` |
| Mixer | Sidechain | `todo` |
| Effects | Filter, EQ, delay, reverb, compressor, distortion | `fx:filter` `fx:eq` `fx:delay` `fx:reverb` `fx:compressor` `fx:distortion` |
| Effects | Chorus | `fx:chorus` |
| Effects | Limiter | `fx:limiter` |
| Instruments | Subtractive synth | `cmd:channel.add_synth` |
| Instruments | CLAP plugins | `todo` |
| Instruments | VST3 plugins | `todo` |
| Windows | Playlist / rack / piano roll / mixer shortcuts | `key:F5=view.toggle playlist` `key:F6=view.toggle channel_rack` `key:F7=view.toggle piano_roll` `key:F9=view.toggle mixer` `spec:keymap.md` |
| Windows | Browser | `key:Alt+F8=view.toggle browser` |
| Browser | Folders, search, click to preview, drag to rack or playlist | `done` |
| Transport | Oscilloscope, CPU meter, master volume | `done` |
| Project | New / open / save / save as | `cmd:project.new` `cmd:project.load` `cmd:project.save` `key:Ctrl+S=ui.save` `key:Ctrl+O=ui.open` `spec:project-file.md` |
| Project | Undo / redo | `key:Ctrl+Z=edit.undo` `key:Ctrl+Y=edit.redo` `spec:commands.md` |
| Project | Export WAV | `cmd:project.export_wav` `key:Ctrl+R=ui.export` |
| Project | Export stems | `cmd:project.export_stems` |
| Project | Export compressed audio (MP3, Ogg, FLAC) | `todo` |
| Project | Autosave and crash recovery | `done` |
| Input | MIDI keyboard plays the selected channel | `done` |
| Input | Record live notes into patterns | `key:Ctrl+E=ui.toggle_record` `done` |
| Input | Computer keyboard as piano | `key:Ctrl+T=ui.typing_keyboard` `done` |
| Automation | Automate any knob or fader from its context menu | `cmd:automation.create` `done` |
| Scripting | Headless CLI and scripts | `done` |
| Scripting | MCP server for AI agents | `spec:mcp.md` |
