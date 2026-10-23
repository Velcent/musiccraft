# Spec: live control (an agent in the open window)

The MCP server (`docs/spec/mcp.md`) normally holds its own headless project. In
live mode an agent works on the project that is open in the desktop app instead,
so the user watches every change land in the channel rack, piano roll, playlist
and mixer, and can play the song while the agent works.

## Behavior

### The app

- `musiccraft --control <port>` opens a control channel on `127.0.0.1:<port>`
  (localhost only, never another interface). Without `--control` nothing listens.
- `--control-token-file <file>` makes the channel private: the app writes a
  fresh random token (32 hex digits) to `<file>`, readable only by the user
  (mode 0600 on Unix), *before* it starts listening, and every connection must
  send that token as its first line. A wrong or missing token closes the
  connection without a reply. Without the option, any local connection is
  accepted.
- If the port can't be opened, the app still starts and shows the error in the
  status bar.
- After the token line, the channel speaks the same JSON-RPC 2.0 as
  `musiccraft-mcp` on stdio, one message per line, with the same tools and
  results. Requests are handled on the main thread, between frames, against the
  open project; each one wakes the window, so a background or idle window still
  answers promptly.
- Commands run exactly as menu items do: they are undoable from the window's
  own history, the window shows their effect on the next frame, the engine
  follows, plugin state is stored before a save, and saving clears the unsaved
  marker.
- A command that would replace the open project (`project.new`, `project.load`,
  `project.recover`) while it has unsaved changes is refused with an error
  telling the agent to save first; the user's work is never discarded behind
  their back.
- Each command the agent runs is named in the status bar ("Assistant: …").
- Several connections may be open; their requests are handled in arrival order.
- A request the window does not answer within two minutes gets a JSON-RPC error
  (-32000, "MusicCraft's window did not answer"), so a client never waits forever.

### The MCP server

- `musiccraft-mcp --connect <host:port> [--control-token-file <file>]` relays
  stdio to a running app's control channel instead of holding its own project:
  it reads the token from `<file>` (trimmed) and sends it first, then copies each
  stdin line to the app and each reply line to stdout. It exits when stdin closes
  or the app closes the connection. Connection failures print one line to stderr
  and exit with status 1.
- `musiccraft-mcp --version` prints `musiccraft-mcp <version>`; `--help` prints
  usage. Unknown options print usage and exit with status 2.

## CreativeCraft

The CreativeCraft hub starts live mode with
`musiccraft --control {port} --control-token-file {token}` and
`musiccraft-mcp --connect 127.0.0.1:{port} --control-token-file {token}`, and
headless mode with plain `musiccraft-mcp`. Releases ship `musiccraft-mcp` next to
`musiccraft` in the Linux and Windows portable archives, and on macOS as
`MusicCraft-cli-<version>-macos-universal.zip` beside the disk image.

## Sources

- Own design, following the CreativeCraft catalog's `live` convention (a GUI
  control port plus an MCP relay). JSON-RPC 2.0 and the Model Context Protocol
  stdio transport (open standards).
