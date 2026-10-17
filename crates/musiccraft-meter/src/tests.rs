use super::*;

fn sig(s: &str) -> TimeSig {
    s.parse().unwrap()
}

#[test]
fn signatures_parse_and_measure() {
    assert_eq!(sig("4/4"), TimeSig::COMMON);
    assert_eq!(sig(" 7 / 8 ").to_string(), "7/8");
    assert_eq!(sig("4/4").bar_ticks(), 384);
    assert_eq!(sig("3/4").bar_steps(), 12);
    assert_eq!(sig("7/8").bar_ticks(), 336);
    assert_eq!(sig("6/8").beat_ticks(), 48);
    assert_eq!(sig("5/16").bar_steps(), 5);
    assert_eq!(sig("2/2").beat_ticks(), 192);
    for bad in ["4", "0/4", "33/4", "4/3", "4/32", "a/b", ""] {
        assert!(bad.parse::<TimeSig>().is_err(), "{bad}");
    }
}

#[test]
fn signatures_serialize_as_text() {
    assert_eq!(serde_json::to_string(&sig("7/8")).unwrap(), "\"7/8\"");
    let back: TimeSig = serde_json::from_str("\"3/4\"").unwrap();
    assert_eq!(back, sig("3/4"));
    assert!(serde_json::from_str::<TimeSig>("\"3/5\"").is_err());
}

#[test]
fn a_single_signature_is_a_regular_grid() {
    let meter = Meter::new(sig("3/4"));
    assert_eq!(meter.bar(0).len, 288);
    let bar = meter.bar(600);
    assert_eq!((bar.index, bar.start, bar.len), (2, 576, 288));
    assert_eq!(meter.bar_start(3), 864);
    assert_eq!(meter.round_up_to_bar(0), 288);
    assert_eq!(meter.round_up_to_bar(288), 288);
    assert_eq!(meter.round_up_to_bar(289), 576);
    assert_eq!(meter.floor_to_beat(200), 192);
    assert!((meter.ticks_to_bars(432.0) - 1.5).abs() < 1e-9);
    assert_eq!(meter.bars_to_ticks(1.5), 432);
    assert_eq!(meter.bars_to_ticks(-1.0), 0);
}

#[test]
fn a_change_starts_a_new_bar_and_cuts_the_one_before() {
    // 4/4 for bars 1-2, then 7/8 from tick 768; a 3/4 change mid-bar at 1000.
    let meter = Meter::with_changes(sig("4/4"), [(1000, sig("3/4")), (768, sig("7/8"))]);
    assert_eq!(meter.sig_at(767), sig("4/4"));
    assert_eq!(meter.sig_at(768), sig("7/8"));
    let cut = meter.bar(900);
    assert_eq!((cut.index, cut.start, cut.len), (2, 768, 232));
    let after = meter.bar(1000);
    assert_eq!((after.index, after.start, after.len), (3, 1000, 288));
    assert_eq!(meter.bar_start(4), 1288);
    assert_eq!(meter.bars_to_ticks(2.5), 884);
    assert_eq!(meter.span(768, 2.0), 520);
    assert_eq!(
        meter.changes().collect::<Vec<_>>(),
        [(768, sig("7/8")), (1000, sig("3/4"))]
    );
}

#[test]
fn a_change_at_zero_replaces_the_start() {
    let meter = Meter::with_changes(sig("4/4"), [(0, sig("6/8"))]);
    assert_eq!(meter.first(), sig("6/8"));
    assert_eq!(meter.changes().count(), 0);
}

#[test]
fn beats_follow_the_signature_and_mark_downbeats() {
    let meter = Meter::with_changes(sig("3/4"), [(288, sig("6/8"))]);
    let beats: Vec<_> = meter
        .beats(0, 576)
        .map(|b| (b.tick, b.bar, b.downbeat))
        .collect();
    let mut want = vec![(0, 0, true), (96, 0, false), (192, 0, false)];
    want.extend((0..6).map(|k| (288 + 48 * k, 1, k == 0)));
    assert_eq!(beats, want);
    // A bar cut short ends on the change, which is a downbeat.
    let cut = Meter::with_changes(sig("4/4"), [(150, sig("4/4"))]);
    let next = cut.next_beat(97.0);
    assert_eq!((next.tick, next.downbeat), (150, true));
}

#[test]
fn positions_read_bar_beat_step() {
    let meter = Meter::new(TimeSig::COMMON);
    let p = meter.position(384 + 96 + 24);
    assert_eq!((p.bar, p.beat, p.step), (2, 2, 2));
    let eighths = Meter::new(sig("7/8"));
    let p = eighths.position(336 + 48 * 3 + 24);
    assert_eq!((p.bar, p.beat, p.step), (2, 4, 2));
}
