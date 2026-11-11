use super::*;
use tenchu_core::tenchu2::{
    actor::Velocity,
    ai::Program,
    collision::{BODY, DISABLED, WEAPON},
    physics::{ContactActor, HIT},
    rig::WeaponMode,
};

pub(super) fn fixture(hero: usize) -> (Session, [usize; 2]) {
    let (mut s, first) = dismemberment_tests::fixture(hero);
    compute(&mut s);
    let second = s
        .spawn_actor(
            &sheathing_tests::disc(),
            spawning::Request {
                source: spawning::Source::Native(0x34),
                position: Position::new(5000, 0, 0),
                yaw: 0,
                snap: true,
            },
            None,
        )
        .unwrap();
    for id in [first, second] {
        let agent = &mut s.world.actors[id];
        agent.behavior.programs.idle = Program::Stationary;
        agent.behavior.programs.combat = Program::Disabled;
        agent.actor.skill = 0;
        agent.actor.max_health = 30000;
    }
    (s, [first, second])
}

fn compute(s: &mut Session) {
    s.world.scene.finish_tick();
    s.world
        .collision
        .compute(|m| s.world.scene.world(m))
        .unwrap();
}

fn swing_contacts(s: &mut Session, id: usize) -> Vec<usize> {
    let actor = &mut s.world.actors[id].actor;
    actor
        .set_weapon(
            &mut s.world.instances[id].rig,
            id,
            &mut s.world.collision,
            &s.assets.resources,
            WeaponMode::Draw,
        )
        .unwrap();
    actor
        .change_motion(0x900, Velocity::Stop, &s.assets.resources)
        .unwrap();
    activate(s, id)
}

fn activate(s: &mut Session, id: usize) -> Vec<usize> {
    let actor = &mut s.world.actors[id].actor;
    actor.attack.index = s
        .assets
        .resources
        .weapons
        .attacks
        .iter()
        .position(|a| Some(a.clip) == actor.motion.clip);
    actor.motion.frame =
        i16::from(s.assets.resources.weapons.attacks[actor.attack.index.unwrap()].start);
    actor.motion.loops = 0;
    actor.target = None;
    actor.pad = Default::default();
    s.world.publish(id, &s.assets);
    s.world
        .control_actor(id, &mut s.assets, &mut bleeding_tests::Recorded::default())
        .unwrap();
    let weapons: Vec<_> = s
        .world
        .collision
        .slots()
        .iter()
        .filter(|slot| slot.owner == Some(id) && slot.flags == WEAPON)
        .map(|slot| slot.model)
        .collect();
    assert!(
        !weapons.is_empty(),
        "actor {} has no active contacts",
        s.world.actors[id].actor.kind
    );
    weapons
}

fn isolate(s: &mut Session, weapons: Vec<usize>, hand: usize) -> usize {
    let chosen = weapons[hand];
    for model in weapons {
        if model != chosen {
            s.world.collision.get_mut(model).unwrap().flags |= DISABLED;
        }
    }
    compute(s);
    chosen
}

fn swing(s: &mut Session, id: usize, hand: usize) -> usize {
    let weapons = swing_contacts(s, id);
    isolate(s, weapons, hand)
}

fn place(s: &mut Session, victim: usize, weapon: usize, blocked: bool) {
    let actor = &mut s.world.actors[victim].actor;
    actor.health = 30000;
    actor.flags &= !(HIT | 8 | 0x10);
    actor.flags |= 2;
    actor.position = Position::ZERO;
    actor.previous = actor.position;
    actor.rotation = Default::default();
    actor.velocity = Default::default();
    actor.pad = Default::default();
    actor
        .set_weapon(
            &mut s.world.instances[victim].rig,
            victim,
            &mut s.world.collision,
            &s.assets.resources,
            WeaponMode::Draw,
        )
        .unwrap();
    actor
        .change_motion(
            if blocked { 0x104 } else { 0x100 },
            Velocity::Stop,
            &s.assets.resources,
        )
        .unwrap();
    actor.motion.frame = 0;
    actor.motion.loops = 0;
    let body = s.world.instances[victim].rig.joints[0];
    s.world.collision.get_mut(body).unwrap().flags = BODY;
    for adjustment in 0..2 {
        s.world.publish(victim, &s.assets);
        compute(s);
        let target = s.world.collision.get_mut(weapon).unwrap().position;
        let center = s.world.collision.get_mut(body).unwrap().position;
        let actor = &mut s.world.actors[victim].actor;
        actor.position = actor.position.wrapping_add(target.wrapping_sub(center));
        actor.previous = actor.position;
        if adjustment == 0 {
            actor.rotation.y = s
                .assets
                .resources
                .math
                .direction(-actor.position.x, -actor.position.z, 0)
                .unwrap();
        }
    }
    // Low kicks should meet the body's lower edge, without sinking it into the floor.
    let actor = &mut s.world.actors[victim].actor;
    actor.position.y = actor.position.y.min(0);
    actor.previous = actor.position;
    s.world.publish(victim, &s.assets);
}

fn contact(s: &mut Session, victim: usize) -> bool {
    for id in 0..s.world.actors.len() {
        let body = s.world.instances[id].rig.joints[0];
        if let Some(slot) = s.world.collision.get_mut(body) {
            if id == victim {
                slot.flags &= !DISABLED;
            } else {
                slot.flags |= DISABLED;
            }
        }
    }
    compute(s);
    resolve(s, victim)
}

fn resolve(s: &mut Session, victim: usize) -> bool {
    let others: Vec<_> = s
        .world
        .actors
        .iter()
        .enumerate()
        .map(|(id, a)| {
            (
                a.actor.position,
                a.actor.motion.action,
                id == s.world.player,
            )
        })
        .collect();
    let actor = &mut s.world.actors[victim].actor;
    actor.flags &= !HIT;
    actor.hit = None;
    actor
        .resolve_contacts(
            &s.world.collision,
            s.world.instances[victim].rig.joints[0],
            &s.assets.resources,
            &mut s.world.random,
            |id| {
                Ok(ContactActor {
                    position: others[id].0,
                    action: others[id].1,
                    is_player: others[id].2,
                })
            },
        )
        .unwrap();
    let hit = actor.flags & HIT != 0;
    if hit {
        s.world
            .control_actor(
                victim,
                &mut s.assets,
                &mut bleeding_tests::Recorded::default(),
            )
            .unwrap();
    }
    hit
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; real melee contacts hit new guards once, with native damage and blocks"]
fn native_multi_target_melee_consumes_each_target_once() {
    for hero in 0..3 {
        for enabled in [false, true] {
            let (mut s, victims) = fixture(hero);
            s.world.controls.multi_target_attacks = enabled;
            let player = s.world.player;
            for hand in 0..swing_contacts(&mut s, player).len() {
                for blocked in [false, true] {
                    let weapon = swing(&mut s, player, hand);
                    for (i, &victim) in victims.iter().enumerate() {
                        place(&mut s, victim, weapon, blocked && i == 0);
                    }
                    s.world.random = Random::new(1);
                    assert!(contact(&mut s, victims[0]));
                    assert_eq!(
                        s.world.actors[victims[0]].actor.health == 30000,
                        blocked,
                        "hero {hero}, enabled {enabled}, blocked {blocked}"
                    );
                    assert_eq!(contact(&mut s, victims[1]), enabled);
                    assert_eq!(s.world.actors[victims[1]].actor.health < 30000, enabled);
                    let health = victims.map(|id| s.world.actors[id].actor.health);
                    let calls = s.world.random.calls;
                    for _ in 0..5 {
                        for victim in victims {
                            assert!(!contact(&mut s, victim));
                        }
                    }
                    assert_eq!(victims.map(|id| s.world.actors[id].actor.health), health);
                    assert_eq!(s.world.random.calls, calls);
                    let slot = s.world.collision.get_mut(weapon).unwrap();
                    assert_eq!(slot.multi_target, enabled);
                    assert_eq!(slot.hit_targets.len(), if enabled { 2 } else { 0 });
                }
            }
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; mid-swing saves retain hit history and reject corrupt references atomically"]
fn native_multi_target_checkpoint_keeps_mid_swing_targets() {
    let (mut original, victims) = fixture(1);
    original.world.controls.multi_target_attacks = true;
    let player = original.world.player;
    let weapon = swing(&mut original, player, 0);
    for victim in victims {
        place(&mut original, victim, weapon, false);
    }
    assert!(contact(&mut original, victims[0]));
    let saved = original.capture_state().unwrap();
    let mut recorder = crate::rewind::Recorder::default();
    recorder.begin();
    original.snapshot().capture(&mut recorder).unwrap();
    assert!(
        recorder.finish().unwrap().bytes().unwrap()
            == postcard::to_allocvec(&original.snapshot()).unwrap()
    );
    for corruption in 0..3 {
        let mut state: state::Owned = crate::savestate::decode(&saved).unwrap();
        let slot = state.core.0.collision.get_mut(weapon).unwrap();
        match corruption {
            0 => slot.hit_targets.push(usize::MAX),
            1 => slot.hit_targets.push(slot.hit_targets[0]),
            _ => slot.multi_target = false,
        }
        assert!(
            original
                .restore_state(&crate::savestate::encode(&state).unwrap())
                .is_err()
        );
        assert!(original.capture_state().unwrap() == saved);
    }
    let (mut restored, _) = fixture(0);
    restored.restore_state(&saved).unwrap();
    for (victim, hit) in [
        (victims[0], false),
        (victims[1], true),
        (victims[1], false),
        (victims[0], false),
    ] {
        assert_eq!(contact(&mut original, victim), hit);
        assert_eq!(contact(&mut restored, victim), hit);
        assert!(original.capture_state().unwrap() == restored.capture_state().unwrap());
    }
    for _ in 0..60 {
        original.step(0).unwrap();
        restored.step(0).unwrap();
    }
    assert!(original.capture_state().unwrap() == restored.capture_state().unwrap());
    assert_eq!(original.world.actors[player].actor.attack.contacts, 0);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; native same-update hits remain, toggles take effect on new swings and authored stop timing stays native"]
fn native_multi_target_preserves_frozen_contacts_and_swing_boundaries() {
    for enabled in [false, true] {
        let (mut s, victims) = fixture(0);
        s.world.controls.multi_target_attacks = enabled;
        let player = s.world.player;
        let weapon = swing(&mut s, player, 0);
        for victim in victims {
            place(&mut s, victim, weapon, false);
        }
        compute(&mut s);
        for victim in victims {
            assert!(resolve(&mut s, victim));
        }
        for victim in victims {
            assert!(s.world.actors[victim].actor.health < 30000);
        }
    }
    let (mut s, victims) = fixture(0);
    s.world.controls.multi_target_attacks = true;
    let player = s.world.player;
    let weapon = swing(&mut s, player, 0);
    for victim in victims {
        place(&mut s, victim, weapon, false);
    }
    assert!(contact(&mut s, victims[0]));
    s.world.controls.multi_target_attacks = false;
    assert!(!contact(&mut s, victims[0]));
    assert!(contact(&mut s, victims[1]));
    let weapon = swing(&mut s, player, 0);
    for victim in victims {
        place(&mut s, victim, weapon, false);
    }
    assert!(contact(&mut s, victims[0]));
    assert!(!contact(&mut s, victims[1]));
    s.world.controls.multi_target_attacks = true;
    let weapon = swing(&mut s, player, 0);
    for victim in victims {
        place(&mut s, victim, weapon, false);
    }
    assert!(contact(&mut s, victims[0]));
    let actor = &mut s.world.actors[player].actor;
    actor.motion.loops = 0;
    actor.motion.frame =
        i16::from(s.assets.resources.weapons.attacks[actor.attack.index.unwrap()].end);
    s.world
        .control_actor(
            player,
            &mut s.assets,
            &mut bleeding_tests::Recorded::default(),
        )
        .unwrap();
    assert!(!contact(&mut s, victims[1]));
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; enemies, scripts and suspended input retain native hit consumption"]
fn native_multi_target_is_limited_to_interactive_player_attacks() {
    use tenchu_core::tenchu2::mission::{Outcome, Phase};
    for case in 0..4 {
        let (mut s, victims) = fixture(0);
        s.world.controls.multi_target_attacks = true;
        let id = if case == 0 {
            victims[0]
        } else {
            s.world.player
        };
        match case {
            1 => s.world.script = 1,
            2 => {
                s.world.mission.phase = Phase::Playing {
                    pending: Some(Outcome::AreaCleared),
                }
            }
            3 => s.world.actors[id].behavior.programs.idle = Program::Disabled,
            _ => {}
        }
        let weapons = swing_contacts(&mut s, id);
        for weapon in weapons {
            assert!(
                !s.world.collision.get_mut(weapon).unwrap().multi_target,
                "case {case}"
            );
        }
    }
}

#[test]
#[ignore = "requires both discs; imported and optional unarmed movesets retain native contacts with per-swing hit consumption"]
fn native_multi_target_supports_imported_and_unarmed_contacts() {
    use crate::{settings::Settings, visuals::appearance::library::Library};
    use tenchu_core::{game::sheathing, tenchu2::input::ATTACK};

    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc = sheathing_tests::disc();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = models::donor(&gaisen).unwrap();
    for source in ["disc:0", "disc:1", "tenchu2-jp:ayamea"] {
        for style in [
            None,
            Some(sheathing::UnarmedStyle::Tatsumaru),
            Some(sheathing::UnarmedStyle::Onikage),
        ] {
            let (mut s, victims) = fixture(0);
            s.models
                .sync(
                    &mut s.world,
                    &mut s.assets,
                    &mut donor,
                    &library,
                    &Arc::default(),
                    &Settings {
                        player_model: source.into(),
                        model_moveset: true,
                        ..Default::default()
                    },
                )
                .unwrap();
            s.world.controls.multi_target_attacks = true;
            let player = s.world.player;
            for (i, victim) in victims.into_iter().enumerate() {
                let actor = &mut s.world.actors[victim].actor;
                actor.position = Position::new(5000 + 3000 * i as i32, 0, 5000);
                actor.previous = actor.position;
                actor.flags |= 2;
                s.world.publish(victim, &s.assets);
            }
            compute(&mut s);
            let weapons = if let Some(style) = style {
                let settings = sheathing::Settings {
                    enabled: true,
                    unarmed: true,
                    unarmed_style: style,
                    ..Default::default()
                };
                s.configure_tatsumaru(&disc, settings).unwrap();
                s.configure_onikage(&gaisen, settings).unwrap();
                s.world.set_player_sheathing(settings, &s.assets).unwrap();
                sheathing_tests::weapon(&mut s, false);
                s.step(ATTACK).unwrap();
                assert!(
                    s.world.actors[player].actor.unarmed_attack(),
                    "{source}, {style:?}: {:x}",
                    s.world.actors[player].actor.motion.action
                );
                activate(&mut s, player)
            } else {
                swing_contacts(&mut s, player)
            };
            let weapon = isolate(&mut s, weapons, 0);
            for victim in victims {
                place(&mut s, victim, weapon, false);
                assert!(contact(&mut s, victim), "{source}, {style:?}");
                assert!(
                    s.world.actors[victim].actor.health < 30000,
                    "{source}, {style:?}, victim {victim}, action {:x}, attack {:?}",
                    s.world.actors[victim].actor.motion.action,
                    s.assets.resources.weapons.attacks
                        [s.world.actors[player].actor.attack.index.unwrap()]
                );
                assert!(!contact(&mut s, victim));
            }
            assert_eq!(s.world.actors[player].actor.weapon_drawn(), style.is_none());
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; terrain-blocked damage does not consume a target or let a swing through a wall"]
fn native_multi_target_respects_terrain_before_consuming_hits() {
    for enabled in [false, true] {
        let (mut s, victims) = fixture(0);
        s.world.controls.multi_target_attacks = enabled;
        let player = s.world.player;
        let weapon = swing(&mut s, player, 0);
        let victim = victims[0];
        place(&mut s, victim, weapon, false);
        let position = s.world.actors[victim].actor.position;
        let x = (position.x / 20) as i16;
        let z = (position.z / 20) as i16;
        s.world.terrain.cells[0].nodes.insert(
            0,
            tenchu_assets::terrain::Node {
                y: 0,
                dy: i16::MAX,
                bounds: [x - 20, z - 20, x + 20, z + 20],
                attributes: 0x100,
                division: u16::MAX,
            },
        );
        s.world.cursor = Default::default();
        assert!(contact(&mut s, victim));
        assert_eq!(s.world.actors[victim].actor.health, 30000);
        assert!(
            s.world
                .collision
                .get_mut(weapon)
                .unwrap()
                .hit_targets
                .is_empty()
        );
        s.world.terrain.cells[0].nodes.remove(0);
        s.world.cursor = Default::default();
        place(&mut s, victim, weapon, false);
        assert!(contact(&mut s, victim));
        assert!(s.world.actors[victim].actor.health < 30000);
        assert!(!contact(&mut s, victim));
    }
}
