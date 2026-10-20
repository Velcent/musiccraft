# nih-plug, patched for MusicCraft

This is nih-plug's main crate (`nih_plug`) and its derive macros
(`nih_plug_derive`) from
<https://github.com/robbert-vdh/nih-plug> at revision
`de421011f41a6d10fc8c7a6084e4f4dee0143683`, under its ISC license
(`LICENSE`, Copyright (c) 2022-2024 Robbert van der Helm). MusicCraft's two
workspaces use it through `[patch."https://github.com/robbert-vdh/nih-plug.git"]`.

The first commit that added this folder holds the upstream files unchanged
(except the manifest, which drops nih-plug's own workspace and profiles and
quiets two compiler lints in upstream code). Every change after it is marked
in the code with `MusicCraft patch`. Each one fixes a clap-validator finding
(`docs/spec/validation.md`):

1. **State with a bad length** (`src/wrapper/clap/wrapper.rs`,
   `ext_state_load`): the state length read from the stream decided the size
   of an up-front allocation, so random data aborted the host on a petabyte
   allocation (`state-invalid-random`). The state is now read in 4 KiB
   chunks; a stream that ends early fails the load.
2. **Rescan after loading state** (same function): after a successful load,
   the plugin asks the host to rescan parameter values, so the host shows the
   loaded values (`state-reproducibility-*`).
3. **Latency changes** (`activate`, `deactivate`, `Task::LatencyChanged`):
   CLAP allows `clap_host_latency::changed()` only inside `activate()`.
   The wrapper now tracks whether the plugin is active and whether
   `activate()` is running: inside it, the change is reported; while active,
   the plugin asks for a restart; while inactive, nothing is sent, and the
   host reads the new latency at the next activation (`param-fuzz-*`,
   `param-set-*`).

To update nih-plug: replace this folder with the new revision's files, keep
the manifest notes, apply the patches again (or drop those fixed upstream)
and run `cargo xtask validate`.
