# Spec: MCP server (M7)

## Behavior

- `musiccraft-mcp` is a Model Context Protocol server speaking JSON-RPC 2.0 over
  stdio, one message per line. It holds one project session for its lifetime.
- `initialize` answers with the client's protocol version when supported
  (2025-06-18, 2025-03-26, 2024-11-05), otherwise the newest; it advertises the
  `tools` capability and short usage instructions.
- Every registry command (including `project.export_wav`, `edit.undo`,
  `edit.redo` and `edit.batch`) is a tool named with `_` for `.`
  (`note.add` → `note_add`). Input: `{"args": ["...", ...]}`; non-string values
  are converted to strings and validated by the command.
- Extra tools: `run` (one line), `run_script` (many lines, stops at the first
  error), `get_state` (readable summary), `get_project` (file-format JSON),
  `list_commands`, and `analyze_audio` (offline render; duration, peak, RMS, and
  per-bar levels, flagging clipping).
- Command failures are tool results with `isError: true`. Unknown tools and bad
  parameters are JSON-RPC errors (-32602); unknown methods -32601; malformed JSON
  -32700. Notifications get no reply.

- `musiccraft-mcp --connect <host:port>` works on the project open in the desktop
  app instead of its own (live mode, `docs/spec/live-control.md`).
  `--version` and `--help` print and exit.

## Client configuration

```json
{ "mcpServers": { "musiccraft": { "command": "/path/to/target/release/musiccraft-mcp" } } }
```

## Sources

- The Model Context Protocol specification (open standard, modelcontextprotocol.io)
  and JSON-RPC 2.0. Tool set: own design.
