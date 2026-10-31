use super::*;

const LIMIT: usize = 32;

pub(super) struct Runoff {
    part: Option<Part>,
    position: Vec3,
    normal: Vec3,
    radius: f32,
    life: u16,
    seed: u32,
}

fn downhill(surface: &Surfaces, position: Vec3, normal: Vec3, step: f32) -> Option<surfaces::Hit> {
    let gravity = Vec3::Y - normal * normal.y;
    if gravity.length_squared() < 0.02 || normal.y > 0.3 {
        return None;
    }
    let end = position + gravity.normalize() * step;
    let offset = normal * 3.0;
    surface
        .cast_excluding(position + offset, end + offset, render_mesh::WATER)
        .or_else(|| {
            surface.cast_excluding(end + normal * 12.0, end - normal * 20.0, render_mesh::WATER)
        })
        .filter(|hit| {
            hit.position.y >= position.y - 0.5
                && hit.position.distance_squared(position) < (step + 24.0).powi(2)
                && hit.normal.y <= 0.3
        })
}

impl Blood {
    pub(super) fn start_runoff(
        &mut self,
        surfaces: &Receivers<'_>,
        hinge: Option<usize>,
        hit: surfaces::Hit,
        radius: f32,
    ) {
        if !self.settings.runoff
            || self.runoff.len() >= LIMIT
            || hit.normal.y < -0.98
            || hit.normal.y > 0.3
            || hit.flags & render_mesh::WATER != 0
        {
            return;
        }
        let (part, position, normal) = if let Some(index) = hinge {
            let (key, matrix, _) = &surfaces.hinges[index];
            let r = rotation(*matrix);
            if r.determinant().abs() < 0.0001 {
                return;
            }
            (
                Some(*key),
                r.inverse() * (hit.position - translation(*matrix)),
                r.transpose() * hit.normal,
            )
        } else {
            (None, hit.position, hit.normal)
        };
        if self.runoff.iter().any(|flow| {
            flow.part == part && flow.position.distance_squared(position) < 200.0_f32.powi(2)
        }) {
            return;
        }
        self.runoff.push(Runoff {
            part,
            position,
            normal,
            radius: (radius * 0.25).clamp(8.0, 28.0),
            life: 90,
            seed: self.seed,
        });
    }

    pub(super) fn advance_runoff(&mut self, game: &Game, surfaces: &Receivers<'_>, limit: usize) {
        let mut flows = std::mem::take(&mut self.runoff);
        flows.retain_mut(|flow| {
            if flow.life == 0 {
                return false;
            }
            let hinge = flow
                .part
                .and_then(|key| surfaces.hinges.iter().position(|(part, _, _)| *part == key));
            if flow.part.is_some() && hinge.is_none() {
                return false;
            }
            let (surface, matrix) = hinge.map_or((surfaces.world, None), |i| {
                (&surfaces.hinges[i].2, Some(surfaces.hinges[i].1))
            });
            let (position, normal) = matrix.map_or((flow.position, flow.normal), |matrix| {
                (
                    rotation(matrix) * flow.position + translation(matrix),
                    (rotation(matrix) * flow.normal).normalize_or_zero(),
                )
            });
            let radius = flow.radius * (0.4 + 0.6 * f32::from(flow.life) / 90.0);
            let Some(hit) = downhill(surface, position, normal, radius * 0.4) else {
                return false;
            };
            (flow.position, flow.normal) = matrix.map_or((hit.position, hit.normal), |matrix| {
                let r = rotation(matrix);
                (
                    r.inverse() * (hit.position - translation(matrix)),
                    r.transpose() * hit.normal,
                )
            });
            flow.life -= 1;
            if flow.life.is_multiple_of(2) || hit.normal.y < -0.98 {
                let mut mark = Self::project_contact(surfaces, hinge, hit, radius, flow.seed);
                if self.settings.ground == BloodStyle::Original {
                    Self::original_texture(
                        &mut mark,
                        game.mission.images[(flow.seed as usize % 2) * 2],
                    );
                }
                self.retain(mark, limit);
            }
            true
        });
        self.runoff = flows;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quad(points: [Vec3; 4]) -> Vec<Vertex> {
        [0, 1, 2, 0, 2, 3]
            .map(|index| Vertex {
                position: points[index].to_array(),
                color: [0.7; 3],
                opacity: 1.0,
                flags: render_mesh::DOUBLE_SIDED,
                ..Vertex::zeroed()
            })
            .to_vec()
    }

    #[test]
    fn trails_follow_gravity_across_slopes_and_stop_at_edges_and_floors() {
        for slope in [0.0, 0.5, 2.0] {
            let surface = Surfaces::new(&quad([
                Vec3::new(-500.0, -500.0, -500.0 * slope),
                Vec3::new(500.0, -500.0, -500.0 * slope),
                Vec3::new(500.0, 500.0, 500.0 * slope),
                Vec3::new(-500.0, 500.0, 500.0 * slope),
            ]));
            let mut position = Vec3::new(0.0, -400.0, -400.0 * slope);
            let mut normal = Vec3::new(0.0, -slope, 1.0).normalize();
            let start = position;
            let mut steps = 0;
            while let Some(hit) = downhill(&surface, position, normal, 15.0) {
                assert!(hit.position.y > position.y);
                assert!((hit.position.z - hit.position.y * slope).abs() < 0.01);
                position = hit.position;
                normal = hit.normal;
                steps += 1;
                assert!(steps < 300);
            }
            assert!(position.distance(start) > 800.0);
        }
        let mut geometry = quad([
            Vec3::new(-500.0, -500.0, 0.0),
            Vec3::new(500.0, -500.0, 0.0),
            Vec3::new(500.0, 500.0, 0.0),
            Vec3::new(-500.0, 500.0, 0.0),
        ]);
        geometry.extend(quad([
            Vec3::new(-500.0, 0.0, -500.0),
            Vec3::new(500.0, 0.0, -500.0),
            Vec3::new(500.0, 0.0, 500.0),
            Vec3::new(-500.0, 0.0, 500.0),
        ]));
        let surface = Surfaces::new(&geometry);
        let hit = downhill(&surface, Vec3::new(0.0, -8.0, 0.0), Vec3::Z, 15.0).unwrap();
        assert!(hit.position.y.abs() < 0.01 && hit.normal == Vec3::NEG_Y);
        assert!(downhill(&surface, hit.position, hit.normal, 15.0).is_none());
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; runoff styles, bounded work, settings changes and presentation-only state"]
    fn retained_trails_are_bounded_and_do_not_change_gameplay() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let game = crate::session::Options::default().load(&assets).unwrap();
        let state = crate::session::tests::state(&game);
        let wall = Surfaces::new(&quad([
            Vec3::new(-20000.0, -4000.0, 0.0),
            Vec3::new(20000.0, -4000.0, 0.0),
            Vec3::new(20000.0, 0.0, 0.0),
            Vec3::new(-20000.0, 0.0, 0.0),
        ]));
        let receivers = Receivers {
            world: &wall,
            hinges: Vec::new(),
        };
        for ground in [BloodStyle::Original, BloodStyle::Modern] {
            let mut blood = Blood::default();
            let settings = settings::Blood {
                ground,
                retain_original: true,
                ..settings::Blood::modern()
            };
            blood.configure(&settings);
            for i in 0..64 {
                let at = Vec3::new((i as f32 - 32.0) * 400.0, -2000.0, 0.0);
                assert!(blood.stain(
                    &game,
                    &receivers,
                    at + Vec3::Z * 20.0,
                    at - Vec3::Z * 20.0,
                    80.0,
                    1024
                ));
            }
            assert_eq!(blood.runoff.len(), LIMIT);
            for _ in 0..120 {
                blood.advance_runoff(&game, &receivers, 1024);
                assert!(blood.runoff.len() <= LIMIT && blood.marks.len() <= 1024);
            }
            assert!(blood.runoff.is_empty());
            let mut lowest = f32::NEG_INFINITY;
            for page in blood.pages() {
                for v in page.iter() {
                    assert_eq!(
                        v.flags & render_mesh::SOFT != 0,
                        ground == BloodStyle::Modern
                    );
                    assert!(v.position[2].abs() <= 0.51);
                    lowest = lowest.max(v.position[1]);
                }
            }
            assert!(
                lowest > -1600.0,
                "Trails must travel visibly downhill: {lowest}"
            );
            if let Some(output) = std::env::var_os("TENCHU_RUNOFF_REVIEW") {
                let output = std::path::PathBuf::from(output);
                std::fs::create_dir_all(&output).unwrap();
                let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
                let backdrop: Vec<_> = quad([
                    Vec3::new(-15000.0, -2600.0, 0.0),
                    Vec3::new(-9000.0, -2600.0, 0.0),
                    Vec3::new(-9000.0, -700.0, 0.0),
                    Vec3::new(-15000.0, -700.0, 0.0),
                ])
                .into_iter()
                .map(|v| Vertex {
                    color: [0.7; 3],
                    opacity: 1.0,
                    flags: render_mesh::DOUBLE_SIDED,
                    ..v
                })
                .collect();
                let backdrop = review.renderer.upload(&backdrop);
                let stains = blood.pages().flat_map(|p| p.to_vec()).collect();
                let camera =
                    glam::Mat4::orthographic_rh(-1800.0, 1800.0, -1012.5, 1012.5, 10.0, 4000.0)
                        * glam::Mat4::look_at_rh(
                            Vec3::new(-12000.0, -1600.0, 2000.0),
                            Vec3::new(-12000.0, -1600.0, 0.0),
                            Vec3::NEG_Y,
                        );
                let effects = review
                    .renderer
                    .upload_effects(crate::capture::review::effects(&[(5, stains)], camera));
                review
                    .capture(
                        camera,
                        &[&backdrop],
                        Some(&effects),
                        false,
                        &output.join(format!("{ground:?}.png")),
                    )
                    .unwrap();
            }
            let at = Vec3::new(0.0, -2500.0, 0.0);
            blood.stain(
                &game,
                &receivers,
                at + Vec3::Z * 20.0,
                at - Vec3::Z * 20.0,
                80.0,
                1024,
            );
            assert!(!blood.runoff.is_empty());
            let count = blood.marks.len();
            blood.configure(&settings::Blood {
                runoff: false,
                ..settings
            });
            assert!(blood.runoff.is_empty() && blood.marks.len() == count);
            blood.configure(&settings);
            blood.stain(
                &game,
                &receivers,
                at + Vec3::Z * 20.0,
                at - Vec3::Z * 20.0,
                80.0,
                1024,
            );
            blood.clear();
            assert!(blood.runoff.is_empty() && blood.marks.is_empty());
        }
        assert_eq!(state, crate::session::tests::state(&game));
    }
}
