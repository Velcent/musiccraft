//! Turning control changes into automation while recording.

use crate::{BATCH, Invocation};

/// The automation target a control command sets, if automation can drive it
/// (`channel:1:volume`, `mixer:0:pan`, `mixer:2:1:mix`, ...).
#[must_use]
pub fn recordable_target(invocation: &Invocation) -> Option<String> {
    let args = &invocation.args;
    match (invocation.name.as_str(), args.as_slice()) {
        ("channel.set_volume", [n, _]) => Some(format!("channel:{n}:volume")),
        ("channel.set_pan", [n, _]) => Some(format!("channel:{n}:pan")),
        ("channel.set_param", [n, param, _]) => Some(format!("channel:{n}:{param}")),
        ("mixer.set_volume", [t, _]) => Some(format!("mixer:{t}:volume")),
        ("mixer.set_pan", [t, _]) => Some(format!("mixer:{t}:pan")),
        ("mixer.set_effect_param", [t, slot, param, _]) => {
            Some(format!("mixer:{t}:{slot}:{param}"))
        }
        _ => None,
    }
}

/// `invocation` followed by `automation.record` of what it sets at song
/// position `tick`, as one batch; `None` when automation cannot drive it.
#[must_use]
pub fn with_automation_record(invocation: &Invocation, tick: u32) -> Option<Invocation> {
    let target = recordable_target(invocation)?;
    Some(Invocation::new(
        BATCH,
        [
            invocation.to_string(),
            Invocation::new("automation.record", [target, format!("{tick}t")]).to_string(),
        ],
    ))
}

#[cfg(test)]
mod tests {
    use musiccraft_project::Project;

    use super::*;
    use crate::{Registry, Session};

    fn session() -> Session {
        let mut s = Session::new(Project::default(), Registry::with_builtin());
        for line in [
            "channel.add_synth",
            "playlist.add_clip 1 1 0 4",
            "mixer.add_effect 1 reverb",
        ] {
            s.run_line(line).unwrap();
        }
        s
    }

    #[test]
    fn controls_map_to_automation_targets() {
        let target = |line: &str| recordable_target(&Invocation::parse(line).unwrap());
        assert_eq!(
            target("channel.set_volume 1 0.5").unwrap(),
            "channel:1:volume"
        );
        assert_eq!(
            target("channel.set_param 2 cutoff 900").unwrap(),
            "channel:2:cutoff"
        );
        assert_eq!(target("mixer.set_pan 0 0.1").unwrap(), "mixer:0:pan");
        assert_eq!(
            target("mixer.set_effect_param 1 1 mix 0.3").unwrap(),
            "mixer:1:1:mix"
        );
        assert_eq!(target("channel.rename 1 Bass"), None);
    }

    #[test]
    fn a_coalesced_gesture_records_points_and_undoes_as_one_step() {
        let mut s = session();
        let depth = s.undo_depth();
        // One drag: three moves at three song positions, one gesture key.
        for (tick, volume) in [(384, 0.2), (480, 0.5), (576, 0.9)] {
            let set = Invocation::parse(&format!("channel.set_volume 1 {volume}")).unwrap();
            let batch = with_automation_record(&set, tick).unwrap();
            s.run_coalesced(&batch, 7).unwrap();
        }
        assert_eq!(s.undo_depth(), depth + 1, "one undo step");
        let project = s.project();
        assert_eq!(project.automations.len(), 1);
        let ticks: Vec<u32> = project.automations[0]
            .points
            .iter()
            .map(|p| p.tick)
            .collect();
        assert_eq!(ticks, [384, 480, 576]);
        let clip = project
            .playlist
            .clips
            .iter()
            .find(|c| c.automation().is_some())
            .unwrap();
        assert_eq!((clip.start, clip.length), (0, 4 * 384), "the whole song");
        assert_ne!(clip.track, 0, "the pattern clip's track is taken");
        s.run_line("edit.undo").unwrap();
        assert!(s.project().automations.is_empty());
        assert!(
            s.project()
                .playlist
                .clips
                .iter()
                .all(|c| c.automation().is_none())
        );
        assert!((s.project().channels[0].volume - 0.8).abs() < 1e-6);
    }

    #[test]
    fn a_second_pass_writes_into_the_existing_clip() {
        let mut s = session();
        let set = Invocation::parse("mixer.set_effect_param 1 1 mix 0.25").unwrap();
        s.run_coalesced(&with_automation_record(&set, 0).unwrap(), 1)
            .unwrap();
        let set = Invocation::parse("mixer.set_effect_param 1 1 mix 0.75").unwrap();
        s.run_coalesced(&with_automation_record(&set, 768).unwrap(), 2)
            .unwrap();
        let project = s.project();
        assert_eq!(project.automations.len(), 1);
        let values: Vec<(u32, f32)> = project.automations[0]
            .points
            .iter()
            .map(|p| (p.tick, p.value))
            .collect();
        assert_eq!(values.len(), 2);
        assert_eq!(values[1].0, 768);
        // Undoing the second gesture keeps the first point.
        s.run_line("edit.undo").unwrap();
        assert_eq!(s.project().automations[0].points.len(), 1);
    }
}
