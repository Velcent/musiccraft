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
