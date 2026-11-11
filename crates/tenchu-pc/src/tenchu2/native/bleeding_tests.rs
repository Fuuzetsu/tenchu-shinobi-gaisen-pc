use super::{Session, campaign::Content, tools::Tools};
use crate::{controls::Controls, settings};
use std::sync::Arc;
use tenchu_core::{
    Result,
    game::bleeding::{Settings, State as Bleeding},
    random::Random,
    tenchu2::{
        actor::Velocity,
        input::{GUARD, ITEM},
        inventory::{Inventory, Item},
        physics::HIT,
        rig::WeaponMode,
        world::controls::{Event, EventSource, Events},
    },
};

#[derive(Default)]
pub(super) struct Recorded(Vec<Event>);
impl tenchu_core::tenchu2::ai::behavior::Events for Recorded {
    fn voice(&mut self, _: &tenchu_core::tenchu2::actor::Actor, id: u8) -> Result<()> {
        self.0.push(Event::Voice(id));
        Ok(())
    }
}
impl tenchu_core::tenchu2::environment::Events for Recorded {
    fn emit(&mut self, _: tenchu_core::tenchu2::environment::Event, _: &mut Random) -> Result<()> {
        Ok(())
    }
}
impl Events for Recorded {
    fn emit(&mut self, _: EventSource<'_>, event: Event, _: &mut Random) -> Result<()> {
        self.0.push(event);
        Ok(())
    }
}

fn settings() -> Settings {
    Settings {
        enabled: true,
        chance: 100,
        seconds: 2,
        damage: 3,
        ..Default::default()
    }
}

pub(super) fn fixture() -> (Session, usize) {
    let (mut world, assets, guard) = crate::tenchu2::tests::native_world::fixture();
    world.actors[world.player].actor.inventory.counts[0] = 255;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let bank = Arc::new(
        tenchu_assets::tenchu2::audio::Audio::load(&disc, 0, world.map)
            .unwrap()
            .bank,
    );
    (Session::from_parts(world, assets, bank), guard)
}

fn status(session: &Session) -> &Bleeding {
    session.world.actors[session.world.player]
        .actor
        .bleeding
        .as_ref()
        .unwrap()
}

fn location(session: &Session) -> String {
    let w = &session.world;
    let a = &w.actors[w.player].actor;
    format!(
        "state {:?}, action {:x}, frame {}, pos {:?}, floor {:?}, height {}, flags {:x}, camera {}, script {}, phase {:?}",
        a.state,
        a.motion.action,
        a.motion.frame,
        a.position,
        a.terrain.floor,
        a.terrain.height,
        a.flags,
        w.camera.mode,
        w.script,
        w.mission.phase
    )
}

fn wound(session: &mut Session) {
    let state = session.world.actors[session.world.player]
        .actor
        .bleeding
        .as_mut()
        .unwrap();
    state.active = true;
    state.remaining = u32::from(state.settings.seconds) * 30;
}

fn strike(
    session: &mut Session,
    guard: usize,
    held: u16,
    object: Option<(u8, Option<usize>)>,
) -> Recorded {
    strike_with_weapon(session, guard, held, object, true)
}

pub(super) fn strike_with_weapon(
    session: &mut Session,
    guard: usize,
    held: u16,
    object: Option<(u8, Option<usize>)>,
    drawn: bool,
) -> Recorded {
    let world = &mut session.world;
    let assets = &mut session.assets;
    for who in [world.player, guard] {
        let actor = &mut world.actors[who].actor;
        actor
            .set_weapon(
                &mut world.instances[who].rig,
                who,
                &mut world.collision,
                &assets.resources,
                if who == world.player && !drawn {
                    WeaponMode::Sheathe
                } else {
                    WeaponMode::Draw
                },
            )
            .unwrap();
        actor
            .change_motion(
                if who == world.player {
                    if held == GUARD {
                        0x110
                    } else if drawn {
                        0x100
                    } else {
                        0
                    }
                } else {
                    0x900
                },
                Velocity::Stop,
                &assets.resources,
            )
            .unwrap();
        actor.motion.frame = 0;
        actor.flags |= 2;
        actor.pad.update(if who == world.player { held } else { 0 });
        actor.attack.index = assets
            .resources
            .weapons
            .attacks
            .iter()
            .position(|a| Some(a.clip) == actor.motion.clip);
        world.publish(who, assets);
    }
    world.finish_tick(assets, 6).unwrap();
    let index = world
        .collision
        .slots()
        .iter()
        .position(|s| s.owner == Some(guard) && s.flags & 0x4000 != 0)
        .unwrap();
    if let Some((kind, owner)) = object {
        let model = world.collision.slot(index).unwrap().model;
        let slot = world.collision.get_mut(model).unwrap();
        slot.flags = 0x5000 | u16::from(kind);
        slot.owner = owner;
    }
    world.actors[world.player].actor.flags |= HIT;
    world.actors[world.player].actor.hit = Some(index);
    let mut events = Recorded::default();
    world
        .control_actor(world.player, assets, &mut events)
        .unwrap();
    events
}

fn recover(session: &mut Session) {
    let world = &mut session.world;
    for (id, agent) in world.actors.iter_mut().enumerate() {
        if id != world.player {
            agent.actor.flags |= 8;
        }
    }
    let actor = &mut world.actors[world.player].actor;
    actor
        .change_motion(0x100, Velocity::Stop, &session.assets.resources)
        .unwrap();
    actor.flags &= !HIT;
    actor.hit = None;
    actor.pad = Default::default();
    actor.terrain.floor = Some(0);
    actor.terrain.height = 0;
    world.publish(world.player, &session.assets);
}

#[test]
#[ignore = "requires TENCHU2_DISC; source damage, blocking, projectile ownership, protection and resurrection"]
fn native_bleeding_follows_real_damage_without_consuming_source_rng() {
    let (mut session, guard) = fixture();
    let original = session.world.clone();
    let mut control_random = None;
    for chance in [0, 100] {
        session.world = original.clone();
        session.world.set_bleeding(Settings {
            chance,
            ..settings()
        });
        let before = session.world.actors[0].actor.health;
        strike(&mut session, guard, 0, None);
        assert!(session.world.actors[0].actor.health < before);
        assert_eq!(status(&session).active, chance != 0);
        if let Some(random) = control_random {
            assert_eq!(session.world.random, random);
        }
        control_random = Some(session.world.random);
    }
    for (held, invulnerable) in [(GUARD, false), (0, true)] {
        session.world = original.clone();
        session.world.set_bleeding(settings());
        session.world.actors[0].actor.invulnerable = invulnerable;
        let before = session.world.actors[0].actor.health;
        strike(&mut session, guard, held, None);
        assert_eq!(session.world.actors[0].actor.health, before);
        assert!(!status(&session).active);
    }
    for kind in [4, 8, 22, 29, 5, 6, 9] {
        for owner in [Some(guard), Some(0), None] {
            session.world = original.clone();
            session.world.set_bleeding(settings());
            strike(&mut session, guard, 0, Some((kind, owner)));
            assert_eq!(
                status(&session).active,
                owner == Some(guard) && matches!(kind, 4 | 8 | 22 | 29),
                "kind {kind}, owner {owner:?}"
            );
        }
    }
    session.world = original;
    session.world.set_bleeding(Settings {
        medicine: false,
        ..settings()
    });
    wound(&mut session);
    session.world.actors[0].actor.health = 1;
    session.world.actors[0].actor.inventory.resurrections = 1;
    strike(&mut session, guard, 0, None);
    assert_eq!(session.world.actors[0].actor.inventory.resurrections, 0);
    assert!(session.world.actors[0].actor.health > 1);
    assert!(!status(&session).active, "{}", location(&session));
}

#[test]
#[ignore = "requires TENCHU2_DISC; native nominal tick cadence, state continuation, nonfatal drain and treatment restrictions"]
fn native_bleeding_timing_and_treatment_survive_world_continuation() {
    let (mut session, guard) = fixture();
    session.world.set_bleeding(settings());
    strike(&mut session, guard, 0, None);
    recover(&mut session);
    let health = session.world.actors[0].actor.health;
    for _ in 0..29 {
        session.step(0).unwrap();
    }
    assert_eq!(session.world.actors[0].actor.health, health);
    assert_eq!(status(&session).remaining, 31);
    let saved = postcard::to_allocvec(&session.world).unwrap();
    session.step(0).unwrap();
    assert_eq!(session.world.actors[0].actor.health, health - 3);
    let expected = postcard::to_allocvec(&session.world).unwrap();
    session.world = postcard::from_bytes(&saved).unwrap();
    session.step(0).unwrap();
    assert!(postcard::to_allocvec(&session.world).unwrap() == expected);
    session.world.script = 1;
    for _ in 0..40 {
        session.step(0).unwrap();
    }
    assert_eq!(status(&session).remaining, 30);
    session.world.script = 0;
    session.world.actors[0].actor.health = 2;
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    assert_eq!(session.world.actors[0].actor.health, 1);
    assert!(!status(&session).active, "{}", location(&session));
    session.world.set_bleeding(Settings {
        seconds: 0,
        ..settings()
    });
    wound(&mut session);
    let grounded = session.world.clone();
    for condition in ["airborne", "aim", "script", "roll", "carry"] {
        session.world = grounded.clone();
        let actor = &mut session.world.actors[0].actor;
        match condition {
            "airborne" => actor.terrain.height = i32::from(actor.height) + 500,
            "aim" => session.world.camera.mode = 20,
            "script" => session.world.script = 1,
            "roll" | "carry" => {
                actor
                    .change_motion(
                        if condition == "roll" { 0x210 } else { 0x120 },
                        Velocity::Stop,
                        &session.assets.resources,
                    )
                    .unwrap();
            }
            _ => unreachable!(),
        }
        actor.bleeding.as_mut().unwrap().request_bandage();
        session.step(0).unwrap();
        assert!(status(&session).active, "treated while {condition}");
        assert_eq!(status(&session).bandages, 3);
    }
    session.world = grounded;
    let actor = &mut session.world.actors[0].actor;
    actor.bleeding.as_mut().unwrap().request_bandage();
    actor
        .change_motion(0x900, Velocity::Stop, &session.assets.resources)
        .unwrap();
    session.step(0).unwrap();
    assert!(status(&session).active);
    assert_eq!(status(&session).bandages, 3);
    recover(&mut session);
    session.step(0).unwrap();
    assert!(
        status(&session).active,
        "blocked request was queued until recovery"
    );
    session.world.actors[0]
        .actor
        .bleeding
        .as_mut()
        .unwrap()
        .request_bandage();
    session.step(0).unwrap();
    assert!(!status(&session).active, "{}", location(&session));
    assert_eq!(status(&session).bandages, 2);
}

#[test]
#[ignore = "requires TENCHU2_DISC; bandage carousel, empty inventory, dedicated input and native F1 edits"]
fn native_bleeding_bandages_use_shared_controls_and_keep_retail_items() {
    use winit::keyboard::KeyCode;
    let (mut session, _) = fixture();
    session.world.set_bleeding(settings());
    recover(&mut session);
    wound(&mut session);
    let amounts = session.world.actors[0].actor.inventory.amounts();
    session.step(tenchu_core::input::L2.swap_bytes()).unwrap();
    assert!(session.world.controls.hud.bandage_selected());
    session.step(ITEM).unwrap();
    assert!(!status(&session).active, "{}", location(&session));
    assert_eq!(status(&session).bandages, 2);
    assert_eq!(session.world.actors[0].actor.inventory.amounts(), amounts);
    let mut content = Content::Gameplay(Box::new(session));
    let mut controls = Controls::default();
    let mut tools = Tools::default();
    for relative in [false, true] {
        let Content::Gameplay(s) = &mut content else {
            unreachable!()
        };
        recover(s);
        wound(s);
        let count = status(s).bandages;
        controls.key(KeyCode::KeyB, true);
        controls.key(KeyCode::KeyB, false);
        let buttons = tools.input(
            &mut content,
            &mut controls,
            &settings::Settings {
                navigation: settings::Navigation {
                    camera_relative: relative,
                    ..Default::default()
                },
                ..Default::default()
            },
            true,
        );
        assert_eq!(buttons, 0);
        let Content::Gameplay(s) = &mut content else {
            unreachable!()
        };
        s.step(buttons).unwrap();
        assert!(!status(s).active);
        assert_eq!(status(s).bandages, count - 1);
        tools.input(&mut content, &mut controls, &Default::default(), true);
    }
    tools.sync(&content, 0);
    tools.change(settings::menu::Item::Bandages, 2).unwrap();
    tools.apply(&mut content, [800, 600]).unwrap();
    let Content::Gameplay(s) = &mut content else {
        unreachable!()
    };
    assert_eq!(status(s).bandages, 2);
    wound(s);
    s.world.actors[0].actor.inventory = Inventory::from_amounts([0; 21]);
    s.step(0).unwrap();
    assert!(s.world.controls.hud.bandage_selected());
    s.step(ITEM).unwrap();
    assert!(!status(s).active);
    assert_eq!(status(s).bandages, 1);
    assert!(
        s.world.actors[0]
            .actor
            .inventory
            .amounts()
            .iter()
            .all(|&n| n == 0)
    );
    wound(s);
    tools.sync(&content, 0);
    tools.change(settings::menu::Item::UseBandage, 1).unwrap();
    tools.apply(&mut content, [800, 600]).unwrap();
    let Content::Gameplay(s) = &mut content else {
        unreachable!()
    };
    s.step(0).unwrap();
    assert!(!status(s).active);
    assert_eq!(status(s).bandages, 0);
    assert!(!s.world.controls.hud.bandage_selected());
    tools.sync(&content, 0);
    tools.change(settings::menu::Item::Bandages, 200).unwrap();
    tools.apply(&mut content, [800, 600]).unwrap();
    let Content::Gameplay(s) = &content else {
        unreachable!()
    };
    assert_eq!(status(s).bandages, 99);
    tools.reset(settings::menu::Item::Bandages).unwrap();
    tools.apply(&mut content, [800, 600]).unwrap();
    let Content::Gameplay(s) = &mut content else {
        unreachable!()
    };
    assert_eq!(status(s).bandages, 0);
    s.world.set_bleeding(Settings::default());
    tools.sync(&content, 0);
    assert!(tools.change(settings::menu::Item::Bandages, 1).is_err());
}

#[test]
#[ignore = "requires TENCHU2_DISC; only an executed healing-potion effect cures optional bleeding"]
fn native_bleeding_medicine_waits_for_the_source_effect() {
    let (mut session, _) = fixture();
    recover(&mut session);
    let original = session.world.clone();
    for medicine in [false, true] {
        for id in [1, 2, 15] {
            session.world = original.clone();
            session.world.set_bleeding(Settings {
                medicine,
                seconds: 0,
                damage: 0,
                ..settings()
            });
            wound(&mut session);
            let actor = &mut session.world.actors[0].actor;
            actor.health = 20;
            actor.inventory.selected = Some(Item::try_from(id).unwrap());
            actor.inventory.counts[id as usize] = 2;
            session.step(ITEM).unwrap();
            assert_eq!(
                session.world.actors[0].actor.state,
                tenchu_core::tenchu2::actor::State::Item,
                "item {id}: {}",
                location(&session)
            );
            assert!(
                status(&session).active,
                "item press cured before medicine took effect"
            );
            assert_eq!(session.world.actors[0].actor.health, 20);
            let mut healed = false;
            for _ in 0..150 {
                session.step(0).unwrap();
                if session.world.actors[0].actor.health != 20 {
                    healed = true;
                    break;
                }
                assert!(status(&session).active);
            }
            assert!(
                healed,
                "item {id} never executed its effect: {}",
                location(&session)
            );
            assert_eq!(!status(&session).active, medicine && id == 1);
            assert_eq!(status(&session).bandages, 3);
        }
    }
}
