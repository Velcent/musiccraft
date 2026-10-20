use std::collections::BTreeSet;

use musiccraft_antiserum::{index_of, params};
use musiccraft_patch::{CATEGORIES, Patch, TableRef};

use crate::{factory, find, label, sequence, synth, text, values, wavetable};

const SR: f32 = 48_000.0;

#[test]
fn the_bank_has_its_categories_and_unique_names() {
    let bank = factory();
    let counts: Vec<usize> = CATEGORIES
        .iter()
        .map(|c| bank.iter().filter(|p| p.category == *c).count())
        .collect();
    assert_eq!(counts, [25, 25, 25, 20, 20, 15, 20]);
    let names: BTreeSet<&str> = bank.iter().map(|p| p.name.as_str()).collect();
    assert_eq!(names.len(), bank.len(), "names are unique");
    assert!(bank.iter().all(|p| !p.tags.is_empty()));
    assert_eq!(
        find("glass reese").map(|p| p.name.as_str()),
        Some("Glass Reese")
    );
}

#[test]
fn every_preset_names_real_parameters_and_tables() {
    for patch in factory() {
        for name in patch.params.keys() {
            assert!(index_of(name).is_some(), "{}: {name}", patch.name);
        }
        for osc in 0..3 {
            wavetable(patch.table(osc)).unwrap_or_else(|e| panic!("{}: {e}", patch.name));
        }
        // Routes point at real parameters.
        for (name, value) in &patch.params {
            if name.ends_with("_dest") {
                assert!(*value >= 1.0, "{}: {name} has no destination", patch.name);
            }
        }
    }
}

fn rms(x: &[f32]) -> f32 {
    (x.iter().map(|v| v * v).sum::<f32>() / x.len().max(1) as f32).sqrt()
}

#[test]
fn every_preset_sounds_and_stays_bounded() {
    for patch in factory() {
        let mut synth = synth(patch, SR);
        let key = if patch.category == "bass" { 36 } else { 60 };
        synth.note_on(key, 0.9);
        let mut left = vec![0.0; 24_000];
        let mut right = vec![0.0; 24_000];
        for (l, r) in left.chunks_mut(256).zip(right.chunks_mut(256)) {
            synth.process(l, r);
        }
        let all = left.iter().chain(&right);
        assert!(all.clone().all(|x| x.is_finite()), "{}", patch.name);
        let peak = all.fold(0.0_f32, |m, x| m.max(x.abs()));
        assert!(peak < 4.0, "{}: peak {peak}", patch.name);
        assert!(rms(&left) + rms(&right) > 1e-3, "{} is silent", patch.name);
    }
}

#[test]
fn patches_apply_values_tables_shapes_and_steps() {
    let mut patch = Patch::default();
    patch.params.insert("osc1_unison".to_owned(), 99.0);
    patch.params.insert("nope".to_owned(), 1.0);
    patch.set_lfo_shape(2, vec![[0.0, 1.0, 0.0], [0.5, 0.0, 0.0]]);
    patch.set_step(1, (false, 5, 0.5, 0.2), (true, 0, 1.0, 0.5));
    let v = values(&patch);
    assert_eq!(v[index_of("osc1_unison").unwrap()], 16.0, "sanitized");
    assert_eq!(sequence(&patch).steps[1].transpose, 5);
    let synth = synth(&patch, SR);
    assert_eq!(synth.lfo_shape(2).unwrap().points().len(), 2);
    assert!(wavetable("factory:nope").is_err());
    let data = TableRef::Data(vec![0.0; 2048 * 2]).to_text();
    assert_eq!(wavetable(&data).unwrap().frames(), 2);
}

#[test]
fn text_and_labels() {
    let i = |n: &str| index_of(n).unwrap();
    assert_eq!(text(i("filter1_type"), 10.0), "ladder");
    assert_eq!(text(i("fx_slot1"), 1.0), "dimension");
    assert_eq!(text(i("filter1_cutoff"), 2_000.0), "2.00 kHz");
    assert_eq!(text(i("osc1_level"), 0.5), "50%");
    assert_eq!(text(i("osc1_pan"), -0.5), "-50%");
    assert_eq!(text(i("arp_rate"), 1.0), "1/16");
    assert!(!text(i("fx_delay_time"), 4.0).is_empty());
    assert_eq!(label("osc2_warp1_amount"), "warp1 amount");
    assert_eq!(label("fx_reverb_decay"), "decay");
    assert_eq!(label("volume"), "volume");
    assert_eq!(params().len(), musiccraft_antiserum::PARAM_COUNT);
}
