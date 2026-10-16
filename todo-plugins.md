# Stock plugin parity TODO

Work list for giving MusicCraft its own equivalent of every stock generator
and effect listed in the public FL Studio online manual ("Plugins" section).
This is separate from `todo.md` (workflow parity), which another agent owns.

Clean-room rules apply to every item (see `docs/clean-room.md`):

- Work only from what the public manual says a plugin is *for* and our own
  textbook DSP designs. Never inspect binaries, presets, skins or sounds.
- Product names appear only in the **Reference** column, for comparison.
  MusicCraft's effects and instruments get their own names, artwork and
  presets.
- Write or update the spec (`docs/spec/mixer.md` for effects,
  `docs/spec/channel-rack.md` for instruments) before the code, then update
  `docs/builtin-coverage.md` and `docs/parity.md` when an item lands.

Difficulty: **easy** = reuses existing DSP blocks or is a small new block;
**medium** = new DSP plus a new editor graph; **hard** = new engine feature,
new UI editor or large DSP.

## Effects

### Easy

- [x] Fruity Limiter (limiter, compressor and gate in one window) →
  `dynamics`: gate → compressor → limiter in one insert.
- [x] Fruity Stereo Shaper → `stereo_shaper`: left/right mixing matrix plus an
  inter-channel delay (Haas widening), also covers channel swap and
  mono-from-one-side.
- [x] Fruity Delay Bank / Multiband Delay → per-tap low/high cut and pan for
  `multi_tap` (new parameters appended, old projects keep their sound).
- [x] Maximus → `multiband_limiter`: the `multiband` crossovers with a
  limiter per band and a master ceiling.
- [x] Soundgoodizer → one-knob "enhance" preset family on `multiband_limiter`.
- [x] Fruity Squeeze → add a "squeeze" (sample-and-hold amplitude) mode to
  `bitcrusher`.
- [x] Fruity Fast Dist → threshold-style hard clipping shape on `distortion`.
- [x] Vintage Chorus → `chorus` gains a "voices" (1–3) parameter and a
  bucket-brigade style low-pass on the wet path.
- [x] Vintage Phaser → `phaser` gains a "warm" option (soft saturation in the
  feedback loop).
- [x] Fruity Send → per-insert send level as an effect (currently only mixer
  routing sends).

### Medium

- [x] Fruity WaveShaper → a drawable transfer curve for `waveshaper` (curve
  points as parameters, dragged in the effect window).
- [x] Fruity Love Philter → drawn per-target envelope shapes for
  `auto_filter` (beyond one LFO and the envelope follower).
- [x] Fruity Delay 3 → modulation, diffusion and a feedback filter for
  `delay`.
- [x] Luxeverb → a higher-density reverb (FDN design) with modulation and
  early reflections as a second reverb kind.
- [x] Hardcore → `guitar_chain`: a fixed chain of compressor, overdrive, tone
  stack, chorus and cabinet filter with per-stage bypass.
- [x] Effector → `multi_fx`: one insert that picks one of several existing
  effect blocks with an X/Y control.
- [x] Distructor → chainable multi-effect (several existing blocks in series
  inside one insert).
- [x] Gross Beat → drawn time (scratch/repeat) and volume envelopes per beat,
  extending `stutter` and `gate`.
- [x] Frequency Splitter → split a track into three bands routed to
  separate mixer inserts (needs multi-output routing).
- [x] Fruity Scratcher → turntable-style scratching of a sample or the input
  buffer (currently "not planned"; revisit).
- [x] Pitcher → MIDI-controlled target notes and harmonies for
  `pitch_correct`.

### Hard

- [ ] Newtone / Newtime → offline note and time editing of audio (needs an
  audio editor view).
- [ ] Edison → an audio editor/recorder window (currently "not planned").
- [x] Vocodex → more bands (up to 100), band distribution and per-band
  envelopes for `vocoder`.

## Generators

### Easy

- [x] SimSynth / MiniSynth → second filter and a second LFO per synth voice.
- [x] BooBass → dedicated "bass" synth preset bank with tone/pluck macros.
- [x] Fruity DX10 → already covered; add more factory presets of our own.

### Medium

- [x] DirectWave → multi-sample key and velocity zones in the sampler.
- [x] Sytrus / Toxic Biohazard → six FM operators, per-operator filters and
  drawn operator waveforms.
- [x] Sakura → bowed string excitation and multi-string model for the pluck
  instrument.
- [x] Morphine → morphing between two or more additive spectra.
- [x] Wasp / PoiZone / Sawer → per-oscillator filters and a modulation matrix
  in the synth.
- [x] Fruity Envelope / Keyboard / X-Y Controllers → controller channels whose
  outputs drive parameters like automation.
- [x] Peak Controller → a controller driven by an insert's audio level.
- [x] Formula Controller → controller driven by a formula of time and inputs.
- [x] MIDI Out → controller knobs, program changes and MIDI clock.

### Hard

- [ ] Harmor / Harmless → image and audio resynthesis, blur and prism.
- [ ] Patcher → modular graph of instruments and effects inside one slot
  (currently "not planned").
- [ ] VST3 hosting (Fruity Wrapper) → extends the `unsafe` exception; needs
  the user's go-ahead before any work.

## Not planned (unchanged)

FLEX, Autogun, BeepMap, Dashboard, Fruity Video Player, ZGameEditor
Visualizer, Fruity Big Clock, Notebook, HTML Notebook, Control Surface: see
`docs/builtin-coverage.md` for the reasons.

## Sources

- What each reference plugin is for: FL Studio online manual, "Plugins"
  section (public). Only the purpose is used; designs, names and presets are
  our own.
- Existing coverage: `docs/builtin-coverage.md`.
