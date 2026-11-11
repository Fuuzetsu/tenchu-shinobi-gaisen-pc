use super::*;
use tenchu_core::{
    game::dismemberment::{HeadRest, Settings},
    tenchu2::{
        actor::Velocity,
        dismemberment::{Cause, Death},
        rig::WeaponMode,
    },
};

pub(super) fn fixture(hero: usize) -> (Session, usize) {
    let mut session = Session::with_options(
        &sheathing_tests::disc(),
        9,
        Options {
            hero,
            difficulty: 2,
            seed: 17,
            inventory: Default::default(),
        },
        false,
    )
    .unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let (world, assets, _) = crate::tenchu2::tests::native_world::fixture();
    session.world.terrain = world.terrain;
    session.assets.level.collision = assets.level.collision;
    let guard = session
        .spawn_actor(
            &sheathing_tests::disc(),
            spawning::Request {
                source: spawning::Source::Native(0x34),
                position: Position::new(0, 0, -1000),
                yaw: 0,
                snap: true,
            },
            None,
        )
        .unwrap();
    let player = session.world.player;
    for (id, agent) in session.world.actors.iter_mut().enumerate() {
        if id != player && id != guard {
            agent.actor.flags |= 8 | 0x10;
            session.world.script_hidden.insert(id);
            for &joint in &session.world.instances[id].rig.joints {
                session.world.collision.remove(joint);
            }
        }
        if id != player {
            agent.behavior.programs.idle = tenchu_core::tenchu2::ai::Program::Disabled;
            agent.behavior.programs.combat = tenchu_core::tenchu2::ai::Program::Disabled;
        }
    }
    for id in [player, guard] {
        let actor = &mut session.world.actors[id].actor;
        actor.flags &= !(3 | 8 | 0x10 | 0x2f000);
        actor.position = Position::new(0, 0, if id == player { 0 } else { -1000 });
        actor.previous = actor.position;
        actor.rotation = Default::default();
        actor.velocity = Default::default();
        actor.terrain = Default::default();
        actor.terrain.floor = Some(0);
        actor
            .change_motion(0, Velocity::Stop, &session.assets.resources)
            .unwrap();
        actor
            .set_weapon(
                &mut session.world.instances[id].rig,
                id,
                &mut session.world.collision,
                &session.assets.resources,
                WeaponMode::Sheathe,
            )
            .unwrap();
        session.world.publish(id, &session.assets);
    }
    session.world.keep_all_active = true;
    (session, guard)
}

pub(super) fn death(session: &mut Session, victim: usize, head: u16, side: u16) {
    session.world.controls.dismemberment = Settings {
        enabled: true,
        player: true,
        dive_head: head,
        arms: 100,
        right_arm: side,
        player_head: head,
        player_arms: 100,
        player_right_arm: side,
        head_rest: HeadRest::FaceUp,
        ..Default::default()
    };
    let actor = &mut session.world.actors[victim].actor;
    actor.health = 0;
    actor
        .change_motion(0xb01, Velocity::Motion, &session.assets.resources)
        .unwrap();
    actor.motion.frame = 5;
    actor.motion.loops = 0;
    actor.dismemberment = Some(Death {
        pending: Some(Cause::Combat { dive: true }),
        part: None,
    });
    session.world.publish(victim, &session.assets);
}

fn lowest(session: &Session, victim: usize) -> i32 {
    let actor = &session.world.actors[victim].actor;
    let part = actor.severed().unwrap();
    let model = &session
        .assets
        .resources
        .registry
        .actor(actor.definition_kind())
        .unwrap()
        .model;
    let matrix =
        session.world.scene.models[session.world.instances[victim].rig.joints[part.index]].local;
    session.assets.models[model].parts[part.index]
        .geometry
        .iter()
        .flat_map(|m| &m.vertices)
        .map(|&v| matrix.transform(v).y)
        .max()
        .unwrap()
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; head and both forearms on all heroes and a native guard, mesh-aware landing and detached checkpoint continuation"]
fn native_dismemberment_parts_settle_and_checkpoint() {
    for hero in 0..3 {
        for player in [false, true] {
            for (head, side, expected) in [(100, 100, 2), (0, 100, 4), (0, 0, 10)] {
                let (mut s, guard) = fixture(hero);
                let victim = if player { s.world.player } else { guard };
                death(&mut s, victim, head, side);
                s.world.controls.dismemberment.head_rest =
                    [HeadRest::FaceUp, HeadRest::Left, HeadRest::Right][hero];
                s.step_world(0, true).unwrap();
                let part = s.world.actors[victim].actor.severed().unwrap();
                assert_eq!(part.index, expected);
                let joint = s.world.instances[victim].rig.joints[expected];
                assert_eq!(s.world.scene.models[joint].parent, Some(0));
                let saved = s.capture_state().unwrap();
                let (mut restored, _) = fixture(hero);
                restored.restore_state(&saved).unwrap();
                for _ in 0..130 {
                    s.step_world(0, true).unwrap();
                    restored.step_world(0, true).unwrap();
                    assert!(
                        postcard::to_allocvec(&s.world).unwrap()
                            == postcard::to_allocvec(&restored.world).unwrap()
                    );
                }
                assert!(s.capture_state().unwrap() == restored.capture_state().unwrap());
                assert!(
                    !s.world.actors[victim].actor.severed().unwrap().moving,
                    "{hero} {player} {expected}"
                );
                assert_eq!(lowest(&s, victim), -2, "{hero} {player} {expected}");
                if expected == 2 {
                    let rotation = s.world.actors[victim].actor.severed().unwrap().rotation;
                    assert_eq!(
                        [rotation[0], rotation[2]],
                        [[-1024, 0], [0, -1024], [0, 1024]][hero]
                    );
                }
                let settled = s.world.scene.models[joint].local;
                s.world.actors[victim].actor.position.x += 2000;
                s.world.publish(victim, &s.assets);
                assert_eq!(
                    s.world.scene.models[joint].local, settled,
                    "publishing a corpse reattached a part"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; rear stealth severing uses the native authored blood frame and front kills remain intact"]
fn native_dismemberment_rear_kills_follow_source_events() {
    for hero in 0..3 {
        for enabled in [false, true] {
            for yaw in [0, 2048] {
                let (mut s, guard) = fixture(hero);
                s.world.actors[guard].actor.rotation.y = yaw;
                s.world.publish(guard, &s.assets);
                s.world.controls.dismemberment.enabled = enabled;
                s.world.controls.dismemberment.stealth_head = 100;
                let player = s.world.player;
                let expected = s
                    .assets
                    .resources
                    .registry
                    .stealth_events(hero, 4)
                    .unwrap()
                    .iter()
                    .find(|e| {
                        matches!(
                            e.effect,
                            tenchu_assets::tenchu2::registry::StealthEffect::Blood { .. }
                        )
                    })
                    .unwrap()
                    .frame;
                s.world.actors[player]
                    .actor
                    .pad
                    .update(tenchu_core::tenchu2::input::ATTACK);
                #[derive(Default)]
                struct Sink;
                impl tenchu_core::tenchu2::environment::Events for Sink {
                    fn emit(&mut self, _: environment::Event, _: &mut Random) -> Result<()> {
                        Ok(())
                    }
                }
                impl Events for Sink {
                    fn emit(&mut self, _: EventSource<'_>, _: Event, _: &mut Random) -> Result<()> {
                        Ok(())
                    }
                }
                s.world
                    .control_actor(player, &mut s.assets, &mut Sink)
                    .unwrap();
                assert_eq!(
                    s.world.actors[guard].actor.motion.action,
                    if yaw == 0 { 0xb11 } else { 0xb10 }
                );
                let mut at = None;
                for _ in 0..180 {
                    s.step_world(0, true).unwrap();
                    let actor = &s.world.actors[guard].actor;
                    if actor.severed().is_some() {
                        at.get_or_insert(actor.motion.frame);
                    }
                }
                assert_eq!(
                    at,
                    (enabled && yaw == 0).then_some(expected + 1),
                    "hero {hero}"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; disabled decisions preserve the native world and removed/revived corpses restore their original hierarchy"]
fn native_dismemberment_disabled_and_cleanup() {
    for player in [false, true] {
        let (mut s, guard) = fixture(0);
        let victim = if player { s.world.player } else { guard };
        death(&mut s, victim, 100, 100);
        s.world.controls.dismemberment = Settings::default();
        let (mut baseline, _) = fixture(0);
        baseline.restore_state(&s.capture_state().unwrap()).unwrap();
        baseline.world.actors[victim].actor.dismemberment = None;
        for _ in 0..20 {
            s.step_world(0, true).unwrap();
            baseline.step_world(0, true).unwrap();
            let decision = s.world.actors[victim].actor.dismemberment.take().unwrap();
            assert!(decision.pending.is_none() && decision.part.is_none());
            assert!(
                postcard::to_allocvec(&s.world).unwrap()
                    == postcard::to_allocvec(&baseline.world).unwrap()
            );
            s.world.actors[victim].actor.dismemberment = Some(decision);
        }
        death(&mut s, victim, 100, 100);
        s.step_world(0, true).unwrap();
        let part = s.world.actors[victim].actor.severed().unwrap().clone();
        let joint = s.world.instances[victim].rig.joints[part.index];
        if player {
            s.world.actors[victim].actor.health = 100;
            s.step_world(0, true).unwrap();
        } else {
            s.world.remove_actor(victim, &s.assets).unwrap();
        }
        assert!(s.world.actors[victim].actor.dismemberment.is_none());
        assert_eq!(s.world.scene.models[joint].parent, part.parent);
        assert_eq!(s.world.scene.models[joint].attribute & 2, 0);
        baseline.restore_state(&s.capture_state().unwrap()).unwrap();
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; detached checkpoints reject invalid part indices, parents, health and repeated decisions atomically"]
fn native_dismemberment_rejects_corrupt_parts() {
    let (mut s, guard) = fixture(0);
    death(&mut s, guard, 100, 100);
    s.step_world(0, true).unwrap();
    let original = s.capture_state().unwrap();
    for change in [
        (|world: &mut World, id: usize| {
            world.actors[id]
                .actor
                .dismemberment
                .as_mut()
                .unwrap()
                .part
                .as_mut()
                .unwrap()
                .index = usize::MAX
        }) as fn(&mut World, usize),
        |world, id| {
            world.actors[id]
                .actor
                .dismemberment
                .as_mut()
                .unwrap()
                .part
                .as_mut()
                .unwrap()
                .parent = Some(0)
        },
        |world, id| world.actors[id].actor.health = 100,
        |world, id| {
            world.actors[id]
                .actor
                .dismemberment
                .as_mut()
                .unwrap()
                .pending = Some(Cause::RearStealth)
        },
        |world, _| world.controls.dismemberment.arms = 101,
    ] {
        let mut saved: state::Owned = crate::savestate::decode(&original).unwrap();
        change(&mut saved.core.0, guard);
        assert!(
            s.restore_state(&crate::savestate::encode(&saved).unwrap())
                .is_err()
        );
        assert!(s.capture_state().unwrap() == original);
    }
}
