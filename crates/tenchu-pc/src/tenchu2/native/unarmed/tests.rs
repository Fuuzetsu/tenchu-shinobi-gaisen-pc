use super::*;
use crate::tenchu2::native::sheathing_tests::{disc, weapon};
use tenchu_core::tenchu2::{
    actor::{State, Velocity},
    input::*,
    world::Options,
};

fn gaisen() -> GameAssets<crate::source::DiscSource> {
    crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap()
}

fn ready(hero: usize) -> Session {
    let mut s = Session::with_options(
        &disc(),
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
        s.step(0).unwrap();
    }
    s.world.actors[s.world.player].actor.invulnerable = true;
    s
}

fn configure(
    s: &mut Session,
    gaisen: &GameAssets<crate::source::DiscSource>,
    style: UnarmedStyle,
) -> Settings {
    let settings = Settings {
        enabled: true,
        unarmed: true,
        unarmed_style: style,
        ..Default::default()
    };
    s.configure_tatsumaru(&disc(), settings).unwrap();
    s.configure_onikage(gaisen, settings).unwrap();
    s.world.set_player_sheathing(settings, &s.assets).unwrap();
    settings
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; both optional unarmed styles use source hit contacts and complete combos while retaining stowed weapons for all heroes"]
fn native_unarmed_combos_contacts_and_checkpoint() {
    let gaisen = gaisen();
    for hero in 0..3 {
        for style in [UnarmedStyle::Onikage, UnarmedStyle::Tatsumaru] {
            let mut s = ready(hero);
            configure(&mut s, &gaisen, style);
            weapon(&mut s, false);
            let id = s.world.player;
            let attachments = s.world.instances[id].rig.weapons;
            let set = s.world.actors[id]
                .actor
                .unarmed
                .as_ref()
                .unwrap()
                .moves
                .clone();
            s.step(ATTACK).unwrap();
            assert!(
                s.world.actors[id].actor.unarmed_attack(),
                "hero {hero}, {style:?}: {:x}",
                s.world.actors[id].actor.motion.action
            );
            let saved = s.capture_state().unwrap();
            let mut recorder = crate::rewind::Recorder::default();
            recorder.begin();
            s.snapshot().capture(&mut recorder).unwrap();
            assert!(
                recorder.finish().unwrap().bytes().unwrap()
                    == postcard::to_allocvec(&s.snapshot()).unwrap()
            );
            let mut restored = ready(hero);
            restored.restore_state(&saved).unwrap();
            let mut actions = std::collections::BTreeSet::new();
            let mut contacts = 0;
            for _ in 0..240 {
                let a = &s.world.actors[id].actor;
                let timing = a
                    .motion
                    .clip
                    .and_then(|c| s.assets.resources.weapons.attack(c));
                let press = a.unarmed_attack()
                    && timing.is_some_and(|t| t.combo > 0 && a.motion.frame == i16::from(t.combo));
                if a.unarmed_attack() {
                    actions.insert(a.motion.action);
                }
                if a.attack.contacts != 0 {
                    let attack = &set.attacks[&a.motion.action];
                    let expected = attack
                        .contacts
                        .iter()
                        .flatten()
                        .fold(0, |mask, c| mask | (1 << c.part));
                    assert_eq!(a.attack.contacts, expected);
                    contacts += 1;
                }
                assert!(!a.weapon_drawn());
                assert_eq!(
                    s.world.instances[id].rig.weapons, attachments,
                    "unarmed attack moved a blade"
                );
                assert_eq!(a.attack.trails, 0);
                let buttons = if press { ATTACK } else { 0 };
                s.step(buttons).unwrap();
                restored.step(buttons).unwrap();
                assert!(
                    postcard::to_allocvec(&s.world).unwrap()
                        == postcard::to_allocvec(&restored.world).unwrap()
                );
            }
            assert!(s.capture_state().unwrap() == restored.capture_state().unwrap());
            assert!(contacts > 0);
            for action in [0x900, 0x901, 0x904] {
                assert!(actions.contains(&action), "{style:?} combo {actions:x?}");
            }
            if style == UnarmedStyle::Tatsumaru {
                assert!(actions.contains(&0x906), "{actions:x?}");
            }
            let a = &s.world.actors[id].actor;
            assert!(!a.unarmed_attack());
            assert_eq!(a.attack.contacts, 0);
            assert_eq!(a.state, State::Normal);
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; optional styles preserve armed source combat, defer switching and disabling until active attacks finish"]
fn native_unarmed_armed_fallback_and_setting_changes() {
    let gaisen = gaisen();
    for hero in 0..3 {
        let mut original = ready(hero);
        weapon(&mut original, true);
        let saved = original.capture_state().unwrap();
        let mut s = ready(hero);
        s.restore_state(&saved).unwrap();
        configure(&mut s, &gaisen, UnarmedStyle::Tatsumaru);
        for tick in 0..150 {
            let input = if tick == 0 { ATTACK | RIGHT } else { 0 };
            original.step(input).unwrap();
            s.step(input).unwrap();
            let mut world = s.world.clone();
            world.actors[world.player].actor.sheathing = None;
            world.actors[world.player].actor.unarmed = None;
            assert!(
                postcard::to_allocvec(&world).unwrap()
                    == postcard::to_allocvec(&original.world).unwrap()
            );
        }
        weapon(&mut s, false);
        s.step(ATTACK).unwrap();
        let id = s.world.player;
        assert!(s.world.actors[id].actor.unarmed_attack());
        let settings = configure(&mut s, &gaisen, UnarmedStyle::Onikage);
        assert_eq!(
            s.world.actors[id]
                .actor
                .unarmed
                .as_ref()
                .unwrap()
                .moves
                .style,
            UnarmedStyle::Tatsumaru
        );
        s.world
            .set_player_sheathing(Settings::default(), &s.assets)
            .unwrap();
        for _ in 0..100 {
            s.step(0).unwrap();
        }
        assert!(!s.world.actors[id].actor.unarmed_attack());
        assert!(!s.world.actors[id].actor.weapon_drawn());
        assert_eq!(s.world.actors[id].actor.attack.contacts, 0);
        s.configure_onikage(&gaisen, settings).unwrap();
        assert_eq!(
            s.world.actors[id]
                .actor
                .unarmed
                .as_ref()
                .unwrap()
                .moves
                .style,
            UnarmedStyle::Onikage
        );
        // The disabled optional controller returns to the original draw-on-attack behavior.
        s.world.actors[id]
            .actor
            .change_motion(0, Velocity::Stop, &s.assets.resources)
            .unwrap();
        s.step(ATTACK).unwrap();
        assert_eq!(s.world.actors[id].actor.motion.action, 0x700);
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; both optional unarmed styles damage native actors through source contacts on a controlled floor, without drawing blades"]
fn native_unarmed_attacks_damage_actors() {
    use tenchu_core::tenchu2::rig::WeaponMode;
    let gaisen = gaisen();
    let (floor, floor_assets, _) = crate::tenchu2::tests::native_world::fixture();
    for hero in 0..3 {
        for style in [UnarmedStyle::Onikage, UnarmedStyle::Tatsumaru] {
            let mut s = ready(hero);
            s.world.actors[s.world.player].actor.invulnerable = true;
            configure(&mut s, &gaisen, style);
            weapon(&mut s, false);
            let id = s.world.player;
            s.world.terrain = floor.terrain.clone();
            s.assets.level.collision = floor_assets.level.collision.clone();
            let player = &mut s.world.actors[id].actor;
            player.position = Position::default();
            player.previous = player.position;
            player.rotation = Default::default();
            player.terrain = Default::default();
            player.turn = 0;
            let p = player.position;
            let yaw = player.rotation.y;
            let guard = s
                .spawn_actor(
                    &disc(),
                    crate::tenchu2::native::spawning::Request {
                        source: crate::tenchu2::native::spawning::Source::Native(0x34),
                        position: Position::new(2000, 0, -2000),
                        yaw: 0,
                        snap: true,
                    },
                    None,
                )
                .unwrap();
            for (i, agent) in s.world.actors.iter_mut().enumerate() {
                if i != id {
                    agent.behavior.programs.idle = tenchu_core::tenchu2::ai::Program::Disabled;
                    agent.behavior.programs.combat = tenchu_core::tenchu2::ai::Program::Disabled;
                    if i != guard {
                        agent.actor.flags |= 8 | 0x10;
                        for &joint in &s.world.instances[i].rig.joints {
                            s.world.collision.remove(joint);
                        }
                    }
                }
            }
            s.world.actors[guard].behavior.has_suspicious_program = true;
            let actor = &mut s.world.actors[guard].actor;
            actor.flags &= !(0x10 | 8);
            actor.velocity = Default::default();
            actor.position = p.wrapping_add(Position::new(2000, 0, -2000));
            actor.previous = actor.position;
            actor.rotation.y = yaw;
            actor.terrain = Default::default();
            actor.health = 30000;
            actor
                .set_weapon(
                    &mut s.world.instances[guard].rig,
                    guard,
                    &mut s.world.collision,
                    &s.assets.resources,
                    WeaponMode::Draw,
                )
                .unwrap();
            actor
                .set_body_collision(
                    s.world.instances[guard].rig.joints[0],
                    guard,
                    &mut s.world.collision,
                    true,
                )
                .unwrap();
            s.world.publish(id, &s.assets);
            s.world.publish(guard, &s.assets);
            for _ in 0..3 {
                s.step(0).unwrap();
            }
            s.step(ATTACK).unwrap();
            assert!(s.world.actors[id].actor.unarmed_attack());
            let mut placed = false;
            for _ in 0..180 {
                let a = &s.world.actors[id].actor;
                let press = a.unarmed_attack()
                    && a.motion
                        .clip
                        .and_then(|c| s.assets.resources.weapons.attack(c))
                        .is_some_and(|t| t.combo > 0 && a.motion.frame == i16::from(t.combo));
                assert!(!a.weapon_drawn());
                if !placed && a.attack.contacts != 0 {
                    let contact = s
                        .world
                        .collision
                        .slots()
                        .iter()
                        .find(|c| {
                            c.owner == Some(id)
                                && c.flags == tenchu_core::tenchu2::collision::WEAPON
                                && c.position.y < -400
                        })
                        .unwrap()
                        .position;
                    let actor = &mut s.world.actors[guard].actor;
                    actor.position.x = contact.x;
                    actor.position.z = contact.z;
                    actor.previous = actor.position;
                    s.world.publish(guard, &s.assets);
                    s.world
                        .collision
                        .compute(|m| s.world.scene.world(m))
                        .unwrap();
                    placed = true;
                }
                s.step(if press { ATTACK } else { 0 }).unwrap();
            }
            assert!(
                s.world.actors[guard].actor.health < 30000,
                "hero {hero}, {style:?}: no unarmed damage"
            );
            eprintln!(
                "hero {hero}, {style:?}: {} damage",
                30000 - s.world.actors[guard].actor.health
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; grounded direction, crouch and dodge attacks select the optional style and leave weapons stowed"]
fn native_unarmed_directional_entries_and_contacts() {
    let gaisen = gaisen();
    for style in [UnarmedStyle::Onikage, UnarmedStyle::Tatsumaru] {
        let mut s = ready(0);
        configure(&mut s, &gaisen, style);
        weapon(&mut s, false);
        let saved = s.capture_state().unwrap();
        for (stance, buttons, expected) in [
            (0, ATTACK | RIGHT, 0x907),
            (0, ATTACK | LEFT, 0x90a),
            (0x200, ATTACK | CROUCH, 0x914),
            (0x111, ATTACK | GUARD, 0x910),
            (0x112, ATTACK | GUARD, 0x913),
        ] {
            s.restore_state(&saved).unwrap();
            let id = s.world.player;
            let a = &mut s.world.actors[id].actor;
            a.change_motion(stance, Velocity::Stop, &s.assets.resources)
                .unwrap();
            a.motion.frame = 2;
            a.pad = Default::default();
            a.turn = 0;
            let yaw = a.rotation.y;
            let supported = a
                .unarmed
                .as_ref()
                .unwrap()
                .moves
                .motions
                .contains_key(&expected);
            let attachments = s.world.instances[id].rig.weapons;
            s.world.publish(id, &s.assets);
            s.step(buttons).unwrap();
            let a = &s.world.actors[id].actor;
            if !supported {
                assert!(!a.unarmed_attack());
                assert!(!a.weapon_drawn());
                continue;
            }
            assert_eq!(a.motion.action, expected, "{style:?}, stance {stance:x}");
            assert!(a.unarmed_attack());
            let exit_turn = if style == UnarmedStyle::Tatsumaru && matches!(expected, 0x907 | 0x90a)
            {
                let idle = s.assets.resources.registry.clip(2, 0x100).unwrap();
                let clips = motions::collect(&disc(), [idle].into()).unwrap();
                let rest = clips[&idle].bones[0][0].value[1];
                let end = s.assets.resources.clips[&a.motion.clip.unwrap()].bones[0]
                    .last()
                    .unwrap()
                    .value[1];
                Some(end.wrapping_sub(rest))
            } else {
                None
            };
            let mut hit = false;
            for _ in 0..100 {
                let a = &s.world.actors[id].actor;
                hit |= a.attack.contacts != 0;
                assert!(!a.weapon_drawn());
                assert_eq!(s.world.instances[id].rig.weapons, attachments);
                s.step(0).unwrap();
            }
            assert!(hit, "{style:?}, action {expected:x}");
            assert!(!s.world.actors[id].actor.unarmed_attack());
            assert_eq!(s.world.actors[id].actor.attack.contacts, 0);
            if let Some(turn) = exit_turn {
                assert_eq!(
                    s.world.actors[id].actor.rotation.y.wrapping_sub(yaw) & 4095,
                    turn & 4095,
                    "source heading after {expected:x}"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and Japanese TENCHU2_DISC; optional unarmed entry leaves script-controlled and airborne input exactly on the native path"]
fn native_unarmed_script_and_air_controls_stay_native() {
    let gaisen = gaisen();
    for style in [UnarmedStyle::Onikage, UnarmedStyle::Tatsumaru] {
        let mut s = ready(0);
        configure(&mut s, &gaisen, style);
        weapon(&mut s, false);
        let original = s.world.clone();
        for (action, script) in [
            (0, 1),
            (0x111, 1),
            (0x210, 1),
            (0, 3),
            (0x111, 3),
            (0x600, 0),
        ] {
            s.world = original.clone();
            let id = s.world.player;
            s.world.script = script;
            let actor = &mut s.world.actors[id].actor;
            actor
                .change_motion(action, Velocity::Stop, &s.assets.resources)
                .unwrap();
            actor.pad.update(ATTACK);
            if action == 0x600 {
                actor.flags |= tenchu_core::tenchu2::physics::FALLING;
                actor.terrain.height = 3000;
                actor.position.y -= 3000;
            }
            let mut reference = s.world.clone();
            reference.actors[id].actor.unarmed = None;
            reference.actors[id].actor.sheathing = None;
            reference
                .control_actor(
                    id,
                    &mut s.assets,
                    &mut crate::tenchu2::native::bleeding_tests::Recorded::default(),
                )
                .unwrap();
            s.world
                .control_actor(
                    id,
                    &mut s.assets,
                    &mut crate::tenchu2::native::bleeding_tests::Recorded::default(),
                )
                .unwrap();
            assert!(!s.world.actors[id].actor.unarmed_attack());
            s.world.actors[id].actor.unarmed = None;
            s.world.actors[id].actor.sheathing = None;
            assert!(
                postcard::to_allocvec(&reference).unwrap()
                    == postcard::to_allocvec(&s.world).unwrap(),
                "{style:?}, action {action:x}, script {script}"
            );
        }
    }
}
