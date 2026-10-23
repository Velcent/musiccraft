MusicCraft (portable)
=====================

No installation needed: run musiccraft.exe from any folder (or a USB stick).

- Windows 10 or later, 64-bit.
- This build is not code-signed, so Windows SmartScreen may warn on first launch:
  choose "More info" > "Run anyway".
- Autosave and recovery files are written to %APPDATA%\MusicCraft, not next to the
  program. Delete that folder to remove all traces.
- onnxruntime.dll is ONNX Runtime (Microsoft, MIT licence, see
  ONNXRUNTIME-LICENSE.txt) for stem splitting with Demucs: on the GPU through
  DirectML where Windows offers it, else on the CPU. The Demucs model downloads to
  %APPDATA%\MusicCraft\models the first time you split stems.
- musiccraft-mcp.exe is MusicCraft's MCP server: point an AI assistant at it (or let
  CreativeCraft do it) to build and edit songs. `musiccraft-mcp.exe --help` shows how to
  work on the project open in the app instead.

Project home: https://github.com/bhimio1/musiccraft
