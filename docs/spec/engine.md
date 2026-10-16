# Spec: real-time engine and audio output (M1)

## Behavior

- The app opens the system's default output device (via `cpal`) at the device's
  preferred sample rate and channel count. Devices offering `f32`, `f64`, `i16`,
  `i32` or `u16` samples are supported.
- The engine renders stereo. Mono devices receive the average of left and right;
  extra channels are silent. Output is clamped to ±4.0 and non-finite samples are
  written as silence.
- Device callbacks of any size are split into internal blocks of at most 256
  frames.
- Invalid samples (NaN or infinite) never reach effect tails: plugin output,
  each mixer track's input and the output of every effect are checked, invalid
  samples become silence, and a built-in effect that produced them is reset.
  Without this, one bad sample from a plugin could circulate in a reverb or delay
  and silence its track until the graph was rebuilt.
- Filter, delay and reverb feedback paths flush values below 1e-15 (about
  -300 dB) to zero, so decaying tails reach exact silence instead of lingering as
  subnormal floats, which are very slow on many CPUs. (Setting the CPU's
  flush-to-zero mode would need `unsafe`.)
- `musiccraft-cli tone [seconds]` plays an A4 test tone (440 Hz) and
  `musiccraft-cli play <script> [seconds]` plays a scripted project; both are the
  headless checks for the audio path.

## Messages

- The main thread talks to the engine only through a fixed-capacity lock-free
  queue (4096 messages). Small updates are plain `Copy` values: play, stop, pause,
  tempo, seek, channel parameters, mixer track parameters, effect parameters,
  note on/off and all-notes-off.
- Two messages carry boxed data built on the main thread: a new **graph**
  (channels, instruments, mixer tracks with effect state and scratch buffers) and a
  new **sequence** (sorted note events plus a loop length). The value each replaces
  is sent back through a return queue (64 slots) and dropped on the main thread.
  If the return queue is full the engine leaves the message queued and applies it
  in a later callback; it never frees memory itself.
- A separate live-note queue (1024 events) lets a MIDI input thread trigger notes
  without going through the main thread.
- The engine publishes playhead position, playing state, active voice count and
  per-track peak levels through atomics. Peaks hold their maximum until read.

## Voices

- 64 voices are preallocated. When all are busy the oldest is stolen.
- A note event starts on the first frame at or after its tick position. Its gate
  closes after its length; the envelope then releases.
- Replacing the graph silences all voices.

## Sources

- Own design. Lock-free single-producer/single-consumer queues and returning
  buffers for deallocation off the audio thread are standard real-time audio
  practice described in many public talks and articles.
