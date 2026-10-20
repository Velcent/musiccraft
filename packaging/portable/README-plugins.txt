MusicCraft plugins
==================

MusicCraft's effects and instruments for other DAWs, in CLAP and VST3:

- MusicCraft Effects.clap / .vst3: the 44 mixer effects.
- MusicCraft Instruments.clap / .vst3: the 11 built-in instruments and
  Antiserum, the wavetable synth.

Each plugin opens the same editor as in MusicCraft. Effect and Antiserum
presets are shared with the app.

Install
-------

Copy the bundles into your plugin folders, then let your DAW rescan. Use one
format or both; your DAW lists whichever it supports.

Linux
  CLAP:  ~/.clap/            (all users: /usr/lib/clap/)
  VST3:  ~/.vst3/            (all users: /usr/lib/vst3/)

Windows
  CLAP:  C:\Program Files\Common Files\CLAP\
  VST3:  C:\Program Files\Common Files\VST3\

macOS
  CLAP:  ~/Library/Audio/Plug-Ins/CLAP/   (all users: /Library/Audio/Plug-Ins/CLAP/)
  VST3:  ~/Library/Audio/Plug-Ins/VST3/   (all users: /Library/Audio/Plug-Ins/VST3/)

  The bundles are not notarized. After copying, clear the download flag once:
    xattr -dr com.apple.quarantine ~/Library/Audio/Plug-Ins/CLAP/MusicCraft*.clap
    xattr -dr com.apple.quarantine ~/Library/Audio/Plug-Ins/VST3/MusicCraft*.vst3

A ".vst3" is a folder; copy the whole folder. On macOS a ".clap" is a folder
too.

Notes
-----

- Samples and impulse files are stored by path in your DAW project. Moving a
  project to another computer needs the files at the same paths.
- Antiserum's user presets are shared with the app, in
    Linux:   ~/.local/share/musiccraft/presets/antiserum/
    Windows: %APPDATA%\MusicCraft\presets\antiserum\
    macOS:   ~/Library/Application Support/MusicCraft/presets/antiserum/
  (the MUSICCRAFT_USER_PRESETS environment variable names another folder).
- MusicCraft is free software under the GNU GPL, version 3 or later. The
  source is at https://github.com/bhimio1/musiccraft (the release's tag).
