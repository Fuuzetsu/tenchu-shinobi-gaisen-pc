use super::*;
use crate::tenchu2::native::Session;
use tenchu_core::tenchu2::{input, inventory::Item, items::State};

#[test]
#[ignore = "requires both discs, TENCHU_MODEL_REVIEW and GPU; controlled item activation, expiry and teleport on native and imported terrain"]
fn special_items_work_on_native_and_imported_terrain() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    let movies = std::env::var_os("TENCHU_ITEM_MOVIES").is_some();
    let graphics = Graphics {
        asset_pack: false,
        ..Default::default()
    };
    for imported in [false, true] {
        for (name, item, target) in [
            ("invisibility", 13, false),
            ("teleport-empty", 16, false),
            ("teleport-target", 16, true),
        ] {
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
            if target {
                for agent in session.world.actors.iter_mut().skip(1) {
                    agent.behavior.programs = Default::default();
                }
                let guard = session
                    .world
                    .actors
                    .iter()
                    .find(|a| (0x20..0x60).contains(&a.actor.kind))
                    .unwrap();
                let origin = guard.actor.position;
                let yaw = guard.actor.rotation.y;
                session.world.actors[session.world.player].actor.rotation.y = yaw;
                let mut placed = false;
                for radius in [3000, 2000, 5000, 6000] {
                    for angle in [0, 512, -512, 1024, -1024, 2048] {
                        let [sin, cos] =
                            session.assets.resources.math.pair(yaw.wrapping_add(angle));
                        let position = origin.wrapping_add(Position::new(
                            sin * radius / 4096,
                            0,
                            cos * radius / 4096,
                        ));
                        if session
                            .world
                            .teleport_player(position, true, &session.assets)
                            .is_ok()
                        {
                            placed = true;
                            break;
                        }
                    }
                    if placed {
                        break;
                    }
                }
                assert!(placed, "No free player position near the guard");
                // The source wakes distant guards as the player approaches. This
                // fixture jumps over that activation band to test the item itself.
                session.world.keep_all_active = true;
                session.step(0).unwrap();
                session.world.keep_all_active = false;
            } else {
                // Keep these isolated activation/expiry fixtures free of combat.
                for agent in session.world.actors.iter_mut().skip(1) {
                    agent.actor.health = 0;
                    agent.actor.state = tenchu_core::tenchu2::actor::State::Dead;
                }
            }
            for _ in 0..30 {
                session.step(0).unwrap();
            }
            let player = session.world.player;
            if target {
                let nearest = tenchu_core::tenchu2::items::teleport::nearest(
                    player,
                    &session.world.actors[player].actor,
                    session
                        .world
                        .actors
                        .iter()
                        .enumerate()
                        .map(|(i, a)| (i, &a.actor)),
                    player,
                    16000,
                    &session.assets.resources.math,
                );
                assert!(
                    nearest.is_some(),
                    "target fixture has no eligible guard: {:?}",
                    session
                        .world
                        .actors
                        .iter()
                        .map(|a| (a.actor.kind, a.actor.flags, a.actor.position, a.actor.state))
                        .collect::<Vec<_>>()
                );
            }
            session.world.actors[player].actor.inventory.counts[item] = 2;
            session.world.actors[player].actor.inventory.selected =
                Some(Item::try_from(item as u8).unwrap());
            let path = output.join(format!(
                "{}-{name}",
                if imported { "gaisen" } else { "native" }
            ));
            std::fs::create_dir_all(&path).unwrap();
            let instance = wgpu::Instance::new(&Default::default());
            let adapter =
                pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
            let format = wgpu::TextureFormat::Rgba8Unorm;
            let mut renderer = pollster::block_on(Renderer::new(
                &adapter,
                format,
                [960, 720],
                &session.assets.sprites.vram,
            ))
            .unwrap();
            renderer.configure(&graphics);
            let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
                label: Some("Special item review"),
                size: wgpu::Extent3d {
                    width: 960,
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
            let mut drawing = Drawing::new(&session, &graphics, &mut renderer).unwrap();
            let mut concealed = false;
            let mut teleported = false;
            let mut leaves = false;
            let mut trail = false;
            let mut observations = Vec::new();
            for tick in 0..if item == 13 { 950 } else { 181 } {
                let target_due = session.world.items.entries.iter().find_map(|e| {
                    let State::Teleport(t) = &e.state else {
                        return None;
                    };
                    (t.age == 95).then_some(t.target).flatten()
                });
                session
                    .step(if tick == 0 { input::ITEM } else { 0 })
                    .unwrap();
                concealed |= session.world.instances[player].translucent;
                trail |= !session.frame.effects.ribbons.is_empty();
                leaves |= session
                    .frame
                    .effects
                    .quads
                    .iter()
                    .any(|q| q.image == SurfaceImage::Item(64));
                if let Some(target) = target_due {
                    let actor = &session.world.actors[target].actor;
                    let [sin, cos] = session.assets.resources.math.pair(actor.rotation.y);
                    let destination =
                        actor
                            .position
                            .wrapping_sub(Position::new(-sin >> 2, 0, -cos >> 2));
                    assert_eq!(
                        session.world.actors[player].actor.position, destination,
                        "{imported}/{name}"
                    );
                    assert_eq!(session.world.camera.mode, 21);
                    teleported = true;
                }
                observations.push(serde_json::json!({
                    "tick": tick,
                    "concealed": session.world.instances[player].translucent,
                    "mode": session.world.script,
                    "items": session.world.items.entries,
                    "position": session.world.actors[player].actor.position,
                }));
                if tick <= 180 && movies || matches!(tick, 0 | 30 | 50 | 60 | 75 | 100 | 150 | 949)
                {
                    let state = postcard::to_allocvec(&session.world).unwrap();
                    drawing.update(&session, &mut renderer, None).unwrap();
                    drawing
                        .scene
                        .draw(
                            drawing.view,
                            &texture.create_view(&Default::default()),
                            &mut renderer,
                        )
                        .unwrap();
                    crate::capture::read_texture(
                        &renderer,
                        &texture,
                        &path.join(format!("{tick:04}.png")),
                    )
                    .unwrap();
                    assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
                }
            }
            std::fs::write(
                path.join("state.json"),
                serde_json::to_vec_pretty(&observations).unwrap(),
            )
            .unwrap();
            assert_eq!(
                session.world.actors[player].actor.inventory.counts[item], 1,
                "{imported}/{name}"
            );
            assert_eq!(session.world.script, 0);
            assert!(!session.world.instances[player].translucent);
            assert!(
                session.world.items.entries.iter().all(|entry| !matches!(
                    entry.state,
                    State::Invisibility(_) | State::Teleport(_)
                ))
            );
            assert_eq!(concealed, item == 13);
            if item == 13 {
                assert!(trail);
            }
            assert_eq!(teleported, target);
            assert_eq!(leaves, item == 16);
        }
    }
}
