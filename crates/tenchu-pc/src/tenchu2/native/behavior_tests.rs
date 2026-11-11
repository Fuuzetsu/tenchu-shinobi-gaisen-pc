use super::{Session, bleeding_tests};
use tenchu_assets::{tenchu2::layout::Waypoint, terrain::Node};
use tenchu_core::{
    ai::{
        Recruitment,
        profiles::{Combat, Idle, Overrides, Settings},
    },
    math::Position,
    roster::ReinforcementCall,
    tenchu2::{
        actor::{State, Velocity},
        ai::{Program, behavior::Behavior},
        input,
        rig::WeaponMode,
        world::Placement,
    },
};

fn profiles(combat: Combat) -> Settings {
    Settings {
        enabled: true,
        combat,
        ..Default::default()
    }
}

fn fixture() -> (Session, usize) {
    let (mut s, guard) = bleeding_tests::fixture();
    s.world.keep_all_active = true;
    for index in 0..s.world.actors.len() {
        if index != s.world.player && index != guard {
            s.world.script_hidden.insert(index);
        }
    }
    s.world.actors[s.world.player].actor.invulnerable = true;
    ready(&mut s, guard, Position::new(0, 0, 1500));
    (s, guard)
}

fn ready(s: &mut Session, id: usize, position: Position) {
    let agent = &mut s.world.actors[id];
    agent.actor.position = position;
    agent.actor.previous = position;
    agent.actor.rotation.y = 0;
    agent.actor.target = Some(s.world.player);
    agent.actor.flags = 0x12;
    agent.actor.pad = Default::default();
    agent
        .actor
        .change_motion(0x100, Velocity::Stop, &s.assets.resources)
        .unwrap();
    agent
        .actor
        .set_weapon(
            &mut s.world.instances[id].rig,
            id,
            &mut s.world.collision,
            &s.assets.resources,
            WeaponMode::Draw,
        )
        .unwrap();
    agent
        .behavior
        .assign(0x2125, s.world.map, s.world.difficulty, &mut s.world.random)
        .unwrap();
    s.world.publish(id, &s.assets);
}

fn decide(s: &mut Session, id: usize) -> u16 {
    s.world.tick += 1;
    s.world
        .decide_actor(
            s.world.player,
            [0; 2],
            &s.assets,
            &mut bleeding_tests::Recorded::default(),
        )
        .unwrap();
    s.world
        .decide_actor(
            id,
            [0; 2],
            &s.assets,
            &mut bleeding_tests::Recorded::default(),
        )
        .unwrap()
}

fn cover(s: &mut Session) {
    let node = s.world.terrain.cells[0].nodes[0];
    let [left, near, right, far] = node.bounds;
    s.world.terrain.cells[0].nodes = vec![
        Node {
            bounds: [left, near, right, 800],
            ..node
        },
        Node {
            y: -1000,
            dy: 0,
            bounds: [left, 800, right, 1000],
            ..node
        },
        Node {
            bounds: [left, 1000, right, far],
            ..node
        },
    ];
    s.world.cursor = Default::default();
}

fn pending(s: &mut Session, guard: usize) {
    ready(s, guard, Position::new(0, 0, 22000));
    s.world.actors[guard].actor.flags |= 3;
    s.world.actors[guard].behavior.profile.recruitment = Recruitment::Requested;
    s.world.ai.global.perception.alert.remaining = 500;
}

#[test]
#[ignore = "requires TENCHU2_DISC; unchanged default dispatch, actual profile decisions and saved continuation"]
fn native_profiles_preserve_default_ai_and_run_native_combat() {
    let (mut s, guard) = fixture();
    let original = s.world.clone();
    let mut control = original.clone();
    let mut assets = postcard::from_bytes(&postcard::to_allocvec(&s.assets).unwrap()).unwrap();
    for _ in 0..180 {
        s.world.configure_profiles(Settings::default(), &s.assets);
        s.world
            .configure_reinforcements(ReinforcementCall::default(), 0);
        s.world
            .step(
                [0; 2],
                &mut s.assets,
                &mut bleeding_tests::Recorded::default(),
            )
            .unwrap();
        control
            .step(
                [0; 2],
                &mut assets,
                &mut bleeding_tests::Recorded::default(),
            )
            .unwrap();
    }
    assert!(postcard::to_allocvec(&s.world).unwrap() == postcard::to_allocvec(&control).unwrap());
    for combat in [
        Combat::Coward,
        Combat::Reckless,
        Combat::Cautious,
        Combat::Flanker,
        Combat::Watchman,
    ] {
        s.world = original.clone();
        s.world.configure_profiles(profiles(combat), &s.assets);
        s.world.configure_reinforcements(
            ReinforcementCall {
                count: 0,
                ..Default::default()
            },
            0,
        );
        let mut inputs = 0;
        let mut dodged = false;
        for _ in 0..90 {
            inputs |= decide(&mut s, guard);
            dodged |= matches!(s.world.actors[guard].actor.motion.action, 0x113 | 0x114);
            s.world.actors[guard].actor.state = State::Ready;
            s.world.actors[guard].actor.motion.action = 0x100;
        }
        assert_eq!(inputs & input::ITEM, 0, "{combat:?} used an item");
        match combat {
            Combat::Coward => assert!(inputs & input::DOWN != 0 && inputs & input::ATTACK == 0),
            Combat::Reckless => assert_ne!(inputs & input::ATTACK, 0),
            Combat::Cautious => assert!(inputs & input::ATTACK != 0 && inputs & input::DOWN != 0),
            Combat::Flanker | Combat::Watchman => assert!(
                dodged && inputs & input::ATTACK != 0,
                "{combat:?}, inputs {inputs:x}"
            ),
            _ => unreachable!(),
        }
        s.world = original.clone();
        s.world.configure_profiles(profiles(combat), &s.assets);
        s.world.configure_reinforcements(
            ReinforcementCall {
                count: 0,
                ..Default::default()
            },
            0,
        );
        let player = &mut s.world.actors[s.world.player].actor;
        player.invulnerable = false;
        player.health = 30000;
        player.max_health = 30000;
        for _ in 0..60 {
            s.step(0).unwrap();
        }
        let saved = s.capture_state().unwrap();
        for _ in 0..60 {
            s.step(0).unwrap();
        }
        let expected = s.capture_state().unwrap();
        s.restore_state(&saved).unwrap();
        for _ in 0..60 {
            s.step(0).unwrap();
        }
        assert!(
            s.capture_state().unwrap() == expected,
            "{combat:?} continuation"
        );
        if combat != Combat::Coward {
            for _ in 120..900 {
                if s.world.actors[s.world.player].actor.health < 30000 {
                    break;
                }
                s.step(0).unwrap();
            }
            let actor = &s.world.actors[guard].actor;
            eprintln!(
                "{combat:?}: tick {}, player health {}, guard {:?}, yaw {}, action {:x}, held {:x}",
                s.world.tick,
                s.world.actors[s.world.player].actor.health,
                actor.position,
                actor.rotation.y,
                actor.motion.action,
                actor.pad.held
            );
            assert!(
                s.world.actors[s.world.player].actor.health < 30000,
                "{combat:?} attacks never connected"
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; native idle policies, protected roles, independent assignment RNG and campaign thresholds"]
fn native_profiles_respect_roles_routes_and_progression() {
    let (mut s, guard) = fixture();
    let before = s.world.random;
    s.world
        .configure_profiles(profiles(Combat::Mixed), &s.assets);
    let assignment = s.world.profile_selection(guard).unwrap();
    assert_ne!(assignment.combat, Combat::Mixed);
    s.world
        .configure_profiles(profiles(Combat::Mixed), &s.assets);
    assert_eq!(s.world.profile_selection(guard), Some(assignment));
    assert_eq!(s.world.random, before);
    let original = s.world.clone();
    for role in [
        "player",
        "civilian",
        "boss",
        "objective",
        "story",
        "controller",
        "disabled",
        "dead",
    ] {
        s.world = original.clone();
        match role {
            "player" => s.world.player = guard,
            "civilian" => s.world.actors[guard].actor.kind = 0x60,
            "boss" => s.world.actors[guard].actor.kind = 0xe0,
            "objective" => s.world.mission.targets[0] = Some(guard),
            "story" => s.world.actors[guard].behavior.profile.protected = true,
            "controller" => s.world.actors[guard].behavior.programs.idle = Program::PlayerTwo,
            "disabled" => s.world.actors[guard].behavior.programs.combat = Program::Disabled,
            "dead" => s.world.actors[guard].actor.health = 0,
            _ => unreachable!(),
        }
        s.world
            .configure_profiles(profiles(Combat::Reckless), &s.assets);
        assert!(s.world.profile_selection(guard).is_none(), "{role}");
        assert!(
            s.world
                .set_actor_behavior(
                    guard,
                    Overrides {
                        combat: Some(Combat::Coward),
                        idle: None
                    }
                )
                .is_err(),
            "{role}"
        );
    }
    s.world = original.clone();
    for difficulty in 1..=3 {
        s.world.difficulty = difficulty;
        for hero in 0..3 {
            s.assets.hero = hero;
            for sequence in 0..s.assets.campaign.sequences.len() {
                let Some(mission) = s.assets.campaign.mission(hero, sequence) else {
                    continue;
                };
                s.assets.sequence = sequence;
                s.world.configure_profiles(
                    Settings {
                        progression: true,
                        ..profiles(Combat::Mixed)
                    },
                    &s.assets,
                );
                assert_eq!(
                    s.world.profile_selection(guard).is_some(),
                    mission >= usize::from(difficulty) * 2
                );
            }
        }
    }
    s.assets.hero = 0;
    s.assets.sequence = usize::from(original.sequence);
    for idle in Idle::ALL {
        s.world = original.clone();
        s.world.ai.ignore_player = true;
        s.world.actors[s.world.player].actor.position = Position::new(5000, 0, 0);
        let agent = &mut s.world.actors[guard];
        agent.actor.flags = 0x10;
        agent
            .actor
            .change_motion(0, Velocity::Stop, &s.assets.resources)
            .unwrap();
        agent
            .actor
            .set_weapon(
                &mut s.world.instances[guard].rig,
                guard,
                &mut s.world.collision,
                &s.assets.resources,
                WeaponMode::Sheathe,
            )
            .unwrap();
        agent.behavior = Behavior::new(
            agent.actor.position,
            &[Waypoint {
                x: 0,
                z: -10000,
                action: 0,
                wait: 0,
            }],
        );
        agent
            .behavior
            .assign(0x2125, s.world.map, s.world.difficulty, &mut s.world.random)
            .unwrap();
        s.world.configure_profiles(
            Settings {
                idle,
                ..profiles(Combat::Original)
            },
            &s.assets,
        );
        let mut inputs = 0;
        for _ in 0..90 {
            inputs |= decide(&mut s, guard);
        }
        match idle {
            Idle::Original => assert_eq!(inputs, 0),
            Idle::Patrol => assert_ne!(inputs & input::UP, 0),
            Idle::Lookout => assert_eq!(inputs & (input::UP | input::DOWN), 0),
            Idle::TracePlayer => assert_ne!(inputs & (input::LEFT | input::RIGHT), 0),
        }
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; covered calls, population bounds, quota, nonrecursive recruits and exact saved continuation"]
fn native_reinforcement_calls_require_cover_and_keep_limits() {
    let (mut s, guard) = fixture();
    s.world
        .configure_profiles(profiles(Combat::Coward), &s.assets);
    let count = s.world.actors.len();
    s.world.configure_reinforcements(
        ReinforcementCall {
            count: 3,
            ..Default::default()
        },
        (count + 2) as u16,
    );
    pending(&mut s, guard);
    let open = s.world.clone();
    s.step(0).unwrap();
    assert_eq!(s.world.actors.len(), count, "spawned in view");
    s.world = open;
    cover(&mut s);
    let covered = s.world.clone();
    for blocked in [
        "near", "quota", "limit", "disabled", "wet", "low wall", "script", "recruit",
    ] {
        s.world = covered.clone();
        match blocked {
            "near" => s.world.actors[guard].actor.position.z = 15000,
            "quota" => {
                s.world.mission.objective =
                    tenchu_assets::tenchu2::campaign::Objective::EliminateAll;
                s.world.mission.population.guards = 255;
                s.world.mission.population.bosses = 0;
            }
            "limit" => s.world.ai.reinforcements.limit = Some(count),
            "disabled" => s.world.ai.reinforcements.settings.count = 0,
            "wet" => s.world.terrain.cells[0].nodes[2].attributes = 0x200,
            "low wall" => s.world.terrain.cells[0].nodes[1].y = -100,
            "script" => s.world.script = 1,
            "recruit" => {
                s.world.actors[guard].behavior.profile.recruitment = Recruitment::Reinforcement
            }
            _ => unreachable!(),
        }
        s.step(0).unwrap();
        assert_eq!(s.world.actors.len(), count, "{blocked}");
    }
    s.world = covered;
    s.actor_voices.insert(guard, None);
    let saved = s.capture_state().unwrap();
    s.step(0).unwrap();
    assert_eq!(s.world.actors.len(), count + 2);
    for id in count..count + 2 {
        assert_eq!(
            s.world.actors[id].behavior.profile.recruitment,
            Recruitment::Reinforcement
        );
        assert_eq!(
            s.world.actors[id].behavior.profile.recruited_by,
            Some(guard)
        );
        assert!(s.world.actors[id].actor.weapon_drawn());
        assert_eq!(
            s.world.actors[id].actor.kind,
            s.world.actors[guard].actor.kind
        );
        assert!(s.actor_voices.get(&id).is_some_and(Option::is_none));
    }
    for _ in 0..45 {
        s.step(0).unwrap();
    }
    let expected = s.capture_state().unwrap();
    s.restore_state(&saved).unwrap();
    for _ in 0..46 {
        s.step(0).unwrap();
    }
    assert!(s.capture_state().unwrap() == expected);
    s.world.ai.reinforcements.limit = None;
    for _ in 0..120 {
        s.step(0).unwrap();
    }
    assert_eq!(
        s.world.actors.len(),
        count + 2,
        "recursive or repeated call"
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; witnessed deaths use real sight and retreat; inactive settings never replay old deaths"]
fn native_witness_reinforcements_require_an_observed_armed_death() {
    let (mut s, victim) = fixture();
    let kind = s.world.actors[victim].actor.kind;
    let witness = s
        .world
        .place_actor(
            Placement {
                kind,
                definition: kind,
                behavior: 0x2125,
                position: Position::new(0, 0, 6000),
                yaw: 0,
                snap: true,
            },
            &s.assets,
        )
        .unwrap();
    ready(&mut s, witness, Position::new(0, 0, 6000));
    s.world.configure_reinforcements(
        ReinforcementCall {
            count: 2,
            witnessed_kills: true,
            random: false,
        },
        0,
    );
    let original = s.world.clone();
    for mode in [
        "clear",
        "stowed",
        "behind",
        "far",
        "cover",
        "sheathed",
        "story",
        "objective",
        "removed",
        "off",
        "recruit",
    ] {
        s.world = original.clone();
        match mode {
            "stowed" => {
                s.world.actors[s.world.player].actor.position.z = 10000;
                let actor = &mut s.world.actors[witness].actor;
                actor.flags = 0x10;
                actor
                    .change_motion(0, Velocity::Stop, &s.assets.resources)
                    .unwrap();
                actor
                    .set_weapon(
                        &mut s.world.instances[witness].rig,
                        witness,
                        &mut s.world.collision,
                        &s.assets.resources,
                        WeaponMode::Sheathe,
                    )
                    .unwrap();
            }
            "behind" => s.world.actors[witness].actor.rotation.y = 2048,
            "far" => s.world.actors[witness].actor.position.z = 28000,
            "cover" => {
                cover(&mut s);
                s.world.actors[witness].actor.position.z = 12000;
            }
            "sheathed" => {
                s.world.actors[victim]
                    .actor
                    .set_weapon(
                        &mut s.world.instances[victim].rig,
                        victim,
                        &mut s.world.collision,
                        &s.assets.resources,
                        WeaponMode::Sheathe,
                    )
                    .unwrap();
            }
            "story" => s.world.actors[victim].behavior.profile.protected = true,
            "objective" => s.world.mission.targets[0] = Some(victim),
            "off" => s.world.keep_all_active = false,
            "recruit" => {
                s.world.actors[witness].behavior.profile.recruitment = Recruitment::Reinforcement
            }
            _ => (),
        }
        s.world.actors[victim].actor.health = if mode == "removed" { -1 } else { 0 };
        s.world.actors[victim].actor.flags |= 8;
        s.world.script_hidden.insert(victim);
        s.step(0).unwrap();
        assert_eq!(
            s.world.actors[witness].behavior.profile.recruitment == Recruitment::Requested,
            matches!(mode, "clear" | "stowed"),
            "{mode}"
        );
        if mode == "clear" {
            assert_ne!(
                decide(&mut s, witness) & input::DOWN,
                0,
                "witness did not retreat"
            );
            cover(&mut s);
            pending(&mut s, witness);
            let count = s.world.actors.len();
            s.step(0).unwrap();
            assert_eq!(s.world.actors.len(), count + 2);
        } else if mode == "stowed" {
            let position = s.world.actors[witness].actor.position;
            for _ in 0..120 {
                s.step(0).unwrap();
            }
            assert!(s.world.actors[witness].actor.weapon_drawn());
            assert!(s.world.actors[witness].actor.position.z < position.z);
        } else if mode == "off" {
            s.world.keep_all_active = true;
            s.step(0).unwrap();
            assert_ne!(
                s.world.actors[witness].behavior.profile.recruitment,
                Recruitment::Requested
            );
        }
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; an actual paired kill cannot trigger aid immediately or after the animation ends"]
fn native_witness_reinforcements_exclude_paired_kills() {
    let (mut s, victim) = bleeding_tests::fixture();
    s.world.keep_all_active = true;
    for id in 0..s.world.actors.len() {
        if id != s.world.player && id != victim {
            s.world.script_hidden.insert(id);
        }
    }
    s.world.actors[s.world.player].actor.invulnerable = true;
    let kind = s.world.actors[victim].actor.kind;
    let witness = s
        .world
        .place_actor(
            Placement {
                kind,
                definition: kind,
                behavior: 0x2125,
                position: Position::new(0, 0, 6000),
                yaw: 0,
                snap: true,
            },
            &s.assets,
        )
        .unwrap();
    ready(&mut s, witness, Position::new(0, 0, 6000));
    s.world.configure_reinforcements(
        ReinforcementCall {
            witnessed_kills: true,
            ..Default::default()
        },
        0,
    );
    let count = s.world.actors.len();
    s.world.actors[s.world.player]
        .actor
        .pad
        .update(input::ATTACK);
    s.world
        .control_actor(
            s.world.player,
            &mut s.assets,
            &mut bleeding_tests::Recorded::default(),
        )
        .unwrap();
    assert_eq!(s.world.actors[victim].actor.health, 0);
    assert_eq!(s.world.statistics.stealth_kills, 1);
    assert!(!s.world.actors[victim].actor.weapon_drawn());
    for _ in 0..240 {
        s.step(0).unwrap();
        assert_ne!(
            s.world.actors[witness].behavior.profile.recruitment,
            Recruitment::Requested
        );
        assert_eq!(s.world.actors.len(), count);
    }
    assert!(!(0x920..=0x92f).contains(&s.world.actors[s.world.player].actor.motion.action));
}

#[test]
#[ignore = "requires both discs; random groups, normal activation cap, long cover rays and imported caller appearance/voice"]
fn native_reinforcement_groups_retain_loaded_callers_and_native_limits() {
    use super::spawning::{Request, Source};
    use crate::visuals::appearance::library::Library;
    let (mut s, original_guard) = fixture();
    let game = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let library = Library::load(&game).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let guard = s
        .spawn_actor(
            &disc,
            Request {
                source: Source::Model("disc:16".into()),
                position: Position::new(0, 0, 22000),
                yaw: 0,
                snap: true,
            },
            Some(&library),
        )
        .unwrap();
    s.world.actors[original_guard].actor.flags |= 8;
    s.world.script_hidden.insert(original_guard);
    s.world
        .configure_profiles(profiles(Combat::Coward), &s.assets);
    pending(&mut s, guard);
    s.world.terrain.origin = [-6000, -6000];
    s.world.terrain.cell_size = 12000;
    s.world.terrain.cells[0].nodes[0].bounds = [-6000, -6000, 6000, 6000];
    s.world.actors[s.world.player].actor.position.z = -25000;
    s.world.actors[s.world.player].actor.previous.z = -25000;
    s.world.publish(s.world.player, &s.assets);
    let count = s.world.actors.len();
    s.world.configure_reinforcements(
        ReinforcementCall {
            count: 3,
            random: true,
            witnessed_kills: false,
        },
        0,
    );
    let open = s.world.clone();
    s.step(0).unwrap();
    assert_eq!(
        s.world.actors.len(),
        count,
        "long unobstructed line counted as cover"
    );
    s.world = open;
    cover(&mut s);
    let original = s.world.clone();
    let voice = s.actor_voices[&guard].clone().unwrap();
    let mut sizes = std::collections::BTreeSet::new();
    for seed in 1..=16 {
        s.world = original.clone();
        s.actor_voices.retain(|&id, _| id < count);
        s.world.random = tenchu_core::random::Random::new(seed);
        s.step(0).unwrap();
        let added = s.world.actors.len() - count;
        assert!((1..=3).contains(&added), "seed {seed}, added {added}");
        sizes.insert(added);
        for id in count..s.world.actors.len() {
            let actor = &s.world.actors[id].actor;
            assert_eq!(
                actor.definition_kind(),
                s.world.actors[guard].actor.definition_kind()
            );
            assert_ne!(actor.definition_kind(), actor.kind);
            assert!(std::sync::Arc::ptr_eq(
                &s.actor_voices[&id].as_ref().unwrap().bank,
                &voice.bank
            ));
        }
    }
    assert_eq!(sizes.len(), 3, "random group size did not vary");
    s.world = original;
    s.actor_voices.retain(|&id, _| id < count);
    s.world.keep_all_active = false;
    s.world.configure_reinforcements(
        ReinforcementCall {
            count: 32,
            random: false,
            witnessed_kills: false,
        },
        (count + 1) as u16,
    );
    assert!(count < 30);
    s.step(0).unwrap();
    assert_eq!(s.world.actors.len(), 30);
}
