use serde::Deserialize;
use std::collections::BTreeMap;
use tenchu_assets::tenchu2::{
    campaign::{Campaign, Kind},
    motions,
    story::Script,
};
use tenchu_core::tenchu2::story::Motion;

#[test]
#[ignore = "requires TENCHU2_DISC; loads every JP story cast, scenery model and texture group"]
fn native_story_models_load_from_their_event_bundles() {
    use tenchu_assets::tenchu2::story::Resources;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let mut pending: Vec<_> = campaign
        .starts
        .iter()
        .flatten()
        .flatten()
        .copied()
        .collect();
    let mut visited = std::collections::BTreeSet::new();
    let mut events = std::collections::BTreeSet::new();
    while let Some(index) = pending.pop() {
        if !visited.insert(index) {
            continue;
        }
        let sequence = campaign.sequences[index];
        if sequence.kind == Kind::Event {
            events.insert(sequence.resource);
        }
        pending.extend(sequence.next);
        pending.extend(sequence.failure);
    }
    // Event 88 has an unused sequence descriptor and no E88.ARC on the JP disc.
    assert_eq!(events.len(), 141);
    let mut models = 0;
    let mut weapons = 0;
    for event in events {
        let resources =
            Resources::load(&disc, event).unwrap_or_else(|e| panic!("event {event}: {e}"));
        for actor in &resources.script.actors {
            if let Some(model) = resources.models.get(&actor.model) {
                tenchu_core::motion::Pose::new(&model.parts).unwrap();
                assert!(!model.parts.is_empty(), "event {event}: {}", model.name);
                assert!(
                    model.vram.words.iter().any(|&v| v != 0),
                    "event {event}: {} has no texture data",
                    model.name
                );
                models += 1;
            } else {
                assert!(resources.effects.contains_key(&actor.model));
            }
        }
        for model in resources.weapons.values() {
            tenchu_core::motion::Pose::new(&model.parts).unwrap();
        }
        weapons += resources.attachments.len();
    }
    assert!(models > 500);
    assert!(weapons > 100);
    eprintln!("{models} native JP story model placements and {weapons} weapon attachments loaded");
}

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
