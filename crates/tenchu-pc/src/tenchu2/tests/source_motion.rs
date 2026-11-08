use std::collections::BTreeSet;
use tenchu_assets::tenchu2::{characters, motions, registry::Registry, weapons::Weapons};

#[test]
#[ignore = "requires TENCHU2_DISC; audits native skeletons and source weapon attachments"]
fn native_rigs_preserve_source_weapon_joints() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let registry = Registry::load(&disc).unwrap();
    let weapons = Weapons::load(&disc).unwrap();
    assert_eq!(weapons.parts.len(), 152);
    assert_eq!(weapons.attacks.len(), 231);
    assert_eq!(
        weapons
            .parts
            .iter()
            .map(|p| p.kind)
            .collect::<BTreeSet<_>>()
            .len(),
        58
    );
    let mut joints = std::collections::BTreeMap::new();
    for actor in &registry.actors {
        let count = *joints.entry(&actor.model).or_insert_with(|| {
            let rig = characters::rig(&disc, &actor.model).unwrap();
            tenchu_core::motion::Pose::new(&rig.parts).unwrap();
            for geometry in rig.parts.iter().flat_map(|p| &p.geometry) {
                assert_eq!(geometry.joints.len(), geometry.vertices.len());
                assert!(
                    geometry
                        .joints
                        .iter()
                        .all(|&j| usize::from(j) < rig.parts.len())
                );
            }
            rig.parts.len()
        });
        let parts: Vec<_> = weapons.parts(actor.weapon).collect();
        assert!(!parts.is_empty(), "{}: weapon {}", actor.kind, actor.weapon);
        for part in parts {
            assert!(
                part.bone < count,
                "{}: bone {} / {count}",
                actor.kind,
                part.bone
            );
        }
    }
    assert_eq!(joints.len(), 78);
    assert!(joints.values().any(|&count| count > 15));
    let clips = motions::collect(&disc, weapons.attacks.iter().map(|a| a.clip).collect()).unwrap();
    for attack in &weapons.attacks {
        if let Some(clip) = clips.get(&attack.clip) {
            assert_eq!(attack.bones >> clip.bones.len(), 0, "clip {}", attack.clip);
        }
    }
    eprintln!(
        "{:?}: {} native rigs, {} weapon parts, {} attacks",
        registry.edition,
        joints.len(),
        weapons.parts.len(),
        weapons.attacks.len()
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC and TENCHU2_POSE_ORACLE from the selected edition's MIPS routines"]
fn native_motion_matches_source_pose_transitions() {
    use serde::Deserialize;
    use std::io::BufRead;
    use tenchu_assets::{model::Part, tenchu2::registry::Registration};
    use tenchu_core::tenchu2::motion::Motion;

    #[derive(Deserialize)]
    struct State {
        result: i16,
        action: i16,
        frame: i16,
        loops: i16,
        mask: u32,
        root: [i32; 3],
        rotation: Vec<[i16; 3]>,
    }
    #[derive(Deserialize)]
    struct Event {
        op: String,
        #[serde(default)]
        value: i16,
        #[serde(default)]
        frame: i16,
        #[serde(default)]
        mask: u32,
        #[serde(default)]
        skip: bool,
        expected: State,
    }
    #[derive(Deserialize)]
    struct Case {
        clip: i16,
        events: Vec<Event>,
    }
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut registry = Registry::load(&disc).unwrap();
    let basis = motions::spline_basis(&disc).unwrap();
    let input = std::io::BufReader::new(
        std::fs::File::open(std::env::var_os("TENCHU2_POSE_ORACLE").unwrap()).unwrap(),
    );
    let cases: Vec<Case> = input
        .lines()
        .map(|line| serde_json::from_str(&line.unwrap()).unwrap())
        .collect();
    let clips = motions::collect(&disc, cases.iter().map(|c| c.clip).collect()).unwrap();
    assert!(cases.len() > 1100);
    let parts: Vec<_> = (0..20)
        .map(|i| Part {
            parent: if i == 0 { -1 } else { 0 },
            child: i,
            position: [0, if i == 0 { -640 } else { 0 }, 0],
            geometry: vec![],
        })
        .collect();
    let mut checked = 0;
    for case in cases {
        registry.actors[0].motions = vec![
            Registration {
                action: 0x900,
                clip: case.clip,
            },
            Registration {
                action: 0x901,
                clip: 70,
            },
        ];
        let mut motion = Motion::new(&parts).unwrap();
        motion.action = -2;
        motion.pose.translation[0] = [379, -777, 93].into();
        for (bone, angles) in motion.pose.rotation.iter_mut().enumerate() {
            *angles =
                std::array::from_fn(|axis| ((bone * 617 + axis * 2111) % 12000) as i16 - 6000);
        }
        for event in case.events {
            let result = match event.op.as_str() {
                "select" => motion
                    .select(event.value, 0, &registry, &clips)
                    .unwrap()
                    .unwrap_or(-1),
                "overlay" => motion
                    .overlay(event.value, event.mask, 0, &registry, &clips)
                    .unwrap()
                    .unwrap_or(-1),
                "step" => motion.advance(&clips, &basis, !event.skip).unwrap(),
                "seek" => {
                    motion.frame = event.frame;
                    motion.loops = 0;
                    motion.mask = event.mask;
                    motion.advance(&clips, &basis, true).unwrap()
                }
                _ => panic!("Unknown oracle command"),
            };
            let expected = event.expected;
            assert_eq!(
                (
                    result,
                    motion.action,
                    motion.frame,
                    motion.loops,
                    motion.mask
                ),
                (
                    expected.result,
                    expected.action,
                    expected.frame,
                    expected.loops,
                    expected.mask
                ),
                "clip {}: {} state",
                case.clip,
                event.op
            );
            assert_eq!(
                motion.pose.translation[0], expected.root,
                "clip {}: {} root",
                case.clip, event.op
            );
            assert_eq!(
                motion.pose.rotation, expected.rotation,
                "clip {}: {} rotation",
                case.clip, event.op
            );
            checked += 1;
        }
    }
    assert!(checked > 10000);
    eprintln!("{:?}: {checked} source pose states match", registry.edition);
}
