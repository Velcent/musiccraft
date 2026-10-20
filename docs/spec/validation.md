# Plugin validation (roadmap Phase 8)

How MusicCraft checks that its exported plugins (`docs/spec/plugin-export.md`,
`docs/spec/plugin-instruments.md`) behave in other hosts and in real time.
Everything here runs in CI on Linux, except the manual smoke tests at the end.

## Round trips

- Our own CLAP and VST3 hosts load both bundles from disk in tests. Every
  effect sounds like the mixer's effect, every instrument like the app's
  voices, Antiserum like itself; state, parameters, the sidechain input,
  MIDI follow and the editors are covered (see the two plugin specs).
- **Offline render** (`tests/render_round_trip.rs`): a project rendered with
  a built-in instrument (synth, FM, keys, pluck, Antiserum) or a master
  effect (delay, chorus, compressor, reverb) matches, within 1e-4, the same
  project rendered with that instrument or effect as our plugin, hosted by
  the app through CLAP and through VST3. This is the user's whole path:
  commands, the bridge, the engine, the host and the plugin. Each effect is
  also checked to change the sound, so a pass is not a silent no-op.

## Validators

`cargo xtask validate` runs two validators on the bundles that
`cargo xtask bundle` builds in `target/bundled`, and fails on any failed or
crashed test:

- **clap-validator** (MIT, free-audio), every test on every plugin.
- **The VST3 SDK's validator** (built from the SDK under its GPLv3 option).

The tools are found through `CLAP_VALIDATOR` and `VST3_VALIDATOR` (or
`PATH`). CI builds them at pinned versions (clap-validator
`b2f1d9b79b1d264a5747f46707d72b1aa40a02ef`, VST3 SDK `v3.8.1_build_84`) and
caches them.

Findings and what became of them:

- nih-plug crashed on random state data, did not ask hosts to rescan values
  after loading state, and reported latency changes outside `activate()`.
  These are fixed in a patched copy of nih-plug (`vendor/nih_plug`, changes
  listed in its `PATCHES.md`).
- Parameter text must read back as the same text. Values that round to zero
  show as `0` (never `-0.0` or `-0%`), the unit is chosen after rounding
  (999.7 Hz is `1.00 kHz`, 0.9997 s is `1.00 s`), and the formant's vowel
  text `a→e` reads back as halfway between the two vowels. A unit test
  checks every parameter of every plugin at 501 points of its range.
- **Known and accepted:**
  - clap-validator's `process-audio-denormals` warns (a warning, not a
    failure) for some effects. That check times its own generation of
    random subnormal numbers along with the plugin, which is slow in itself,
    so cheap effects look slow. Our own measurement through our CLAP host,
    with inputs made beforehand, shows no slowdown (see below), and it shows
    10–18× slowdowns if the input flush is removed, so it does detect the
    real problem.
  - The VST3 validator's bypass persistence test prints "The bypass
    parameter was not correctly set" and still passes. nih-plug ignores
    `setParamNormalized` from the controller while the plugin processes,
    because the same change arrives through `process` and writing it from
    another thread would race the audio thread's smoothers. The value
    arrives with the next processed block, and the test's final check (the
    restored state) passes.

## Denormals

- Effect plugins replace subnormal input samples (main and sidechain) with
  zero before processing (`musiccraft_dsp::flush_subnormals`); nih-plug sets
  flush-to-zero only for the plugin's own results, not denormals-are-zero
  for its input. The app's mixer does the same where it cleans each track's
  signal.
- Every effect's and instrument's state flushes values below about -300 dB
  to zero where they are fed back (`flush_denormal`), so tails end in exact
  silence.
- Tests: every exported effect fed 0.5 s of noise, every instrument and a
  cross-section of Antiserum's factory presets playing a chord for 0.5 s,
  then 8 s of silence, never output a subnormal sample, run without
  flush-to-zero as the app runs them (`tests/tails.rs`). Through our CLAP
  host, every effect processes subnormal input at most 1.6× as long as noise
  (`tests/denormals.rs`; inputs generated before timing, fastest of five
  runs, re-measured up to three times when over the limit).

## Performance

`cargo run --release -p musiccraft-plugin-export --example bench` runs in CI
and fails when one plugin goes over its budget at 48 kHz in 512-frame
blocks (fastest of three runs):

- every effect at default settings on noise: at most 10% of one core (the
  heaviest, convolution, uses about 2%);
- every built-in instrument playing an eight-note chord: at most 30% (the
  heaviest, keys, uses about 13%).

Antiserum has its own benchmark (`musiccraft-antiserum --example bench`,
`docs/spec/antiserum-oscillators.md`).

## Manual smoke tests

Hosts we cannot run in CI are checked by hand before a release, with the
checklist in `docs/smoke-tests.md`: load both bundles, play, automate, save
and reopen a project, open and close editors. The results go in that file,
with the host, its version and the date.

## Sources

- Own design. clap-validator (MIT) and the VST3 SDK's validator (GPLv3
  option) are run as tools; no code from them is copied.
- The CLAP specification's rules for `clap_host_latency::changed()`
  (`ext/latency.h`) and state streams (`ext/state.h`).
- nih-plug (ISC), patched as listed in `vendor/nih_plug/PATCHES.md`.
