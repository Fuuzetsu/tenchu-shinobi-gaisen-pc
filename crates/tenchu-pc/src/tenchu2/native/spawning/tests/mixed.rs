use super::*;
use crate::{
    renderer::Renderer,
    settings::{Settings, presets},
    tenchu2::native::{campaign::Action, models, preferences::Menu, presentation::ContentDrawing},
};
use std::sync::Arc;

#[test]
#[ignore = "requires both discs, TENCHU_MODEL_REVIEW and GPU; imported NPC rigs survive player swaps, rendering, edits and retry"]
fn cross_game_spawn_bodies_render_and_survive_player_swaps() {
    let game = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut library = Library::load(&game).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = models::donor(&game).unwrap();
    let mut playback = Playback::load(disc, 9, 2, 1, Some([0; 21]), None).unwrap();
    for _ in 0..60 {
        playback.step(0).unwrap();
    }
    let Content::Gameplay(session) = &mut playback.content else {
        unreachable!()
    };
    session.world.actors[0].actor.invulnerable = true;
    session.world.keep_all_active = true;
    let original = session.world.clone();
    let position = original.actors[0]
        .actor
        .position
        .wrapping_add(Position::new(1200, 0, 0));
    let mut menu = Menu::default();
    menu.tools.sync(&playback.content, playback.generation);
    let mut camera = crate::debug::camera::Camera {
        position: glam::Vec3::ZERO,
        yaw: -0.35,
        pitch: -0.4,
        fov: 50.0,
    };
    camera.position = glam::Vec3::from(position.to_array().map(|v| v as f32))
        - glam::Vec3::Y * 400.0
        - camera.forward() * 3600.0;
    menu.tools.debug.camera = Some(camera);
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let size = [960, 720];
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer =
        pollster::block_on(Renderer::new(&adapter, format, size, playback.vram())).unwrap();
    let mut settings = Settings::default();
    settings.graphics.asset_pack = false;
    let graphics = super::super::super::preferences::graphics(&settings);
    renderer.configure(&graphics);
    let mut drawing = ContentDrawing::new(&playback.content, &graphics, &mut renderer).unwrap();
    drawing
        .update(
            &playback.content,
            &mut renderer,
            &menu.tools.debug,
            &settings,
        )
        .unwrap();
    crate::capture::render_to_png(
        &mut renderer,
        size,
        format,
        &output.join("body-before.png"),
        |renderer, target| drawing.draw(&playback.content, target, renderer),
    )
    .unwrap();

    let mut choices = vec![
        "disc:16",
        "disc:162",
        "disc:161",
        "disc:5",
        "tenchu2-jp:tora",
    ];
    if library.get("tenchu2:kei").is_some() {
        choices.push("tenchu2:kei");
    }
    for choice in &choices {
        let Content::Gameplay(session) = &mut playback.content else {
            unreachable!()
        };
        session.world = original.clone();
        session.actor_voices.clear();
        menu.apply_spawn(
            &mut playback,
            Request {
                source: Source::Model((*choice).into()),
                position,
                yaw: 2048,
                snap: true,
            },
            Some(&library),
        );
        assert!(
            menu.actor_assets_changed,
            "{}: {}",
            choice, menu.panel.notice
        );
        let Content::Gameplay(session) = &mut playback.content else {
            unreachable!()
        };
        assert_eq!(session.world.actors.len(), original.actors.len() + 1);
        let id = menu.tools.debug.selected_actor;
        let actor = &session.world.actors[id].actor;
        if actor.max_health == 0 {
            assert_eq!(actor.health, 0);
            session
                .world
                .reposition_actor(id, position, 2048, true, &session.assets)
                .unwrap();
            assert_eq!(session.world.actors[id].actor.health, 0);
            session.reset_interpolation().unwrap();
        }
        let before = postcard::to_allocvec(&session.world).unwrap();
        for preset in [0, presets::MODERN_HIGH, presets::MODERN_ULTRA] {
            settings.graphics.set_preset(preset);
            let graphics = super::super::super::preferences::graphics(&settings);
            renderer.configure(&graphics);
            drawing
                .replace(
                    &playback.content,
                    playback.vram(),
                    &graphics,
                    &mut renderer,
                    true,
                )
                .unwrap();
            drawing
                .update(
                    &playback.content,
                    &mut renderer,
                    &menu.tools.debug,
                    &settings,
                )
                .unwrap();
            let path = output.join(format!("body-{}-{preset}.png", choice.replace(':', "-")));
            crate::capture::render_to_png(
                &mut renderer,
                size,
                format,
                &path,
                |renderer, target| drawing.draw(&playback.content, target, renderer),
            )
            .unwrap();
            let Content::Gameplay(session) = &playback.content else {
                unreachable!()
            };
            assert!(postcard::to_allocvec(&session.world).unwrap() == before);
        }
    }

    let Content::Gameplay(session) = &mut playback.content else {
        unreachable!()
    };
    session.world = original.clone();
    session.actor_voices.clear();
    let mut placed = Vec::new();
    for choice in choices.iter().chain(std::iter::once(&"disc:16")) {
        let id = session
            .spawn_actor(
                &playback.disc,
                Request {
                    source: Source::Model((*choice).into()),
                    position,
                    yaw: 2048,
                    snap: true,
                },
                Some(&library),
            )
            .unwrap();
        let actor = &session.world.actors[id].actor;
        let definition = session
            .assets
            .resources
            .registry
            .actor(actor.definition_kind())
            .unwrap()
            .clone();
        let parts = session.assets.models[&definition.model].parts.clone();
        let voice = session.actor_voices[&id].clone();
        placed.push((id, actor.kind, definition, parts, voice));
    }
    assert_eq!(placed[0].2.kind, placed.last().unwrap().2.kind);
    for player in ["disc:1", "tenchu2-jp:ayame", "disc:0", ""] {
        let settings = Settings {
            player_model: player.into(),
            model_moveset: !player.is_empty(),
            ..Default::default()
        };
        session
            .models
            .sync(
                &mut session.world,
                &mut session.assets,
                &mut donor,
                &library,
                &Arc::default(),
                &settings,
            )
            .unwrap();
        session.reset_interpolation().unwrap();
        for (id, kind, definition, parts, voice) in &placed {
            let actor = &session.world.actors[*id].actor;
            assert_eq!(actor.kind, *kind);
            assert_eq!(actor.definition_kind(), definition.kind);
            let current = session
                .assets
                .resources
                .registry
                .actor(definition.kind)
                .unwrap();
            assert_eq!(current.motions, definition.motions);
            assert_eq!(current.weapon, definition.weapon);
            assert!(session.assets.models[&current.model].parts == *parts);
            for motion in &current.motions {
                assert!(session.assets.resources.clips.contains_key(&motion.clip));
            }
            match (voice, &session.actor_voices[id]) {
                (Some(before), Some(after)) => assert!(Arc::ptr_eq(before, after)),
                (None, None) => {}
                _ => panic!("NPC voice changed with the player"),
            }
        }
        for _ in 0..90 {
            session.step(0).unwrap();
        }
    }
    let id = placed[0].0;
    session.world.kill_actor(id, &session.assets).unwrap();
    assert!(
        session
            .world
            .reposition_actor(id, position, 0, true, &session.assets)
            .is_err()
    );
    for _ in 0..180 {
        session.step(0).unwrap();
    }
    session.world.remove_actor(id, &session.assets).unwrap();
    assert_eq!(session.world.mission.targets, original.mission.targets);
    playback.content = Content::Failed;
    playback.action = Some(Action::Retry);
    playback.step(0).unwrap();
    let Content::Gameplay(session) = &playback.content else {
        unreachable!()
    };
    assert_eq!(session.world.actors.len(), original.actors.len());
    assert!(session.actor_voices.is_empty());
    assert!(
        session
            .assets
            .resources
            .registry
            .actors
            .iter()
            .all(|a| a.kind >= 0)
    );
}
