MusicCraft MCP server (macOS)
=============================

musiccraft-mcp is MusicCraft's MCP server: point an AI assistant at it (or let
CreativeCraft install and wire it up) to build and edit songs.

- Universal binary (Apple silicon and Intel), macOS 11 or later.
- Not code-signed with a Developer ID: if macOS blocks it, run
  `xattr -d com.apple.quarantine musiccraft-mcp` once.
- `musiccraft-mcp` on its own holds its own project; with
  `--connect 127.0.0.1:<port>` it works on the project open in MusicCraft
  started with `--control <port>`. `musiccraft-mcp --help` shows the options.

The app itself is on the .dmg from the same release.

Project home: https://github.com/bhimio1/musiccraft
