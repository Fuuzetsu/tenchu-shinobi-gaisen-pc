use super::*;

mod mixed;

#[test]
#[ignore = "requires both discs; cross-game actor gallery preserves native definitions, source bodies and voice ownership"]
fn cross_game_spawn_catalog() {
    use tenchu_assets::game::GameAssets;
    let game = GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&game).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut session = Session::load(&disc, 9, 1).unwrap();
    session.world.keep_all_active = true;
    session.world.actors[0].actor.invulnerable = true;
    for _ in 0..60 {
        session.step(0).unwrap();
    }
    let original = session.world.clone();
    let position = original.actors[0].actor.position;
    let mut failures = Vec::new();
    let mut placed = 0;
    for body in &library.bodies {
        session.world = original.clone();
        session.actor_voices.clear();
        let result = session.spawn_actor(
            &disc,
            Request {
                source: Source::Model(body.id.clone()),
                position,
                yaw: 2048,
                snap: true,
            },
            Some(&library),
        );
        let id = match result {
            Ok(id) => id,
            Err(error) => {
                eprintln!(
                    "unavailable {} ({} joints, {:?}): {error}",
                    body.id,
                    body.parts.len(),
                    body.character.as_ref().map(|c| c.kind)
                );
                assert!(
                    postcard::to_allocvec(&session.world).unwrap()
                        == postcard::to_allocvec(&original).unwrap()
                );
                failures.push((body.id.clone(), error));
                continue;
            }
        };
        placed += 1;
        let actor = &session.world.actors[id].actor;
        assert!(actor.definition_kind() < -1);
        let original_definition = session.assets.resources.registry.actor(actor.kind).unwrap();
        let imported = session
            .assets
            .resources
            .registry
            .actor(actor.definition_kind())
            .unwrap();
        assert_eq!(original_definition.motions, imported.motions);
        assert_eq!(original_definition.weapon, imported.weapon);
        assert_eq!(session.world.mission.targets, original.mission.targets);
        let rig = &session.assets.models[&imported.model];
        if let Some(native) = &body.native {
            assert!(rig.parts == native.rig.parts);
        }
        let name = imported.model.clone();
        for tick in 0..180 {
            session
                .step(0)
                .unwrap_or_else(|e| panic!("{} tick {tick}: {e}", body.id));
        }
        eprintln!(
            "placed {}: {} source joints, native {:#x}, {} native joints, action {:#x}",
            body.id,
            body.parts.len(),
            session.world.actors[id].actor.kind,
            session.assets.models[&name].parts.len(),
            session.world.actors[id].actor.motion.action
        );
    }
    eprintln!(
        "placed {placed}, {} unavailable: {failures:?}",
        failures.len()
    );
    assert!(placed >= 100);
    assert!(
        failures.iter().all(|(id, reason)| {
            let missing = id == "resource:HUMAN/KARASU.MAD"
                || ["tenchu2:", "tenchu2-jp:"].iter().any(|prefix| {
                    id.strip_prefix(prefix)
                        .is_some_and(|name| matches!(name, "crow" | "horse" | "taka"))
                });
            missing && reason.contains("disc omits animation clips")
                || id == "disc:168" && reason.contains("projectile model")
                || matches!(id.as_str(), "disc:166" | "disc:167")
                    && reason.contains("scripted scenery model")
        }),
        "unexpected unavailable actors: {failures:?}"
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; F1 placement refreshes rendering and retry restores authored actors"]
fn native_spawn_menu_render_and_retry() {
    use super::super::{campaign::Action, preferences::Menu, presentation::ContentDrawing};
    use crate::{renderer::Renderer, settings::Settings};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut playback = Playback::load(disc, 9, 2, 1, Some([0; 21]), None).unwrap();
    for _ in 0..60 {
        playback.step(0).unwrap();
    }
    let mut menu = Menu::default();
    menu.tools.sync(&playback.content, playback.generation);
    let Content::Gameplay(session) = &playback.content else {
        unreachable!()
    };
    let count = session.world.actors.len();
    let tick = session.world.tick;
    let position = session.world.actors[0].actor.position;
    let generation = playback.generation;
    menu.tools.debug.camera = Some(crate::debug::camera::Camera {
        position: glam::Vec3::from(position.to_array().map(|v| v as f32)) - glam::Vec3::Y * 5000.0,
        yaw: 0.0,
        pitch: -1.5,
        fov: 50.0,
    });
    assert_eq!(menu.tools.actor_models.len(), 130);
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let size = [960, 540];
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer =
        pollster::block_on(Renderer::new(&adapter, format, size, playback.vram())).unwrap();
    let mut settings = Settings::default();
    settings.graphics.asset_pack = false;
    let graphics = super::super::preferences::graphics(&settings);
    renderer.configure(&graphics);
    let mut drawing = ContentDrawing::new(&playback.content, &graphics, &mut renderer).unwrap();
    let mut images = Vec::new();
    for mode in ["before", "placed", "removed"] {
        if mode == "placed" {
            menu.apply_spawn(
                &mut playback,
                Request {
                    source: Source::Native(0xe0),
                    position: position.wrapping_add(Position::new(1200, 0, 0)),
                    yaw: 1024,
                    snap: true,
                },
                None,
            );
            assert!(menu.actor_assets_changed, "{}", menu.panel.notice);
            assert!(menu.tools.debug.actors_edited);
            assert_eq!(menu.tools.debug.selected_actor, count);
            assert_eq!(playback.generation, generation);
            drawing
                .replace(
                    &playback.content,
                    playback.vram(),
                    &graphics,
                    &mut renderer,
                    true,
                )
                .unwrap();
        } else if mode == "removed" {
            let Content::Gameplay(session) = &mut playback.content else {
                unreachable!()
            };
            session.world.remove_actor(count, &session.assets).unwrap();
            session.reset_interpolation().unwrap();
        }
        let Content::Gameplay(session) = &playback.content else {
            unreachable!()
        };
        assert_eq!(session.world.tick, tick);
        let before = postcard::to_allocvec(&session.world).unwrap();
        drawing
            .update(
                &playback.content,
                &mut renderer,
                &menu.tools.debug,
                &settings,
            )
            .unwrap();
        let path = output.join(format!("spawn-{mode}.png"));
        crate::capture::render_to_png(&mut renderer, size, format, &path, |renderer, target| {
            drawing.draw(&playback.content, target, renderer)
        })
        .unwrap();
        assert!(postcard::to_allocvec(&session.world).unwrap() == before);
        images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
    }
    assert!(
        images[0].rgba != images[1].rgba,
        "placed actor did not render"
    );
    assert!(
        images[0].rgba == images[2].rgba,
        "removing the new actor did not restore the scene"
    );
    playback.content = Content::Failed;
    playback.action = Some(Action::Retry);
    playback.step(0).unwrap();
    let Content::Gameplay(session) = &playback.content else {
        unreachable!()
    };
    assert_eq!(session.world.actors.len(), count);
    assert!(session.actor_voices.is_empty());
    menu.tools.sync(&playback.content, playback.generation);
    assert!(!menu.tools.debug.actors_edited);
}

#[test]
#[ignore = "requires both discs; spawned bosses leave imported T1 story bindings and objectives intact for all heroes"]
fn native_spawn_in_imported_stages_keeps_story_bindings() {
    use super::super::gaisen;
    use tenchu_assets::script::Language;
    use tenchu_core::tenchu2::world::Options;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let path = std::env::var_os("TENCHU_DISC").unwrap();
    for hero in 0..3 {
        let source =
            gaisen::Source::open(path.as_ref(), 0, i16::from(hero == 1), 1, Language::English)
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
                4000,
            )
            .unwrap();
        session.step(0).unwrap();
        for _ in 0..6000 {
            if !session.cinematic() {
                break;
            }
            session.step(0).unwrap();
        }
        assert!(!session.cinematic());
        let director = &session.gaisen.as_ref().unwrap().director;
        let bindings: Vec<_> = director
            .source
            .roster
            .order
            .iter()
            .map(|&id| (id, director.native_actor(id)))
            .collect();
        let boss = bindings
            .iter()
            .filter_map(|&(source, native)| native.map(|native| (source, native)))
            .find(|&(source, _)| director.source.roster.get(source).kind & 0xf0 == 0x80)
            .unwrap();
        let before = session.world.actors[boss.1].actor.health;
        let source_health = director.source.roster.get(boss.0).health;
        let position = session.world.actors[0].actor.position;
        let kind = session.world.actors[boss.1].actor.kind;
        let id = session
            .spawn_actor(
                &disc,
                Request {
                    source: Source::Native(kind),
                    position,
                    yaw: 0,
                    snap: true,
                },
                None,
            )
            .unwrap();
        session.world.kill_actor(id, &session.assets).unwrap();
        let gaisen = session.gaisen.as_mut().unwrap();
        gaisen
            .director
            .update(&mut session.world, &session.assets, 0)
            .unwrap();
        for (source, native) in bindings {
            assert_eq!(gaisen.director.native_actor(source), native);
        }
        assert_eq!(session.world.actors[boss.1].actor.health, before);
        assert_eq!(
            gaisen.director.source.roster.get(boss.0).health,
            source_health
        );
        assert!(session.world.actors[id].slot < 0);
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; extra voices retain source banks and independent NPC bark timing"]
fn native_spawn_voice_banks_are_independent_of_stage_slots() {
    use super::super::{Arc, BTreeSet, Event, EventSource, Events, Output};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = Session::load(&disc, 9, 1).unwrap();
    let kinds: Vec<_> = session
        .assets
        .resources
        .registry
        .actors
        .iter()
        .filter(|a| (0x20..0x60).contains(&a.kind) && !session.assets.cast.contains(&a.kind))
        .map(|a| a.kind)
        .take(2)
        .collect();
    let voices = voices_for(&disc, &session.assets.resources.registry, &kinds).unwrap();
    let position = session.world.actors[0].actor.position;
    let mut ids = Vec::new();
    for &kind in &kinds {
        ids.push(
            session
                .spawn_actor(
                    &disc,
                    Request {
                        source: Source::Native(kind),
                        position,
                        yaw: 0,
                        snap: true,
                    },
                    None,
                )
                .unwrap(),
        );
    }
    let mut pending = BTreeSet::new();
    let mut output = Output {
        player: session.world.player,
        feedback: &mut session.feedback,
        voice: None,
        weapon_voice: None,
        actor_voices: &session.actor_voices,
        listener: session.world.listener(),
        requests: Vec::new(),
        wounds: Vec::new(),
        melee_hits: BTreeSet::new(),
        pending: &mut pending,
    };
    let random = session.world.random;
    for (&kind, &id) in kinds.iter().zip(&ids) {
        for cue in [6, 10, 13, 15, 29] {
            output
                .emit(
                    EventSource {
                        listener: session.world.listener(),
                        voice: session.world.instances[id].voice,
                        index: id,
                        actor: &session.world.actors[id].actor,
                        rig: &session.world.instances[id].rig,
                        scene: &mut session.world.scene,
                    },
                    Event::Voice(cue),
                    &mut session.world.random,
                )
                .unwrap();
            let request = output.requests.pop().unwrap();
            if cue < 16 {
                assert_eq!(request.extra_voice, Some(kind));
                let (bank, actual) = request.source.unwrap();
                assert_eq!(actual, cue);
                assert!(Arc::ptr_eq(
                    &bank,
                    session.actor_voices[&id].as_ref().map(|v| &v.bank).unwrap()
                ));
                assert_eq!(bank.samples.len(), voices[&kind].bank.samples.len());
                assert!(!Arc::ptr_eq(&bank, &session.bank));
            } else {
                assert!(request.source.is_none() && request.extra_voice.is_none());
                assert_eq!(request.id, cue);
            }
        }
    }
    assert_eq!(random, session.world.random);
    let audio = &mut session.world.audio;
    assert!(audio.extra_voice_ready(kinds[0], 6, 60));
    assert!(audio.extra_voice_ready(kinds[1], 6, 60));
    assert!(!audio.extra_voice_ready(kinds[0], 7, 89));
    assert!(audio.extra_voice_ready(kinds[0], 7, 90));
    assert!(audio.extra_voice_ready(kinds[0], 13, 91));
    // A fresh original allocator receives exactly the same sound IDs/slots.
    let mut original = tenchu_core::tenchu2::audio::Audio::default();
    for tick in 100..160 {
        let id = if tick & 1 == 0 { 0x56 } else { 0x4d };
        assert_eq!(
            audio.play(id, 0, 127, tick, &session.assets.resources.math),
            original.play(id, 0, 127, tick, &session.assets.resources.math)
        );
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; invalid placements roll back, extra guards update quotas and bosses retain authored IDs"]
fn native_spawn_preserves_objectives_and_rejects_invalid_positions() {
    use tenchu_assets::tenchu2::campaign::{Campaign, Kind, Objective};
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = Session::load(&disc, 9, 1).unwrap();
    let position = session.world.actors[0].actor.position;
    let baseline = postcard::to_allocvec(&session.world).unwrap();
    for (kind, position) in [
        (0x30, Position::new(i32::MAX, 0, 0)),
        (0x73, position),
        (-1, position),
    ] {
        assert!(
            session
                .spawn_actor(
                    &disc,
                    Request {
                        source: Source::Native(kind),
                        position,
                        yaw: 0,
                        snap: true
                    },
                    None,
                )
                .is_err()
        );
        assert!(postcard::to_allocvec(&session.world).unwrap() == baseline);
    }
    session.world.mission.objective = Objective::EliminateAll;
    let guards = session.world.mission.population.guards;
    let id = session
        .spawn_actor(
            &disc,
            Request {
                source: Source::Native(0x30),
                position,
                yaw: 0,
                snap: true,
            },
            None,
        )
        .unwrap();
    assert_eq!(session.world.mission.population.guards, guards + 1);
    assert!(session.world.remove_actor(id, &session.assets).is_err());
    session.world.kill_actor(id, &session.assets).unwrap();
    assert_eq!(session.world.statistics.killed, 1);
    let campaign = Campaign::load(&disc).unwrap();
    let mut count = 0;
    for (sequence, entry) in campaign.sequences.iter().enumerate() {
        if !matches!(
            entry.kind,
            Kind::Gameplay(Objective::Defeat { .. } | Objective::Weaken { .. })
        ) {
            continue;
        }
        let mut session = Session::load(&disc, sequence, 1).unwrap();
        let targets = session.world.mission.targets;
        for target in targets
            .into_iter()
            .flatten()
            .filter(|&id| id != session.world.player)
            .collect::<Vec<_>>()
        {
            let position = session.world.actors[target].actor.position;
            let kind = session.world.actors[target].actor.kind;
            let extra = session
                .spawn_actor(
                    &disc,
                    Request {
                        source: Source::Native(kind),
                        position,
                        yaw: 0,
                        snap: false,
                    },
                    None,
                )
                .unwrap();
            session.world.kill_actor(extra, &session.assets).unwrap();
            assert_eq!(session.world.mission.targets, targets);
            assert!(session.world.actors[target].actor.health > 0);
            assert!(session.world.mission.input_allowed());
            count += 1;
        }
    }
    assert_eq!(count, 26);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; places the native catalog with original resources and exercises AI/death"]
fn native_spawn_catalog_uses_source_resources() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = Session::load(&disc, 9, 1).unwrap();
    session.world.actors[0].actor.invulnerable = true;
    session.world.keep_all_active = true;
    for _ in 0..60 {
        session.step(0).unwrap();
    }
    let baseline = session.world.clone();
    let position = baseline.actors[0].actor.position;
    let catalog = session.assets.resources.registry.actors.clone();
    let mut placed = 0;
    let mut unavailable = Vec::new();
    for definition in catalog {
        session.world = baseline.clone();
        session.actor_voices.clear();
        let kind = definition.kind;
        let id = match session.spawn_actor(
            &disc,
            Request {
                source: Source::Native(kind),
                position,
                yaw: 2048,
                snap: true,
            },
            None,
        ) {
            Ok(id) => id,
            Err(error) if error.contains("disc omits animation clips") => {
                eprintln!("unavailable {kind:#x}: {error}");
                unavailable.push(kind);
                assert!(
                    serde_json::to_vec(&session.world).unwrap()
                        == serde_json::to_vec(&baseline).unwrap()
                );
                continue;
            }
            Err(error) => panic!("spawn {kind:#x} {}: {error}", definition.model),
        };
        placed += 1;
        assert_eq!(id, baseline.actors.len());
        assert_eq!(session.world.tick, baseline.tick);
        assert_eq!(session.world.mission.targets, baseline.mission.targets);
        assert!(
            session
                .world
                .actors
                .iter()
                .take(id)
                .zip(&baseline.actors)
                .all(|(a, b)| a.slot == b.slot && a.actor.position == b.actor.position)
        );
        assert!(session.world.actors[id].slot < 0);
        assert_eq!(session.world.actors[id].actor.kind, kind);
        assert!(
            definition
                .motions
                .iter()
                .all(|m| session.assets.resources.clips.contains_key(&m.clip))
        );
        let mut sounds = 0;
        for tick in 0..180 {
            sounds += session
                .step(0)
                .unwrap_or_else(|e| panic!("kind {kind:#x} tick {tick}: {e}"))
                .len();
        }
        let motion = session.world.actors[id].actor.motion.action;
        let killed = match session.world.kill_actor(id, &session.assets) {
            Ok(()) => {
                for tick in 0..150 {
                    session
                        .step(0)
                        .unwrap_or_else(|e| panic!("death {kind:#x} tick {tick}: {e}"));
                }
                assert!(session.world.actors[id].actor.health <= 0);
                true
            }
            Err(error) => {
                assert!(
                    error.contains("no death animation")
                        || session.world.actors[id].actor.health <= 0,
                    "{kind:#x}: {error}"
                );
                false
            }
        };
        eprintln!(
            "placed {kind:#04x} {}: action {motion:#x}, {sounds} sounds, death {killed}",
            definition.model
        );
    }
    eprintln!("native catalog: {placed} placed, unavailable {unavailable:x?}");
    assert_eq!(placed, 127);
    unavailable.sort_unstable();
    assert_eq!(unavailable, [0x73, 0x74, 0x78]);
}
