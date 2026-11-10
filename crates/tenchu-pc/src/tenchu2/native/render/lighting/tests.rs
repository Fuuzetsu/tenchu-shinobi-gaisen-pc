use super::*;
use tenchu_core::{math::Position, tenchu2::effects::Emission};

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; checks native burn/explosion positions, expiry, switches and source independence"]
fn native_fire_lights_follow_source_particles_and_expire() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut session = crate::tenchu2::native::Session::load(&disc, 119, 1).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let player = session.world.player;
    let center = session.world.actors[player].actor.position;
    session
        .world
        .effects
        .burn(
            &session.world.instances[player].rig,
            2,
            24,
            &session.assets.effect_rules,
        )
        .unwrap();
    session
        .world
        .effects
        .explosion(
            center.wrapping_add(Position::new(10000, 0, 0)),
            0,
            &session.assets.resources.math,
            &mut session.world.random,
        )
        .unwrap();
    let mut lights = Lighting::new(&session.assets.level);
    let settings = crate::settings::Lighting {
        enabled: true,
        fires: true,
        lamps: false,
        ..Default::default()
    };
    let mut positions = Vec::new();
    let mut saw_explosion = false;
    let mut saw_embers = false;
    for _ in 0..90 {
        session.step(tenchu_core::tenchu2::input::UP).unwrap();
        let state = postcard::to_allocvec(&session.world).unwrap();
        lights.update_emitters(&session.frame.effects.emitters, &settings);
        assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
        let mut scene = session.world.scene.clone();
        let bones = session
            .assets
            .effect_rules
            .burn_bones
            .iter()
            .filter_map(|&bone| {
                session.world.instances[player]
                    .rig
                    .joints
                    .get(usize::from(bone))
                    .map(|&id| scene.world(id).unwrap().translation)
            })
            .collect::<Vec<_>>();
        for emitter in &session.frame.effects.emitters {
            if emitter.kind == Emission::BurningActor {
                assert!(
                    bones.contains(&emitter.position),
                    "burn light left the moving rig"
                );
            }
        }
        if let Some(source) = lights
            .flashes
            .iter()
            .find(|s| s.name == "Burning character")
        {
            positions.push(source.light.position);
            assert!(source.light.flame && source.light.emission_radius > 0.0);
        }
        saw_explosion |= lights.flashes.iter().any(|s| s.name == "Explosion");
        saw_embers |= lights.flashes.iter().any(|s| s.name == "Burning embers");
        assert!(
            lights
                .flashes
                .iter()
                .all(|s| s.light.position.is_finite() && s.light.radius > 0.0)
        );
        let without_fire = crate::settings::Lighting {
            fires: false,
            ..settings.clone()
        };
        assert!(lights.named_sources(&without_fire, Vec3::ZERO).is_empty());
        let disabled = crate::settings::Lighting {
            enabled: false,
            ..settings.clone()
        };
        assert!(lights.named_sources(&disabled, Vec3::ZERO).is_empty());
    }
    assert!(
        positions
            .windows(2)
            .any(|pair| pair[0].distance(pair[1]) > 10.0)
    );
    assert!(saw_explosion && saw_embers);
    assert!(lights.flashes.is_empty(), "light outlived its source flame");
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_LIGHT_REVIEW and GPU; compares fire lighting on/off and restores the same source image"]
fn native_fire_lights_illuminate_bodies_and_scenery() {
    use super::super::{Drawing, capture_frame};
    use crate::settings::Graphics;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_LIGHT_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    for kind in ["burn", "explosion"] {
        let mut session = crate::tenchu2::native::Session::load(&disc, 119, 1).unwrap();
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        let player = session.world.player;
        let position = session.world.actors[player].actor.position;
        if kind == "burn" {
            session
                .world
                .effects
                .burn(
                    &session.world.instances[player].rig,
                    0,
                    24,
                    &session.assets.effect_rules,
                )
                .unwrap();
        } else {
            session
                .world
                .effects
                .explosion(
                    position.wrapping_add(Position::new(900, 0, 0)),
                    0,
                    &session.assets.resources.math,
                    &mut session.world.random,
                )
                .unwrap();
        }
        session.step(0).unwrap();
        let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
        let center = Vec3::from(position.to_array().map(|v| v as f32)) - Vec3::Y * 1000.0;
        camera.position = center - camera.forward() * 5500.0;
        let state = postcard::to_allocvec(&session.world).unwrap();
        let mut images = Vec::new();
        for mode in ["off", "on", "restored"] {
            let mut graphics = Graphics {
                asset_pack: false,
                fog: false,
                ..Default::default()
            };
            graphics.lighting.enabled = true;
            graphics.lighting.shadows = true;
            graphics.lighting.fires = mode != "off";
            let path = output.join(format!("jp-fire-{kind}-{mode}.png"));
            capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                drawing.show_hud = false;
                drawing.camera = Some(camera);
                // Isolate transient light from the level's static braziers.
                drawing.scene.lighting.fires.clear();
                drawing.update(&session, renderer, None)?;
                assert_eq!(
                    drawing.scene.lighting.active(&graphics.lighting),
                    mode != "off"
                );
                if mode != "off" {
                    assert!(
                        drawing.scene.environment.is_some(),
                        "transient-only scene has no shadow geometry"
                    );
                    assert!(!drawing.scene.lighting.flashes.is_empty());
                }
                if mode == "restored" {
                    let mut original = graphics.clone();
                    original.lighting.fires = false;
                    renderer.configure(&original);
                    drawing.update(&session, renderer, None)?;
                    assert!(drawing.scene.lighting.flashes.is_empty());
                }
                assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
                Ok((drawing.scene, camera.source_view(), Vec::new()))
            })
            .unwrap();
            images.push(
                crate::visuals::pack::decode_png(&std::fs::read(path).unwrap())
                    .unwrap()
                    .rgba,
            );
        }
        let brighter = images[0]
            .chunks_exact(4)
            .zip(images[1].chunks_exact(4))
            .filter(|(off, on)| {
                u32::from(on[0]) + u32::from(on[1]) + u32::from(on[2])
                    > u32::from(off[0]) + u32::from(off[1]) + u32::from(off[2]) + 20
            })
            .count();
        assert!(
            brighter > 1000,
            "{kind} did not illuminate nearby surfaces: {brighter}"
        );
        assert_eq!(
            images[0], images[2],
            "fire toggle did not restore original pixels"
        );
    }
}
