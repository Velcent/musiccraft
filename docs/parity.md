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
| Transport | Record audio into the playlist | `done` |
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
| Piano roll | Per-note pan lane | `cmd:note.set_pan` `done` |
| Piano roll | Per-note pitch slides | `cmd:note.set_slide` `spec:piano-roll.md` |
| Piano roll | Portamento and muted notes, mute tool | `cmd:note.set_prop` `spec:piano-roll.md` |
| Piano roll | Note release velocity, fine pitch, Mod X/Y and colour, edited in the event lane | `cmd:note.set_prop` `spec:piano-roll.md` |
| Piano roll | Right-drag deletes notes | `spec:piano-roll.md` `done` |
| Piano roll | Draw, move, resize, delete notes | `cmd:note.add` `cmd:note.move` `cmd:note.resize` `cmd:note.remove` `spec:piano-roll.md` |
| Piano roll | Velocity | `cmd:note.set_velocity` |
| Piano roll | Quantize | `cmd:note.quantize` |
| Piano roll | Transpose | `cmd:note.transpose` |
| Piano roll | Snap to grid | `done` |
| Piano roll | Ghost notes of other channels | `done` |
| Piano roll | Audition notes from the keyboard | `done` |
| Piano roll | Portamento (glide) | `done` |
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
| Mixer | Sidechain compression | `fx:compressor` `done` |
| Effects | Filter, EQ, delay, reverb, compressor, distortion | `fx:filter` `fx:eq` `fx:delay` `fx:reverb` `fx:compressor` `fx:distortion` |
| Effects | Chorus | `fx:chorus` |
| Effects | Limiter | `fx:limiter` |
| Effects | Phaser, flanger | `fx:phaser` `fx:flanger` |
| Effects | Bitcrusher, stereo tool | `fx:bitcrusher` `fx:stereo` |
| Effects | Tempo-synced trance gate (16 steps) and auto-pan | `fx:gate` `fx:autopan` |
| Instruments | Subtractive synth with unison, sub, drive, filter types, LFO and presets | `cmd:channel.add_synth` `spec:piano-roll.md` |
| Instruments | Drum synth (kick, snare, hat, tom, clap) with presets | `cmd:channel.add_drum` `spec:channel-rack.md` |
| Instruments | Plucked string with presets | `cmd:channel.add_pluck` `spec:channel-rack.md` |
| Sampler | Loop slicing to keys and to the pattern | `cmd:channel.slice` `spec:channel-rack.md` |
| Instruments | Four-operator FM synth with presets | `cmd:channel.add_fm` `spec:channel-rack.md` |
| Instruments | Keys: piano, electric piano, organ | `cmd:channel.add_keys` `spec:channel-rack.md` |
| Instruments | Additive synth | `cmd:channel.add_additive` `spec:channel-rack.md` |
| Instruments | Granular sampler | `cmd:channel.add_granular` `spec:channel-rack.md` |
| Instruments | 16-pad drum kit | `cmd:channel.add_kit` `cmd:channel.set_pad` `spec:channel-rack.md` |
| Instruments | Layer channel playing several channels | `cmd:channel.add_layer` `cmd:channel.set_layer` `spec:channel-rack.md` |
| Instruments | Third synth oscillator | `spec:channel-rack.md` `done` |
| Effects | Effect windows with response graphs and presets | `spec:mixer.md` `done` |
| Instruments | CLAP plugins (instruments, effects, plugin windows) | `cmd:channel.add_plugin` `cmd:mixer.add_plugin` `cmd:plugin.list` `spec:plugins.md` |
| Instruments | VST3 plugins | `todo` |
| Windows | Playlist / rack / piano roll / mixer shortcuts | `key:F5=view.toggle playlist` `key:F6=view.toggle channel_rack` `key:F7=view.toggle piano_roll` `key:F9=view.toggle mixer` `spec:keymap.md` |
| Windows | Browser | `key:Alt+F8=view.toggle browser` |
| Options | Settings window: audio device, rate, buffer, MIDI port, scale, folders, project info | `key:F10=ui.settings` `spec:settings.md` |
| Interface | Hint bar showing what the hovered control does | `done` |
| Editing | Draw, paint, delete and select tools in the piano roll and playlist | `spec:piano-roll.md` `spec:playlist.md` |
| Piano roll | Ghost notes of other channels | `spec:piano-roll.md` |
| Piano roll | Strum, humanize, legato, chop, glue, flip, snap to scale, arpeggiate | `cmd:note.strum` `cmd:note.humanize` `cmd:note.legato` `cmd:note.chop` `cmd:note.glue` `cmd:note.flip` `cmd:note.to_scale` `cmd:note.arpeggiate` |
| Channel rack | Fill every N steps, shift steps | `cmd:step.fill` `cmd:step.shift` |
| Channel | Arpeggiator (up, down, up-down, random; rate, octaves, gate) | `spec:piano-roll.md` `done` |
| Browser | Folders, search, click to preview, drag to rack or playlist | `done` |
| Transport | Oscilloscope, CPU meter, master volume | `done` |
| Project | New / open / save / save as | `cmd:project.new` `cmd:project.load` `cmd:project.save` `key:Ctrl+S=ui.save` `key:Ctrl+O=ui.open` `spec:project-file.md` |
| Project | Undo / redo | `key:Ctrl+Z=edit.undo` `key:Ctrl+Y=edit.redo` `spec:commands.md` |
| Project | Export WAV | `cmd:project.export_wav` `key:Ctrl+R=ui.export` |
| Project | Export stems | `cmd:project.export_stems` |
| Project | Export FLAC (lossless) | `cmd:project.export_flac` |
| Project | Export lossy audio (MP3, Ogg) | `todo` |
| Project | Autosave and crash recovery | `done` |
| Input | MIDI keyboard plays the selected channel | `done` |
| Input | Record live notes into patterns | `key:Ctrl+E=ui.toggle_record` `done` |
| Input | Computer keyboard as piano | `key:Ctrl+T=ui.typing_keyboard` `done` |
| Automation | Automate any knob or fader from its context menu | `cmd:automation.create` `done` |
| Scripting | Headless CLI and scripts | `done` |
| Scripting | MCP server for AI agents | `spec:mcp.md` |
