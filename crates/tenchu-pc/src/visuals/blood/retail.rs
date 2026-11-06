use super::*;
use tenchu_core::effects::{Billboard, Graphic};

impl Blood {
    fn retain_spot(&mut self, sample: Sample, game: &Game, surfaces: &Receivers<'_>) {
        if self.settings.conform_retained {
            let start = sample.position - Vec3::Y * 200.0;
            let end = sample.position + Vec3::Y * 700.0;
            let mut nearest = surfaces.world.cast(start, end).map(|hit| (None, hit));
            for (index, (_, _, surface)) in surfaces.hinges.iter().enumerate() {
                if let Some(hit) = surface.cast(start, end)
                    && nearest.is_none_or(|(_, old)| hit.fraction < old.fraction)
                {
                    nearest = Some((Some(index), hit));
                }
            }
            let Some((hinge, hit)) = nearest.filter(|(_, hit)| hit.flags & render_mesh::WATER == 0)
            else {
                return;
            };
            let image = game.mission.images[[0, 2, 4, 56][usize::from(sample.sprite)]];
            let half_size = glam::Vec2::from(
                image
                    .size
                    .map(|n| f32::from(n) * sample.scale.abs() as f32 / 2048.0),
            );
            let stamp = Stamp {
                center: hit.position,
                normal: hit.normal,
                radius: half_size.length(),
                depth: 80.0,
                seed: 0,
                shape: None,
            };
            let angle = (sample.rotation as f32 / 4096.0).to_radians();
            let tangent = Vec3::new(angle.cos(), 0.0, angle.sin());
            let surface = hinge.map_or(surfaces.world, |i| &surfaces.hinges[i].2);
            let mut vertices = surface.project_rectangle(&stamp, tangent, half_size);
            let part = hinge.map(|i| {
                let (key, matrix, _) = &surfaces.hinges[i];
                let r = rotation(*matrix);
                for v in &mut vertices {
                    v.position =
                        (r.inverse() * (Vec3::from(v.position) - translation(*matrix))).to_array();
                    v.normal = (r.transpose() * Vec3::from(v.normal)).to_array();
                }
                *key
            });
            let mut mark = Mark(
                vec![Patch {
                    part,
                    vertices,
                    ..Default::default()
                }],
                Kind::Ground,
            );
            Self::original_texture(&mut mark, image, self.settings.ground);
            self.retain(mark, usize::from(self.settings.marks));
            return;
        }
        self.original.push_back(Billboard {
            gore_source: None,
            graphic: Graphic::Blood {
                sprite: sample.sprite,
                stain: false,
                fading: false,
                airborne: false,
            },
            position: sample.position.to_array().map(|v| v as i32).into(),
            scale: [sample.scale; 2],
            color: [128; 3],
            rotation: sample.rotation,
        });
    }

    pub(super) fn retain_missing_retail(
        &mut self,
        sample: Option<Sample>,
        game: &Game,
        surfaces: &Receivers<'_>,
    ) {
        if let Some(sample) = sample.filter(|s| s.mode > 0 && !s.stained) {
            self.retain_spot(sample, game, surfaces);
        }
    }

    pub(super) fn retain_retail(
        &mut self,
        old: Option<Sample>,
        next: &mut Sample,
        continuous: bool,
        game: &Game,
        surfaces: &Receivers<'_>,
    ) {
        if continuous {
            next.stained = old.is_some_and(|s| s.stained);
        } else {
            self.retain_missing_retail(old, game, surfaces);
        }
        if next.mode == 3 && !next.stained {
            self.retain_spot(
                old.filter(|s| continuous && s.mode > 0).unwrap_or(*next),
                game,
                surfaces,
            );
            next.stained = true;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; v0.17.0 floating-blood report locations and real effect lifetime"]
    fn retained_blood_rests_on_the_reported_mission_one_floors() {
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let mut game = crate::session::Options::default().load(&assets).unwrap();
        game.events = Default::default();
        let models =
            Models::build(&game.mission, &Default::default(), &Default::default()).unwrap();
        let terrain = render_mesh::stage(&game.mission, &game.math, Some(&models));
        let surface = Surfaces::new(&terrain);
        let output = std::env::var_os("TENCHU_RETAINED_REVIEW").map(std::path::PathBuf::from);
        if let Some(output) = &output {
            std::fs::create_dir_all(output).unwrap();
        }
        for position in [
            [19280, 0, -38744],
            [-49196, 0, -16784],
            [-15877, 0, -36744],
            [-13141, 0, -37568],
        ] {
            for conform_retained in [false, true] {
                let settings = settings::Blood {
                    hit: BloodStyle::Modern,
                    ground: BloodStyle::OriginalHd,
                    retain_original: true,
                    conform_retained,
                    runoff: true,
                    amount: 200,
                    marks: 65535,
                    ..Default::default()
                };
                let mut blood = Blood::default();
                game.effects = Default::default();
                blood.tick(&game, &models, &surface, &settings, None);
                game.effects.slots[0] = Some(Effect::Blood(tenchu_core::effects::Blood {
                    hint: None,
                    position: position.into(),
                    velocity: [0; 3],
                    scale: 18000,
                    rotation: 57 * 4096,
                    time: 2,
                    brightness: 128,
                    sprite: 1,
                    mode: 2,
                    gore: false,
                }));
                for _ in 0..100 {
                    game.step(0).unwrap();
                    blood.tick(&game, &models, &surface, &settings, None);
                }
                assert_eq!(blood.original.is_empty(), conform_retained);
                if conform_retained {
                    let vertices: Vec<_> = blood.pages().flat_map(|p| p.iter()).collect();
                    assert!(!vertices.is_empty(), "no stain at {position:?}");
                    for v in vertices {
                        let p = Vec3::from(v.position);
                        let ground = surface.cast(p - Vec3::Y, p + Vec3::Y).unwrap();
                        assert!(p.distance(ground.position) < 0.6, "hover at {p:?}");
                        assert!(render_mesh::original_hd_blood(v.flags));
                    }
                }
                if position[0] == 19280
                    && let Some(output) = &output
                {
                    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
                    review.configure(settings::Graphics {
                        fog: false,
                        blood: settings,
                        ..Default::default()
                    });
                    let stage = review.renderer.upload(&terrain);
                    let focus = Vec3::from(position.map(|v| v as f32));
                    let eye = focus + Vec3::new(1200.0, -500.0, 1800.0);
                    let delta = focus - eye;
                    let camera = crate::debug::camera::Camera {
                        position: eye,
                        yaw: delta.x.atan2(delta.z),
                        pitch: (-delta.y).atan2(delta.x.hypot(delta.z)),
                        fov: 50.0,
                    };
                    let effects = review.renderer.upload_effects(blood.geometry(
                        &game,
                        &models,
                        None,
                        Some(camera.source_view()),
                        1.0,
                    ));
                    review
                        .capture(
                            camera.matrix(1280.0 / 720.0),
                            &[&stage],
                            Some(&effects),
                            false,
                            &output.join(format!("conform-{conform_retained}.png")),
                        )
                        .unwrap();
                }
            }
        }
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; original pixel artwork remains on horizontal and sloped receivers"]
    fn retained_sprites_conform_without_changing_the_selected_artwork() {
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let game = crate::session::Options::default().load(&assets).unwrap();
        for slope in [0.0, 0.4] {
            let corners = [
                [-1000.0, -1000.0],
                [1000.0, -1000.0],
                [-1000.0, 1000.0],
                [1000.0, 1000.0],
            ];
            let vertices: Vec<_> = [0, 1, 2, 1, 3, 2]
                .map(|i| Vertex {
                    position: [corners[i][0], corners[i][0] * slope, corners[i][1]],
                    color: [1.0; 3],
                    ..Vertex::zeroed()
                })
                .into();
            let surface = Surfaces::new(&vertices);
            let receivers = Receivers {
                world: &surface,
                hinges: vec![],
            };
            for style in [BloodStyle::Original, BloodStyle::OriginalHd] {
                let mut blood = Blood::default();
                blood.configure(&settings::Blood {
                    ground: style,
                    retain_original: true,
                    conform_retained: true,
                    ..Default::default()
                });
                blood.retain_spot(
                    Sample {
                        position: Vec3::ZERO,
                        mode: 2,
                        time: 0,
                        stained: false,
                        scale: 8192,
                        rotation: 30 * 4096,
                        sprite: 1,
                    },
                    &game,
                    &receivers,
                );
                assert!(blood.original.is_empty());
                let page = blood.pages().next().unwrap();
                assert!(!page.is_empty());
                let image = game.mission.images[2];
                for v in page.iter() {
                    assert!((v.position[1] - v.position[0] * slope).abs() < 0.6);
                    assert!(v.flags & render_mesh::TEXTURED != 0);
                    assert_eq!(
                        render_mesh::original_hd_blood(v.flags),
                        style == BloodStyle::OriginalHd
                    );
                    assert_eq!(v.page, u32::from(image.page));
                    assert_eq!(v.palette, u32::from(image.palette));
                    for axis in 0..2 {
                        assert!(
                            v.uv[axis] >= f32::from(image.uv[axis])
                                && v.uv[axis]
                                    <= f32::from(image.uv[axis]) + f32::from(image.size[axis])
                        );
                    }
                }
                let saved = postcard::to_allocvec(&blood).unwrap();
                let restored: Blood = postcard::from_bytes(&saved).unwrap();
                assert_eq!(postcard::to_allocvec(&restored).unwrap(), saved);
            }
        }
    }
}
