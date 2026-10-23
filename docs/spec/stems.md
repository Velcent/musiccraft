# Spec: stem splitting

Splits a sound into four stems, **vocals**, **drums**, **bass** and
**other**, each a new sampler channel playing its own file, so a mixed song
or loop can be remixed, sampled or practised along with.

## Behaviour

- `sample.split_stems <channel>` reads the sampler channel's sample, splits
  it and writes four 32-bit float WAV files at the sample's own rate into
  `stems/<sample name>-<n>/` beside the project (the first free number):
  `vocals.wav`, `drums.wav`, `bass.wav` and `other.wav`. Then it does what
  `sample.add_stems` does with them. The project must have been saved, so
  the files have a home. The original file is never changed.
- `sample.add_stems <channel> <vocals> <drums> <bass> <other>` adds the four
  stems of a sampler channel from files that already exist (paths absolute
  or relative to the project):
  - Four sampler channels are added at the end of the channel rack, copies
    of the source channel (volume, pan, mixer track, root key, fades and the
    other sampler settings) named "<name> (vocals)", "<name> (drums)",
    "<name> (bass)" and "<name> (other)", each playing its stem.
  - Every audio clip of the source channel gets four clips of the stems, at
    the same start, length and offset, on the first four neighbouring tracks
    below it that are free for the clip's whole length (the command fails if
    there are none). The source clip is muted, and the stem clips take its
    old muted state, so the song sounds the same until the stems are changed.
  - One undo step removes the four channels and their clips and unmutes the
    source clips. The files stay on disk.
- Only sampler channels can be split (the stems must play the sound as it
  is). Built-in sounds are split as rendered at 48 kHz.
- The four stems always add up to the original sound (to rounding), so
  nothing is lost: what the splitter cannot place goes to **other**.

## The built-in splitter

The splitter needs no model or download. It works on a short-time spectrum
(Hann-root windows of about 85 ms at quarter-window hops, overlap-added
back) and gives every frequency bin of every frame a share of each stem;
the shares add up to one.

- **Drums**: harmonic–percussive separation by median filtering
  (Fitzgerald, 2010). Across time, a bin's median (17 frames) follows
  sustained tones; across frequency (17 bins), it follows clicks and hits.
  The percussive share (soft mask, squared magnitudes) is the drums stem.
- **Bass**: the harmonic share below about 120 Hz, fading out by 250 Hz.
- **Vocals**: the harmonic share between about 150 Hz and 8 kHz, weighted
  by how centred the bin is: the left and right channels' agreement
  (`2·Re(L·R̄) / (|L|² + |R|²)`, clamped at 0), raised to the sixth power
  so only near-centre content counts. Mono sounds count as centred.
- **Other**: the rest, taken as the original minus the other three stems.

This is a rough split. Drums and bass come out well on most mixes; vocals
work on stereo mixes with a centred voice, and take any centred lead
instrument with them. A trained model can replace the splitter later
behind the same commands.

## Interface

- **Split into stems** appears in the channel menu of sampler channels
  (channel rack and channel settings), the playlist's Clip menu (one split
  for each sound the selected audio clips play) and the right-click menu of
  an audio item in the playlist's clip list (with Channel settings…).
- The app splits in the background: the hint bar shows "Splitting ‹name›
  into stems: 40%", and the app stays usable. When the split finishes, the
  app runs `sample.add_stems` with the new files (one undo step), on the
  channel wherever it has moved meanwhile. One split runs at a time; more
  wait in turn ("Splitting ‹name› into stems; 2 more waiting"). The CLI and
  MCP run `sample.split_stems` in one go.

## Sources

- Own design. Four-stem splitting (vocals, drums, bass, other) is the
  common split of open source separators; harmonic–percussive separation by
  median filtering is the published method of D. Fitzgerald, "Harmonic/
  percussive separation using median filtering" (DAFx 2010); centre
  extraction by inter-channel similarity follows C. Avendano and J.-M. Jot,
  "Frequency domain techniques for stereo to multichannel upmix" (AES 2002).
