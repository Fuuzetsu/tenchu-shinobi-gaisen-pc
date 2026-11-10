use super::*;
use glam::Vec2;
use tenchu_assets::texture::Image;

pub(crate) struct SurfacePool {
    pub id: usize,
    pub player: bool,
    pub corners: [Vec3; 4],
    pub image: Image,
    pub finished: bool,
}

impl Blood {
    pub(crate) fn ground_tick(
        &mut self,
        tick: u64,
        surfaces: &Surfaces,
        images: &[Image],
        settings: &settings::Blood,
    ) -> bool {
        let pools_changed = self.settings.player_death != settings.player_death
            || self.settings.death != settings.death
            || self.settings.death_size != settings.death_size
            || self.settings.retain_original != settings.retain_original
            || self.settings.amount != settings.amount;
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
        pools_changed
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

impl Blood {
    pub(crate) fn surface_pool(&mut self, surfaces: &Surfaces, sample: SurfacePool) {
        let SurfacePool {
            id,
            player,
            corners,
            image,
            finished,
        } = sample;
        if !self.settings.custom_death(player)
            || self.dead.contains_key(&id) && !self.pools.contains_key(&id)
        {
            return;
        }
        let center = corners.iter().copied().sum::<Vec3>() * 0.25;
        let Some(hit) = surfaces
            .cast(center - Vec3::Y * 250.0, center + Vec3::Y * 350.0)
            .filter(|hit| hit.flags & render_mesh::WATER == 0 && hit.normal.y < -0.3)
        else {
            return;
        };
        let right = (corners[1] - corners[0]) * 0.5;
        let up = (corners[2] - corners[0]) * 0.5;
        let size = (f32::from(self.settings.amount) / 100.0).sqrt()
            * f32::from(self.settings.death_size)
            / 100.0;
        let half_size = Vec2::new(right.length(), up.length()) * size;
        let receivers = Receivers {
            world: surfaces,
            hinges: Vec::new(),
        };
        let seed = if let Some(pool) = self.pools.get(&id) {
            pool.seed
        } else {
            self.random();
            self.seed
        };
        let mut mark = if self.settings.death.uses_original() {
            Self::project_original(
                &receivers,
                None,
                &Stamp {
                    center: hit.position,
                    normal: hit.normal,
                    radius: half_size.length(),
                    depth: 80.0,
                    seed,
                    shape: None,
                },
                right.normalize_or_zero(),
                half_size,
                image,
                self.settings.death,
            )
        } else {
            Self::project_contact(&receivers, None, hit, half_size.max_element(), seed)
        };
        mark.1 = Kind::Death { player };
        self.dead.insert(id, player);
        if finished {
            self.pools.remove(&id);
            self.retain(mark, usize::from(self.settings.marks));
        } else {
            self.pools.insert(id, Pool { age: 0, seed, mark });
        }
    }

    pub(crate) fn forget_surface_pool(&mut self, id: usize) {
        self.pools.remove(&id);
        self.dead.remove(&id);
    }

    pub(crate) fn pool_geometry(&self) -> Geometry {
        let vertices: Vec<_> = self.pool_vertices().collect();
        let batches = if vertices.is_empty() {
            Vec::new()
        } else {
            vec![Batch {
                blend: 5,
                range: 0..vertices.len() as u32,
            }]
        };
        Geometry { vertices, batches }
    }
}
