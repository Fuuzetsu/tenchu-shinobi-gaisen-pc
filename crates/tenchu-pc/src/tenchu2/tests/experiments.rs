use super::native_world::{Recorded, fixture};
use tenchu_core::tenchu2::{
    actor::{State, Velocity},
    collision::WEAPON,
    physics::{Depth, FallDeath, HIT, Water},
    rig::WeaponMode,
};

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
