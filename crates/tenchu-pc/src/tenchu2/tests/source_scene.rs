use super::source_control::{cases, template};
use serde::Deserialize;
use tenchu_core::{
    math::{Matrix, Position},
    scene::Scene,
    tenchu2::{actor::State, rig::Rig},
};

#[derive(Deserialize)]
struct Cache {
    flags: u32,
    matrix: Matrix,
}

#[derive(Deserialize)]
struct Recovery {
    state: i16,
    health: i16,
    flags: u32,
    input: u16,
    height: i16,
    terrain: i32,
    rotation: [i16; 3],
    mask: u32,
    expected: Recovered,
}

#[derive(Deserialize)]
struct Recovered {
    position: Position,
    rotation: [i16; 3],
    mask: u32,
    action: i16,
}

#[derive(Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
enum Operation {
    Tick,
    Query { id: usize, expected: Matrix },
    Update { id: usize, matrix: Matrix },
    Translate { id: usize, position: Position },
    Normal(Recovery),
    Root(Recovery),
}

#[derive(Deserialize)]
struct Event {
    #[serde(flatten)]
    operation: Operation,
    cache: Vec<Cache>,
}

#[derive(Deserialize)]
struct Case {
    parents: Vec<Option<usize>>,
    matrices: Vec<Matrix>,
    events: Vec<Event>,
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_SCENE_ORACLE from original SDK/root recovery"]
fn native_recovery_uses_published_scene_and_source_cache_order() {
    let resources = super::source_control::resources();
    let template = template(&resources);
    let mut checked = 0;
    for (case_id, case) in cases::<Case>("TENCHU2_SCENE_ORACLE").enumerate() {
        let mut scene = Scene::default();
        scene.models.clear();
        for (parent, matrix) in case.parents.into_iter().zip(case.matrices) {
            scene.add(parent, matrix);
        }
        let rig = Rig {
            root: 1,
            joints: vec![2],
            weapons: [None; 5],
        };
        for (step, event) in case.events.into_iter().enumerate() {
            match event.operation {
                Operation::Tick => scene.finish_tick(),
                Operation::Query { id, expected } => {
                    assert_eq!(scene.world(id).unwrap(), expected, "{case_id}/{step}");
                }
                Operation::Update { id, matrix } => scene.update(id, matrix),
                Operation::Translate { id, position } => scene.set_translation(id, position),
                Operation::Normal(ref recovery) | Operation::Root(ref recovery) => {
                    let mut actor = template.clone();
                    actor.position = scene.models[rig.root].local.translation;
                    actor.motion.pose.translation[0] =
                        scene.models[rig.joints[0]].local.translation;
                    actor.state = State::from_action(recovery.state << 8).unwrap();
                    actor.health = recovery.health;
                    actor.flags = recovery.flags;
                    actor.height = recovery.height;
                    actor.terrain.height = recovery.terrain;
                    actor.rotation = recovery.rotation.into();
                    actor.motion.mask = recovery.mask;
                    // Filtered control input can differ from the physical pad.
                    actor.pad.held = !recovery.input;
                    if matches!(event.operation, Operation::Normal(_)) {
                        let action = actor
                            .normal_motion(recovery.input, |actor| {
                                rig.root_position(actor, &mut scene)
                            })
                            .unwrap();
                        assert_eq!(action, recovery.expected.action, "{case_id}/{step}");
                    } else {
                        actor.commit_root(rig.root_position(&actor, &mut scene).unwrap());
                    }
                    assert_eq!(
                        actor.position, recovery.expected.position,
                        "{case_id}/{step}"
                    );
                    assert_eq!(
                        actor.rotation.to_array(),
                        recovery.expected.rotation,
                        "{case_id}/{step}"
                    );
                    assert_eq!(
                        actor.motion.mask, recovery.expected.mask,
                        "{case_id}/{step}"
                    );
                    scene.set_translation(rig.root, actor.position);
                }
            }
            for (id, (actual, expected)) in scene.models.iter().zip(event.cache).enumerate() {
                assert_eq!(actual.flags, expected.flags, "{case_id}/{step} joint {id}");
                assert_eq!(
                    actual.cached, expected.matrix,
                    "{case_id}/{step} joint {id}"
                );
            }
            checked += 1;
        }
    }
    assert!(checked >= 7000);
    eprintln!("{checked} native scene/cache/recovery operations match original instructions");
}
