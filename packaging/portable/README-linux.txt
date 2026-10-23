MusicCraft (portable)
=====================

No installation needed: run ./musiccraft from any folder.

- 64-bit Linux (x86_64) with glibc 2.35 or newer.
- Needs the system's ALSA library (libasound2) and an X11 or Wayland session with
  OpenGL. Install libasound2 (Debian/Ubuntu) or alsa-lib (Arch/Fedora) if the app
  fails to start with a missing-library error.
- Autosave and recovery files are written to ~/.local/share/musiccraft.
- libonnxruntime.so is ONNX Runtime (Microsoft, MIT licence, see
  ONNXRUNTIME-LICENSE.txt) for stem splitting with Demucs, on the CPU. For the GPU,
  install your distribution's ONNX Runtime package built with CUDA or ROCm
  (version 1.22 or newer); MusicCraft prefers a system copy. The Demucs model downloads to ~/.local/share/musiccraft/models the
  first time you split stems.
- For a sandboxed install with a menu entry, use the .flatpak from the same release.
- ./musiccraft-mcp is MusicCraft's MCP server: point an AI assistant at it (or let
  CreativeCraft do it) to build and edit songs. `./musiccraft-mcp --help` shows how to
  work on the project open in the app instead.

Project home: https://github.com/bhimio1/musiccraft
