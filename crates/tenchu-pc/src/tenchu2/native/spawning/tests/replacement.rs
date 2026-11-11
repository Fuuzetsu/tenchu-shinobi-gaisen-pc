use super::*;
use tenchu_assets::tenchu2::campaign::{Campaign, Kind, Objective};
use tenchu_core::tenchu2::{mission::Phase, world::Options};

fn bytes(value: &impl serde::Serialize) -> Vec<u8> {
    postcard::to_allocvec(value).unwrap()
}

#[test]
#[ignore = "requires both discs; starting NPC replacements retain routes, collision ownership, source RNG and saved continuation across the gallery"]
fn native_replacements_preserve_slots_and_saved_continuation() {
    let game = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&game).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut session = Session::load(&disc, 9, 1).unwrap();
    session.world.keep_all_active = true;
    session.world.actors[session.world.player]
        .actor
        .invulnerable = true;
    let original = session.world.clone();
    let id = original
        .actors
        .iter()
        .position(|agent| {
            (0x20..0x60).contains(&agent.actor.kind)
                && agent.actor.health > 0
                && agent.behavior.patrol.waypoints().next().is_some()
        })
        .expect("Mission 1 patrol");
    let previous = &original.actors[id];
    let old_joints = &original.instances[id].rig.joints;
    let mut replaced = 0;
    let mut unavailable = Vec::new();
    for body in &library.bodies {
        session.world = original.clone();
        session.actor_voices.clear();
        if let Err(error) = session.replace_actor(&disc, id, &body.id, Some(&library)) {
            assert_eq!(bytes(&session.world), bytes(&original), "{}", body.id);
            unavailable.push((body.id.clone(), error));
            continue;
        }
        replaced += 1;
        let next = &session.world.actors[id];
        assert_eq!(next.slot, previous.slot);
        assert_eq!(next.actor.mission_kind(), previous.actor.kind);
        assert_eq!(next.actor.position, previous.actor.position);
        assert_eq!(next.actor.previous, previous.actor.previous);
        assert_eq!(next.actor.rotation.y, previous.actor.rotation.y);
        assert_eq!(next.behavior.home, previous.behavior.home);
        assert_eq!(
            bytes(&next.behavior.patrol),
            bytes(&previous.behavior.patrol)
        );
        assert_eq!(session.world.random, original.random);
        assert_eq!(session.world.actors.len(), original.actors.len());
        assert_eq!(bytes(&session.world.mission), bytes(&original.mission));
        for (index, actor) in session.world.actors.iter().enumerate() {
            if index != id {
                assert_eq!(bytes(actor), bytes(&original.actors[index]));
            }
        }
        assert!(
            old_joints
                .iter()
                .all(|&joint| session.world.collision.index(joint).is_none())
        );
        let rig = &session.world.instances[id].rig;
        assert!(
            session
                .world
                .collision
                .slots()
                .iter()
                .filter(|slot| slot.owner == Some(id))
                .all(|slot| rig.joints.contains(&slot.model))
        );
        assert!(session.actor_voices.contains_key(&id));
        for tick in 0..120 {
            session
                .step(0)
                .unwrap_or_else(|e| panic!("{} tick {tick}: {e}", body.id));
        }
        assert_eq!(
            session.world.actors[id].actor.mission_kind(),
            previous.actor.kind
        );
        if matches!(body.id.as_str(), "disc:16" | "disc:135") || replaced == 1 {
            let checkpoint = session.capture_state().unwrap();
            for _ in 0..60 {
                session.step(0).unwrap();
            }
            let expected = bytes(&session.world);
            let voices = bytes(&session.actor_voices);
            session.restore_state(&checkpoint).unwrap();
            for _ in 0..60 {
                session.step(0).unwrap();
            }
            assert_eq!(bytes(&session.world), expected);
            assert_eq!(bytes(&session.actor_voices), voices);
        }
        let before = bytes(&session.world);
        assert!(session.replace_actor(&disc, id, "native:32", None).is_err());
        assert_eq!(bytes(&session.world), before);
    }
    eprintln!("Replaced {replaced} gallery bodies; unavailable: {unavailable:?}");
    assert!(replaced >= 100);
    assert!(unavailable.iter().all(|(id, reason)| {
        reason.contains("disc omits animation clips")
            || id == "disc:168" && reason.contains("projectile model")
            || matches!(id.as_str(), "disc:166" | "disc:167")
                && reason.contains("scripted scenery model")
            || reason.contains("no living actor controller")
                && body::kind(&library, library.get(id).unwrap()).is_ok_and(|kind| {
                    session
                        .assets
                        .resources
                        .registry
                        .actor(kind)
                        .unwrap()
                        .health
                        <= 0
                })
    }));
    session.world = original;
    let before = bytes(&session.world);
    assert!(
        session
            .replace_actor(&disc, session.world.player, "native:32", None)
            .is_err()
    );
    assert!(
        session
            .replace_actor(&disc, usize::MAX, "native:32", None)
            .is_err()
    );
    assert_eq!(bytes(&session.world), before);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; civilian bodies keep authored kill-quota and boss-objective roles"]
fn native_replacements_preserve_mission_objectives() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    for boss in [false, true] {
        let sequence = campaign
            .sequences
            .iter()
            .enumerate()
            .find_map(|(index, row)| {
                (if boss {
                    matches!(row.kind, Kind::Gameplay(Objective::Defeat { .. }))
                } else {
                    matches!(row.kind, Kind::Gameplay(Objective::EliminateAll))
                })
                .then_some(index)
            })
            .unwrap();
        let mut session = Session::load(&disc, sequence, 1).unwrap();
        let mission = bytes(&session.world.mission);
        let victims: Vec<_> = session
            .world
            .actors
            .iter()
            .enumerate()
            .filter_map(|(index, agent)| {
                (index != session.world.player
                    && agent.actor.health > 0
                    && (if boss {
                        session.world.mission.targets.contains(&Some(index))
                    } else {
                        !matches!(agent.actor.kind & 0xf0, 0x10 | 0x60 | 0x70 | 0x80 | 0x90)
                    }))
                .then_some(index)
            })
            .collect();
        assert!(!victims.is_empty());
        for &index in &victims {
            session
                .replace_actor(&disc, index, "native:96", None)
                .unwrap();
        }
        assert_eq!(bytes(&session.world.mission), mission);
        let before = session.capture_state().unwrap();
        for &index in &victims {
            session.world.kill_actor(index, &session.assets).unwrap();
        }
        assert_eq!(usize::from(session.world.statistics.killed), victims.len());
        assert_eq!(session.world.statistics.innocents, 0);
        for _ in 0..200 {
            session.world.advance_mission(&session.assets).unwrap();
            session.world.tick += 1;
        }
        assert!(
            matches!(session.world.mission.phase, Phase::Finished { outcome, .. } if outcome.success()),
            "sequence {sequence}: {:?}",
            session.world.mission
        );
        session.restore_state(&before).unwrap();
        assert!(
            victims
                .iter()
                .all(|&i| session.world.actors[i].actor.health > 0)
        );
    }
}

#[test]
#[ignore = "requires both discs; imported story bindings, hidden actors and health triggers survive replacements and checkpoints"]
fn native_replacements_preserve_imported_story_roles() {
    use super::super::super::gaisen;
    use tenchu_assets::script::Language;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let path = std::env::var_os("TENCHU_DISC").unwrap();
    let mut hidden = 0;
    let mut thresholds = 0;
    for stage in [0, 4] {
        for hero in 0..3 {
            let source = gaisen::Source::open(
                path.as_ref(),
                stage,
                i16::from(hero == 1),
                1,
                Language::English,
            )
            .unwrap();
            let mut session = source
                .session(
                    &disc,
                    Options {
                        hero,
                        difficulty: 2,
                        seed: 1,
                        inventory: Default::default(),
                    },
                    false,
                    tenchu_assets::tenchu2::collision::Gaisen::DEFAULT_WATER_DEPTH,
                )
                .unwrap();
            // A controlled 75-HP roll exercises scaling even when the two
            // source character definitions happen to have identical health.
            session
                .assets
                .resources
                .registry
                .actors
                .iter_mut()
                .find(|actor| actor.kind == 0x20)
                .unwrap()
                .health = 75;
            let director = &session.gaisen.as_ref().unwrap().director;
            let bindings: Vec<_> = director
                .source
                .roster
                .order
                .iter()
                .copied()
                .filter_map(|source| {
                    let index = director.native_actor(source)?;
                    (index != session.world.player
                        && session.world.actors[index].actor.max_health > 0
                        && session.world.actors[index].actor.health != 0)
                        .then_some((source, index))
                })
                .collect();
            let original_hidden = session.world.script_hidden.clone();
            let random = session.world.random;
            for &(source, index) in &bindings {
                let old = session.world.actors[index].actor.clone();
                let role = session.world.actors[index].behavior.profile.protected;
                // Exercise a nonzero threshold on a real story binding, then
                // remove the synthetic condition before advancing the script.
                if role {
                    let director = &mut session.gaisen.as_mut().unwrap().director;
                    let kind = director.source.roster.get(source).kind;
                    let mut event = director.source.mission.scripts.events[0].clone();
                    event.target = u8::try_from(kind).unwrap();
                    event.mode = 5;
                    event.value = (old.max_health / 2).max(1);
                    director.source.mission.scripts.events.push(event);
                }
                let old_events = session
                    .gaisen
                    .as_ref()
                    .unwrap()
                    .director
                    .source
                    .mission
                    .scripts
                    .events
                    .clone();
                session
                    .replace_actor(&disc, index, "native:32", None)
                    .unwrap();
                let director = &session.gaisen.as_ref().unwrap().director;
                let actor = &session.world.actors[index].actor;
                assert_eq!(director.native_actor(source), Some(index));
                assert_eq!(
                    director.source.roster.get(source).max_health,
                    actor.max_health
                );
                assert_eq!(actor.mission_kind(), old.kind);
                assert_eq!(session.world.actors[index].behavior.profile.protected, role);
                if old.health < 0 {
                    hidden += 1;
                    assert_eq!(actor.health, old.health);
                    assert!(
                        session
                            .world
                            .collision
                            .slots()
                            .iter()
                            .filter(|slot| slot.owner == Some(index))
                            .all(
                                |slot| slot.flags & tenchu_core::tenchu2::collision::DISABLED != 0
                            )
                    );
                }
                for (before, after) in old_events
                    .iter()
                    .zip(&director.source.mission.scripts.events)
                {
                    if before.value != after.value {
                        thresholds += 1;
                        assert!(after.value > 0);
                        assert!(
                            (i32::from(after.value) * i32::from(old.max_health)
                                - i32::from(before.value) * i32::from(actor.max_health))
                            .abs()
                                < i32::from(old.max_health)
                        );
                    }
                }
                if role {
                    session
                        .gaisen
                        .as_mut()
                        .unwrap()
                        .director
                        .source
                        .mission
                        .scripts
                        .events
                        .pop();
                }
            }
            assert_eq!(session.world.random, random);
            assert_eq!(session.world.script_hidden, original_hidden);
            let checkpoint = session.capture_state().unwrap();
            for _ in 0..180 {
                session.step(0).unwrap();
            }
            let expected = bytes(&session.world);
            session.restore_state(&checkpoint).unwrap();
            for _ in 0..180 {
                session.step(0).unwrap();
            }
            assert_eq!(bytes(&session.world), expected);
        }
    }
    eprintln!("Preserved {hidden} hidden actor bindings and rescaled {thresholds} health triggers");
    assert!(hidden > 0 && thresholds > 0);
}
