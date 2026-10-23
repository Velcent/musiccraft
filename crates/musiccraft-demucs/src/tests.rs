use super::*;

use musiccraft_stems::Stem;

/// HTDemucs's own transforms at 256 bins (`tools/stem-model/spectrum_fixture.py`).
#[derive(Deserialize)]
struct Fixture {
    x: Vec<f32>,
    spec_re: Vec<f32>,
    spec_im: Vec<f32>,
    frames: usize,
    z_re: Vec<f32>,
    z_im: Vec<f32>,
    inverse: Vec<f32>,
}

fn fixture() -> Fixture {
    serde_json::from_str(include_str!("spectrum.json")).unwrap()
}

#[test]
fn the_transforms_match_htdemucs() {
    let f = fixture();
    let t = Transform::new(256);
    assert_eq!(t.frames(f.x.len()), f.frames);
    let z = t.forward(&f.x);
    for (i, c) in z.iter().enumerate() {
        assert!((c.0 - f.spec_re[i]).abs() < 1e-4, "bin {i}: {c:?}");
        assert!((c.1 - f.spec_im[i]).abs() < 1e-4, "bin {i}: {c:?}");
    }
    let w: Vec<_> = f.z_re.iter().zip(&f.z_im).map(|(r, i)| (*r, *i)).collect();
    let y = t.inverse(&w, f.frames, f.x.len());
    for (i, (a, b)) in y.iter().zip(&f.inverse).enumerate() {
        assert!((a - b).abs() < 1e-4, "sample {i}: {a} vs {b}");
    }
}

#[test]
fn cross_fades_follow_demucs() {
    assert_eq!(weights(4), [0.5, 1.0, 1.0, 0.5]);
    assert_eq!(
        weights(5),
        [1.0 / 3.0, 2.0 / 3.0, 1.0, 2.0 / 3.0, 1.0 / 3.0]
    );
    let x = [1.0, 2.0, 3.0, 4.0, 5.0];
    assert_eq!(padded(&x, 0, 5, 5), x);
    // A short last chunk is centred, taking context from before it.
    assert_eq!(padded(&x, 3, 2, 4), [3.0, 4.0, 5.0, 0.0]);
    assert_eq!(padded(&x, 4, 1, 4), [4.0, 5.0, 0.0, 0.0]);
}

fn small() -> Info {
    Info {
        sources: ["drums", "bass", "other", "vocals"]
            .map(String::from)
            .to_vec(),
        sample_rate: 22_050,
        segment: 2_000,
        nfft: 256,
        hop: 64,
    }
}

/// Hands the mixture back as one source, from either branch.
struct Pass {
    info: Info,
    source: usize,
    spectral: bool,
    runs: usize,
}

impl Network for Pass {
    fn run(&mut self, mix: Vec<f32>, spec: Vec<f32>) -> Result<(Vec<f32>, Vec<f32>), String> {
        self.runs += 1;
        let n = self.info.sources.len();
        let mut freq = vec![0.0; n * spec.len()];
        let mut time = vec![0.0; n * mix.len()];
        if self.spectral {
            freq[self.source * spec.len()..(self.source + 1) * spec.len()].copy_from_slice(&spec);
        } else {
            time[self.source * mix.len()..(self.source + 1) * mix.len()].copy_from_slice(&mix);
        }
        Ok((freq, time))
    }
}

fn mixture(len: usize) -> (Vec<f32>, Vec<f32>) {
    let left = (0..len).map(|i| 0.4 * (i as f32 * 0.07).sin()).collect();
    let right = (0..len).map(|i| 0.3 * (i as f32 * 0.11).cos()).collect();
    (left, right)
}

fn close(a: &[f32], b: &[f32], tolerance: f32) {
    assert_eq!(a.len(), b.len());
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        assert!((x - y).abs() < tolerance, "sample {i}: {x} vs {y}");
    }
}

#[test]
fn segments_line_up_through_both_branches() {
    // 5 300 samples: three overlapping segments and a short last one.
    let (left, right) = mixture(5_300);
    for (source, stem, spectral) in [
        (3, Stem::Vocals, true),
        (0, Stem::Drums, false),
        (1, Stem::Bass, true),
    ] {
        let mut net = Pass {
            info: small(),
            source,
            spectral,
            runs: 0,
        };
        let stems = separate(&mut net, &small(), &left, &right, 22_050.0, &mut |_| true).unwrap();
        assert_eq!(net.runs, 4);
        // The spectral branch loses each segment's first and last two
        // frames, as in HTDemucs (the cross-fades hide most of it). The
        // time branch only carries the mean Demucs adds to every source.
        let (edge, tolerance) = if spectral { (256, 1e-2) } else { (0, 2e-3) };
        let inside = edge..left.len() - edge;
        let got = stems.get(stem);
        close(&got.left[inside.clone()], &left[inside.clone()], tolerance);
        close(
            &got.right[inside.clone()],
            &right[inside.clone()],
            tolerance,
        );
        let other = stems.get(Stem::Other);
        close(
            &other.left[inside.clone()],
            &vec![0.0; inside.len()],
            tolerance,
        );
    }
}

#[test]
fn other_rates_are_resampled_and_the_stems_add_up() {
    let (left, right) = mixture(4_800);
    let mut net = Pass {
        info: small(),
        source: 3,
        spectral: false,
        runs: 0,
    };
    let stems = separate(&mut net, &small(), &left, &right, 48_000.0, &mut |_| true).unwrap();
    let vocals = stems.get(Stem::Vocals);
    assert_eq!(vocals.left.len(), left.len());
    close(&vocals.left[10..4_700], &left[10..4_700], 2e-2);
    for (i, x) in left.iter().enumerate() {
        let sum: f32 = Stem::ALL.iter().map(|s| stems.get(*s).left[i]).sum();
        assert!((sum - x).abs() < 1e-5);
    }
}

#[test]
fn progress_counts_segments_and_can_cancel() {
    let (left, right) = mixture(5_300);
    let mut net = Pass {
        info: small(),
        source: 0,
        spectral: false,
        runs: 0,
    };
    let mut seen = Vec::new();
    let result = separate(&mut net, &small(), &left, &right, 22_050.0, &mut |p| {
        seen.push(p);
        seen.len() < 2
    });
    assert_eq!(result.err(), Some(Error::Cancelled));
    assert_eq!(seen, [0.25, 0.5]);
}

/// Splits the test case `tools/stem-model/export.py --reference` wrote and
/// compares with Demucs itself. Needs ONNX Runtime and the exported model:
/// `MUSICCRAFT_STEM_MODEL=model.onnx MUSICCRAFT_STEM_REFERENCE=dir/
/// ORT_DYLIB_PATH=libonnxruntime.so cargo test -p musiccraft-demucs -- --ignored`.
#[test]
#[ignore = "needs ONNX Runtime and an exported model"]
fn the_model_matches_demucs() {
    let model = PathBuf::from(std::env::var("MUSICCRAFT_STEM_MODEL").unwrap());
    let folder = PathBuf::from(std::env::var("MUSICCRAFT_STEM_REFERENCE").unwrap());
    let read = |name: &str| -> Vec<f32> {
        std::fs::read(folder.join(name))
            .unwrap()
            .chunks_exact(4)
            .map(|b| f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
            .collect()
    };
    #[derive(Deserialize)]
    struct Reference {
        frames: usize,
        sources: Vec<String>,
    }
    let reference: Reference =
        serde_json::from_slice(&std::fs::read(folder.join("reference.json")).unwrap()).unwrap();
    let (mix, want) = (read("mix.f32"), read("stems.f32"));
    let n = reference.frames;
    let mut demucs = Demucs::load(&model).unwrap();
    eprintln!("running on {}", demucs.backend());
    let stems = demucs
        .split(&mix[..n], &mix[n..], 44_100.0, &mut |_| true)
        .unwrap();
    for (s, name) in reference.sources.iter().enumerate() {
        let stem = match name.as_str() {
            "vocals" => Stem::Vocals,
            "drums" => Stem::Drums,
            "bass" => Stem::Bass,
            _ => continue,
        };
        let got = stems.get(stem);
        for (c, side) in [&got.left, &got.right].into_iter().enumerate() {
            let expected = &want[(s * 2 + c) * n..(s * 2 + c + 1) * n];
            let err = side
                .iter()
                .zip(expected)
                .map(|(a, b)| (a - b).abs())
                .fold(0.0_f32, f32::max);
            assert!(err < 1e-3, "{name} channel {c}: largest difference {err}");
        }
    }
}
