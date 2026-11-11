use super::*;
use tenchu_core::tenchu2::{actor::Velocity, input::ATTACK};

fn place(s: &mut Session, id: usize, position: Position) {
    let agent = &mut s.world.actors[id];
    agent.actor.position = position;
    agent.actor.previous = position;
    agent.actor.flags &= !(8 | 0x10);
    agent.actor.flags |= 2;
    agent.actor.health = 30000;
    agent.actor.velocity = Default::default();
    agent.behavior.home = position;
    agent
        .actor
        .set_weapon(
            &mut s.world.instances[id].rig,
            id,
            &mut s.world.collision,
            &s.assets.resources,
            tenchu_core::tenchu2::rig::WeaponMode::Draw,
        )
        .unwrap();
    agent
        .actor
        .change_motion(0x100, Velocity::Stop, &s.assets.resources)
        .unwrap();
    s.world.publish(id, &s.assets);
}

fn ready(hero: usize, enabled: bool) -> (Session, [usize; 2]) {
    let (mut s, victims) = melee_tests::fixture(hero);
    s.world.controls.attack_assist = enabled;
    sheathing_tests::weapon(&mut s, true);
    place(&mut s, victims[0], Position::new(-700, 0, -1800));
    place(&mut s, victims[1], Position::new(0, 0, 1000));
    (s, victims)
}

fn drive(s: &mut Session) {
    s.world
        .control_actor(
            s.world.player,
            &mut s.assets,
            &mut bleeding_tests::Recorded::default(),
        )
        .unwrap();
}

fn prepare(s: &mut Session) {
    let actor = &mut s.world.actors[s.world.player].actor;
    let clip = &s.assets.resources.clips[&actor.motion.clip.unwrap()];
    actor.motion.frame = 1 - clip.blend;
    actor.motion.loops = 0;
    actor.pad = Default::default();
    drive(s);
}

fn begin(s: &mut Session) {
    s.world.actors[s.world.player].actor.pad.held = ATTACK;
    drive(s);
    assert_eq!(s.world.actors[s.world.player].actor.motion.action, 0x900);
    prepare(s);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; native forward selection and exact wind-up steering boundaries on all heroes"]
fn native_forward_assist_selects_front_and_stops_at_active_frames() {
    for hero in 0..3 {
        for enabled in [false, true] {
            let (mut s, victims) = ready(hero, enabled);
            begin(&mut s);
            let player = s.world.player;
            assert_eq!(
                s.world.actors[player].actor.target,
                Some(victims[usize::from(!enabled)])
            );
            if !enabled {
                continue;
            }
            let timing = s.assets.resources.weapons.attacks
                [s.world.actors[player].actor.attack.index.unwrap()]
            .clone();
            let limit = i16::from(timing.start.min(timing.steering));
            assert!(limit > 0);
            for frame in 0..i16::from(timing.start) + 3 {
                let actor = &mut s.world.actors[player].actor;
                actor.rotation.y = 0;
                actor.motion.frame = frame;
                actor.motion.loops = 0;
                drive(&mut s);
                let turn = s.world.actors[player].actor.rotation.y;
                assert_eq!(
                    turn,
                    if frame < limit { 136 } else { 0 },
                    "hero {hero}, frame {frame}"
                );
            }
        }
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; native angle wrapping keeps assistance symmetric across a full turn"]
fn native_forward_assist_preserves_native_angle_wrapping() {
    for (yaw, x, turn) in [
        (0, -700, 136),
        (0, 700, -136),
        (4090, -700, 136),
        (4090, 700, -136),
    ] {
        let (mut s, victims) = ready(0, true);
        let player = s.world.player;
        place(&mut s, victims[0], Position::new(x, 0, -1800));
        s.world.actors[player].actor.rotation.y = yaw;
        begin(&mut s);
        assert_eq!(s.world.actors[player].actor.target, Some(victims[0]));
        s.world.actors[player].actor.rotation.y = yaw;
        s.world.actors[player].actor.motion.frame = 0;
        drive(&mut s);
        assert_eq!(
            s.world.actors[player].actor.rotation.y,
            yaw.wrapping_add(turn)
        );
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; assist rejects blocked, dead, suspended, distant and elevated targets without changing the terrain cursor"]
fn native_forward_assist_filters_unreachable_targets() {
    for case in 0..7 {
        let (mut s, victims) = ready(0, true);
        match case {
            0 => s.world.actors[victims[0]].actor.health = 0,
            1 => s.world.actors[victims[0]].actor.flags |= 8,
            2 => s.world.actors[victims[0]].actor.flags |= 0x10,
            3 => place(&mut s, victims[0], Position::new(0, 0, -3000)),
            4 => place(&mut s, victims[0], Position::new(0, -1201, -1800)),
            5 => place(&mut s, victims[0], Position::new(1800, 0, 0)),
            _ => {
                s.world.terrain.cells[0].nodes.insert(
                    0,
                    tenchu_assets::terrain::Node {
                        y: 0,
                        dy: i16::MAX,
                        bounds: [-45, -100, -25, -80],
                        attributes: 0x100,
                        division: u16::MAX,
                    },
                );
                s.world.cursor = Default::default();
            }
        }
        let cursor = postcard::to_allocvec(&s.world.cursor).unwrap();
        begin(&mut s);
        assert_eq!(
            s.world.actors[s.world.player].actor.target, None,
            "case {case}"
        );
        assert_eq!(postcard::to_allocvec(&s.world.cursor).unwrap(), cursor);
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; combo boundaries retain a valid target, replace invalid targets and preserve checkpoint continuation"]
fn native_forward_assist_retains_combo_target_and_checkpoint() {
    let (mut s, victims) = ready(0, true);
    begin(&mut s);
    let player = s.world.player;
    place(&mut s, victims[1], Position::new(0, 0, -1000));
    s.world.actors[player]
        .actor
        .change_motion(0x901, Velocity::Motion, &s.assets.resources)
        .unwrap();
    prepare(&mut s);
    assert_eq!(s.world.actors[player].actor.target, Some(victims[0]));
    s.world.actors[victims[0]].actor.health = 0;
    s.world.actors[player]
        .actor
        .change_motion(0x904, Velocity::Motion, &s.assets.resources)
        .unwrap();
    prepare(&mut s);
    assert_eq!(s.world.actors[player].actor.target, Some(victims[1]));
    let saved = s.capture_state().unwrap();
    place(&mut s, victims[1], Position::new(0, 0, 1000));
    s.world.actors[player].actor.rotation.y = 0;
    s.world.actors[player].actor.motion.frame = 0;
    drive(&mut s);
    assert_eq!(s.world.actors[player].actor.target, None);
    assert_eq!(s.world.actors[player].actor.rotation.y, 0);
    s.restore_state(&saved).unwrap();
    let (mut restored, _) = ready(1, false);
    restored.restore_state(&saved).unwrap();
    assert!(restored.world.controls.attack_assist);
    for _ in 0..60 {
        s.step(0).unwrap();
        restored.step(0).unwrap();
    }
    assert!(s.capture_state().unwrap() == restored.capture_state().unwrap());
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; scripts, disabled controls, backward/dive/crouched attacks and source bow attacks retain native targeting"]
fn native_forward_assist_preserves_source_exclusions() {
    use tenchu_core::tenchu2::{
        ai::Program,
        mission::{Outcome, Phase},
    };
    for case in 0..7 {
        let (mut s, victims) = ready(0, true);
        let player = s.world.player;
        let action = match case {
            0 => {
                s.world.script = 1;
                0x900
            }
            1 => {
                s.world.actors[player].behavior.programs.idle = Program::Disabled;
                0x900
            }
            2 => {
                s.world.mission.phase = Phase::Playing {
                    pending: Some(Outcome::AreaCleared),
                };
                0x900
            }
            3 => 0x913,
            4 => 0x911,
            5 => 0x914,
            _ => {
                let clips = tenchu_assets::tenchu2::motions::collect(
                    &sheathing_tests::disc(),
                    [0x205].into(),
                )
                .unwrap();
                s.assets.resources.clips.extend(clips);
                let actor = s
                    .assets
                    .resources
                    .registry
                    .actors
                    .iter_mut()
                    .find(|a| a.kind == 0)
                    .unwrap();
                actor
                    .motions
                    .iter_mut()
                    .find(|m| m.action == 0x900)
                    .unwrap()
                    .clip = 0x205;
                0x900
            }
        };
        s.world.actors[player]
            .actor
            .change_motion(action, Velocity::Motion, &s.assets.resources)
            .unwrap();
        prepare(&mut s);
        assert_eq!(
            s.world.actors[player].actor.target,
            Some(victims[1]),
            "case {case}"
        );
    }
}

#[test]
#[ignore = "requires both discs; forward assist selects reachable targets for imported armed movesets and both optional unarmed styles"]
fn native_forward_assist_supports_imported_and_unarmed_movesets() {
    use crate::{settings::Settings, visuals::appearance::library::Library};
    use tenchu_core::game::sheathing;
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
            let (mut s, victims) = ready(0, true);
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
            if let Some(style) = style {
                let settings = sheathing::Settings {
                    enabled: true,
                    unarmed: true,
                    unarmed_style: style,
                    ..Default::default()
                };
                s.configure_tatsumaru(&disc, settings).unwrap();
                s.configure_onikage(&gaisen, settings).unwrap();
                s.world.set_player_sheathing(settings, &s.assets).unwrap();
            }
            sheathing_tests::weapon(&mut s, style.is_none());
            begin(&mut s);
            assert_eq!(
                s.world.actors[s.world.player].actor.target,
                Some(victims[0]),
                "{source}, {style:?}"
            );
        }
    }
}
