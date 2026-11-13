use super::*;
use crate::{settings::Settings, tenchu2::native::Session, visuals::appearance::library::Library};
use tenchu_core::tenchu2::{input, inventory::Item};

#[test]
#[ignore = "requires both discs, TENCHU_MODEL_REVIEW and GPU; invisibility blends T1/T2 bodies against matched backgrounds and leaves equipment unchanged"]
fn invisibility_blends_replacement_bodies_without_hiding_equipment() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let library = Library::load(&gaisen).unwrap();
    let mut donor = crate::tenchu2::native::models::donor(&gaisen).unwrap();
    let mut failures = Vec::new();
    let mut observations = Vec::new();
    for imported in [false, true] {
        for body in ["", "disc:0", "disc:1"] {
            let mut session = if imported {
                let mission = tenchu_assets::stage::MissionAssets::load(&gaisen, 0, 0, 1).unwrap();
                Session::gaisen(
                    &disc,
                    &mission,
                    tenchu_core::tenchu2::world::Options {
                        hero: 0,
                        difficulty: 2,
                        seed: 1,
                        inventory: Default::default(),
                    },
                    false,
                )
                .unwrap()
            } else {
                Session::load(&disc, 9, 1).unwrap()
            };
            if let Some(scene) = &mut session.gaisen {
                scene.director.source.events = Default::default();
            }
            for agent in session.world.actors.iter_mut().skip(1) {
                agent.actor.health = 0;
                agent.actor.state = tenchu_core::tenchu2::actor::State::Dead;
            }
            let player = session.world.player;
            session.world.actors[player].actor.rotation.y = 0;
            for _ in 0..30 {
                session.step(0).unwrap();
            }
            session
                .models
                .sync(
                    &mut session.world,
                    &mut session.assets,
                    &mut donor,
                    &library,
                    &Default::default(),
                    &Settings {
                        player_model: body.into(),
                        weapon_model: body.into(),
                        model_moveset: false,
                        ..Default::default()
                    },
                )
                .unwrap();
            let actor = &mut session.world.actors[player].actor;
            actor.inventory.counts[13] = 2;
            actor.inventory.selected = Some(Item::try_from(13).unwrap());
            for tick in 0..100 {
                session
                    .step(if tick == 0 { input::ITEM } else { 0 })
                    .unwrap();
            }
            assert!(session.world.instances[player].translucent);
            let name = format!(
                "{}-{}",
                if imported { "gaisen" } else { "native" },
                if body.is_empty() {
                    "t2-rikimaru".into()
                } else {
                    body.replace(':', "-")
                }
            );
            let definition = session
                .assets
                .resources
                .registry
                .actor(session.world.actors[player].actor.definition_kind())
                .unwrap();
            eprintln!(
                "{name}: model={} shade128={:?}",
                definition.model,
                session.assets.level.shading.color(128)
            );
            session.world.actors[player]
                .actor
                .set_weapon(
                    &mut session.world.instances[player].rig,
                    player,
                    &mut session.world.collision,
                    &session.assets.resources,
                    tenchu_core::tenchu2::rig::WeaponMode::Draw,
                )
                .unwrap();
            for artwork in [false, true] {
                let mut graphics = Graphics {
                    fog: false,
                    asset_pack: artwork,
                    ..Default::default()
                };
                graphics.set_filtering(if artwork { 2 } else { 0 });
                let output = if artwork {
                    output.join("filtered-artwork")
                } else {
                    output.clone()
                };
                std::fs::create_dir_all(&output).unwrap();
                let instance = wgpu::Instance::new(&Default::default());
                let adapter =
                    pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
                let format = wgpu::TextureFormat::Rgba8Unorm;
                let mut renderer = pollster::block_on(Renderer::new(
                    &adapter,
                    format,
                    [1280, 720],
                    &session.assets.sprites.vram,
                ))
                .unwrap();
                renderer.configure(&graphics);
                let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
                    label: Some("Invisibility comparison"),
                    size: wgpu::Extent3d {
                        width: 1280,
                        height: 720,
                        depth_or_array_layers: 1,
                    },
                    mip_level_count: 1,
                    sample_count: 1,
                    dimension: wgpu::TextureDimension::D2,
                    format,
                    usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
                    view_formats: &[],
                });
                let models =
                    Models::new(&session.assets, &session.world, &graphics, &mut renderer).unwrap();
                for concealed in [false, true] {
                    session.world.instances[player].translucent = concealed;
                    let state = postcard::to_allocvec(&session.world).unwrap();
                    // Keep pose and framing fixed while varying only body concealment.
                    let view = Matrix {
                        translation: Position::new(0, 1000, 6000),
                        ..Default::default()
                    };
                    for background in [32u8, 128] {
                        for equipment in [false, true] {
                            let path = output
                                .join(format!("{name}-{background}-{equipment}-{concealed}.png"));
                            let mut builder = Builder::new(view, &session.assets.level, &graphics);
                            let instance = &session.world.instances[player];
                            let origin = session.world.actors[player].actor.position;
                            models
                                .visit_actors(
                                    &session.world,
                                    &session.assets,
                                    &mut session.world.scene.clone(),
                                    None,
                                    |key, meshes, part, matrix, style| {
                                        let crate::visuals::interpolation::Part::Actor {
                                            root,
                                            model,
                                            ..
                                        } = key
                                        else {
                                            return;
                                        };
                                        if root != instance.rig.root
                                            || equipment == instance.rig.joints.contains(&model)
                                        {
                                            return;
                                        }
                                        builder.model(meshes, part, matrix, style);
                                    },
                                )
                                .unwrap();
                            for vertex in
                                builder.actors.vertices.iter_mut().chain(
                                    builder.surfaces.iter_mut().flat_map(|s| &mut s.vertices),
                                )
                            {
                                for axis in 0..3 {
                                    vertex.position[axis] -= origin[axis] as f32;
                                }
                            }
                            let mut backdrop = render_effects::quad(
                                [
                                    [-20000.0, -20000.0, 10000.0],
                                    [20000.0, -20000.0, 10000.0],
                                    [-20000.0, 20000.0, 10000.0],
                                    [20000.0, 20000.0, 10000.0],
                                ],
                                [[f32::from(background) / 255.0; 3]; 4],
                                None,
                                0,
                            );
                            for vertex in &mut backdrop {
                                vertex.flags = render_mesh::DOUBLE_SIDED;
                            }
                            let geometry = Geometry {
                                lamps: Vec::new(),
                                backgrounds: Vec::new(),
                                terrain: backdrop,
                                props: Vec::new(),
                                actors: builder.actors,
                                effects: render_effects::surface_geometry(builder.surfaces),
                            };
                            SceneDrawing::new(geometry, &mut renderer, &session.assets.level)
                                .draw(
                                    view,
                                    &texture.create_view(&Default::default()),
                                    &mut renderer,
                                )
                                .unwrap();
                            crate::capture::read_texture(&renderer, &texture, &path).unwrap();
                            let image =
                                crate::visuals::pack::decode_png(&std::fs::read(&path).unwrap())
                                    .unwrap();
                            let darkened = image
                                .rgba
                                .chunks_exact(4)
                                .filter(|p| {
                                    p[..3].iter().any(|&c| c.saturating_add(1) < background)
                                })
                                .count();
                            let changed = image
                                .rgba
                                .chunks_exact(4)
                                .filter(|p| p[..3] != [background; 3])
                                .count();
                            observations.push(serde_json::json!({"fixture": name, "background":background,
                            "equipment":equipment,"concealed":concealed,"darkened":darkened,"changed":changed,"artwork":artwork}));
                            assert!(
                                changed > 100,
                                "{name}/{equipment}/{concealed}: fixture is outside the image"
                            );
                            if concealed && !equipment && darkened != 0 {
                                failures.push(format!(
                                    "{name}/{background}: invisible body occludes {darkened} pixels"
                                ));
                            }
                            if concealed && equipment {
                                let previous = output
                                    .join(format!("{name}-{background}-{equipment}-false.png"));
                                if std::fs::read(previous).unwrap() != std::fs::read(path).unwrap()
                                {
                                    failures.push(format!(
                                        "{name}/{background}: concealment changes equipment"
                                    ));
                                }
                            }
                        }
                    }
                    assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
                }
            }
        }
    }
    std::fs::write(
        output.join("observations.json"),
        serde_json::to_vec_pretty(&observations).unwrap(),
    )
    .unwrap();
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
