use super::native_world::{Recorded, fixture};
use tenchu_core::tenchu2::{
    actor::{State, Velocity},
    collision::WEAPON,
    physics::{Depth, FallDeath, HIT, Water},
    rig::WeaponMode,
};

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; ignore-player must preserve patrols, stop new detections and remain reversible"]
fn native_perception_options_preserve_idle_and_restore_detection() {
    use tenchu_core::{
        math::Position,
        tenchu2::{
            ai::{Program, status::Ki},
            input,
        },
    };
    let (mut base, mut assets, guard) = fixture();
    base.script = 0;
    base.actors[base.player].actor.position = Position::new(0, 0, -5000);
    base.actors[base.player].actor.previous = base.actors[base.player].actor.position;
    base.actors[base.player].actor.invulnerable = true;
    let agent = &mut base.actors[guard];
    agent.actor.position = Position::ZERO;
    agent.actor.previous = Position::ZERO;
    agent.actor.rotation.y = 0;
    agent.actor.flags = 0x10;
    agent.actor.target = Some(base.player);
    agent
        .behavior
        .assign(0x1125, base.map, base.difficulty, &mut base.random)
        .unwrap();
    agent.behavior.home = Position::ZERO;
    for id in [base.player, guard] {
        base.publish(id, &assets);
    }
    for enabled in [false, true] {
        let mut world = base.clone();
        world.ai.ignore_player = enabled;
        let mut events = Recorded::default();
        for _ in 0..120 {
            world.step([0; 2], &mut assets, &mut events).unwrap();
        }
        assert_eq!(world.ai.detections == 0, enabled);
        if enabled {
            assert_eq!(world.actors[guard].actor.flags & 3, 0);
            assert!(!matches!(
                world.ai.global.radar,
                Ki::Spotted | Ki::Searching | Ki::Suspicious
            ));
            world.ai.ignore_player = false;
            for _ in 0..90 {
                world.step([0; 2], &mut assets, &mut events).unwrap();
            }
            assert!(world.ai.detections > 0);
        } else {
            let count = world.ai.detections;
            world.ai.ignore_player = true;
            for _ in 0..180 {
                world.step([0; 2], &mut assets, &mut events).unwrap();
                assert_eq!(world.ai.detections, count);
                assert_eq!(
                    world.actors[guard].actor.pad.held & (input::ATTACK | input::ITEM),
                    0
                );
            }
            assert_eq!(world.actors[guard].actor.flags & 3, 0);
            assert!(!matches!(
                world.actors[guard].actor.state,
                State::Attack | State::Hurt
            ));
        }
    }
    let mut world = base.clone();
    world.ai.ignore_player = true;
    world.actors[guard].behavior.programs.idle = Program::Wander;
    let start = world.actors[guard].actor.position;
    let mut events = Recorded::default();
    for _ in 0..240 {
        world.step([0; 2], &mut assets, &mut events).unwrap();
    }
    assert_ne!(
        world.actors[guard].actor.position, start,
        "idle AI was frozen"
    );
    assert_eq!(world.ai.detections, 0);

    let mut scripted = base.clone();
    scripted.script = 2;
    let mut ignored = scripted.clone();
    ignored.ai.ignore_player = true;
    for _ in 0..90 {
        scripted.step([0; 2], &mut assets, &mut events).unwrap();
        ignored.step([0; 2], &mut assets, &mut events).unwrap();
        ignored.ai.ignore_player = false;
        assert_eq!(
            postcard::to_allocvec(&ignored).unwrap(),
            postcard::to_allocvec(&scripted).unwrap()
        );
        ignored.ai.ignore_player = true;
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; perception overrides must cover bait/corpse checks while preserving non-player fights"]
fn native_perception_options_cover_bait_corpses_and_non_player_targets() {
    use tenchu_core::{math::Position, tenchu2::ai::bait::Bait};
    let (mut base, assets, guard) = fixture();
    base.script = 0;
    base.actors[base.player].actor.position = Position::new(0, 0, -5000);
    let agent = &mut base.actors[guard];
    agent.actor.position = Position::ZERO;
    agent.actor.flags = 0x10;
    agent.actor.target = Some(base.player);
    agent
        .behavior
        .assign(0x1125, base.map, base.difficulty, &mut base.random)
        .unwrap();
    base.tick = 0u32.wrapping_sub(agent.slot as u32);
    let mut events = Recorded::default();
    for (percent, ignored) in [(100, false), (0, false), (100, true)] {
        let suppressed = percent == 0 || ignored;
        let mut world = base.clone();
        world.ai.sight_percent = percent;
        world.ai.ignore_player = ignored;
        let agent = &mut world.actors[guard];
        agent.actor.target = None;
        agent.behavior.bait = Some(Bait {
            model: 0,
            position: Position::new(1000, 0, -6000),
        });
        world
            .decide_actor(guard, [0; 2], &assets, &mut events)
            .unwrap();
        assert_eq!(world.ai.detections == 0, suppressed);
        assert_eq!(world.actors[guard].behavior.bait.is_some(), suppressed);

        let mut world = base.clone();
        world.ai.sight_percent = percent;
        world.ai.ignore_player = ignored;
        let corpse = world
            .actors
            .iter()
            .enumerate()
            .find(|(id, a)| {
                *id != guard && *id != world.player && matches!(a.actor.kind & 0xf0, 0x20..=0x50)
            })
            .unwrap()
            .0;
        let body = &mut world.actors[corpse].actor;
        body.health = 0;
        body.state = State::Dead;
        body.motion.loops = i16::MIN;
        body.flags = 0;
        body.position = Position::new(0, 0, -4000);
        world.ai.corpses.observer = guard - 1;
        world
            .decide_actor(corpse, [0; 2], &assets, &mut events)
            .unwrap();
        assert_eq!(world.ai.global.perception.alert.remaining == 0, suppressed);
        assert_eq!(world.actors[guard].actor.flags & 0x80 == 0, suppressed);
    }
    let target = base
        .actors
        .iter()
        .enumerate()
        .find(|(id, _)| *id != base.player && *id != guard)
        .unwrap()
        .0;
    base.actors[target].actor.position = Position::new(0, 0, -5000);
    base.actors[target].actor.flags &= !8;
    base.actors[guard].actor.target = Some(target);
    let mut ignored = base.clone();
    ignored.ai.ignore_player = true;
    base.decide_actor(guard, [0; 2], &assets, &mut events)
        .unwrap();
    ignored
        .decide_actor(guard, [0; 2], &assets, &mut events)
        .unwrap();
    ignored.ai.ignore_player = false;
    assert_eq!(
        postcard::to_allocvec(&ignored).unwrap(),
        postcard::to_allocvec(&base).unwrap()
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; god mode must prevent death at each native damage boundary"]
fn native_god_mode_preserves_hits_and_prevents_combat_poison_drowning_and_fall_death() {
    let (base, mut assets, guard) = fixture();
    for protected in [false, true] {
        let mut world = base.clone();
        let mut events = Recorded::default();
        let attacker = &mut world.actors[guard].actor;
        attacker
            .set_weapon(
                &mut world.instances[guard].rig,
                guard,
                &mut world.collision,
                &assets.resources,
                WeaponMode::Draw,
            )
            .unwrap();
        attacker
            .change_motion(0x900, Velocity::Motion, &assets.resources)
            .unwrap();
        attacker.attack.index = assets
            .resources
            .weapons
            .attacks
            .iter()
            .position(|a| Some(a.clip) == attacker.motion.clip);
        assert!(attacker.attack.index.is_some());
        world.finish_tick(&assets, 6).unwrap();
        let weapon = world
            .collision
            .slots()
            .iter()
            .position(|s| s.owner == Some(guard) && s.flags & WEAPON != 0)
            .unwrap();
        let player = &mut world.actors[0].actor;
        player.health = 1;
        player.invulnerable = protected;
        player.flags |= HIT;
        player.hit = Some(weapon);
        world.control_actor(0, &mut assets, &mut events).unwrap();
        assert_eq!(world.actors[0].actor.health, i16::from(protected));
        assert!(matches!(
            world.actors[0].actor.state,
            State::Hurt | State::Dead
        ));
        assert!(!world.effects.entries.is_empty(), "hits lost their effects");
        assert_eq!(world.actors[0].actor.state == State::Dead, !protected);

        world = base.clone();
        let player = &mut world.actors[0].actor;
        player.health = 1;
        player.invulnerable = protected;
        player.flags |= 0x100;
        player.motion.frame = 1;
        player.motion.loops = 1;
        world.control_actor(0, &mut assets, &mut events).unwrap();
        assert_eq!(world.actors[0].actor.health, i16::from(protected), "poison");

        world = base.clone();
        let player = &mut world.actors[0].actor;
        player.health = 1;
        player.invulnerable = protected;
        player.breath = 0;
        player
            .change_motion(0x530, Velocity::Stop, &assets.resources)
            .unwrap();
        world.water = Water {
            depth: Some(Depth::Submerged),
            surface: -4000,
            bottom: 0,
        };
        for _ in 0..9 {
            world.control_actor(0, &mut assets, &mut events).unwrap();
            world.animate_actor(0, &assets).unwrap();
            world.finish_tick(&assets, 6).unwrap();
        }
        assert_eq!(
            world.actors[0].actor.health,
            i16::from(protected),
            "drowning"
        );
        assert_eq!(world.actors[0].actor.state == State::Dead, !protected);

        world = base.clone();
        let player = &mut world.actors[0].actor;
        player.health = 1;
        player.invulnerable = protected;
        player
            .change_motion(0x702, Velocity::Stop, &assets.resources)
            .unwrap();
        player.velocity.y = 400;
        player.position.y = i32::from(player.height) - 25400;
        let movement = world.move_actor(0, &assets, &mut events).unwrap();
        assert_eq!(
            matches!(movement.death, Some(FallDeath::LongFall)),
            !protected
        );
        assert_eq!(world.actors[0].actor.health, i16::from(protected), "fall");
    }
}
