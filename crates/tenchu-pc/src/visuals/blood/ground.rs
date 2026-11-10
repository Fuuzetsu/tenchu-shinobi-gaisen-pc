use super::*;
use glam::Vec2;
use tenchu_assets::texture::Image;

impl Blood {
    pub(crate) fn ground_tick(
        &mut self,
        tick: u64,
        surfaces: &Surfaces,
        images: &[Image],
        settings: &settings::Blood,
    ) {
        self.configure(settings);
        let start = self
            .tick
            .map_or(tick, |old| (old + 1).max(tick.saturating_sub(29)));
        if !self.runoff.is_empty() {
            let receivers = Receivers {
                world: surfaces,
                hinges: Vec::new(),
            };
            for _ in start..=tick {
                self.advance_runoff(images, &receivers, usize::from(settings.marks));
            }
        }
        self.tick = Some(tick);
    }

    pub(crate) fn ground_stamp(&mut self, surfaces: &Surfaces, corners: [Vec3; 4], image: Image) {
        if !self.settings.retained_ground() {
            return;
        }
        let center = corners.iter().copied().sum::<Vec3>() * 0.25;
        let right = (corners[1] - corners[0]) * 0.5;
        let up = (corners[2] - corners[0]) * 0.5;
        let normal = right.cross(up).normalize_or_zero();
        if normal == Vec3::ZERO {
            return;
        }
        let hit = surfaces
            .cast(center + normal * 250.0, center - normal * 350.0)
            .filter(|hit| hit.flags & render_mesh::WATER == 0);
        let receivers = Receivers {
            world: surfaces,
            hinges: Vec::new(),
        };
        let original = self.settings.ground.uses_original();
        let amount = f32::from(self.settings.amount.min(200)) / 100.0;
        let radius = 45.0 + amount * 65.0;
        let mark = if original && !self.settings.conform_retained {
            let mut vertices =
                crate::render_effects::quad(corners.map(|p| p.to_array()), [[1.0; 3]; 4], None, 0);
            for (v, i) in vertices.iter_mut().zip([0, 1, 2, 1, 3, 2]) {
                v.color = [
                    if i & 1 == 0 { -1.0 } else { 1.0 },
                    if i & 2 == 0 { -1.0 } else { 1.0 },
                    1.0,
                ];
                v.normal = normal.to_array();
            }
            let mut mark = Mark(
                vec![Patch {
                    vertices,
                    ..Default::default()
                }],
                Kind::Ground,
            );
            Self::original_texture(&mut mark, image, self.settings.ground);
            mark
        } else {
            let Some(hit) = hit else { return };
            if original {
                let half_size = Vec2::new(right.length(), up.length());
                Self::project_original(
                    &receivers,
                    None,
                    &Stamp {
                        center: hit.position,
                        normal: hit.normal,
                        radius: half_size.length(),
                        depth: 80.0,
                        seed: 0,
                        shape: None,
                    },
                    right.normalize(),
                    half_size,
                    image,
                    self.settings.ground,
                )
            } else {
                self.random();
                Self::project_contact(&receivers, None, hit, radius, self.seed)
            }
        };
        self.retain(mark, usize::from(self.settings.marks));
        if let Some(hit) = hit {
            self.start_runoff(&receivers, None, hit, radius);
        }
    }
}
