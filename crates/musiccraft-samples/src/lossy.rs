//! Lossy export (`docs/spec/project-file.md`): MP3 through LAME and Ogg Vorbis
//! through libvorbis (aoTuV), both C libraries built from source by their
//! crates. Main thread only.

use std::num::{NonZeroU8, NonZeroU32};
use std::path::Path;

use mp3lame_encoder::{Bitrate, Builder, DualPcm, FlushNoGap, Id3Tag, Quality};
use vorbis_rs::{VorbisBitrateManagementStrategy, VorbisEncoderBuilder};

use crate::SampleError;

/// MP3 bit rates offered, in kbit/s.
pub const MP3_BITRATES: [u32; 4] = [128, 192, 256, 320];
/// The highest sample rate MP3 carries.
pub const MP3_MAX_RATE: u32 = 48_000;
/// The Ogg stream's serial number: fixed, so the same audio always gives the
/// same file.
const OGG_SERIAL: i32 = 0x4D43_5446;
/// Frames handed to an encoder at a time.
const CHUNK: usize = 4096;

fn format(what: &str, error: impl std::fmt::Display) -> SampleError {
    SampleError::Format(format!("{what}: {error}"))
}

fn write(path: &Path, bytes: &[u8]) -> Result<(), SampleError> {
    std::fs::write(path, bytes)
        .map_err(|e| SampleError::Io(format!("cannot write {}: {e}", path.display())))
}

/// Stereo `left`/`right` at `sample_rate` as MP3 at `kbps` (one of
/// [`MP3_BITRATES`]), constant bit rate, titled `title`.
///
/// # Errors
///
/// An unsupported bit rate or sample rate, or an encoder failure.
pub fn encode_mp3(
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
    kbps: u32,
    title: &str,
) -> Result<Vec<u8>, SampleError> {
    let bitrate = match kbps {
        128 => Bitrate::Kbps128,
        192 => Bitrate::Kbps192,
        256 => Bitrate::Kbps256,
        320 => Bitrate::Kbps320,
        other => {
            return Err(SampleError::Format(format!(
                "{other} kbit/s is not offered"
            )));
        }
    };
    if !(8_000..=MP3_MAX_RATE).contains(&sample_rate) {
        return Err(SampleError::Format(format!(
            "MP3 carries 8000–48000 Hz, not {sample_rate}"
        )));
    }
    let mut builder = Builder::new().ok_or_else(|| format("MP3", "LAME did not start"))?;
    builder.set_num_channels(2).map_err(|e| format("MP3", e))?;
    builder
        .set_sample_rate(sample_rate)
        .map_err(|e| format("MP3", e))?;
    builder.set_brate(bitrate).map_err(|e| format("MP3", e))?;
    builder
        .set_quality(Quality::Best)
        .map_err(|e| format("MP3", e))?;
    builder
        .set_id3_tag(Id3Tag {
            title: title.as_bytes(),
            artist: &[],
            album: &[],
            year: &[],
            comment: b"MusicCraft",
            album_art: &[],
        })
        .map_err(|e| format("MP3 tag", format!("{e:?}")))?;
    let mut encoder = builder.build().map_err(|e| format("MP3", e))?;
    let frames = left.len().min(right.len());
    let mut out = Vec::new();
    for start in (0..frames).step_by(CHUNK) {
        let end = (start + CHUNK).min(frames);
        let input = DualPcm {
            left: &left[start..end],
            right: &right[start..end],
        };
        out.reserve(mp3lame_encoder::max_required_buffer_size(end - start));
        encoder
            .encode_to_vec(input, &mut out)
            .map_err(|e| format("MP3", e))?;
    }
    out.reserve(7_200);
    encoder
        .flush_to_vec::<FlushNoGap>(&mut out)
        .map_err(|e| format("MP3", e))?;
    Ok(out)
}

/// [`encode_mp3`] written to `path`.
///
/// # Errors
///
/// As [`encode_mp3`], or the file cannot be written.
pub fn write_mp3(
    path: &Path,
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
    kbps: u32,
    title: &str,
) -> Result<(), SampleError> {
    write(path, &encode_mp3(left, right, sample_rate, kbps, title)?)
}

/// Stereo `left`/`right` at `sample_rate` as Ogg Vorbis at `quality` 0–10
/// (variable bit rate), titled `title`.
///
/// # Errors
///
/// A quality or sample rate out of range, or an encoder failure.
pub fn encode_ogg(
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
    quality: u32,
    title: &str,
) -> Result<Vec<u8>, SampleError> {
    if quality > 10 {
        return Err(SampleError::Format(format!(
            "Ogg quality is 0–10, not {quality}"
        )));
    }
    let rate = NonZeroU32::new(sample_rate).ok_or_else(|| format("Ogg", "no sample rate"))?;
    let channels = NonZeroU8::new(2).ok_or_else(|| format("Ogg", "no channels"))?;
    let mut builder = VorbisEncoderBuilder::new_with_serial(rate, channels, Vec::new(), OGG_SERIAL);
    builder.bitrate_management_strategy(VorbisBitrateManagementStrategy::QualityVbr {
        target_quality: quality as f32 / 10.0,
    });
    builder
        .comment_tags([("TITLE", title), ("ENCODER", "MusicCraft")])
        .map_err(|e| format("Ogg tags", e))?;
    let mut encoder = builder.build().map_err(|e| format("Ogg", e))?;
    let frames = left.len().min(right.len());
    for start in (0..frames).step_by(CHUNK) {
        let end = (start + CHUNK).min(frames);
        encoder
            .encode_audio_block([&left[start..end], &right[start..end]])
            .map_err(|e| format("Ogg", e))?;
    }
    encoder.finish().map_err(|e| format("Ogg", e))
}

/// [`encode_ogg`] written to `path`.
///
/// # Errors
///
/// As [`encode_ogg`], or the file cannot be written.
pub fn write_ogg(
    path: &Path,
    left: &[f32],
    right: &[f32],
    sample_rate: u32,
    quality: u32,
    title: &str,
) -> Result<(), SampleError> {
    write(path, &encode_ogg(left, right, sample_rate, quality, title)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symphonia::core::audio::SampleBuffer;
    use symphonia::core::codecs::DecoderOptions;
    use symphonia::core::formats::FormatOptions;
    use symphonia::core::io::MediaSourceStream;
    use symphonia::core::meta::MetadataOptions;
    use symphonia::core::probe::Hint;

    /// One second: 440 Hz on the left, 660 Hz on the right.
    fn tone(rate: u32) -> (Vec<f32>, Vec<f32>) {
        (0..rate)
            .map(|i| {
                let t = i as f32 / rate as f32;
                let tau = std::f32::consts::TAU;
                (0.5 * (tau * 440.0 * t).sin(), 0.5 * (tau * 660.0 * t).sin())
            })
            .unzip()
    }

    /// Decodes `bytes` with symphonia (a separate, pure-Rust decoder):
    /// sample rate and the two channels.
    fn decode(bytes: Vec<u8>, extension: &str) -> (u32, Vec<f32>, Vec<f32>) {
        let source =
            MediaSourceStream::new(Box::new(std::io::Cursor::new(bytes)), Default::default());
        let mut hint = Hint::new();
        hint.with_extension(extension);
        let mut format = symphonia::default::get_probe()
            .format(
                &hint,
                source,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .unwrap()
            .format;
        let track = format.default_track().unwrap();
        let id = track.id;
        let rate = track.codec_params.sample_rate.unwrap();
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .unwrap();
        let (mut left, mut right) = (Vec::new(), Vec::new());
        while let Ok(packet) = format.next_packet() {
            if packet.track_id() != id {
                continue;
            }
            let decoded = decoder.decode(&packet).unwrap();
            let mut buffer = SampleBuffer::<f32>::new(decoded.capacity() as u64, *decoded.spec());
            buffer.copy_interleaved_ref(decoded);
            for frame in buffer.samples().chunks(2) {
                left.push(frame[0]);
                right.push(frame[1]);
            }
        }
        (rate, left, right)
    }

    /// Zero crossings per second over the middle of `samples`.
    fn pitch(samples: &[f32], rate: u32) -> f32 {
        let middle = &samples[samples.len() / 4..samples.len() * 3 / 4];
        let crossings = middle
            .windows(2)
            .filter(|w| w[0] <= 0.0 && w[1] > 0.0)
            .count();
        crossings as f32 * rate as f32 / middle.len() as f32
    }

    fn check(name: &str, (rate, left, right): (u32, Vec<f32>, Vec<f32>), expected_rate: u32) {
        assert_eq!(rate, expected_rate, "{name}");
        // Encoder delay and padding add a little; nothing is lost.
        assert!(
            (expected_rate as usize..expected_rate as usize * 11 / 10).contains(&left.len()),
            "{name}: {} frames",
            left.len()
        );
        let (l, r) = (pitch(&left, rate), pitch(&right, rate));
        assert!((l - 440.0).abs() < 3.0, "{name}: left at {l} Hz");
        assert!((r - 660.0).abs() < 3.0, "{name}: right at {r} Hz");
        let peak = left.iter().fold(0.0f32, |p, s| p.max(s.abs()));
        assert!((0.4..0.6).contains(&peak), "{name}: peak {peak}");
    }

    #[test]
    fn mp3_decodes_to_the_same_tones() {
        for kbps in MP3_BITRATES {
            let (l, r) = tone(44_100);
            let bytes = encode_mp3(&l, &r, 44_100, kbps, "Tones").unwrap();
            // Constant bit rate: about kbps/8 kilobytes per second.
            let expected = (kbps * 1000 / 8) as usize;
            assert!(
                bytes.len() > expected * 9 / 10 && bytes.len() < expected * 13 / 10,
                "{kbps} kbit/s: {} bytes",
                bytes.len()
            );
            check(&format!("mp3 {kbps}"), decode(bytes, "mp3"), 44_100);
        }
    }

    #[test]
    fn ogg_decodes_to_the_same_tones_and_quality_sets_the_size() {
        let (l, r) = tone(48_000);
        let low = encode_ogg(&l, &r, 48_000, 0, "Tones").unwrap();
        let high = encode_ogg(&l, &r, 48_000, 10, "Tones").unwrap();
        assert!(high.len() > low.len(), "{} vs {}", high.len(), low.len());
        check(
            "ogg",
            decode(encode_ogg(&l, &r, 48_000, 8, "Tones").unwrap(), "ogg"),
            48_000,
        );
        assert!(low.windows(5).any(|w| w == b"Tones"), "title comment");
    }

    #[test]
    fn exports_are_deterministic() {
        let (l, r) = tone(44_100);
        assert_eq!(
            encode_ogg(&l, &r, 44_100, 6, "x").unwrap(),
            encode_ogg(&l, &r, 44_100, 6, "x").unwrap()
        );
        assert_eq!(
            encode_mp3(&l, &r, 44_100, 192, "x").unwrap(),
            encode_mp3(&l, &r, 44_100, 192, "x").unwrap()
        );
    }

    #[test]
    fn bad_settings_are_refused() {
        let (l, r) = tone(8_000);
        assert!(encode_mp3(&l, &r, 44_100, 100, "").is_err());
        assert!(encode_mp3(&l, &r, 96_000, 320, "").is_err());
        assert!(encode_ogg(&l, &r, 44_100, 11, "").is_err());
    }
}
