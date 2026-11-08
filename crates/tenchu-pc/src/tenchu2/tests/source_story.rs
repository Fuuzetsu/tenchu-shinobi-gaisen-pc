use serde::Deserialize;
use std::collections::BTreeMap;
use tenchu_assets::tenchu2::{
    campaign::{Campaign, Kind},
    motions,
    story::Script,
};
use tenchu_core::tenchu2::story::Motion;

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_STORY_MOTION_ORACLE; source event root/joint tracks"]
fn native_story_tracks_match_original_event_animations() {
    #[derive(Deserialize)]
    struct State {
        frame: i16,
        position: [i32; 3],
        rotation: Vec<[i16; 3]>,
    }
    #[derive(Deserialize)]
    struct Case {
        event: u16,
        slot: usize,
        states: Vec<State>,
    }
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let basis = motions::spline_basis(&disc).unwrap();
    let mut scripts = BTreeMap::new();
    for sequence in campaign.sequences {
        if sequence.kind == Kind::Event {
            scripts.entry(sequence.resource).or_insert_with(|| {
                Script::load(&disc, sequence.resource)
                    .unwrap_or_else(|e| panic!("event {}: {e}", sequence.resource))
            });
        }
    }
    assert_eq!(scripts.len(), 142);
    let mut checked = 0;
    for case in super::source_control::cases::<Case>("TENCHU2_STORY_MOTION_ORACLE") {
        let script = scripts
            .entry(case.event)
            .or_insert_with(|| Script::load(&disc, case.event).unwrap());
        let animation = if case.slot == 0 {
            &script.camera
        } else {
            &script.actors[case.slot - 1]
        };
        let mut motion = Motion::new(animation);
        for expected in case.states {
            motion.update(animation, expected.frame, &basis).unwrap();
            assert_eq!(
                motion.position, expected.position,
                "event {} slot {} frame {} position",
                case.event, case.slot, expected.frame
            );
            assert_eq!(
                motion.rotation, expected.rotation,
                "event {} slot {} frame {} rotation",
                case.event, case.slot, expected.frame
            );
            motion = serde_json::from_value(serde_json::to_value(&motion).unwrap()).unwrap();
            checked += 1;
        }
    }
    assert!(checked > 3000);
    eprintln!("{checked} original JP event animation states; all 142 campaign events parsed");
}
