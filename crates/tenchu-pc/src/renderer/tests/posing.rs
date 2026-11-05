use super::*;
use crate::visuals::{models::Models, motion::Motion, pack::Pack};

#[test]
#[ignore = "requires TENCHU_DISC and a compute-capable GPU; compare actual moving character poses, motion, cache replacements and CPU fallbacks"]
fn gpu_poses_preserve_moving_characters_and_cpu_fallbacks() {
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets =
        tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
            .unwrap();
    let mut worst_position = 0.0_f32;
    let mut worst_normal = 0.0_f32;
    let mut checked = 0;
    for character in 0..2 {
        let mut game = crate::session::Options {
            mission: 1,
            character,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        game.events = Default::default();
        game.roster.keep_all_active = true;
        let pack = Pack::from_env(&game.mission).unwrap();
        let (mut renderer, _) = fixture(&game.mission.vram);
        assert!(renderer.pose_frame().is_some());
        renderer.set_asset_pack(&pack);
        let mut graphics = crate::settings::Graphics::default();
        graphics.set_preset(crate::settings::presets::MODERN_HIGH);
        let mut actor = renderer.upload(&[]);
        for detail in [0, 2, 1] {
            graphics.models.characters = detail;
            let mut models = Models::build(&game.mission, &pack, &graphics).unwrap();
            renderer.bind_models(&mut models);
            let mut motion = Motion::default();
            let mut geometry = render_mesh::Indexed::default();
            let mut reference = render_mesh::Indexed::default();
            for tick in 0..90 {
                use tenchu_core::input::{CROSS, SQUARE, UP};
                let buttons = if tick < 30 {
                    UP
                } else if tick == 40 {
                    CROSS
                } else if tick == 70 {
                    SQUARE
                } else {
                    0
                };
                game.step(buttons).unwrap();
                motion.tick(&game, &models, &graphics.motion, None);
                models.motion = motion.sample(0.75);
                if ![0, 1, 15, 44, 60, 89].contains(&tick) {
                    continue;
                }
                geometry.poses = match tick {
                    44 => Some(crate::visuals::posing::Frame::new(262144, 64)),
                    60 => None,
                    _ => renderer.pose_frame(),
                };
                render_mesh::update_indexed_characters(
                    &mut geometry,
                    &game,
                    Some(&models),
                    true,
                    None,
                );
                render_mesh::update_indexed_characters(
                    &mut reference,
                    &game,
                    Some(&models),
                    true,
                    None,
                );
                assert_eq!(geometry.indices, reference.indices);
                assert_eq!(geometry.vertices.len(), reference.vertices.len());
                for (a, b) in geometry.vertices.iter().zip(&reference.vertices) {
                    let mut attributes = *a;
                    attributes.position = b.position;
                    attributes.normal = b.normal;
                    assert_eq!(bytemuck::bytes_of(&attributes), bytemuck::bytes_of(b));
                }
                if let Some(frame) = &geometry.poses {
                    for group in &geometry.groups {
                        let bounds = frame.bounds(&geometry.vertices, group.vertices.clone());
                        for vertex in &reference.vertices[group.vertices.clone()] {
                            let position = glam::Vec3A::from(vertex.position);
                            if vertex.flags & render_mesh::SCENERY != 0 {
                                let [min, max] = bounds.scenery.unwrap();
                                assert!(position.cmpge(min).all() && position.cmple(max).all());
                            }
                            if vertex.flags
                                & (render_mesh::SCREEN
                                    | render_mesh::DEBUG
                                    | render_mesh::ALPHA
                                    | render_mesh::WATER)
                                == 0
                            {
                                let [min, max] = bounds.shadow.unwrap();
                                assert!(position.cmpge(min).all() && position.cmple(max).all());
                                assert!(vertex.flags & render_mesh::FOLIAGE == 0 || bounds.foliage);
                            }
                        }
                    }
                }
                if tick == 44 {
                    let parts = &geometry.poses.as_ref().unwrap().parts;
                    assert!(!parts.is_empty());
                    assert!(
                        parts.iter().map(|p| p.source.vertices.len()).sum::<usize>()
                            < geometry.vertices.len()
                    );
                }
                let size = (geometry.vertices.len() * size_of::<render_mesh::Pose>()) as u64;
                if actor.vertices.pose.size() < size
                    || !actor
                        .vertices
                        .pose
                        .usage()
                        .contains(wgpu::BufferUsages::COPY_SRC)
                {
                    actor.vertices.pose = renderer.device.create_buffer(&wgpu::BufferDescriptor {
                        label: Some("Pose comparison output"),
                        size: size.next_power_of_two(),
                        usage: wgpu::BufferUsages::VERTEX
                            | wgpu::BufferUsages::STORAGE
                            | wgpu::BufferUsages::COPY_SRC
                            | wgpu::BufferUsages::COPY_DST,
                        mapped_at_creation: false,
                    });
                }
                renderer.update_indexed_mesh(&mut actor, &mut geometry);
                let mut encoder = renderer.device.create_command_encoder(&Default::default());
                if let Some(posed) = &actor.posed {
                    posed.draw(&mut encoder);
                    posed.draw(&mut encoder);
                } else {
                    assert_eq!(tick, 60);
                }
                let readback = renderer.device.create_buffer(&wgpu::BufferDescriptor {
                    label: Some("Pose comparison readback"),
                    size,
                    usage: wgpu::BufferUsages::COPY_DST | wgpu::BufferUsages::MAP_READ,
                    mapped_at_creation: false,
                });
                encoder.copy_buffer_to_buffer(&actor.vertices.pose, 0, &readback, 0, size);
                renderer.queue.submit([encoder.finish()]);
                let (tx, rx) = std::sync::mpsc::channel();
                readback
                    .slice(..)
                    .map_async(wgpu::MapMode::Read, move |result| tx.send(result).unwrap());
                renderer.device.poll(wgpu::Maintain::Wait);
                rx.recv().unwrap().unwrap();
                let bytes = readback.slice(..).get_mapped_range();
                let poses: &[[f32; 6]] = bytemuck::cast_slice(&bytes);
                for (actual, expected) in poses.iter().zip(&reference.vertices) {
                    for (i, value) in actual.iter().enumerate() {
                        assert!(value.is_finite());
                        let error = if i < 3 {
                            let error = (value - expected.position[i]).abs();
                            worst_position = worst_position.max(error);
                            error / 0.02
                        } else {
                            let error = (value - expected.normal[i - 3]).abs();
                            worst_normal = worst_normal.max(error);
                            error / 0.00001
                        };
                        assert!(
                            error <= 1.0,
                            "character {character}, detail {detail}, tick {tick}, component {i}: {actual:?} versus {:?}/{:?}",
                            expected.position,
                            expected.normal
                        );
                    }
                }
                checked += poses.len();
            }
        }
    }
    eprintln!(
        "Compared {checked} posed vertices; maximum position error {worst_position}, normal error {worst_normal}"
    );
}
