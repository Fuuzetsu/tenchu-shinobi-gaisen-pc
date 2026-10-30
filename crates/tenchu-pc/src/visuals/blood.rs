use super::{
    appearance,
    interpolation::{Frame, Part},
    models::{Models, rotation},
    surfaces::{self, Stamp, Surfaces},
};
use crate::{
    render_effects::{Batch, Geometry},
    render_mesh::{self, Vertex},
    settings,
};
use bytemuck::Zeroable;
use glam::{Mat3, Vec3};
use std::collections::{BTreeMap, VecDeque};
use tenchu_core::{effects::Effect, game::Game, math::Matrix};

const VERTEX_LIMIT: usize = 98_304;
const MARK_VERTEX_LIMIT: usize = 3_072;
const WOUND_LIMIT: usize = 64;
const DROP_LIMIT: usize = 128;

struct Patch {
    part: Option<Part>,
    vertices: Vec<Vertex>,
}

struct Mark(Vec<Patch>);

impl Mark {
    fn len(&self) -> usize {
        self.0.iter().map(|p| p.vertices.len()).sum()
    }
}

struct Wound {
    part: Part,
    position: Vec3,
    remaining: u16,
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
    marks: VecDeque<Mark>,
    wounds: VecDeque<Wound>,
    drops: Vec<Drop>,
    seed: u32,
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
    pub fn clear(&mut self) {
        self.marks.clear();
        self.wounds.clear();
        self.drops.clear();
    }

    fn random(&mut self) -> f32 {
        self.seed = self
            .seed
            .wrapping_mul(1_664_525)
            .wrapping_add(1_013_904_223);
        (self.seed >> 8) as f32 / 16_777_216.0
    }

    fn retain(&mut self, mark: Mark, limit: usize) {
        let len = mark.len();
        if len == 0 || len > MARK_VERTEX_LIMIT {
            return;
        }
        self.marks.push_back(mark);
        self.trim(limit);
    }

    fn trim(&mut self, limit: usize) {
        let mut vertices: usize = self.marks.iter().map(Mark::len).sum();
        while self.marks.len() > limit || vertices > VERTEX_LIMIT {
            vertices -= self.marks.pop_front().unwrap().len();
        }
    }

    fn stain(
        &mut self,
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
        self.random();
        let stamp = Stamp {
            center: hit.position,
            normal: hit.normal,
            radius,
            depth: 15.0,
            seed: self.seed,
            shape: None,
        };
        let patch = if let Some(index) = hinge {
            let (key, matrix, surface) = &surfaces.hinges[index];
            let r = rotation(*matrix);
            if r.determinant().abs() < 0.0001 {
                return false;
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
        self.retain(Mark(vec![patch]), limit);
        true
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
        self.retain(Mark(patches), usize::from(settings.marks.clamp(32, 1024)));
        let (part, matrix, _) = &parts[index];
        let r = rotation(*matrix);
        if r.determinant().abs() < 0.0001 {
            return;
        }
        self.wounds.push_back(Wound {
            part: *part,
            position: r.inverse() * (center - translation(*matrix)),
            remaining: settings.seconds.min(60) * 30,
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
        if !settings.enabled || settings.amount == 0 {
            *self = Self::default();
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
        if self.tick.is_none_or(|tick| tick + 1 != game.tick) {
            *self = Self {
                tick: Some(game.tick),
                health,
                ..Self::default()
            };
            self.samples = game
                .effects
                .slots
                .iter()
                .map(|slot| sample(slot.as_ref(), true))
                .collect();
            return;
        }
        self.tick = Some(game.tick);
        let receivers = Receivers::new(game, models, surfaces);
        let limit = usize::from(settings.marks.clamp(32, 1024));
        let amount = f32::from(settings.amount.min(200)) / 100.0;
        let mut new_blood = Vec::new();
        let mut samples = Vec::with_capacity(game.effects.slots.len());
        let mut impacts = 0;
        for (index, slot) in game.effects.slots.iter().enumerate() {
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
                    if !old.stained && impacts < 12 {
                        let (start, end) = if old.mode == 0 && next.mode == 1 {
                            (
                                next.position - Vec3::Y * 250.0,
                                next.position + Vec3::Y * 350.0,
                            )
                        } else {
                            (old.position, next.position)
                        };
                        next.stained =
                            self.stain(&receivers, start, end, 45.0 + amount * 65.0, limit);
                        impacts += usize::from(next.stained);
                    }
                }
            }
            samples.push(current);
        }
        self.samples = samples;
        for &id in &game.roster.order {
            let actor = game.roster.get(id);
            if self
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
        let transforms = if self.wounds.is_empty() {
            BTreeMap::new()
        } else {
            transforms(game, models, None, true)
        };
        self.marks.retain(|mark| {
            mark.0.iter().all(|p| {
                p.part.is_none_or(|key| match key {
                    Part::Actor { root, .. } => self.health.contains_key(&root),
                    _ => true,
                })
            })
        });
        self.wounds.retain_mut(|wound| {
            wound.remaining = wound.remaining.saturating_sub(1);
            wound.remaining != 0 && transforms.contains_key(&wound.part)
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
                    &receivers,
                    drop.previous,
                    drop.position,
                    40.0 + amount * 25.0,
                    limit,
                )
        });
        self.drops = drops;
        self.trim(limit);
    }

    pub fn geometry(
        &self,
        game: &Game,
        models: &Models,
        frame: Option<&Frame>,
        camera: Option<Matrix>,
        alpha: f32,
    ) -> Geometry {
        let mut vertices = Vec::new();
        if !self.marks.is_empty() || !self.drops.is_empty() {
            let transforms = if self
                .marks
                .iter()
                .any(|m| m.0.iter().any(|p| p.part.is_some()))
            {
                transforms(
                    game,
                    models,
                    frame,
                    camera.is_some() || game.roster.keep_all_active,
                )
            } else {
                BTreeMap::new()
            };
            for mark in &self.marks {
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
            let right = camera * Vec3::X * 9.0;
            let up = camera * Vec3::Y * 18.0;
            for drop in &self.drops {
                let center = drop.previous.lerp(drop.position, alpha.clamp(0.0, 1.0));
                let points = [
                    center - right - up,
                    center + right - up,
                    center - right + up,
                    center + right + up,
                ];
                vertices.extend([0, 1, 2, 1, 3, 2].map(|i| Vertex {
                    position: points[i].to_array(),
                    color: [0.3, 0.008, 0.012],
                    opacity: 0.85,
                    flags: render_mesh::ALPHA | render_mesh::FOG | render_mesh::DOUBLE_SIDED,
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
