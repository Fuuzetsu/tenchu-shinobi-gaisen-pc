use super::{
    appearance,
    interpolation::{Frame, Part},
    models::{Models, rotation},
    surfaces::{self, Stamp, Surfaces},
};
use crate::{
    render_effects::{Batch, Geometry},
    render_mesh::{self, Vertex},
    settings::{self, BloodStyle},
};
use bytemuck::Zeroable;
use glam::{Mat3, Quat, Vec3};
use std::collections::{BTreeMap, VecDeque};
use tenchu_core::{effects::Effect, game::Game, math::Matrix};

mod storage;
use storage::Marks;
#[cfg(test)]
use storage::VERTEX_LIMIT;
const WOUND_LIMIT: usize = 64;
const DROP_LIMIT: usize = 128;

struct Patch {
    part: Option<Part>,
    vertices: Vec<Vertex>,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Kind {
    Ground,
    Body,
    Death { player: bool },
}

struct Mark(Vec<Patch>, Kind);

impl Mark {
    fn len(&self) -> usize {
        self.0.iter().map(|p| p.vertices.len()).sum()
    }
}

struct Wound {
    part: Part,
    position: Vec3,
    age: u16,
}

struct Pool {
    age: u16,
    seed: u32,
    mark: Mark,
}

struct Drop {
    previous: Vec3,
    position: Vec3,
    velocity: Vec3,
    life: u16,
}

#[derive(Clone, Copy)]
struct Sample {
    position: Vec3,
    mode: u8,
    time: i16,
    stained: bool,
}

#[derive(Default)]
pub struct Blood {
    tick: Option<u64>,
    health: BTreeMap<usize, i16>,
    samples: Vec<Option<Sample>>,
    marks: Marks,
    wounds: VecDeque<Wound>,
    drops: Vec<Drop>,
    pools: BTreeMap<usize, Pool>,
    dead: BTreeMap<usize, bool>,
    seed: u32,
    settings: settings::Blood,
}

fn translation(matrix: Matrix) -> Vec3 {
    Vec3::from(matrix.translation.map(|v| v as f32).to_array())
}

fn transforms(
    game: &Game,
    models: &Models,
    frame: Option<&Frame>,
    inspection: bool,
) -> BTreeMap<Part, Matrix> {
    let mut parts = BTreeMap::new();
    appearance::visit_parts(game, inspection, Some(models), frame, |key, _, matrix| {
        parts.insert(key.canonical(), matrix);
    });
    render_mesh::visit_hinged_parts(game, |key, _, matrix| {
        parts.insert(key, Frame::transform(frame, key, matrix));
    });
    parts
}

struct Receivers<'a> {
    world: &'a Surfaces,
    hinges: Vec<(Part, Matrix, Surfaces)>,
}

impl<'a> Receivers<'a> {
    fn new(game: &Game, models: &Models, world: &'a Surfaces) -> Self {
        let mut hinges = Vec::new();
        render_mesh::visit_hinged_parts(game, |key, geometry, matrix| {
            let mut vertices = Vec::new();
            render_mesh::append_geometry(&mut vertices, geometry, matrix, Some(models));
            hinges.push((key, matrix, Surfaces::new(&vertices)));
        });
        Self { world, hinges }
    }
}

impl Blood {
    pub fn configure(&mut self, settings: &settings::Blood) {
        if self.settings == *settings {
            return;
        }
        if !settings.active() {
            *self = Self {
                settings: *settings,
                ..Self::default()
            };
            return;
        }
        let previous = self.settings;
        let ground_changed = previous.ground != settings.ground
            || previous.retain_original != settings.retain_original;
        let death_changed = previous.death != settings.death
            || previous.retain_original != settings.retain_original;
        let player_changed = previous.player_death != settings.player_death;
        self.marks.retain_kind(|kind| match kind {
            Kind::Ground => !ground_changed,
            Kind::Body => settings.model_stains,
            Kind::Death { player } => !death_changed && !(player && player_changed),
        });
        if death_changed || player_changed {
            self.pools
                .retain(|_, pool| !death_changed && pool.mark.1 != Kind::Death { player: true });
            self.dead.retain(|_, player| !death_changed && !*player);
        }
        if settings.drips == BloodStyle::Off {
            self.wounds.clear();
            self.drops.clear();
        }
        self.trim(usize::from(settings.marks.max(32)));
        self.settings = *settings;
    }

    pub fn clear(&mut self) {
        self.marks = Marks::default();
        self.wounds.clear();
        self.drops.clear();
        self.pools.clear();
    }

    fn random(&mut self) -> f32 {
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / 16_777_216.0
    }

    fn retain(&mut self, mark: Mark, limit: usize) {
        self.marks.insert(mark, limit);
    }

    fn trim(&mut self, limit: usize) {
        self.marks.trim(limit);
    }

    pub fn pages(&self) -> impl Iterator<Item = &std::sync::Arc<Vec<Vertex>>> {
        self.marks.pages()
    }

    fn stain(
        &mut self,
        game: &Game,
        surfaces: &Receivers<'_>,
        start: Vec3,
        end: Vec3,
        radius: f32,
        limit: usize,
    ) -> bool {
        let mut nearest = surfaces.world.cast(start, end).map(|hit| (None, hit));
        for (index, (_, _, surface)) in surfaces.hinges.iter().enumerate() {
            if let Some(hit) = surface.cast(start, end)
                && nearest.is_none_or(|(_, old)| hit.fraction < old.fraction)
            {
                nearest = Some((Some(index), hit));
            }
        }
        let Some((hinge, hit)) = nearest else {
            return false;
        };
        if !self.settings.retained_ground() {
            return true;
        }
        self.random();
        let mut mark = Self::project_contact(surfaces, hinge, hit, radius, self.seed);
        if self.settings.ground == BloodStyle::Original {
            // The adjacent white sprite is a subtraction mask used only while fading.
            Self::original_texture(&mut mark, game.mission.images[(self.seed as usize % 2) * 2]);
        }
        self.retain(mark, limit);
        true
    }

    fn project_contact(
        surfaces: &Receivers<'_>,
        hinge: Option<usize>,
        hit: surfaces::Hit,
        radius: f32,
        seed: u32,
    ) -> Mark {
        let stamp = Stamp {
            center: hit.position,
            normal: hit.normal,
            radius,
            depth: 15.0,
            seed,
            shape: None,
        };
        let patch = if let Some(index) = hinge {
            let (key, matrix, surface) = &surfaces.hinges[index];
            let r = rotation(*matrix);
            if r.determinant().abs() < 0.0001 {
                return Mark(Vec::new(), Kind::Ground);
            }
            let mut vertices = surface.project(&stamp);
            for v in &mut vertices {
                v.position =
                    (r.inverse() * (Vec3::from(v.position) - translation(*matrix))).to_array();
                v.normal = (r.transpose() * Vec3::from(v.normal)).to_array();
            }
            Patch {
                part: Some(*key),
                vertices,
            }
        } else {
            Patch {
                part: None,
                vertices: surfaces.world.project(&stamp),
            }
        };
        Mark(vec![patch], Kind::Ground)
    }

    fn original_texture(mark: &mut Mark, image: tenchu_assets::texture::Image) {
        for v in mark.0.iter_mut().flat_map(|p| &mut p.vertices) {
            v.uv = std::array::from_fn(|axis| {
                f32::from(image.uv[axis])
                    + (v.color[axis] * 0.5 + 0.5).clamp(0.0, 1.0)
                        * f32::from(image.size[axis].saturating_sub(1))
            });
            v.color = [1.0; 3];
            v.page = image.page.into();
            v.palette = image.palette.into();
            v.material = 0;
            v.flags = render_mesh::TEXTURED
                | render_mesh::ALPHA
                | render_mesh::FOG
                | render_mesh::DOUBLE_SIDED
                | render_mesh::STAIN;
            v.opacity = 0.9;
        }
    }

    fn wound(
        &mut self,
        game: &Game,
        models: &Models,
        root: usize,
        position: Vec3,
        settings: &settings::Blood,
    ) {
        let Some(actor) = game
            .roster
            .actors
            .iter()
            .flatten()
            .find(|actor| actor.root_model == root)
        else {
            return;
        };
        let mut parts = Vec::new();
        appearance::visit_parts(game, true, Some(models), None, |key, geometry, matrix| {
            if matches!(key, Part::Actor { root: r, model, .. } if r == root && actor.parts.contains(&model))
            {
                let mut vertices = Vec::new();
                render_mesh::append_geometry(&mut vertices, geometry, matrix, Some(models));
                parts.push((key.canonical(), matrix, vertices));
            }
        });
        let approach = game
            .roster
            .actors
            .iter()
            .flatten()
            .filter(|actor| actor.root_model != root && actor.health > 0)
            .map(|actor| Vec3::new(actor.position.x as f32, position.y, actor.position.z as f32))
            .filter(|point| point.distance_squared(position) < 3000.0_f32.powi(2))
            .min_by(|a, b| {
                a.distance_squared(position)
                    .total_cmp(&b.distance_squared(position))
            })
            .map(|point| (point - position).normalize_or_zero());
        let contact = approach.and_then(|direction| {
            parts
                .iter()
                .enumerate()
                .flat_map(|(index, (_, _, vertices))| {
                    vertices.chunks_exact(3).filter_map(move |t| {
                        surfaces::intersect(
                            &[t[0], t[1], t[2]],
                            position + direction * 2000.0,
                            position - direction * 600.0,
                        )
                        .map(|hit| (index, hit))
                    })
                })
                .min_by(|a, b| a.1.fraction.total_cmp(&b.1.fraction))
                .map(|(index, hit)| (index, hit.position, hit.normal))
        });
        let nearest = contact.or_else(|| {
            parts
                .iter()
                .enumerate()
                .flat_map(|(i, (_, _, vertices))| {
                    vertices.chunks_exact(3).map(move |t| {
                        let triangle = [t[0], t[1], t[2]];
                        let point = surfaces::closest(&triangle, position);
                        let [a, b, c] = triangle.map(|v| Vec3::from(v.position));
                        (i, point, (b - a).cross(c - a).normalize_or_zero())
                    })
                })
                .min_by(|a, b| {
                    a.1.distance_squared(position)
                        .total_cmp(&b.1.distance_squared(position))
                })
        });
        let Some((index, center, normal)) = nearest else {
            return;
        };
        if center.distance_squared(position) > 1200.0_f32.powi(2) {
            return;
        }
        self.random();
        let stamp = Stamp {
            center,
            normal,
            radius: 120.0 + f32::from(settings.amount.min(200)) * 0.8,
            depth: 32.0,
            seed: self.seed,
            shape: None,
        };
        let mut patches = Vec::new();
        for &(key, matrix, ref vertices) in &parts {
            let r = rotation(matrix);
            if r.determinant().abs() < 0.0001 {
                continue;
            }
            let inverse = r.inverse();
            let offset = translation(matrix);
            let mut projected = Vec::new();
            for triangle in vertices.chunks_exact(3) {
                stamp.project(&[triangle[0], triangle[1], triangle[2]], &mut projected);
            }
            for v in &mut projected {
                v.position = (inverse * (Vec3::from(v.position) - offset)).to_array();
                v.normal = (r.transpose() * Vec3::from(v.normal)).to_array();
            }
            if !projected.is_empty() {
                patches.push(Patch {
                    part: Some(key),
                    vertices: projected,
                });
            }
        }
        if settings.model_stains {
            self.retain(
                Mark(patches, Kind::Body),
                usize::from(settings.marks.max(32)),
            );
        }
        if settings.drips == BloodStyle::Off || settings.seconds == 0 {
            return;
        }
        let (part, matrix, _) = &parts[index];
        let r = rotation(*matrix);
        if r.determinant().abs() < 0.0001 {
            return;
        }
        self.wounds.push_back(Wound {
            part: *part,
            position: r.inverse() * (center - translation(*matrix)),
            age: 0,
        });
        while self.wounds.len() > WOUND_LIMIT {
            self.wounds.pop_front();
        }
    }

    pub fn tick(
        &mut self,
        game: &Game,
        models: &Models,
        surfaces: &Surfaces,
        settings: &settings::Blood,
    ) {
        if self.settings != *settings {
            self.configure(settings);
        }
        if !settings.active() {
            return;
        }
        let health: BTreeMap<_, _> = game
            .roster
            .order
            .iter()
            .map(|&id| {
                let actor = game.roster.get(id);
                (actor.root_model, actor.health)
            })
            .collect();
        if self.tick == Some(game.tick) {
            return;
        }
        if self.tick.is_none_or(|tick| tick + 1 != game.tick) {
            *self = Self {
                tick: Some(game.tick),
                health,
                settings: *settings,
                ..Self::default()
            };
            self.samples = game
                .effects
                .active_slots()
                .iter()
                .map(|slot| sample(slot.as_ref(), true))
                .collect();
            return;
        }
        self.tick = Some(game.tick);
        let receivers = std::cell::LazyCell::new(|| Receivers::new(game, models, surfaces));
        let limit = usize::from(settings.marks.max(32));
        let amount = f32::from(settings.amount.min(200)) / 100.0;
        let mut new_blood = Vec::new();
        let mut samples = Vec::with_capacity(game.effects.active_slots().len());
        let mut impacts = 0;
        for (index, slot) in game.effects.active_slots().iter().enumerate() {
            let mut current = sample(slot.as_ref(), false);
            if let Some(next) = &mut current {
                let old = self.samples.get(index).copied().flatten();
                let continuous = old.filter(|old| {
                    (old.mode != next.mode || next.time <= old.time + 1)
                        && (old.mode == next.mode || old.mode + 1 == next.mode)
                        && old.position.distance_squared(next.position) < 1000.0_f32.powi(2)
                });
                if continuous.is_none() && next.mode == 0 {
                    new_blood.push(next.position);
                }
                if let Some(old) = continuous {
                    next.stained = old.stained;
                    if !old.stained && impacts < 12 && settings.retained_ground() {
                        let (start, end) = if old.mode == 0 && next.mode == 1 {
                            (
                                next.position - Vec3::Y * 250.0,
                                next.position + Vec3::Y * 350.0,
                            )
                        } else {
                            (old.position, next.position)
                        };
                        next.stained =
                            self.stain(game, &receivers, start, end, 45.0 + amount * 65.0, limit);
                        impacts += usize::from(next.stained);
                    }
                }
            }
            samples.push(current);
        }
        self.samples = samples;
        for &id in &game.roster.order {
            let actor = game.roster.get(id);
            if (settings.model_stains || settings.drips != BloodStyle::Off && settings.seconds != 0)
                && self
                    .health
                    .get(&actor.root_model)
                    .is_some_and(|&old| old > actor.health)
            {
                let center =
                    Vec3::from(actor.position.map(|v| v as f32).to_array()) - Vec3::Y * 750.0;
                if let Some(&position) = new_blood
                    .iter()
                    .filter(|p| p.distance_squared(center) < 1600.0_f32.powi(2))
                    .min_by(|a, b| {
                        a.distance_squared(center)
                            .total_cmp(&b.distance_squared(center))
                    })
                {
                    self.wound(game, models, actor.root_model, position, settings);
                }
            }
        }
        self.health = health;
        self.dead
            .retain(|root, _| self.health.get(root) == Some(&0));
        let transforms = if self.wounds.is_empty() {
            BTreeMap::new()
        } else {
            transforms(game, models, None, true)
        };
        self.marks.retain_attached(|mark| {
            mark.0.iter().all(|p| {
                p.part.is_none_or(|key| match key {
                    Part::Actor { root, .. } => self.health.contains_key(&root),
                    _ => true,
                })
            })
        });
        self.wounds.retain_mut(|wound| {
            wound.age = wound.age.saturating_add(1);
            wound.age < settings.seconds.min(60) * 30 && transforms.contains_key(&wound.part)
        });
        if game.tick.is_multiple_of(8) {
            let sources: Vec<_> = self
                .wounds
                .iter()
                .filter_map(|wound| {
                    transforms
                        .get(&wound.part)
                        .map(|&m| rotation(m) * wound.position + translation(m))
                })
                .collect();
            for position in sources {
                if self.drops.len() >= DROP_LIMIT || self.random() > amount.min(1.0) {
                    break;
                }
                let velocity = Vec3::new(
                    (self.random() - 0.5) * 18.0,
                    12.0,
                    (self.random() - 0.5) * 18.0,
                );
                self.drops.push(Drop {
                    previous: position,
                    position,
                    velocity,
                    life: 90,
                });
            }
        }
        let mut drops = std::mem::take(&mut self.drops);
        drops.retain_mut(|drop| {
            drop.previous = drop.position;
            drop.velocity.y += 7.0;
            drop.position += drop.velocity;
            drop.life = drop.life.saturating_sub(1);
            drop.life != 0
                && !self.stain(
                    game,
                    &receivers,
                    drop.previous,
                    drop.position,
                    40.0 + amount * 25.0,
                    limit,
                )
        });
        self.drops = drops;
        if !self.pools.is_empty()
            || game.roster.actors.iter().flatten().any(|actor| {
                actor.health == 0
                    && !self.dead.contains_key(&actor.root_model)
                    && settings.custom_death(actor.id == game.player_id)
                    && actor.status == tenchu_core::actor::State::Dead
            })
        {
            self.update_pools(game, &receivers, amount, limit);
        }
        self.trim(limit);
    }

    fn update_pools(&mut self, game: &Game, surfaces: &Receivers<'_>, amount: f32, limit: usize) {
        for &id in &game.roster.order {
            let actor = game.roster.get(id);
            if actor.health != 0
                || actor.status != tenchu_core::actor::State::Dead
                || actor.motion.action == tenchu_core::ids::MOT_DEAD_DROWN
                || !self.settings.custom_death(id == game.player_id)
            {
                continue;
            }
            if self
                .dead
                .insert(actor.root_model, id == game.player_id)
                .is_none()
            {
                self.random();
                self.pools.insert(
                    actor.root_model,
                    Pool {
                        age: 0,
                        seed: self.seed,
                        mark: Mark(
                            Vec::new(),
                            Kind::Death {
                                player: id == game.player_id,
                            },
                        ),
                    },
                );
            }
        }
        let mut pools = std::mem::take(&mut self.pools);
        pools.retain(|root, pool| {
            let Some(actor) = game
                .roster
                .actors
                .iter()
                .flatten()
                .find(|a| a.root_model == *root)
            else {
                return false;
            };
            if actor.health != 0 {
                return false;
            }
            if pool.age >= 90 && pool.mark.len() != 0 {
                return true;
            }
            pool.age = pool.age.saturating_add(1);
            if pool.age < 24 || actor.velocity[1] != 0 {
                return pool.age < 300;
            }
            let center = actor
                .rendered
                .first()
                .copied()
                .map(translation)
                .unwrap_or_else(|| Vec3::from(actor.position.map(|v| v as f32).to_array()));
            let center = Vec3::new(center.x, actor.position.y as f32, center.z);
            if let Some(hit) = surfaces
                .world
                .cast(center - Vec3::Y * 800.0, center + Vec3::Y * 500.0)
                && hit.flags & render_mesh::WATER == 0
                && hit.normal.y < -0.3
            {
                let growth = (f32::from(pool.age - 24) / 66.0).clamp(0.0, 1.0);
                let radius = (f32::from(game.mission.characters[actor.definition].width) * 1.4)
                    .clamp(300.0, 900.0)
                    * amount.sqrt()
                    * (0.2 + 0.8 * growth.sqrt());
                pool.mark = if self.settings.death == BloodStyle::Original {
                    Self::original_pool(game, actor, hit, growth, amount)
                } else {
                    let mut mark = Self::project_contact(surfaces, None, hit, radius, pool.seed);
                    mark.1 = Kind::Death {
                        player: actor.id == game.player_id,
                    };
                    mark
                };
            }
            if pool.age >= 90
                && (self.settings.death == BloodStyle::Modern || self.settings.retain_original)
            {
                let kind = pool.mark.1;
                self.retain(
                    std::mem::replace(&mut pool.mark, Mark(Vec::new(), kind)),
                    limit,
                );
                false
            } else {
                true
            }
        });
        self.pools = pools;
    }

    fn original_pool(
        game: &Game,
        actor: &tenchu_core::actor::Actor,
        hit: surfaces::Hit,
        growth: f32,
        amount: f32,
    ) -> Mark {
        let height = game.mission.models[&actor.mesh_kind][0].position[1];
        let scale = ((-f32::from(height) * 4.0 - (actor.map.height >> 1) as f32)
            * (0.2 + 0.8 * growth)
            * amount.sqrt())
        .max(0.0)
            / 4096.0;
        let mut vertices = Vec::new();
        for geometry in &game.mission.common_models[&31] {
            for face in render_mesh::draw_tmd_faces(geometry) {
                render_mesh::append_face(&mut vertices, geometry, face, Matrix::default(), true);
            }
        }
        let rotation = Quat::from_rotation_arc(Vec3::NEG_Y, hit.normal);
        for v in &mut vertices {
            v.position =
                (rotation * Vec3::from(v.position) * scale + hit.position + hit.normal).to_array();
            v.normal = hit.normal.to_array();
            v.opacity = if v.flags & render_mesh::TRANSLUCENT != 0 {
                0.5
            } else {
                1.0
            };
            v.flags = (v.flags & !render_mesh::TRANSLUCENT)
                | render_mesh::ALPHA
                | render_mesh::DOUBLE_SIDED
                | render_mesh::STAIN;
        }
        Mark(
            vec![Patch {
                part: None,
                vertices,
            }],
            Kind::Death {
                player: actor.id == game.player_id,
            },
        )
    }

    #[cfg(test)]
    pub fn geometry(
        &self,
        game: &Game,
        models: &Models,
        frame: Option<&Frame>,
        camera: Option<Matrix>,
        alpha: f32,
    ) -> Geometry {
        let vertices: Vec<_> = self.pages().flat_map(|page| page.iter().copied()).collect();
        let mut geometry = Geometry {
            batches: vec![Batch {
                blend: 5,
                range: 0..vertices.len() as u32,
            }],
            vertices,
        };
        geometry.append(self.animated_geometry(game, models, frame, camera, alpha));
        geometry
    }

    pub fn animated_geometry(
        &self,
        game: &Game,
        models: &Models,
        frame: Option<&Frame>,
        camera: Option<Matrix>,
        alpha: f32,
    ) -> Geometry {
        let mut vertices = Vec::new();
        for pool in self.pools.values() {
            vertices.extend(pool.mark.0.iter().flat_map(|p| p.vertices.iter().copied()));
        }
        if !self.marks.attached.is_empty() || !self.drops.is_empty() {
            let transforms = if !self.marks.attached.is_empty() {
                transforms(
                    game,
                    models,
                    frame,
                    camera.is_some() || game.roster.keep_all_active,
                )
            } else {
                BTreeMap::new()
            };
            for (_, mark) in &self.marks.attached {
                for patch in &mark.0 {
                    let (rotation, offset) = if let Some(key) = patch.part {
                        let Some(&matrix) = transforms.get(&key) else {
                            continue;
                        };
                        (rotation(matrix), translation(matrix))
                    } else {
                        (Mat3::IDENTITY, Vec3::ZERO)
                    };
                    if rotation.determinant().abs() < 0.0001 {
                        continue;
                    }
                    let normal = rotation.inverse().transpose();
                    let deformation = patch.part.and_then(|key| {
                        transforms
                            .get(&key)
                            .and_then(|&matrix| models.deformation(key, matrix))
                    });
                    vertices.extend(patch.vertices.iter().map(|v| {
                        let mut vertex = Vertex {
                            position: (rotation * Vec3::from(v.position) + offset).to_array(),
                            normal: (normal * Vec3::from(v.normal)).to_array(),
                            ..*v
                        };
                        if let Some(deformation) = &deformation {
                            deformation.vertex(&mut vertex);
                        }
                        vertex
                    }));
                }
            }
            let camera =
                rotation(camera.unwrap_or_else(|| frame.map_or(game.rendered_view, |f| f.view)))
                    .transpose();
            for drop in &self.drops {
                let center = drop.previous.lerp(drop.position, alpha.clamp(0.0, 1.0));
                let projected = drop.velocity - camera.z_axis * drop.velocity.dot(camera.z_axis);
                let modern = self.settings.drips == BloodStyle::Modern;
                let along = if modern {
                    projected.try_normalize().unwrap_or(camera.y_axis)
                } else {
                    camera.y_axis
                };
                let right = along.cross(camera.z_axis) * if modern { 7.0 } else { 11.0 };
                let up = along
                    * if modern {
                        9.0 + projected.length().min(150.0) * 0.12
                    } else {
                        11.0
                    };
                let points = [
                    center - right - up,
                    center + right - up,
                    center - right + up,
                    center + right + up,
                ];
                vertices.extend([0, 1, 2, 1, 3, 2].map(|i| Vertex {
                    position: points[i].to_array(),
                    color: if modern {
                        [0.38, 0.008, 0.016]
                    } else {
                        [127.0 / 255.0, 16.0 / 255.0, 23.0 / 255.0]
                    },
                    uv: [
                        if i & 1 == 0 { -1.0 } else { 1.0 },
                        if i & 2 == 0 { -1.0 } else { 1.0 },
                    ],
                    opacity: 0.9,
                    flags: render_mesh::ALPHA
                        | render_mesh::FOG
                        | render_mesh::DOUBLE_SIDED
                        | if modern { render_mesh::SOFT } else { 0 },
                    ..Vertex::zeroed()
                }));
            }
        }
        let count = vertices.len() as u32;
        Geometry {
            vertices,
            batches: vec![Batch {
                blend: 5,
                range: 0..count,
            }],
        }
    }
}

fn sample(effect: Option<&Effect>, stained: bool) -> Option<Sample> {
    match effect {
        Some(Effect::Blood(blood)) => Some(Sample {
            position: Vec3::from(blood.position.map(|v| v as f32).to_array()),
            mode: blood.mode,
            time: blood.time,
            stained,
        }),
        _ => None,
    }
}

#[cfg(test)]
mod tests;
