# Plugin smoke tests in other hosts

Checked by hand before each release (`docs/spec/validation.md`): CI cannot
run these hosts. Use the bundles from `cargo xtask bundle --release`
(`target/bundled`), installed in the host's CLAP and VST3 folders.

Hosts: Ardour (free), Bitwig Studio (demo), REAPER (evaluation). Each host
that supports both formats is checked with CLAP and with VST3.

## Checklist (per host and format)

1. The host scans both bundles: 44 effects in MusicCraft Effects; 11
   instruments and Antiserum in MusicCraft Instruments; vendor "MusicCraft".
2. An instrument track with the synth plays notes from the host's piano roll,
   with pitch bend; Antiserum plays a factory preset chosen in its editor.
3. An effect (delay) on that track follows the host's tempo; automating a
   knob from the host moves it in the plugin's editor, and dragging it in
   the editor records automation in the host.
4. The compressor keyed from another track through the host's sidechain
   routing ducks when `sidechain` is on; the pitch corrector follows a MIDI
   track when `follow` is on (hosts that route MIDI to effects).
5. The convolution reverb loads a WAV impulse typed into its editor.
6. Save the project, close the host, reopen: every value, the Antiserum
   patch and the impulse file come back.
7. Open and close each editor several times, resize the host window, and
   close the project with editors open: no crash, no frozen window.
8. Play with the host's smallest buffer size: no dropouts with one Antiserum
   instance and five effects.

## Results

| Date | Host and version | OS | Format | Result | Notes |
| --- | --- | --- | --- | --- | --- |
| — | not run yet | — | — | — | Needs a person with the host installed. |
