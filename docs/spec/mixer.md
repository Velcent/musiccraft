# Spec: mixer and effects (M3)

## Behavior

- Mixer track 0 is the **master**; tracks 1… are **inserts**. New projects have
  eight inserts. At most 128 tracks.
- Each track has a name, volume (linear 0–1.25, default 1.0), pan (-1–1, a
  balance), mute,
  solo and up to ten insert effects processed in order. The mixer is a floating
  window (F9) with the strips on the left and the selected track's effect slots
  on the right.
- Channels feed exactly one track. Each insert feeds the master unless
  `mixer.route_to_master <track> off`, and may also have up to eight post-fader
  **sends** to higher-numbered inserts (`mixer.set_send <track> <target>
  <level>`; level 0 removes it). Sends only go "up", so routing can never form a
  feedback loop and tracks are processed in number order. This builds buses (for
  example drums 1–3 into insert 10) and shared effect returns.
- Removing a track removes sends into it and renumbers sends past it; undo
  restores them.
- Solo: when any insert is soloed, every insert that is not soloed, not feeding
  a soloed insert, and not fed by one is silenced. The master is never silenced
  by solo.
- Commands: `mixer.add_track`, `mixer.remove_track` (channels routed to it move to
  the master; undo restores them), `mixer.rename`, `mixer.set_volume`,
  `mixer.set_pan`, `mixer.set_mute`, `mixer.toggle_mute`, `mixer.set_solo`,
  `mixer.toggle_solo`, `mixer.set_send`, `mixer.route_to_master`, `mixer.add_effect`, `mixer.remove_effect`,
  `mixer.move_effect`, `mixer.set_effect_param`, `mixer.set_effect_enabled`,
  `mixer.toggle_effect`. Effect slots are numbered from 1. All are undoable.
- **Track stereo options** (undoable, saved; right-click a strip's name):
  `mixer.set_separation <track> <-1..1>` (-1 folds the track to mono, 0
  leaves it, 1 doubles the side signal), `mixer.set_phase_invert <track>
  <on|off>` (flips the polarity of both sides) and `mixer.set_swap_sides
  <track> <on|off>` (exchanges left and right). They apply to the track's
  input, before its effects and fader.
- **Pan law** (project setting, `project.set_pan_law <circular|triangular>`,
  undoable, saved as `"pan_law"`; Settings → Project). It applies to every
  channel pan. Mixer track pan stays a balance (0 dB at centre; a track
  carries stereo, and a law at every stage would stack its centre cut), and
  note pan stays a balance on top of the channel pan. For pan `p` in -1…1:
  - `circular` (default): left = cos θ, right = sin θ with θ = (p + 1)·π/4.
    Centre is -3 dB on each side (equal power), a hard pan is 0 dB on its side
    and silent on the other.
  - `triangular`: left = (1 − p)/2, right = (1 + p)/2. Centre is -6 dB on each
    side (equal amplitude), a hard pan is 0 dB on its side.
  - Project files without `"pan_law"` (saved before it existed, when pan was a
    balance with 0 dB at centre) load as `circular`, so centred sounds play
    3 dB quieter than they did.
- Changing parameters never interrupts audio. Adding, removing or reordering
  effects rebuilds the track's processing and resets effect tails.

## Effects

| Kind | Parameters |
| --- | --- |
| `filter` | mode (0 low-pass, 1 band-pass, 2 high-pass), cutoff Hz, resonance, mix |
| `eq` | low gain/freq (shelf), mid gain/freq/q (peak), high gain/freq (shelf) |
| `delay` | time in beats (tempo-synced), feedback, mix, ping_pong, damping |
| `reverb` | size, damping, width, mix |
| `chorus` | rate Hz, depth ms, delay ms, mix |
| `compressor` | threshold dB, ratio, attack ms, release ms, makeup dB, sidechain (0 = off, else the insert whose post-fader output drives detection) |
| `limiter` | input gain dB, ceiling dB, release ms (instant attack; output never exceeds the ceiling) |
| `distortion` | drive dB, mix, output dB |
| `phaser` | rate Hz, depth, feedback, stages (2–12 all-pass stages), mix |
| `flanger` | rate Hz, depth ms, delay ms, feedback (−0.95–0.95), mix |
| `bitcrusher` | bits (1–16), downsample (hold every 1–32 samples), mix, output dB |
| `stereo` | width (0 mono – 2 extra wide, mid/side), balance, invert left, invert right |
| `stereo_shaper` | left_to_left, right_to_left, left_to_right, right_to_right (−1–1; each output side is a mix of both inputs, so swapping sides, mono from one side and polarity flips are all settings), delay ms (−20–20: positive delays the right side, negative the left, for Haas widening), output dB |
| `gate` | rate (1/32 note – 1 bar per step), duty (open share of a step), depth, smooth ms, steps (16-step on/off pattern as a number, bit 0 = first step) |
| `autopan` | rate (1/32 note – 1 bar per cycle), depth, shape (sine, triangle, square) |
| `graphic_eq` | seven peak bands at 63, 160, 400, 1k, 2.5k, 6.3k, 16k Hz (±18 dB each), output dB |
| `parametric_eq` | seven bands of freq/gain/q (band 1 a low shelf, 7 a high shelf, 2–6 peaks), output dB; the window's points drag frequency and gain |
| `bass_boost` | freq Hz, boost dB (low shelf), drive (gentle saturation after the shelf) |
| `overdrive` | drive dB, tone Hz (low-pass after the clipper), bias (asymmetry, adds even harmonics), mix, output dB |
| `soft_clipper` | threshold dB, knee (how early the curve bends), output dB; never exceeds the threshold |
| `waveshaper` | shape (tanh, hard clip, fold, sine, asymmetric, rectify), drive dB, mix, output dB |
| `dc_block` | freq Hz (one-pole high-pass that removes DC offset) |
| `ring_mod` | freq Hz (sine carrier the signal is multiplied by), mix |
| `freq_shift` | shift Hz (−2000–2000; every frequency moves by the same amount, via a Hilbert transformer), mix |
| `formant` | vowel (0 a – 4 u, fractional values morph), shift semitones, resonance, mix |
| `auto_filter` | mode, cutoff Hz, resonance, rate (tempo-synced LFO), depth octaves, envelope octaves (input level opens or closes the filter), mix |
| `dynamics` | an all-in-one chain of noise gate → compressor → limiter: gate_threshold dB (at its minimum, −100, the gate is off), gate_attack / gate_hold / gate_release ms, gate_range dB; comp_threshold dB, comp_ratio (1 = off), comp_attack / comp_release ms, comp_makeup dB; limit_gain dB (before the limiter), limit_ceiling dB, limit_release ms. Each stage behaves exactly like the separate `noise_gate`, `compressor` and `limiter`; by default only the limiter acts. Its window draws the combined static curve; drag the knee for the compressor threshold and the right-hand point for the ceiling |
| `noise_gate` | threshold dB, attack ms, hold ms, release ms, range dB (attenuation while closed) |
| `transient` | attack (−1 soften – 1 boost), sustain (−1–1), output dB; from the difference of a fast and a slow envelope |
| `multi_tap` | four taps of time (beats) and level, alternating left/right, feedback, mix, damping |
| `multiband` | low/mid and mid/high crossover Hz (Linkwitz–Riley, bands sum flat), threshold dB / ratio / gain dB per band, attack ms, release ms, output dB |
| `vocoder` | sidechain (the modulator, e.g. a voice; this insert's audio is the carrier), bandwidth, attack ms, release ms, shift semitones (moves the carrier bands), noise (hiss mixed into the carrier for consonants), mix, output dB; 16 bands 120 Hz–7.5 kHz. Without a sidechain only the dry share passes |
| `pitch_shift` | pitch semitones (−24–24), fine cents, window ms (longer is smoother, shorter tracks transients better), mix |
| `pitch_correct` | key (C–B), scale (chromatic, major, minor, harmonic minor, major pentatonic, minor pentatonic), speed ms (how quickly the pitch glides to the target; 0 snaps at once for a hard-tuned sound), amount (share of the distance to the target note that is corrected), reference Hz (the frequency of A4, 415–466), mix. For one voice or instrument at a time; see *Pitch correction and tuner* |
| `tuner` | reference Hz (the frequency of A4). Leaves audio untouched; its window shows the note heard |
| `stutter` | slice (1/32 note – 1 bar), every (period, 1/32 note – 1 bar), portion (share at the end of each period that repeats), decay (level of each repeat relative to the last), mix |
| `convolution` | mix, predelay ms, low cut Hz, high cut Hz, output dB; the impulse response is chosen with `mixer.set_effect_file` |

Effects with a `sidechain` parameter (the compressor and the vocoder) read the
post-fader output of the insert it names: a lower-numbered insert, or any insert
for the master. Removing that insert is refused while it is used; removing
another renumbers the source.

`mixer.set_effect_file <track> <slot> <file|ir:name|->` sets the convolution
reverb's impulse response: a WAV/FLAC file (any length, cut at six seconds) or
a generated room (`ir:room`, `ir:chamber`, `ir:plate`, `ir:hall`,
`ir:cathedral`); `-` restores the default hall. The response is normalized to
unit energy, so swapping rooms keeps the level. The convolution adds 512
samples of latency to the wet signal. Undoable.

The **spectrum analyzer** (View menu, or click the toolbar oscilloscope) shows
the master output's last 2048 samples by frequency (Hann window, log frequency
axis, 0 to −90 dB) with a slowly falling peak line.

The gate, auto-pan, auto filter and stutter follow the song position while playing (so they stay in
time with the beat) and a free-running clock at the project tempo while
stopped. The gate's window shows the 16 steps; clicking one turns it on or off.

Ranges and defaults are defined in `musiccraft-params`.

## Pitch correction and tuner

- Both effects detect the pitch of a single voice or instrument (monophonic,
  60–1000 Hz) from the mid signal, with a new estimate about every 5 ms. Silence
  (below −60 dBFS) and noise have no pitch.
- The **pitch corrector** finds the note of the chosen scale nearest to the
  detected pitch and shifts the audio toward it: the correction (in semitones)
  is `(target − detected) × amount`, approached with a one-pole glide whose time
  constant is `speed`. Between notes, where no pitch is detected, the last
  correction is held so consonants do not wobble. The shifted copy fades in over
  the first 10 cents of correction, so notes already in tune pass through
  untouched; with amount 0 the audio is unchanged.
- The **tuner** passes audio through unchanged.
- The engine publishes each detecting slot's latest pitch (bypassed slots
  publish none) for the UI. Their windows show the note name (A4 = MIDI note
  69 at the reference frequency), the frequency, a ±50-cent meter that turns
  the accent colour within 5 cents, and, for the corrector, the target note
  and a twelve-key strip with the scale's notes lit.

## Effect windows and presets

- Clicking an effect's name in a slot opens its window (again closes it).
  Windows of removed slots close.
- Each built-in effect window has a bypass button, a preset bar (previous,
  list, next; "(custom)" when no preset matches) and a graph:
  - filter and EQ: the frequency response; drag the filter's point for cutoff
    (across) and resonance (up), drag an EQ band's point for its frequency and
    gain, scroll on the middle band for its width;
  - compressor and limiter: the static input/output curve; drag the knee for
    the threshold, or the ceiling point;
  - distortion: the waveshaping curve; delay: the echoes over time; reverb:
    the decay; chorus: the delay modulation.
  Drags and presets are each one undo step (`edit.batch` of
  `mixer.set_effect_param`). Presets never change the sidechain source.
- Hosted plugin slots open a window with the plugin's parameter panel and a
  button for its own window.
- Changes to channel and track volume, pan and mute ramp over one engine
  block (at most 256 frames), so moving a fader never clicks.

## Sidechain

- A compressor's `sidechain` parameter names an insert whose post-fader output
  drives its level detection (classic "kick ducks the bass"). The source must be
  a lower-numbered insert, or any insert for a compressor on the master, so it is
  always processed first. A source can be taken off the master
  (`mixer.route_to_master <t> off`) to key without being heard.
- Removing a track used as a sidechain source is refused; removing or inserting
  other tracks renumbers sources.

## Metering

- The engine publishes a peak level per track; the UI shows it decaying.

## Sources

- Mixer layout (master plus numbered inserts, channel routing, per-insert effect
  slots) follows the workflow described in the FL Studio online manual, "Mixer"
  section. Effects are our own implementations of textbook designs: RBJ "Audio EQ
  Cookbook" biquads, Freeverb-style reverb (public domain algorithm by Jezar),
  feed-forward compression, peak limiting, a modulated-delay chorus, tanh soft
  clipping. The frequency shifter uses Olli Niemitalo's public-domain
  all-pass Hilbert transformer coefficients; vowel formant frequencies are
  textbook averages (Peterson & Barney style tables). The range of effect types
  (graphic and parametric EQ, gate, transient processing, multi-tap delay,
  waveshaping, frequency shifting, an all-in-one dynamics unit, a stereo
  mixing matrix) mirrors the categories in the FL Studio
  manual's effects list; the designs are ours. Convolution is uniformly
  partitioned overlap-save (textbook); the vocoder is a classic channel
  vocoder (band-pass filter banks and envelope followers); the pitch shifter is
  a two-tap crossfaded delay-line design; pitch detection is the published
  YIN method (de Cheveigné and Kawahara, JASA 2002), and the corrector drives
  the same delay-line shifter; the generated impulse responses are
  filtered decaying noise with early reflections, our own.
- Pan law: the FL Studio online manual, "Project general settings", names a
  circular (default, -3 dB at centre) and a triangular panning law. The gain
  formulas are the textbook sine/cosine and linear laws; the triangular
  formula and the handling of older files are our own choices.
- Effect windows (preset bar, graph, knobs) follow the common layout of plugin
  windows in pattern-based DAWs; the graphs and factory presets are our own.
- Track stereo separation, phase invert and swap left/right are mixer track
  properties described in the FL Studio online manual, "Mixer" (track
  properties). The mid/side formula is textbook; the ranges are our own.
