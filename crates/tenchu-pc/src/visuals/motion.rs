use super::{
    appearance,
    interpolation::Part,
    models::{Material, Models, rotation},
    pack::Pack,
};
use crate::{render_mesh::Vertex, settings};
use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tenchu_assets::Result;
use tenchu_core::{game::Game, math::Matrix};

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Hair,
    Cloth,
    Equipment,
}

impl Kind {
    fn enabled(self, options: &settings::Motion) -> bool {
        match self {
            Self::Hair => options.hair,
            Self::Cloth => options.cloth,
            Self::Equipment => options.equipment,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Rule {
    pub kind: Kind,
    pub axis: [f32; 3],
    pub start: f32,
    pub end: f32,
    pub anchor: [f32; 3],
    pub limit: f32,
    #[serde(default)]
    pub absolute: bool,
    #[serde(default)]
    pub materials: Vec<String>,
}

impl Rule {
    pub fn validate(&self) -> Result<()> {
        if self
            .axis
            .iter()
            .chain(&self.anchor)
            .chain([&self.start, &self.end, &self.limit])
            .any(|v| !v.is_finite() || v.abs() > 100000.0)
            || !(0.99..=1.01).contains(&Vec3::from(self.axis).length())
            || self.end - self.start < 1.0
            || self.limit <= 0.0
            || self.limit > 100.0
            || self.limit > (self.end - self.start) * 0.25
            || self.materials.len() > 64
        {
            return Err(
                "Invalid secondary-motion axis, interval, displacement limit or material list"
                    .into(),
            );
        }
        Ok(())
    }

    fn influence(&self, position: Vec3) -> (f32, Vec3) {
        let axis = Vec3::from(self.axis);
        let signed = position.dot(axis);
        let distance = if self.absolute { signed.abs() } else { signed };
        let t = ((distance - self.start) / (self.end - self.start)).clamp(0.0, 1.0);
        let gradient = axis
            * (6.0 * t * (1.0 - t) / (self.end - self.start))
            * if self.absolute { signed.signum() } else { 1.0 };
        (t * t * (3.0 - 2.0 * t), gradient)
    }
}

pub fn rules(pack: &Pack, artwork: bool) -> BTreeMap<String, Vec<Rule>> {
    let mut rules: BTreeMap<_, _> =
        serde_json::from_str(include_str!("../../data/motion.json")).unwrap();
    if artwork {
        rules.extend(pack.motion.clone());
    }
    rules
}

pub struct BoundRule {
    pub rule: Rule,
    materials: Option<Vec<u32>>,
}

impl BoundRule {
    pub fn new(rule: &Rule, materials: &[Material]) -> Option<Self> {
        let ids: Vec<_> = materials
            .iter()
            .enumerate()
            .filter(|(_, m)| rule.materials.contains(&m.id))
            .map(|(i, _)| i as u32 + 1)
            .collect();
        if !rule.materials.is_empty() && ids.is_empty() {
            return None;
        }
        Some(Self {
            rule: rule.clone(),
            materials: (!rule.materials.is_empty()).then_some(ids),
        })
    }

    pub fn bind(&mut self, handles: &[u32]) {
        if let Some(materials) = &mut self.materials {
            for id in materials {
                *id = handles[*id as usize - 1];
            }
        }
    }
}

#[derive(Clone)]
pub struct Pose {
    pub geometry: usize,
    pub offsets: [Vec3; 4],
}

pub type Frame = BTreeMap<Part, Pose>;

pub struct Deformation<'a> {
    pub rules: &'a [BoundRule],
    pub pose: &'a Pose,
    inverse: Mat3,
    translation: Vec3,
}

impl<'a> Deformation<'a> {
    pub fn new(rules: &'a [BoundRule], pose: &'a Pose, matrix: Matrix) -> Option<Self> {
        let rotation = rotation(matrix);
        (rotation.determinant().abs() > 0.0001).then(|| Self {
            rules,
            pose,
            inverse: rotation.inverse(),
            translation: Vec3::from(matrix.translation.map(|v| v as f32).to_array()),
        })
    }

    pub fn vertex(&self, vertex: &mut Vertex) {
        let position = Vec3::from(vertex.position);
        let local = self.inverse * (position - self.translation);
        let mut p = position;
        let mut normal = Vec3::from(vertex.normal);
        for (bound, &delta) in self.rules.iter().zip(&self.pose.offsets) {
            if delta == Vec3::ZERO
                || bound
                    .materials
                    .as_ref()
                    .is_some_and(|ids| !ids.contains(&vertex.material))
            {
                continue;
            }
            let (weight, gradient) = bound.rule.influence(local);
            p += delta * weight;
            let gradient = self.inverse.transpose() * gradient;
            normal -= gradient * (normal.dot(delta) / (1.0 + gradient.dot(delta)).max(0.25));
        }
        vertex.position = p.to_array();
        vertex.normal = normal.normalize_or_zero().to_array();
    }
}

struct Input {
    key: (Part, usize),
    geometry: usize,
    anchor: Vec3,
    limit: f32,
}

struct Spring {
    geometry: usize,
    anchor: Vec3,
    drive: Vec3,
    velocity: Vec3,
    previous: Vec3,
    offset: Vec3,
}

impl Spring {
    fn new(input: &Input) -> Self {
        Self {
            geometry: input.geometry,
            anchor: input.anchor,
            drive: Vec3::ZERO,
            velocity: Vec3::ZERO,
            previous: Vec3::ZERO,
            offset: Vec3::ZERO,
        }
    }

    fn step(&mut self, input: &Input, response: u16, wind: Vec3) {
        if self.geometry != input.geometry || self.anchor.distance(input.anchor) > 2000.0 {
            *self = Self::new(input);
            return;
        }
        let drive = (input.anchor - self.anchor) * 30.0;
        let acceleration = (drive - self.drive) * 30.0;
        self.drive = drive;
        self.anchor = input.anchor;
        self.offset = self.offset.clamp_length_max(input.limit);
        self.previous = self.offset;
        let omega = 6.0 + f32::from(response) * 0.08;
        let forcing = -acceleration.clamp_length_max(100000.0) * 0.08 + wind * omega * omega;
        for _ in 0..4 {
            self.velocity +=
                (forcing - self.offset * omega * omega - self.velocity * omega * 1.4) / 120.0;
            self.offset += self.velocity / 120.0;
            if self.offset.length_squared() > input.limit * input.limit {
                self.offset = self.offset.clamp_length_max(input.limit);
                let normal = self.offset.normalize_or_zero();
                self.velocity -= normal * self.velocity.dot(normal).max(0.0);
            }
        }
    }
}

#[derive(Default)]
pub struct Motion {
    tick: Option<u64>,
    springs: BTreeMap<(Part, usize), Spring>,
}

impl Motion {
    pub fn tick(&mut self, game: &Game, models: &Models, options: &settings::Motion) {
        let mut inputs = Vec::new();
        if options.actors() {
            appearance::visit_parts(game, true, Some(models), None, |key, geometry, matrix| {
                if let Some(rules) = models.motion_rules(geometry.as_ptr() as usize) {
                    let orientation = rotation(matrix);
                    let translation = Vec3::from(matrix.translation.map(|v| v as f32).to_array());
                    for (i, bound) in rules.iter().enumerate() {
                        if bound.rule.kind.enabled(options) {
                            inputs.push(Input {
                                key: (key.canonical(), i),
                                geometry: geometry.as_ptr() as usize,
                                anchor: orientation * Vec3::from(bound.rule.anchor) + translation,
                                limit: bound.rule.limit * f32::from(options.strength) / 100.0,
                            });
                        }
                    }
                }
            });
        }
        inputs.truncate(256);
        self.update(game.tick, &inputs, options);
    }

    fn update(&mut self, tick: u64, inputs: &[Input], options: &settings::Motion) {
        if self.tick == Some(tick) {
            return;
        }
        if self.tick.is_none_or(|previous| previous + 1 != tick) {
            self.springs.clear();
        }
        self.tick = Some(tick);
        let active: BTreeSet<_> = inputs.iter().map(|input| input.key).collect();
        self.springs.retain(|key, _| active.contains(key));
        let time = tick as f64 / 30.0;
        for input in inputs {
            let phase = (input.anchor.x + input.anchor.z) * 0.001;
            let wind = Vec3::new(
                (time * 1.3 + f64::from(phase)).sin() as f32,
                0.0,
                (time * 0.9 + f64::from(phase)).cos() as f32,
            ) * (input.limit * f32::from(options.wind) / 400.0);
            self.springs
                .entry(input.key)
                .or_insert_with(|| Spring::new(input))
                .step(input, options.response, wind);
        }
    }

    pub fn sample(&self, alpha: f32) -> Frame {
        let mut frame = BTreeMap::new();
        for (&(part, index), spring) in &self.springs {
            let pose = frame.entry(part).or_insert(Pose {
                geometry: spring.geometry,
                offsets: [Vec3::ZERO; 4],
            });
            pose.offsets[index] = spring.previous.lerp(spring.offset, alpha.clamp(0.0, 1.0));
        }
        frame
    }
}

#[cfg(test)]
mod tests;
