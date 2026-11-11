use super::*;
use tenchu_core::tenchu2::input::{ATTACK, CROUCH, JUMP, RIGHT, UP};

mod state;

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; controller-only approach and combat from mission start, with guard-only mixed voice audio"]
fn native_guards_fight_and_speak_from_mission_start() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = Session::load(&disc, 9, 1).unwrap();
    let target = 1;
    assert_eq!(session.world.actors[target].actor.kind, 0x34);
    let voice = session.world.instances[target].voice;
    let mut mixer = tenchu_audio::Mixer::new(44100);
    mixer.set_bank(session.bank.clone());
    let mut voiced = 0;
    let mut peak = 0.0f32;
    for tick in 0..1800 {
        let player = &session.world.actors[session.world.player].actor;
        let delta = session.world.actors[target]
            .actor
            .position
            .wrapping_sub(player.position);
        let yaw = session
            .assets
            .resources
            .math
            .atan2(-delta.x, -delta.z)
            .unwrap();
        let turn = (i32::from(yaw) - i32::from(player.rotation[1]) + 2048).rem_euclid(4096) - 2048;
        let held = if tick < 30 {
            0
        } else if delta.x.abs().max(delta.z.abs()) > 1400 {
            if turn > 64 {
                RIGHT
            } else if turn < -64 {
                tenchu_core::tenchu2::input::LEFT
            } else {
                UP
            }
        } else if tick % 45 == 0 {
            ATTACK
        } else {
            0
        };
        for event in session.step(held).unwrap() {
            let sound = event.sound;
            if sound.id >> 4 == voice {
                voiced += 1;
                mixer.play_at(
                    usize::from(sound.slot),
                    i16::from(sound.id),
                    sound.volume,
                    sound.note,
                );
            }
        }
        for _ in 0..1470 {
            peak = mixer.frame().into_iter().fold(peak, |v, x| v.max(x.abs()));
        }
        if session.world.actors[session.world.player].actor.health <= 0
            || session.world.actors[target].actor.health <= 0
        {
            break;
        }
    }
    let player = &session.world.actors[session.world.player].actor;
    assert!(player.health > 0 && player.health < player.max_health);
    assert_eq!(session.world.actors[target].actor.health, 0);
    assert!(
        voiced > 5 && peak > 0.01,
        "guard voices missing: {voiced}, {peak}"
    );
    assert!(session.pending.is_empty(), "{:?}", session.pending);
    eprintln!(
        "Controller-only JP encounter: tick {}, player HP {}, guard HP 0; {voiced} guard voice cues, mixed peak {peak:.3}",
        session.world.tick, player.health
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC and TENCHU2_SHADING_RAM captured during retail training"]
fn native_character_shading_matches_the_retail_color_table() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let ram = std::fs::read(std::env::var_os("TENCHU2_SHADING_RAM").unwrap()).unwrap();
    assert_eq!(ram.len(), 0x200000);
    assert_eq!(
        u16::from_le_bytes(ram[0x8e938..0x8e93a].try_into().unwrap()),
        0
    );
    let session = Session::load(&disc, 1, 1).unwrap();
    let mut corners = 0;
    let mut tinted = 0;
    for (name, shaded) in &session.assets.models {
        let original = tenchu_assets::tenchu2::characters::rig(&disc, name).unwrap();
        for (part, shaded) in original.parts.iter().zip(&shaded.parts) {
            for (geometry, shaded) in part.geometry.iter().zip(&shaded.geometry) {
                for (face, shaded) in geometry.faces.iter().zip(&shaded.faces) {
                    for (corner, shaded) in face.corners.iter().zip(&shaded.corners) {
                        let at = 0x81544 + usize::from(corner.color[0]) * 4;
                        assert_eq!(shaded.color, corner.color, "{name}: retained shade index");
                        let color = session.assets.level.shading.color(shaded.color[0]);
                        assert_eq!(color, ram[at..at + 3], "{name}");
                        assert_eq!(shaded.uv, corner.uv);
                        tinted += usize::from(color[0] != color[2]);
                        corners += 1;
                    }
                }
            }
        }
    }
    assert!(tinted > 100);
    eprintln!("{corners} native corners checked against the retail palette; {tinted} tinted");
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; the training cutscenes replace the closed gameplay hatch with its scripted model"]
fn training_trapdoor_opens_without_duplicate_scenery() {
    use super::story::Story;
    use tenchu_assets::tenchu2::level::{Level, objects::StaticKind};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let has_closed_hatch = |level: &Level| {
        level.scene.statics.iter().any(|object| {
            matches!(object.kind, StaticKind::Model(_)) && object.position == [29000, -9000, 26500]
        })
    };
    assert!(has_closed_hatch(&Level::load(&disc, 0).unwrap()));
    for (sequence, event, closed, opened, angle) in
        [(2, 2021, 504, 516, -838), (103, 2031, 365, 371, -935)]
    {
        for skips in [false, true] {
            let mut story = Story::load(&disc, sequence, event, 1).unwrap();
            assert!(!has_closed_hatch(&story.level), "event {event}");
            let hatch = story
                .resources
                .script
                .actors
                .iter()
                .position(|actor| story.resources.models[&actor.model].name == "ana")
                .unwrap();
            for tick in 0..=opened + 30 {
                story.step(!skips || tick % 3 == 0).unwrap();
                let rotation = story.state.actors[hatch].rotation[0];
                if tick <= closed {
                    assert_eq!(rotation, [0, 0, 0], "event {event}, tick {tick}");
                } else if tick >= opened {
                    assert_eq!(rotation, [angle, 0, 0], "event {event}, tick {tick}");
                }
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; live story state and facial textures continue after a snapshot, with complete XA decoding"]
fn native_story_session_restores_faces_camera_and_captions() {
    use super::story::Story;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for (sequence, event) in [(0, 201), (10, 4), (197, 111)] {
        let mut story = Story::load(&disc, sequence, event, 17).unwrap();
        for _ in 0..340 {
            story.step(true).unwrap();
        }
        let mut restored = Story::load(&disc, sequence, event, 17).unwrap();
        restored
            .restore(serde_json::from_value(serde_json::to_value(&story.state).unwrap()).unwrap());
        for (&id, model) in &story.resources.models {
            assert_eq!(
                model.vram.words, restored.resources.models[&id].vram.words,
                "event {event}, model {id}: restored expression"
            );
        }
        for frame in 0..180 {
            let draw = frame % 7 != 3;
            story.step(draw).unwrap();
            restored.step(draw).unwrap();
            assert_eq!(
                serde_json::to_value(&story.state).unwrap(),
                serde_json::to_value(&restored.state).unwrap()
            );
        }
        let clip = tenchu_assets::audio::stream::Clip::load(
            &disc,
            story.resources.audio.as_ref().unwrap(),
        )
        .unwrap();
        assert!(
            clip.frames
                .iter()
                .any(|f| f[0].unsigned_abs() > 256 || f[1].unsigned_abs() > 256)
        );
        eprintln!(
            "JP event {event}: snapshot continuation and {:.2}s XA audio for {:.2}s scene",
            clip.frames.len() as f64 / f64::from(clip.rate),
            f64::from(story.resources.script.duration) / 30.0
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; runs the native frame and all three campaign heroes"]
fn native_frame_controls_and_audio_continue_after_restoring_a_world() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layouts = Layout::available(&disc).unwrap();
    for hero in 0..3 {
        let sequence = layouts
            .iter()
            .find(|l| match hero {
                0 => l.sequence < 100,
                1 => (100..185).contains(&l.sequence),
                _ => l.sequence >= 185,
            })
            .unwrap()
            .sequence;
        let mut session = Session::load(&disc, sequence, 1).unwrap();
        let start = session.world.actors[0].actor.position;
        let mut moved = false;
        let mut sounds = 0;
        for tick in 0..180 {
            let held = match tick {
                0..30 => UP,
                30..60 => CROUCH | UP,
                60..70 => RIGHT,
                70 => JUMP,
                130 => ATTACK,
                _ => 0,
            };
            sounds += session
                .step(held)
                .unwrap_or_else(|e| panic!("hero {hero}, tick {tick}: {e}"))
                .len();
            moved |= session.world.actors[0].actor.position != start;
        }
        assert!(moved, "hero {hero} did not move");
        let mut restored = Session::load(&disc, sequence, 1).unwrap();
        restored.world =
            serde_json::from_slice(&serde_json::to_vec(&session.world).unwrap()).unwrap();
        restored
            .assets
            .resources
            .clips
            .clone_from(&session.assets.resources.clips);
        for tick in 0..120 {
            let held = if tick == 0 { ATTACK } else { 0 };
            assert_eq!(
                session
                    .step(held)
                    .unwrap()
                    .into_iter()
                    .map(|s| s.sound)
                    .collect::<Vec<_>>(),
                restored
                    .step(held)
                    .unwrap()
                    .into_iter()
                    .map(|s| s.sound)
                    .collect::<Vec<_>>(),
                "hero {hero} sound continuation {tick}"
            );
        }
        assert_eq!(
            serde_json::to_value(&session.world).unwrap(),
            serde_json::to_value(&restored.world).unwrap()
        );
        eprintln!(
            "JP hero {hero} sequence {sequence}: 300 native ticks, {sounds} sounds, pending {:?}",
            session.pending
        );
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; walks off the starting bank into the training pond without editing state"]
fn native_training_pond_can_be_entered_from_the_bank() {
    use tenchu_core::tenchu2::{effects::Effect, physics::IN_WATER};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = Session::load(&disc, 1, 1).unwrap();
    let mut wet = 0;
    let mut ripples = false;
    for (buttons, ticks) in [(0, 30), (RIGHT, 19), (UP, 70)] {
        for _ in 0..ticks {
            session.step(buttons).unwrap();
            wet += usize::from(session.world.actors[0].actor.flags & IN_WATER != 0);
            ripples |= session
                .world
                .effects
                .entries
                .iter()
                .any(|e| matches!(e, Effect::Ripple(_)));
        }
    }
    let player = &session.world.actors[0].actor;
    eprintln!(
        "Training pond: {:?}, flags {:x}, {wet} wet ticks, ripples {ripples}",
        player.position, player.flags
    );
    assert!(wet > 30 && ripples);
    assert_eq!(player.position.y, 4000);
    assert_ne!(player.flags & IN_WATER, 0);
    if let Some(directory) = std::env::var_os("TENCHU_MODEL_REVIEW") {
        let path = std::path::PathBuf::from(directory).join("jp-native-pond-entry.png");
        let graphics = crate::settings::Graphics {
            asset_pack: false,
            ..Default::default()
        };
        render::capture(
            &session,
            &crate::settings::Settings {
                graphics,
                ..Default::default()
            },
            &path,
        )
        .unwrap();
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC; enters real deep water through the complete native frame"]
fn native_swimming_uses_the_imported_water_surface() {
    use tenchu_core::tenchu2::{
        actor::State,
        terrain::{self, Cursor, Query},
    };
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let sequence = Layout::available(&disc)
        .unwrap()
        .into_iter()
        .find(|l| l.map == 0)
        .unwrap()
        .sequence;
    let mut session = Session::load(&disc, sequence, 1).unwrap();
    let terrain = &session.world.terrain;
    let mut cursor = Cursor::default();
    let mut water = None;
    for node in terrain
        .cells
        .iter()
        .flat_map(|c| &c.nodes)
        .filter(|n| n.attributes & 0xf00 == 0x200)
    {
        let [x1, z1, x2, z2] = node.bounds.map(i32::from);
        for x in [x1 + (x2 - x1) / 4, (x1 + x2) / 2, x2 - (x2 - x1) / 4] {
            for z in [z1 + (z2 - z1) / 4, (z1 + z2) / 2, z2 - (z2 - z1) / 4] {
                let p = Position::new(x * 10, i32::from(node.y) * 10, z * 10);
                if terrain::level(
                    terrain,
                    &mut cursor,
                    p.wrapping_sub(Position::new(0, 1000, 0)),
                    Query::Uncached,
                )
                .unwrap()
                    == Some(p.y)
                    && terrain::level(
                        terrain,
                        &mut cursor,
                        p.wrapping_add(Position::new(0, 10, 0)),
                        Query::Uncached,
                    )
                    .unwrap()
                    .is_some_and(|y| y - p.y > 1000)
                {
                    water = Some(p);
                    break;
                }
            }
            if water.is_some() {
                break;
            }
        }
        if water.is_some() {
            break;
        }
    }
    let water = water.expect("No deep water in the source map");
    let actor = &mut session.world.actors[0].actor;
    actor.position = water;
    actor.previous = water;
    actor.velocity = Default::default();
    session.world.publish(0, &session.assets);
    session.world.update_camera(&session.assets, true).unwrap();
    let mut entered = false;
    for _ in 0..60 {
        session.step(UP).unwrap();
        entered |= session.world.actors[0].actor.state == State::Swimming;
    }
    assert!(entered, "player never entered native swimming");
    assert!(session.world.actors[0].actor.health > 0);
    assert_ne!(session.world.actors[0].actor.position, water);
    eprintln!(
        "Native water at {water:?}: {:?}, {:?}",
        session.world.actors[0].actor.position, session.world.actors[0].actor.state
    );
}

#[test]
#[ignore = "requires TENCHU2_DISC; reaches a real exit and restores during the native end transition"]
fn native_mission_exit_stops_input_and_survives_restoring_mid_fade() {
    use tenchu_assets::tenchu2::campaign::Objective;
    use tenchu_core::tenchu2::mission::{Outcome, Phase};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = Session::load(&disc, 1, 1).unwrap();
    let Objective::Reach { region, .. } = session.world.mission.objective else {
        panic!("not an exit mission")
    };
    let actor = &mut session.world.actors[0].actor;
    actor.position = region.center.map(|v| i32::from(v) * 1000).into();
    actor.previous = actor.position;
    actor.velocity = Default::default();
    session.world.publish(0, &session.assets);
    session.world.update_camera(&session.assets, true).unwrap();
    for _ in 0..60 {
        session.step(0).unwrap();
        if matches!(session.world.mission.phase, Phase::Ending { .. }) {
            break;
        }
    }
    assert!(
        matches!(
            session.world.mission.phase,
            Phase::Ending {
                outcome: Outcome::Arrived,
                remaining: 100,
                ..
            }
        ),
        "{:?}",
        session.world.mission.phase
    );
    for _ in 0..50 {
        session.step(UP | JUMP).unwrap();
    }
    assert_eq!(session.world.actors[0].actor.pad.held, 0);
    assert!(session.frame.transition_fade.is_some());
    let mut restored = Session::load(&disc, 1, 1).unwrap();
    restored.world = serde_json::from_slice(&serde_json::to_vec(&session.world).unwrap()).unwrap();
    restored
        .assets
        .resources
        .clips
        .clone_from(&session.assets.resources.clips);
    for _ in 0..60 {
        assert_eq!(
            session
                .step(UP)
                .unwrap()
                .into_iter()
                .map(|s| s.sound)
                .collect::<Vec<_>>(),
            restored
                .step(UP)
                .unwrap()
                .into_iter()
                .map(|s| s.sound)
                .collect::<Vec<_>>()
        );
        assert_eq!(session.world.mission.phase, restored.world.mission.phase);
    }
    assert!(matches!(
        session.world.mission.phase,
        Phase::Finished {
            outcome: Outcome::Arrived,
            ..
        }
    ));
    assert_eq!(
        serde_json::to_value(&session.world).unwrap(),
        serde_json::to_value(&restored.world).unwrap()
    );
    let tick = session.world.tick;
    session.step(ATTACK).unwrap();
    assert_eq!(session.world.tick, tick);
}

#[test]
#[ignore = "requires TENCHU2_DISC; completes a JP exit that directly enters results"]
fn native_results_follow_the_mission_and_preserve_campaign_rewards() {
    use tenchu_assets::tenchu2::campaign::{Campaign, Objective};
    use tenchu_core::tenchu2::{
        campaign::{Destination, Run},
        mission::Phase,
    };
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let mut run = Run::new(&campaign, 1, 134, 2, 1).unwrap();
    assert!(run.prepare_area(&campaign).unwrap());
    run.carry.inventory[0] = 255;
    let mut session = Session::for_run(&disc, &run).unwrap();
    let Objective::Reach { region, .. } = session.world.mission.objective else {
        panic!("not an exit mission")
    };
    let actor = &mut session.world.actors[0].actor;
    actor.position = region.center.map(|v| i32::from(v) * 1000).into();
    actor.previous = actor.position;
    actor.velocity = Default::default();
    session.world.publish(0, &session.assets);
    session.world.update_camera(&session.assets, true).unwrap();
    for _ in 0..160 {
        session.step(0).unwrap();
        if matches!(session.world.mission.phase, Phase::Finished { .. }) {
            break;
        }
    }
    assert_eq!(
        run.finish_area(&session.world, &campaign).unwrap(),
        Destination::Sequence(135)
    );
    let result = run.enter_results(&campaign).unwrap().clone();
    assert_eq!(result.record.score, 400);
    assert_eq!(result.record.rank, 4);
    assert!(result.new_best);
    let mission = session.assets.campaign.result_mission(1, 135).unwrap();
    assert_eq!(run.progress.records[1][mission], result.record);
    assert_eq!(run.progress.stock[1].items[4], Some(11));
    let before = serde_json::to_value(&run).unwrap();
    run = serde_json::from_value(before.clone()).unwrap();
    run.enter_results(&campaign).unwrap();
    assert_eq!(serde_json::to_value(&run).unwrap(), before);
}

#[test]
#[ignore = "requires TENCHU2_DISC; carries actual world state through a story into its boss area, results and retry"]
fn native_campaign_carries_state_through_story_and_boss() {
    use tenchu_assets::tenchu2::campaign::Campaign;
    use tenchu_core::tenchu2::{
        campaign::Destination,
        mission::{Outcome, Phase},
        world::Statistics,
    };
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let campaign = Campaign::load(&disc).unwrap();
    let mut run = Run::new(&campaign, 0, 9, 2, 17).unwrap();
    assert!(run.prepare_area(&campaign).unwrap());
    run.carry.inventory[0] = 255;
    let mut session = Session::for_run(&disc, &run).unwrap();
    session.world.statistics = Statistics {
        killed: 6,
        stealth_kills: 5,
        combat_kills: 1,
        innocents: 0,
    };
    session.world.effort = 1234;
    session.world.ai.detections = 3;
    let actor = &mut session.world.actors[0].actor;
    actor.health = 61;
    actor.flags |= 0x140;
    actor.breath = 58;
    actor.inventory.counts[4] = 7;
    actor.inventory.resurrections = 2;
    session.world.mission.phase = Phase::Finished {
        outcome: Outcome::Arrived,
        finished_at: 3061,
    };
    assert_eq!(
        run.finish_area(&session.world, &campaign).unwrap(),
        Destination::Sequence(10)
    );
    let counts = run.summary.population;
    let mut story = super::story::Story::load(&disc, 10, 4, run.random.seed).unwrap();
    story.state.random = run.random;
    for _ in 0..120 {
        story.step(true).unwrap();
    }
    run.random = story.state.random;
    assert_eq!(
        run.advance(&campaign, false).unwrap(),
        Destination::Sequence(11)
    );
    assert!(
        !run.prepare_area(&campaign).unwrap(),
        "boss entry lost the loadout"
    );
    run = serde_json::from_value(serde_json::to_value(run).unwrap()).unwrap();
    let mut boss = Session::for_run(&disc, &run).unwrap();
    let actor = &boss.world.actors[0].actor;
    assert_eq!(actor.health, 61);
    assert_eq!(actor.breath, 58);
    assert!(actor.weapon_drawn());
    assert_eq!(actor.flags & 0x100, 0x100);
    assert_eq!(actor.inventory.amounts(), run.carry.inventory);
    assert_eq!(boss.world.statistics.stealth_kills, 5);
    assert_eq!(boss.world.ai.detections, 3);
    assert_eq!(boss.world.effort, 1234);
    assert_eq!(boss.world.mission.population.guards, counts.guards);
    assert_eq!(boss.world.mission.population.bosses, counts.bosses + 1);
    assert!(boss.world.random.calls > run.random.calls);
    boss.world.statistics.combat_kills += 1;
    boss.world.statistics.killed += 1;
    boss.world.actors[0].actor.inventory.counts[4] = 6;
    boss.world.mission.phase = Phase::Finished {
        outcome: Outcome::TargetsDefeated,
        finished_at: 119,
    };
    assert_eq!(
        run.finish_area(&boss.world, &campaign).unwrap(),
        Destination::Sequence(12)
    );
    assert_eq!(
        run.advance(&campaign, false).unwrap(),
        Destination::Sequence(13)
    );
    let result = run.enter_results(&campaign).unwrap();
    assert_eq!(result.summary.time_tenths, 1059);
    assert_eq!(result.summary.kills.stealth_kills, 5);
    assert_eq!(result.summary.kills.combat_kills, 2);
    assert_eq!(result.record.score, 420);
    assert_eq!(run.progress.stock[0].items[4], Some(17));
    let snapshot = serde_json::to_value(&run).unwrap();
    run.enter_results(&campaign).unwrap();
    assert_eq!(serde_json::to_value(&run).unwrap(), snapshot);
    assert_eq!(run.retry(&campaign).unwrap(), Destination::Sequence(9));
    assert!(run.prepare_area(&campaign).unwrap());
    assert_eq!(run.carry.health, 0);
    assert_eq!(run.summary.kills.killed, 0);
    assert_eq!(run.summary.time_tenths, 0);
    assert_eq!(run.progress.stock[0].items[4], Some(17));
}

pub(super) fn player_corpse(
    disc: &Disc<impl ReadAt>,
    sequence: usize,
    training_rules: bool,
) -> Session {
    use tenchu_core::tenchu2::actor::{State, Velocity};
    let mut session = Session::load(disc, sequence, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    if training_rules {
        session.world.map = 0;
    }
    let player = session.world.player;
    let actor = &mut session.world.actors[player].actor;
    actor.health = 0;
    actor.state = State::Dead;
    assert!(
        actor
            .change_motion(0xb01, Velocity::Motion, &session.assets.resources)
            .unwrap()
            .is_some()
    );
    actor.motion.frame = 0;
    actor.motion.loops = 0;
    session.world.publish(player, &session.assets);
    let mut first = None;
    for _ in 0..300 {
        session.step(0).unwrap();
        let saved = postcard::to_allocvec(&session.world).unwrap();
        session.contact.record_player(&session.world);
        assert!(
            postcard::to_allocvec(&session.world).unwrap() == saved,
            "corpse presentation mutated the native world"
        );
        if let Some(corpse) = session.contact.player_corpse {
            assert_eq!(
                session.world.actors[player].actor.motion.loops,
                i16::MIN,
                "player pool appeared during the death animation"
            );
            assert_eq!(corpse.source_pool, !matches!(session.world.map, 0 | 12));
            let start = *first.get_or_insert(corpse.tick);
            assert_eq!(
                start, corpse.tick,
                "paused corpse acquisition restarted its growth"
            );
            if u64::from(session.world.tick) >= start + 40 {
                break;
            }
        } else {
            assert_ne!(session.world.actors[player].actor.motion.loops, i16::MIN);
        }
    }
    let corpse = session
        .contact
        .player_corpse
        .expect("native death never reached a settled corpse");
    assert!(u64::from(session.world.tick) >= corpse.tick + 40);
    session
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; all three heroes supply read-only settled corpse facts and exclude underwater/falling bodies"]
fn native_player_pools_wait_for_source_death_and_ignore_falls_and_water() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for sequence in [9, 119, 191] {
        let mut session = player_corpse(&disc, sequence, false);
        let player = session.world.player;
        let corpse = session.contact.player_corpse.unwrap();
        let mut scene = session.world.scene.clone();
        let torso = scene
            .world(session.world.instances[player].rig.joints[1])
            .unwrap()
            .translation;
        assert_eq!([corpse.position.x, corpse.position.z], [torso.x, torso.z]);
        session.world.actors[player].actor.velocity.y = 10;
        session.contact.record_player(&session.world);
        assert!(session.contact.player_corpse.is_none());
        session.world.actors[player].actor.velocity.y = 0;
        session.world.actors[player].actor.motion.action = 0xb08;
        session.contact.record_player(&session.world);
        assert!(session.contact.player_corpse.is_none());
        session.world.actors[player].actor.motion.action = 0xb00;
        session.world.actors[player].actor.health = 100;
        session.contact.record_player(&session.world);
        assert!(session.contact.player_corpse.is_none());
        session.world.actors[player].actor.health = 0;
        session.world.actors[player].actor.motion.action = 0xb01;
        session.world.map = 1;
        session.contact.record_player(&session.world);
        assert!(
            session.contact.player_corpse.unwrap().source_pool,
            "source-owned player pools must not be duplicated"
        );
    }
}
