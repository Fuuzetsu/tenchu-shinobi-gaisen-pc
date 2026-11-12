use super::*;

#[test]
#[ignore = "requires both discs, TENCHU_GAISEN_REVIEW and GPU; cached terrain matches full rebuilds through camera, phase, atmosphere and scene changes"]
fn cached_terrain_matches_rebuilt_frames() {
    use super::super::{Drawing, Models, Presentation, SceneDrawing};
    use crate::{renderer::Renderer, settings::Graphics, tenchu2::native::Session};

    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_GAISEN_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let mut transparent = 0;
    for stage in [Some(0), Some(7), None] {
        let mut session = if let Some(stage) = stage {
            let source = tenchu_assets::stage::MissionAssets::load(&gaisen, stage, 0, 1).unwrap();
            Session::gaisen(
                &disc,
                &source,
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
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        if stage.is_none() {
            // Include transparent stage faces even when the disc map is opaque.
            for face in session
                .assets
                .level
                .map
                .iter_mut()
                .take(2)
                .flat_map(|p| &mut p.geometry)
                .flat_map(|g| &mut g.faces)
            {
                face.translucent = true;
            }
        }
        let mut graphics = Graphics {
            fog: false,
            asset_pack: false,
            ..Default::default()
        };
        graphics.models.foliage = 4;
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let mut renderer = pollster::block_on(Renderer::new(
            &adapter,
            format,
            [640, 480],
            &session.assets.sprites.vram,
        ))
        .unwrap();
        renderer.configure(&graphics);
        let models =
            Models::new(&session.assets, &session.world, &graphics, &mut renderer).unwrap();
        let initial = || {
            models
                .geometry(
                    &session.world,
                    &session.assets,
                    &session.frame,
                    &graphics,
                    Presentation::default(),
                )
                .unwrap()
        };
        let mut cached = SceneDrawing::new(initial(), &mut renderer, &session.assets.level);
        let mut rebuilt = SceneDrawing::new(initial(), &mut renderer, &session.assets.level);
        let mut terrain = Terrain::default();
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Cached terrain comparison"),
            size: wgpu::Extent3d {
                width: 640,
                height: 480,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target = texture.create_view(&Default::default());
        for frame in 0..6 {
            match frame {
                2 => {
                    session.world.camera.view.translation.x += 2500;
                    session.world.camera.view.translation.z -= 4000;
                }
                3 => {
                    let index = session
                        .assets
                        .level
                        .scene
                        .statics
                        .iter()
                        .position(|s| {
                            matches!(
                                s.kind,
                                tenchu_assets::tenchu2::level::objects::StaticKind::Model(_)
                            )
                        })
                        .unwrap();
                    session.world.environment.static_phases[index] += 128;
                }
                4 => {
                    graphics.post.enabled = true;
                    graphics.post.fog_tint = 1;
                    renderer.configure(&graphics);
                }
                5 => {
                    terrain.invalidate();
                    cached.terrain = renderer.upload(&[]);
                }
                _ => {}
            }
            let state = postcard::to_allocvec(&session.world).unwrap();
            terrain.prepare(
                Static {
                    level: &session.assets.level,
                    meshes: &models.scenery,
                    math: &session.assets.resources.math,
                    phases: Some(&session.world.environment.static_phases),
                },
                session.world.camera.view,
                &mut renderer,
                &mut cached.terrain,
            );
            transparent += terrain.surfaces.len();
            let geometry = models
                .geometry(
                    &session.world,
                    &session.assets,
                    &session.frame,
                    &graphics,
                    Presentation {
                        terrain: Some(terrain.surfaces()),
                        actors: std::mem::take(&mut cached.actor_vertices),
                        effect_vertices: std::mem::take(&mut cached.effect_vertices),
                        ..Default::default()
                    },
                )
                .unwrap();
            assert!(geometry.terrain.is_empty());
            let reference = models
                .geometry(
                    &session.world,
                    &session.assets,
                    &session.frame,
                    &graphics,
                    Presentation::default(),
                )
                .unwrap();
            assert_eq!(
                bytemuck::cast_slice::<_, u8>(&geometry.effects.vertices),
                bytemuck::cast_slice::<_, u8>(&reference.effects.vertices),
                "transparent face ordering changed"
            );
            cached.update_dynamic(geometry, &mut renderer);
            rebuilt.update(reference, &mut renderer);
            let mut images = Vec::new();
            for (name, scene) in [("cached", &cached), ("rebuilt", &rebuilt)] {
                scene
                    .draw(session.world.camera.view, &target, &mut renderer)
                    .unwrap();
                let path = output.join(format!("terrain-{stage:?}-{frame}-{name}.png"));
                crate::capture::read_texture(&renderer, &texture, &path).unwrap();
                images.push(std::fs::read(path).unwrap());
            }
            assert!(images[0] == images[1], "stage {stage:?}, frame {frame}");
            assert!(state == postcard::to_allocvec(&session.world).unwrap());
        }
        // Switching source fog off again must replace the fogged terrain.
        let mut drawing = Drawing::new(&session, &graphics, &mut renderer).unwrap();
        drawing.update(&session, &mut renderer, None).unwrap();
        assert!(drawing.terrain.key.is_some());
        graphics.fog = true;
        renderer.configure(&graphics);
        drawing.update(&session, &mut renderer, None).unwrap();
        assert!(drawing.terrain.key.is_none());
        graphics.fog = false;
        renderer.configure(&graphics);
        drawing.update(&session, &mut renderer, None).unwrap();
        assert!(drawing.terrain.key.is_some());
    }
    assert!(
        transparent > 0,
        "fixtures must exercise transparent terrain"
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; checks dormant doors, live poses, removal and source-state independence"]
fn native_distant_scenery_preserves_poses_and_activation() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let layouts = tenchu_assets::tenchu2::layout::Layout::available(&disc).unwrap();
    let mut hidden = 0;
    for map in [0, 1, 4, 12, 15] {
        let sequence = layouts.iter().find(|l| l.map == map).unwrap().sequence;
        let mut session = super::super::super::Session::load(&disc, sequence, 1).unwrap();
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        let level = &session.assets.level;
        let before = postcard::to_allocvec(&session.world).unwrap();
        let models: Vec<_> = distant_objects(
            &session.world,
            level,
            &session.frame.scenery,
            &session.assets.resources.math,
        )
        .collect();
        hidden += models.len();
        assert!(postcard::to_allocvec(&session.world).unwrap() == before);
        for model in &models {
            let Source::Object(index) = model.source else {
                panic!("non-object scenery")
            };
            assert!(
                session
                    .world
                    .collision
                    .index(session.world.environment.objects[index].coordinate)
                    .is_none()
            );
            let mut moved = session.world.clone();
            let object = &mut moved.environment.objects[index];
            object.kind = -1;
            object.yaw = object.yaw.wrapping_add(512) & 4095;
            object.position.x += 500;
            let expected = (object.position, object.yaw);
            let pose = distant_objects(
                &moved,
                level,
                &session.frame.scenery,
                &session.assets.resources.math,
            )
            .find(|pose| pose.source == model.source)
            .unwrap();
            assert_eq!((pose.position, pose.rotation[1]), expected);
            moved.environment.objects[index].model = None;
            assert!(
                !distant_objects(
                    &moved,
                    level,
                    &session.frame.scenery,
                    &session.assets.resources.math
                )
                .any(|pose| pose.source == model.source)
            );
        }
        session.frame.scenery.models.extend(models);
        assert_eq!(
            distant_objects(
                &session.world,
                level,
                &session.frame.scenery,
                &session.assets.resources.math
            )
            .count(),
            0
        );
        assert!(postcard::to_allocvec(&session.world).unwrap() == before);
    }
    assert!(hidden > 0, "disc fixtures must contain dormant scenery");
}
