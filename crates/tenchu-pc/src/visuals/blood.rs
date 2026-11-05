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
use glam::{Mat3, Mat4, Quat, Vec3};
use std::{
    collections::{BTreeMap, BTreeSet, VecDeque},
    sync::{Arc, OnceLock},
};
use tenchu_core::{effects::Effect, game::Game, math::Matrix};

mod runoff;
mod storage;
mod weapons;
mod wounds;
use runoff::Runoff;
use storage::Marks;
#[cfg(test)]
use storage::VERTEX_LIMIT;
use wounds::Wound;
const WOUND_LIMIT: usize = 64;
const DROP_LIMIT: usize = 128;

#[derive(serde::Serialize, serde::Deserialize, Clone, Default)]
struct Patch {
    part: Option<Part>,
    vertices: Vec<Vertex>,
    sources: Vec<surfaces::SourceRange>,
    #[serde(skip)]
    bounds: OnceLock<[Vec3; 2]>,
}

impl Patch {
    fn visible(
        &self,
        frustum: &crate::renderer::Frustum,
        rotation: Mat3,
        offset: Vec3,
        deformation: Option<&super::motion::Deformation<'_>>,
        wind: f32,
    ) -> bool {
        let [min, max] = *self.bounds.get_or_init(|| {
            self.vertices.iter().fold(
                [Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)],
                |[min, max], vertex| {
                    let point = Vec3::from(vertex.position);
                    [min.min(point), max.max(point)]
                },
            )
        });
        let center = rotation * ((min + max) * 0.5) + offset;
        let local_extent = (max - min) * 0.5;
        let extent = rotation.x_axis.abs() * local_extent.x
            + rotation.y_axis.abs() * local_extent.y
            + rotation.z_axis.abs() * local_extent.z;
        let mut bounds = [center - extent, center + extent];
        if let Some(deformation) = deformation {
            for delta in deformation.pose.offsets {
                bounds[0] += delta.min(Vec3::ZERO);
                bounds[1] += delta.max(Vec3::ZERO);
            }
        }
        frustum.contains(bounds, wind.abs() * 100.0 + 2.0)
    }
}

#[derive(Default)]
pub struct View {
    pub camera: Option<Matrix>,
    pub projection: Option<Mat4>,
    pub alpha: f32,
    pub wind: f32,
}

#[derive(Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Kind {
    Ground,
    Body,
    Death { player: bool },
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Mark(Vec<Patch>, Kind);

impl Mark {
    fn len(&self) -> usize {
        self.0.iter().map(|p| p.vertices.len()).sum()
    }
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Pool {
    age: u16,
    seed: u32,
    mark: Mark,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Drop {
    weapon: bool,
    previous: Vec3,
    position: Vec3,
    velocity: Vec3,
    life: u16,
}

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
struct Sample {
    position: Vec3,
    mode: u8,
    time: i16,
    stained: bool,
}

struct Contact {
    position: Vec3,
    source: Option<usize>,
}

#[derive(Default, Clone, serde::Serialize, serde::Deserialize)]
pub struct Blood {
    tick: Option<u64>,
    health: BTreeMap<usize, i16>,
    pending_wounds: BTreeSet<usize>,
    samples: Vec<Option<Sample>>,
    marks: Marks,
    wounds: VecDeque<Wound>,
    weapons: weapons::Weapons,
    drops: Vec<Drop>,
    runoff: Vec<Runoff>,
    pools: BTreeMap<usize, Pool>,
    dead: BTreeMap<usize, bool>,
    seed: u32,
    #[serde(with = "crate::savestate::json")]
    settings: settings::Blood,
}
impl Blood {
    pub(crate) fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        let Self {
            tick,
            health,
            pending_wounds,
            samples,
            marks,
            wounds,
            weapons,
            drops,
            runoff,
            pools,
            dead,
            seed,
            settings,
        } = self;
        out.value(tick)?;
        out.value(health)?;
        out.value(pending_wounds)?;
        out.value(samples)?;
        marks.capture(out)?;
        out.value(wounds)?;
        out.value(weapons)?;
        out.value(drops)?;
        out.value(runoff)?;
        out.value(pools)?;
        out.value(dead)?;
        out.value(seed)?;
        out.value(&crate::savestate::json::Borrowed(settings))?;
        Ok(())
    }
}

fn translation(matrix: Matrix) -> Vec3 {
    Vec3::from(matrix.translation.map(|v| v as f32).to_array())
}

fn corpse_ready(actor: &tenchu_core::actor::Actor, frame: Option<&Frame>) -> bool {
    if let Some(presented) = super::death::blood::for_actor(frame, actor) {
        return actor.health == 0
            && actor.status == tenchu_core::actor::State::Dead
            && presented.settled;
    }
    actor.health == 0
        && actor.status == tenchu_core::actor::State::Dead
        && actor.motion.loops < 0
        && actor.velocity[1] == 0
        && actor.map.attribute & 12 == 0
        && actor.motion.action != tenchu_core::ids::MOT_DEAD_DROWN
}

fn transforms(
    game: &Game,
    models: &Models,
    frame: Option<&Frame>,
    inspection: bool,
    needed: &BTreeSet<Part>,
) -> BTreeMap<Part, Matrix> {
    let mut parts = BTreeMap::new();
    if needed.is_empty() {
        return parts;
    }
    appearance::visit_parts_filtered(
        game,
        inspection,
        Some(models),
        frame,
        |key| needed.contains(&key.canonical()),
        |key, _, matrix| {
            parts.insert(key.canonical(), matrix);
        },
    );
    render_mesh::visit_hinged_parts(game, |key, _, matrix| {
        if needed.contains(&key) {
            parts.insert(key, Frame::transform(frame, key, matrix));
        }
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
            || previous.death_size != settings.death_size
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
            self.drops.retain(|drop| drop.weapon && settings.weapons);
        }
        if !settings.model_stains && (settings.drips == BloodStyle::Off || settings.seconds == 0) {
            self.pending_wounds.clear();
        }
        if !settings.weapons {
            self.drops.retain(|d| !d.weapon);
            self.weapons.clear();
        }
        if !settings.runoff || ground_changed || !settings.retained_ground() {
            self.runoff.clear();
        }
        self.trim(usize::from(settings.marks.max(32)));
        self.settings = *settings;
    }

    pub fn clear(&mut self) {
        self.marks = Marks::default();
        self.wounds.clear();
        self.pending_wounds.clear();
        self.weapons.clear();
        self.drops.clear();
        self.runoff.clear();
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

    #[cfg(test)]
    pub fn pages(&self) -> impl Iterator<Item = &Arc<Vec<Vertex>>> {
        self.marks.pages()
    }

    pub(crate) fn projected_pages(
        &self,
    ) -> impl Iterator<Item = (&Arc<Vec<Vertex>>, &[surfaces::SourceRange])> {
        self.marks.projected_pages()
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
        if self.settings.ground.uses_original() {
            // The adjacent white sprite is a subtraction mask used only while fading.
            Self::original_texture(
                &mut mark,
                game.mission.images[(self.seed as usize % 2) * 2],
                self.settings.ground,
            );
        }
        self.retain(mark, limit);
        self.start_runoff(surfaces, hinge, hit, radius);
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
                sources: Vec::new(),
                bounds: Default::default(),
            }
        } else {
            let projected = surfaces.world.project_decal(&stamp);
            Patch {
                part: None,
                vertices: projected.vertices,
                sources: projected.sources,
                bounds: Default::default(),
            }
        };
        Mark(vec![patch], Kind::Ground)
    }

    fn original_texture(mark: &mut Mark, image: tenchu_assets::texture::Image, style: BloodStyle) {
        for patch in &mut mark.0 {
            patch.sources.clear();
        }
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
                | render_mesh::STAIN
                | if style == BloodStyle::OriginalHd {
                    render_mesh::BLOOD_HD
                } else {
                    0
                };
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
        frame: Option<&Frame>,
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
        appearance::visit_parts(game, true, Some(models), frame, |key, geometry, matrix| {
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
        let projection = stamp.prepare();
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
                projection.project(&[triangle[0], triangle[1], triangle[2]], &mut projected);
            }
            for v in &mut projected {
                v.position = (inverse * (Vec3::from(v.position) - offset)).to_array();
                v.normal = (r.transpose() * Vec3::from(v.normal)).to_array();
            }
            if !projected.is_empty() {
                patches.push(Patch {
                    part: Some(key),
                    vertices: projected,
                    sources: Vec::new(),
                    bounds: Default::default(),
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
            normal: r.transpose() * normal,
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
        frame: Option<&Frame>,
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
            self.samples = super::death::blood::slots(&game.effects, frame)
                .map(|slot| sample(slot, true))
                .collect();
            return;
        }
        self.tick = Some(game.tick);
        let receivers = std::cell::LazyCell::new(|| Receivers::new(game, models, surfaces));
        let limit = usize::from(settings.marks.max(32));
        let amount = f32::from(settings.amount.min(200)) / 100.0;
        let mut new_blood = Vec::new();
        let mut samples = Vec::with_capacity(self.samples.len());
        let mut impacts = 0;
        for (index, slot) in super::death::blood::slots(&game.effects, frame).enumerate() {
            let mut current = sample(slot, false);
            if let Some(next) = &mut current {
                let old = self.samples.get(index).copied().flatten();
                let continuous = old.filter(|old| {
                    (old.mode != next.mode || next.time <= old.time + 1)
                        && (old.mode == next.mode || old.mode + 1 == next.mode)
                        && old.position.distance_squared(next.position) < 1000.0_f32.powi(2)
                });
                if continuous.is_none() && next.mode == 0 {
                    new_blood.push(Contact {
                        position: next.position,
                        source: game.effects.gore_sources.get(index).copied().flatten(),
                    });
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
        let wounds =
            settings.model_stains || settings.drips != BloodStyle::Off && settings.seconds != 0;
        for &id in &game.roster.order {
            let actor = game.roster.get(id);
            let damaged = self
                .health
                .get(&actor.root_model)
                .is_some_and(|&old| old > actor.health);
            let stealth = actor.health == 0
                && actor.motion.loops >= 0
                && (tenchu_core::ids::MOT_DEAD_STEALTH_BACK
                    ..=tenchu_core::ids::MOT_DEAD_STEALTH_SIDE_AYAME)
                    .contains(&actor.motion.action);
            if !stealth {
                self.pending_wounds.remove(&actor.root_model);
            } else if wounds && damaged {
                self.pending_wounds.insert(actor.root_model);
            }
            let pending = self.pending_wounds.contains(&actor.root_model);
            if wounds && (damaged || pending) {
                let center =
                    Vec3::from(actor.position.map(|v| v as f32).to_array()) - Vec3::Y * 750.0;
                if let Some(position) = new_blood
                    .iter()
                    .filter(|contact| {
                        !pending
                            || contact
                                .source
                                .is_some_and(|part| actor.parts.contains(&part))
                    })
                    .map(|contact| contact.position)
                    .filter(|p| p.distance_squared(center) < 1600.0_f32.powi(2))
                    .min_by(|a, b| {
                        a.distance_squared(center)
                            .total_cmp(&b.distance_squared(center))
                    })
                {
                    self.pending_wounds.remove(&actor.root_model);
                    self.wound(game, models, actor.root_model, position, settings, frame);
                }
            }
        }
        self.health = health;
        self.pending_wounds
            .retain(|root| self.health.contains_key(root));
        self.dead
            .retain(|root, _| self.health.get(root) == Some(&0));
        let hits = if settings.weapons {
            weapons::contacts(game, &new_blood)
        } else {
            Default::default()
        };
        let weapons = settings.weapons && (!self.weapons.is_empty() || !hits.is_empty());
        let transforms =
            if self.wounds.is_empty() && !weapons {
                BTreeMap::new()
            } else {
                let mut needed: BTreeSet<_> = self.wounds.iter().map(|wound| wound.part).collect();
                if weapons {
                    needed.extend(self.weapons.parts());
                    for &id in game.roster.order.iter().filter(|id| hits.contains(id)) {
                        let actor = game.roster.get(id);
                        needed.extend(actor.weapons[..2].iter().flatten().map(|weapon| {
                            Part::Actor {
                                root: actor.root_model,
                                model: weapon.model,
                                kind: weapon.kind,
                                inspection: false,
                            }
                        }));
                    }
                }
                transforms(game, models, frame, true, &needed)
            };
        if weapons {
            self.weapons
                .tick(game, models, &transforms, &hits, &mut self.drops);
        }
        self.marks.retain_attached(|mark| {
            mark.0.iter().all(|p| {
                p.part.is_none_or(|key| match key {
                    Part::Actor { root, .. } => self.health.contains_key(&root),
                    _ => true,
                })
            })
        });
        self.emit_wounds(game.tick, settings, &transforms);
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
        if !self.runoff.is_empty() {
            self.advance_runoff(game, &receivers, limit);
        }
        if !self.pools.is_empty()
            || game.roster.actors.iter().flatten().any(|actor| {
                corpse_ready(actor, frame)
                    && !self.dead.contains_key(&actor.root_model)
                    && settings.custom_death(actor.id == game.player_id)
            })
        {
            self.update_pools(game, &receivers, amount, limit, frame);
        }
        self.trim(limit);
    }

    fn update_pools(
        &mut self,
        game: &Game,
        surfaces: &Receivers<'_>,
        amount: f32,
        limit: usize,
        frame: Option<&Frame>,
    ) {
        for &id in &game.roster.order {
            let actor = game.roster.get(id);
            if !corpse_ready(actor, frame) || !self.settings.custom_death(id == game.player_id) {
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
                        age: if super::death::blood::for_actor(frame, actor).is_some() {
                            24
                        } else {
                            0
                        },
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
            if pool.age < 24
                || (actor.velocity[1] != 0
                    && super::death::blood::for_actor(frame, actor).is_none())
            {
                return pool.age < 300;
            }
            let center = actor
                .motion
                .pose
                .matrices(&game.math, actor.root(&game.math))
                .first()
                .copied()
                .map(|matrix| {
                    translation(Frame::transform(
                        frame,
                        Part::Actor {
                            root: actor.root_model,
                            model: actor.parts[0],
                            kind: actor.mesh_kind,
                            inspection: true,
                        },
                        matrix,
                    ))
                })
                .unwrap_or_else(|| Vec3::from(actor.position.map(|v| v as f32).to_array()));
            let floor = super::death::blood::for_actor(frame, actor)
                .map_or(actor.position.y, |p| p.position.y);
            let center = Vec3::new(center.x, floor as f32, center.z);
            if let Some(hit) = surfaces
                .world
                .cast(center - Vec3::Y * 800.0, center + Vec3::Y * 500.0)
                && hit.flags & render_mesh::WATER == 0
                && hit.normal.y < -0.3
            {
                let growth = (f32::from(pool.age - 24) / 66.0).clamp(0.0, 1.0);
                let size = f32::from(self.settings.death_size) / 100.0;
                let radius = (f32::from(game.mission.characters[actor.definition].width) * 1.4)
                    .clamp(300.0, 900.0)
                    * amount.sqrt()
                    * size
                    * (0.2 + 0.8 * growth.sqrt());
                pool.mark = if self.settings.death.uses_original() {
                    let mut mark =
                        Self::original_pool(game, actor, hit, growth, amount * size * size);
                    if self.settings.death == BloodStyle::OriginalHd {
                        for vertex in mark.0.iter_mut().flat_map(|patch| &mut patch.vertices) {
                            vertex.flags |= render_mesh::BLOOD_HD;
                        }
                    }
                    mark
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
                sources: Vec::new(),
                bounds: Default::default(),
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
        geometry.append(self.animated_geometry(
            game,
            models,
            frame,
            View {
                camera,
                alpha,
                ..Default::default()
            },
            Vec::new(),
        ));
        geometry
    }

    pub fn animated_geometry(
        &self,
        game: &Game,
        models: &Models,
        frame: Option<&Frame>,
        view: View,
        mut vertices: Vec<Vertex>,
    ) -> Geometry {
        let camera = view.camera;
        let alpha = view.alpha;
        let frustum = view.projection.map(crate::renderer::Frustum::new);
        vertices.clear();
        vertices.reserve(self.marks.attached.iter().map(|(_, mark)| mark.len()).sum());
        for pool in self.pools.values() {
            vertices.extend(pool.mark.0.iter().flat_map(|p| p.vertices.iter().copied()));
        }
        if !self.marks.attached.is_empty() || !self.drops.is_empty() || !self.weapons.is_empty() {
            let transforms = if !self.marks.attached.is_empty() || !self.weapons.is_empty() {
                let needed = self
                    .marks
                    .attached
                    .iter()
                    .flat_map(|(_, mark)| mark.0.iter().filter_map(|patch| patch.part))
                    .chain(self.weapons.parts())
                    .collect();
                transforms(
                    game,
                    models,
                    frame,
                    camera.is_some() || game.roster.keep_all_active,
                    &needed,
                )
            } else {
                BTreeMap::new()
            };
            self.weapons.append(&mut vertices, &transforms);
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
                    let deformation = patch.part.and_then(|key| {
                        transforms
                            .get(&key)
                            .and_then(|&matrix| models.deformation(key, matrix))
                    });
                    if frustum.as_ref().is_some_and(|frustum| {
                        !patch.visible(frustum, rotation, offset, deformation.as_ref(), view.wind)
                    }) {
                        continue;
                    }
                    let normal = rotation.inverse().transpose();
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
                let modern = drop.weapon || self.settings.drips == BloodStyle::Modern;
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
                        | if modern { render_mesh::SOFT } else { 0 }
                        | if !drop.weapon && self.settings.drips == BloodStyle::OriginalHd {
                            render_mesh::STAIN | render_mesh::BLOOD_HD
                        } else {
                            0
                        },
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
