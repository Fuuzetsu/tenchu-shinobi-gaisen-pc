use super::super::bleeding_tests::{Recorded, fixture, strike_with_weapon};
use super::*;
use tenchu_core::tenchu2::{
    actor::State,
    physics::{HIT, LANDED},
};

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; actual melee/projectile hits apply stowed health modifiers without changing recoil/RNG; God mode and resurrection still work"]
fn native_sheathing_damage_modifiers() {
    let (mut session, guard) = fixture();
    let player = session.world.player;
    session.world.actors[player].actor.health = 2000;
    session.world.actors[player].actor.max_health = 2000;
    let original = session.world.clone();
    for drawn in [false, true] {
        for object in [None, Some(4), Some(8), Some(22), Some(29)] {
            let object = object.map(|kind| (kind, Some(guard)));
            session.world = original.clone();
            strike_with_weapon(&mut session, guard, 0, object, drawn);
            let baseline = session.world.clone();
            let damage = 2000 - baseline.actors[player].actor.health;
            assert!(damage > 0, "object {object:?}");
            for (multiplier, one_hit, god, resurrection) in [
                (250, false, false, false),
                (100, true, false, false),
                (400, true, true, false),
                (400, true, false, true),
            ] {
                session.world = original.clone();
                configure(
                    &mut session,
                    Sheathing {
                        enabled: true,
                        damage: multiplier,
                        one_hit,
                        ..Default::default()
                    },
                );
                session.world.actors[player].actor.invulnerable = god;
                session.world.actors[player].actor.inventory.resurrections = u8::from(resurrection);
                strike_with_weapon(&mut session, guard, 0, object, drawn);
                let actor = &session.world.actors[player].actor;
                let fatal = !drawn && one_hit && !god;
                let expected = if god || fatal && resurrection {
                    2000
                } else if fatal {
                    0
                } else if drawn {
                    2000 - damage
                } else {
                    2000 - (i32::from(damage) * i32::from(multiplier) / 100) as i16
                };
                assert_eq!(
                    actor.health, expected,
                    "drawn {drawn}, {object:?}, {multiplier}, one hit {one_hit}, god {god}, resurrection {resurrection}"
                );
                if !fatal && !god {
                    assert_eq!(actor.velocity, baseline.actors[player].actor.velocity);
                    assert_eq!(
                        actor.motion.action,
                        baseline.actors[player].actor.motion.action
                    );
                    assert_eq!(session.world.random, baseline.random);
                }
                if fatal && resurrection {
                    assert_eq!(actor.inventory.resurrections, 0);
                }
            }
        }
    }
    session.world = original;
    configure(
        &mut session,
        Sheathing {
            enabled: true,
            damage: 400,
            one_hit: true,
            ..Default::default()
        },
    );
    strike_with_weapon(&mut session, guard, GUARD, None, true);
    assert_eq!(session.world.actors[player].actor.health, 2000);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; native injury/knockdown already retains weapon state and stowed penalties leave falling/ongoing poison unchanged"]
fn native_sheathing_preserves_source_injury_rules() {
    let (mut session, guard) = fixture();
    let player = session.world.player;
    for drawn in [false, true] {
        for knockdown in [false, true] {
            let (fresh, _) = fixture();
            session = fresh;
            session.world.actors[player].actor.health = 500;
            session.world.actors[player].actor.max_health = 500;
            if knockdown {
                for attack in &mut session.assets.resources.weapons.attacks {
                    attack.power = 100;
                }
            }
            strike_with_weapon(&mut session, guard, 0, None, drawn);
            assert!(session.world.actors[player].actor.health > 0);
            assert_eq!(session.world.actors[player].actor.state, State::Hurt);
            assert_eq!(
                matches!(
                    session.world.actors[player].actor.motion.action,
                    0xa21..=0xa26
                ),
                knockdown,
                "drawn {drawn}, knockdown {knockdown}, action {:x}, health {}",
                session.world.actors[player].actor.motion.action,
                session.world.actors[player].actor.health
            );
            for id in 0..session.world.actors.len() {
                if id != player {
                    let actor = &mut session.world.actors[id].actor;
                    actor.flags |= 8;
                    actor
                        .set_weapon(
                            &mut session.world.instances[id].rig,
                            id,
                            &mut session.world.collision,
                            &session.assets.resources,
                            WeaponMode::Remove,
                        )
                        .unwrap();
                }
            }
            session.world.actors[player].actor.flags &= !HIT;
            session.world.actors[player].actor.hit = None;
            for _ in 0..300 {
                session.step(0).unwrap();
                let actor = &session.world.actors[player].actor;
                if matches!(actor.state, State::Normal | State::Ready) {
                    break;
                }
            }
            let actor = &session.world.actors[player].actor;
            assert!(
                matches!(actor.state, State::Normal | State::Ready),
                "drawn {drawn}, knockdown {knockdown}: action {:x}, HP {}, pos {:?}, frame {}, loops {}, phase {:?}",
                actor.motion.action,
                actor.health,
                actor.position,
                actor.motion.frame,
                actor.motion.loops,
                session.world.mission.phase
            );
            assert_eq!(actor.weapon_drawn(), drawn);
        }
    }
    for falling in [false, true] {
        let (mut session, _) = fixture();
        session.world.difficulty = 1;
        let actor = &mut session.world.actors[player].actor;
        actor
            .change_motion(
                if falling { 0x702 } else { 0 },
                Velocity::Stop,
                &session.assets.resources,
            )
            .unwrap();
        actor.flags = if falling { LANDED } else { 0x100 };
        actor.last_vertical_velocity = 400;
        actor.motion.frame = 1;
        actor.motion.loops = 1;
        let original = session.world.clone();
        session
            .world
            .control_actor(player, &mut session.assets, &mut Recorded::default())
            .unwrap();
        let expected = session.world.clone();
        assert!(expected.actors[player].actor.health < original.actors[player].actor.health);
        session.world = original;
        configure(
            &mut session,
            Sheathing {
                enabled: true,
                damage: 400,
                one_hit: true,
                ..Default::default()
            },
        );
        session
            .world
            .control_actor(player, &mut session.assets, &mut Recorded::default())
            .unwrap();
        session.world.actors[player].actor.sheathing = None;
        assert!(
            postcard::to_allocvec(&session.world).unwrap()
                == postcard::to_allocvec(&expected).unwrap()
        );
    }
}
