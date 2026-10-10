# Spec: project files and audio export (M6)

## Project files

- Projects are saved as UTF-8 JSON, conventionally with the `.mcproj` extension:

  ```json
  { "format": "musiccraft-project", "version": 1, "project": { ... } }
  ```

- Instrument and effect parameters are stored as `name → value` maps, so files
  stay readable and older files load when parameters are added (missing values
  take defaults; unknown names are ignored; values are clamped into range).
- Loading validates the file: other formats and newer versions are rejected; an
  invalid tempo is rejected; dangling references (notes for removed channels,
  clips for removed patterns, routes to missing tracks) are dropped or reset.
- Sample paths are stored as given. Relative paths resolve against the project
  file's directory first, then the working directory.
- `project.save [path]` writes atomically (temporary file, then rename) and
  remembers the path; without an argument it saves to the remembered path.
  `project.load <path>` and `project.new` replace the project and clear undo
  history. The transport state is not saved.

## Export

- `project.export_wav <path> [16|24|32f] [loops] [sample rate]` renders offline,
  faster than real time, with the same engine used for playback. Defaults: 16-bit
  with TPDF dither, one loop, 44.1 kHz.
- What is rendered follows the play mode: the selected pattern or the song,
  repeated `loops` times, followed by the tail of sounding notes and effects. The
  tail ends after half a second below -90 dBFS, or after ten seconds.
- Rendering is deterministic: the same project always produces the same file.

## Sources

- Own design. WAV/RIFF is an open format, written with the `hound` crate. We do
  not read or write any other DAW's project format.
