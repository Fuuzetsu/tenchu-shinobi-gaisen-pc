use super::*;
use crate::{
    tenchu2::native::{
        Session,
        render::{Drawing, capture_frame},
    },
    visuals::{catalog::Role, vegetation},
};
use std::collections::BTreeSet;

fn graphics() -> Graphics {
    let mut graphics = Graphics {
        asset_pack: false,
        fog: false,
        ..Default::default()
    };
    graphics.models.foliage = 3;
    graphics.models.variation = true;
    graphics.effects.leaves = true;
    graphics.effects.leaf_density = 200;
    graphics.effects.scatter = true;
    graphics
}

fn output() -> std::path::PathBuf {
    let path = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&path).unwrap();
    path
}

fn camera(target: Vec3) -> crate::debug::camera::Camera {
    let position = target + Vec3::new(3500.0, -1500.0, -5500.0);
    let forward = (target - position).normalize();
    crate::debug::camera::Camera {
        position,
        yaw: forward.x.atan2(forward.z),
        pitch: (-forward.y).asin(),
        fov: (2.0_f32 * (120.0_f32 / 300.0).atan()).to_degrees(),
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; audits native foliage materials, geometry budgets and seasonal emitters on every map"]
fn native_vegetation_materials_and_geometry() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let rules = vegetation::rules(&Default::default(), false);
    let mut found = BTreeSet::new();
    let mut seasonal = BTreeSet::new();
    for map in Level::available(&disc).unwrap() {
        let level = Level::load(&disc, map).unwrap();
        let sources = || {
            let mut out =
                super::super::sources(&format!("map{map}/terrain"), &level.map, Role::Terrain);
            out.extend(super::super::sources(
                &format!("map{map}/props"),
                &level.objects,
                Role::Prop,
            ));
            out
        };
        let original = Meshes::build(
            &level.vram,
            &Default::default(),
            &Graphics {
                asset_pack: false,
                ..Default::default()
            },
            sources(),
            true,
        )
        .unwrap();
        let modern = Meshes::build(
            &level.vram,
            &Default::default(),
            &graphics(),
            sources(),
            true,
        )
        .unwrap();
        assert!(
            modern
                .materials()
                .iter()
                .zip(original.materials())
                .all(|(a, b)| a.id == b.id && a.image.rgba == b.image.rgba),
            "foliage changed displayed texture colors or blending in map {map}"
        );
        found.extend(
            modern
                .materials()
                .iter()
                .filter(|m| rules.contains_key(&m.id))
                .map(|m| m.id.clone()),
        );
        let mut counts = [0, 0];
        for part in level.map.iter().chain(&level.objects) {
            for (i, meshes) in [&original, &modern].into_iter().enumerate() {
                meshes.visit(
                    &part.geometry,
                    Matrix::default(),
                    false,
                    0,
                    None,
                    |triangle| {
                        counts[i] += 3;
                        if i == 0 {
                            assert_eq!(triangle[0].flags & render_mesh::FOLIAGE, 0);
                        }
                    },
                );
            }
            modern.visit_canopies(&part.geometry, Matrix::default(), |canopy| {
                assert!(canopy.min.is_finite() && canopy.max.is_finite());
                assert!(canopy.color.iter().any(|&v| v > 0.1));
                seasonal.insert(map);
            });
        }
        assert!(counts[1] <= counts[0] + 1_572_864, "map {map}: {counts:?}");
        eprintln!("native foliage map {map}: vertices {counts:?}");
    }
    assert!(found.len() >= 42, "classified materials: {}", found.len());
    assert!(
        seasonal.contains(&9),
        "autumn map has no emitters: {seasonal:?}"
    );
    eprintln!(
        "native foliage: {} classified materials; seasonal maps {seasonal:?}",
        found.len()
    );
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; seasonal particles and movement scatter tick independently of redraws and survive checkpoints"]
fn native_foliage_ticks_and_checkpoints() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let sequence = tenchu_assets::tenchu2::layout::Layout::available(&disc)
        .unwrap()
        .into_iter()
        .find(|layout| layout.map == 9)
        .unwrap()
        .sequence;
    let mut session = Session::load(&disc, sequence, 1).unwrap();
    for _ in 0..30 {
        session.step_world(0, true).unwrap();
    }
    let settings = graphics();
    let vram = session.assets.sprites.vram.clone();
    let out = output();
    let mut view = None;
    capture_frame(
        &vram,
        &settings,
        &out.join("native-autumn-after.png"),
        |renderer| {
            let mut drawing = Drawing::new(&session, &settings, renderer)?;
            let mut sparse = Drawing::new(&session, &settings, renderer)?;
            let environment = drawing.scene.environment.as_ref().unwrap();
            drawing.show_hud = false;
            sparse.show_hud = false;
            let mut walking = None;
            'trees: for canopy in &environment.contact.canopies {
                for x in [
                    canopy.min.x,
                    (canopy.min.x + canopy.max.x) * 0.5,
                    canopy.max.x,
                ] {
                    for z in [
                        canopy.min.z,
                        (canopy.min.z + canopy.max.z) * 0.5,
                        canopy.max.z,
                    ] {
                        let start = Vec3::new(x, canopy.min.y - 100.0, z);
                        let Some(hit) = environment.surfaces.cast_excluding(
                            start,
                            start + Vec3::Y * 50000.0,
                            render_mesh::FOLIAGE | render_mesh::WATER,
                        ) else {
                            continue;
                        };
                        if hit.normal.y > -0.8 {
                            continue;
                        }
                        let start = hit.position - Vec3::Y * 100.0;
                        if [start, start + Vec3::X * 910.0]
                            .into_iter()
                            .all(|position| {
                                session
                                    .world
                                    .teleport_player(
                                        position.as_ivec3().to_array().into(),
                                        true,
                                        &session.assets,
                                    )
                                    .is_ok()
                            })
                        {
                            walking = Some((
                                start,
                                (canopy.min + canopy.max) * 0.5,
                                (canopy.max - canopy.min).max_element(),
                            ));
                            break 'trees;
                        }
                    }
                }
            }
            let (start, center, span) =
                walking.expect("walkable ground beneath a native autumn tree");
            let mut camera = camera(center);
            camera.position = center + (camera.position - center) * (span / 4500.0).max(1.8);
            drawing.camera = Some(camera);
            sparse.camera = Some(camera);
            view = Some(camera);
            let mut scattered = false;
            for step in 0..180 {
                // A controlled movement fixture beneath the seasonal canopy.
                let position = start + Vec3::X * ((step % 14) as f32 * 70.0);
                session.world.teleport_player(
                    position.as_ivec3().to_array().into(),
                    true,
                    &session.assets,
                )?;
                session.world.tick += 1;
                let source = postcard::to_allocvec(&session.world).unwrap();
                drawing.tick(&session, &settings)?;
                sparse.tick(&session, &settings)?;
                let particles =
                    serde_json::to_value(&drawing.scene.environment.as_ref().unwrap().contact)
                        .unwrap();
                scattered |= particles["particles"].as_array().unwrap().iter().any(|p| {
                    p["age"].as_f64() == Some(0.0)
                        && p["position"][1].as_f64().unwrap() > f64::from(start.y - 100.0)
                });
                if step % 3 == 0 {
                    drawing.update(&session, renderer, None)?;
                }
                assert_eq!(postcard::to_allocvec(&session.world).unwrap(), source);
                assert!(
                    postcard::to_allocvec(
                        &drawing.scene.environment.as_ref().unwrap().capture_state()
                    )
                    .unwrap()
                        == postcard::to_allocvec(
                            &sparse.scene.environment.as_ref().unwrap().capture_state()
                        )
                        .unwrap(),
                    "redraw changed environment at {step}"
                );
                if step == 90 {
                    let state = drawing.scene.environment.as_ref().unwrap().capture_state();
                    let mut recorder = crate::rewind::Recorder::default();
                    recorder.begin();
                    state.capture(&mut recorder).unwrap();
                    let checkpoint = recorder.finish().unwrap();
                    assert!(checkpoint.bytes().unwrap() == postcard::to_allocvec(&state).unwrap());
                    sparse
                        .scene
                        .environment
                        .as_mut()
                        .unwrap()
                        .restore_state(checkpoint.decode().unwrap());
                }
            }
            assert!(scattered, "movement never scattered leaves from the ground");
            let environment = drawing.scene.environment.as_ref().unwrap();
            let contact = serde_json::to_value(&environment.contact).unwrap();
            assert!(!contact["steps"]["actors"].as_object().unwrap().is_empty());
            assert!(contact["particles"].as_array().unwrap().len() > 10);
            assert!(
                contact["particles"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .any(|p| p["ground"].is_array()),
                "no leaves reached the ground"
            );
            let paused = postcard::to_allocvec(&environment.capture_state()).unwrap();
            for _ in 0..4 {
                drawing.update(&session, renderer, None)?;
            }
            assert!(
                paused
                    == postcard::to_allocvec(
                        &drawing.scene.environment.as_ref().unwrap().capture_state()
                    )
                    .unwrap()
            );
            let mut rebuilt = Drawing::new(&session, &settings, renderer)?;
            rebuilt.camera = Some(camera);
            rebuilt.show_hud = false;
            rebuilt.preserve_environment(&mut drawing);
            rebuilt.update(&session, renderer, None)?;
            assert!(
                paused
                    == postcard::to_allocvec(
                        &rebuilt.scene.environment.as_ref().unwrap().capture_state()
                    )
                    .unwrap()
            );
            Ok((rebuilt.scene, camera.source_view(), Vec::new()))
        },
    )
    .unwrap();
    let original = Graphics {
        asset_pack: false,
        fog: false,
        ..Default::default()
    };
    capture_frame(
        &vram,
        &original,
        &out.join("native-autumn-before.png"),
        |renderer| {
            let mut drawing = Drawing::new(&session, &original, renderer)?;
            drawing.camera = view;
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            assert!(drawing.scene.environment.is_none());
            Ok((drawing.scene, view.unwrap().source_view(), Vec::new()))
        },
    )
    .unwrap();
}

#[test]
#[ignore = "requires both discs, TENCHU_MODEL_REVIEW and GPU; native feet leave tracks on original T1 snow without custom weather"]
fn native_baked_snow_tracks() {
    let source = tenchu_assets::game::GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let id = source
        .stages()
        .unwrap()
        .into_iter()
        .find(|stage| stage.order == 5)
        .unwrap()
        .id;
    let mission = tenchu_assets::stage::MissionAssets::load(&source, id, 0, 1).unwrap();
    let mut session = Session::gaisen(
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
    .unwrap();
    for _ in 0..6000 {
        session.step(0).unwrap();
        if !session.cinematic() {
            break;
        }
    }
    assert!(!session.cinematic());
    let mut settings = Graphics {
        asset_pack: false,
        fog: false,
        ..Default::default()
    };
    settings.weather.baked_snow = true;
    let vram = session.assets.sprites.vram.clone();
    let out = output();
    let mut view = None;
    capture_frame(
        &vram,
        &settings,
        &out.join("native-baked-snow.png"),
        |renderer| {
            let mut drawing = Drawing::new(&session, &settings, renderer)?;
            let mut candidates = Vec::new();
            for part in &session.assets.level.map {
                drawing.models.scenery.visit(
                    &part.geometry,
                    Matrix {
                        translation: part.position.map(|v| i32::from(v) * 10).into(),
                        ..Default::default()
                    },
                    false,
                    0,
                    None,
                    |triangle| {
                        if triangle[0].flags & render_mesh::SNOW == 0 {
                            return;
                        }
                        let [a, b, c] = triangle.map(|v| Vec3::from(v.position));
                        let normal = (b - a).cross(c - a);
                        if normal.length() < 500000.0 || normal.normalize().y.abs() < 0.9 {
                            return;
                        }
                        let uv = std::array::from_fn(|i| {
                            (triangle.iter().map(|v| v.uv[i]).sum::<f32>() / 3.0) as u16
                        });
                        let color = tenchu_assets::texture::rgba8(session.assets.level.vram.texel(
                            triangle[0].page as u16,
                            triangle[0].palette as u16,
                            uv,
                        ));
                        if color[..3].iter().all(|&v| v >= 185) {
                            candidates.push((a + b + c) / 3.0);
                        }
                    },
                );
            }
            let start = candidates
                .into_iter()
                .find(|p| {
                    session
                        .world
                        .teleport_player(
                            ((*p - Vec3::Y * 100.0).as_ivec3().to_array()).into(),
                            true,
                            &session.assets,
                        )
                        .is_ok()
                })
                .expect("walkable original snow");
            drawing.clear_weather(true);
            for _ in 0..20 {
                session.step_world(tenchu_core::tenchu2::input::UP, true)?;
                drawing.tick(&session, &settings)?;
            }
            let position = Vec3::from(
                session.world.actors[session.world.player]
                    .actor
                    .position
                    .to_array()
                    .map(|v| v as f32),
            );
            let target = (start + position) * 0.5;
            let mut camera = camera(target);
            camera.position = target + Vec3::new(2200.0, -6500.0, -3500.0);
            let forward = (target - camera.position).normalize();
            camera.yaw = forward.x.atan2(forward.z);
            camera.pitch = (-forward.y).asin();
            view = Some(camera);
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            let environment = drawing.scene.environment.as_ref().unwrap();
            assert!(environment.surfaces.contains(render_mesh::SNOW));
            assert!(
                !environment
                    .footprints
                    .geometry(&settings.weather, None, 0.0)
                    .vertices
                    .is_empty()
            );
            let saved = postcard::to_allocvec(&environment.footprints).unwrap();
            for _ in 0..3 {
                drawing.update(&session, renderer, None)?;
            }
            assert!(
                saved
                    == postcard::to_allocvec(
                        &drawing.scene.environment.as_ref().unwrap().footprints
                    )
                    .unwrap()
            );
            drawing.clear_weather(true);
            drawing.update(&session, renderer, None)?;
            assert!(
                drawing
                    .scene
                    .environment
                    .as_ref()
                    .unwrap()
                    .footprints
                    .geometry(&settings.weather, None, 0.0)
                    .vertices
                    .is_empty()
            );
            drawing.scene.environment.as_mut().unwrap().footprints =
                postcard::from_bytes(&saved).unwrap();
            drawing.update(&session, renderer, None)?;
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        },
    )
    .unwrap();
    settings.weather.footprints = false;
    capture_frame(
        &vram,
        &settings,
        &out.join("native-baked-snow-before.png"),
        |renderer| {
            let mut drawing = Drawing::new(&session, &settings, renderer)?;
            drawing.camera = view;
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            Ok((drawing.scene, view.unwrap().source_view(), Vec::new()))
        },
    )
    .unwrap();
    let read =
        |name| crate::visuals::pack::decode_png(&std::fs::read(out.join(name)).unwrap()).unwrap();
    let before = read("native-baked-snow-before.png");
    let after = read("native-baked-snow.png");
    let changed = before
        .rgba
        .chunks_exact(4)
        .zip(after.rgba.chunks_exact(4))
        .filter(|(a, b)| a != b)
        .count();
    assert!(
        changed > 100,
        "tracks were generated but invisible: {changed} pixels changed"
    );
    eprintln!("native snow tracks change {changed} pixels with Original weather");
}
