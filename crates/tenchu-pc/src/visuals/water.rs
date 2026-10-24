use super::{models::Material, pack::Pack, smooth};
use crate::{
    render_effects::{Batch, Geometry},
    render_mesh::{self, Vertex},
    settings,
};
use glam::{Mat4, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Off,
    Surface,
    Waterfall,
}

pub fn rules(pack: &Pack, artwork: bool) -> BTreeMap<String, Kind> {
    let mut rules: BTreeMap<_, _> =
        serde_json::from_str(include_str!("../../data/water.json")).unwrap();
    if artwork {
        rules.extend(pack.water.clone());
    }
    rules
}

pub fn prepare(
    vertices: &mut Vec<Vertex>,
    materials: &[Material],
    rules: &BTreeMap<String, Kind>,
    settings: &settings::Water,
    budget: &mut usize,
) {
    for vertex in vertices.iter_mut() {
        if let Some(material) = vertex
            .material
            .checked_sub(1)
            .and_then(|i| materials.get(i as usize))
        {
            match rules.get(&material.id) {
                Some(Kind::Surface) => vertex.flags |= render_mesh::WATER,
                Some(Kind::Waterfall) => {
                    vertex.flags |= render_mesh::WATER | render_mesh::WATERFALL
                }
                _ => {}
            }
        }
    }
    if settings.detail == 0 {
        return;
    }
    let mut groups: BTreeMap<(u32, bool), Vec<Vertex>> = BTreeMap::new();
    let mut unchanged = Vec::with_capacity(vertices.len());
    for triangle in vertices.chunks_exact(3) {
        let v = triangle[0];
        if v.flags & render_mesh::WATERFALL != 0 {
            groups
                .entry(((v.page >> 5) & 3, v.flags & render_mesh::TRANSLUCENT != 0))
                .or_default()
                .extend_from_slice(triangle);
        } else {
            unchanged.extend_from_slice(triangle);
        }
    }
    for mut group in groups.into_values() {
        let divisions = if settings.detail == 1 { 4 } else { 8 };
        let cost = group.len() * divisions * divisions;
        if cost <= *budget {
            smooth::curve(&mut group, true, divisions, 1.0, 250.0);
            for vertex in &mut group {
                vertex.normal = [0.0; 3];
            }
            smooth::prepare(&mut group, true, 0);
            *budget -= cost;
        }
        unchanged.extend(group);
    }
    *vertices = unchanged;
}

#[derive(Clone, Copy)]
struct Edge {
    a: Vec3,
    b: Vec3,
    impact: bool,
}

#[derive(Default)]
pub struct Spray {
    edges: Vec<Edge>,
}

fn point(position: [f32; 3]) -> [i32; 3] {
    position.map(|n| n.round() as i32)
}

fn root(parents: &mut [usize], mut index: usize) -> usize {
    while parents[index] != index {
        parents[index] = parents[parents[index]];
        index = parents[index];
    }
    index
}

impl Spray {
    pub fn new(vertices: &[Vertex]) -> Self {
        let mut edges: BTreeMap<[[i32; 3]; 2], (Edge, u32, bool)> = BTreeMap::new();
        let mut triangles = std::collections::BTreeSet::new();
        let mut points = BTreeMap::new();
        let mut parents = Vec::new();
        let mut heights: Vec<(f32, f32)> = Vec::new();
        for triangle in vertices
            .chunks_exact(3)
            .filter(|t| t[0].flags & render_mesh::WATERFALL != 0)
        {
            let mut key: [_; 3] = std::array::from_fn(|i| point(triangle[i].position));
            key.sort();
            if !triangles.insert(key) {
                continue;
            }
            let nodes: [usize; 3] = key.map(|key| {
                *points.entry(key).or_insert_with(|| {
                    let index = parents.len();
                    parents.push(index);
                    heights.push((key[1] as f32, key[1] as f32));
                    index
                })
            });
            for other in &nodes[1..] {
                let a = root(&mut parents, nodes[0]);
                let b = root(&mut parents, *other);
                if a != b {
                    parents[b] = a;
                    heights[a] = (
                        heights[a].0.min(heights[b].0),
                        heights[a].1.max(heights[b].1),
                    );
                }
            }
            let center_y = triangle.iter().map(|v| v.position[1]).sum::<f32>() / 3.0;
            for i in 0..3 {
                let a = triangle[i].position;
                let b = triangle[(i + 1) % 3].position;
                let mut key = [point(a), point(b)];
                key.sort();
                let downstream = center_y < (a[1] + b[1]) * 0.5 - 10.0;
                let entry = edges.entry(key).or_insert((
                    Edge {
                        a: Vec3::from(a),
                        b: Vec3::from(b),
                        impact: false,
                    },
                    0,
                    downstream,
                ));
                entry.1 += 1;
            }
        }
        let mut out = Vec::new();
        for (mut edge, count, downstream) in edges.into_values() {
            let (top, bottom) = heights[root(&mut parents, points[&point(edge.a.to_array())])];
            if count != 1 || bottom - top <= 500.0 || edge.a.distance(edge.b) < 30.0 {
                continue;
            }
            edge.impact =
                downstream && (edge.a.y - edge.b.y).abs() < 50.0 && edge.a.y >= bottom - 100.0;
            let steep = (edge.a.y - edge.b.y).abs() > edge.a.distance(edge.b) * 0.8;
            if edge.impact || steep {
                out.push(edge);
            }
        }
        out.truncate(256);
        Self { edges: out }
    }

    pub fn geometry(&self, time: f32, camera: Mat4, settings: &settings::Water) -> Geometry {
        let mut vertices = Vec::new();
        if !settings.enabled || settings.spray == 0 {
            return Geometry {
                vertices,
                batches: Vec::new(),
            };
        }
        let inverse = camera.inverse();
        let near = inverse.project_point3(Vec3::ZERO);
        let forward = (inverse.project_point3(Vec3::Z) - near).normalize_or_zero();
        let right = forward.cross(Vec3::NEG_Y).normalize_or_zero();
        let up = right.cross(forward).normalize_or_zero();
        let mut nearby: Vec<_> = self
            .edges
            .iter()
            .enumerate()
            .filter(|(_, e)| ((e.a + e.b) * 0.5).distance_squared(near) < 25000.0_f32.powi(2))
            .collect();
        nearby.sort_unstable_by(|(_, a), (_, b)| {
            ((a.a + a.b) * 0.5)
                .distance_squared(near)
                .total_cmp(&((b.a + b.b) * 0.5).distance_squared(near))
        });
        for (index, edge) in nearby.into_iter().take(32) {
            let density = if edge.impact { 90.0 } else { 300.0 };
            let per_edge = (edge.a.distance(edge.b) / density).ceil() as usize;
            let per_edge = per_edge.clamp(1, [8, 12, 20][usize::from(settings.detail.min(2))]);
            for particle in 0..per_edge {
                let seed = (index * 31 + particle * 17) as f32;
                let phase = (time * f32::from(settings.flow) / 200.0 + seed * 0.618034).fract();
                let age = phase * 1.6;
                let along = (seed * 0.754878).fract();
                let base = edge.a.lerp(edge.b, along);
                let angle = seed * 2.399963;
                let velocity = Vec3::new(
                    angle.cos() * 180.0,
                    if edge.impact {
                        -300.0 - along * 140.0
                    } else {
                        300.0
                    },
                    angle.sin() * 180.0,
                );
                let position = base + velocity * age + Vec3::Y * (180.0 * age * age);
                let size = 12.0 + age * 120.0;
                super::weather::quad(
                    &mut vertices,
                    position,
                    right * size,
                    up * size,
                    [0.65, 0.72, 0.78],
                    phase.mul_add(-1.0, 1.0) * phase * f32::from(settings.spray)
                        / if edge.impact { 120.0 } else { 360.0 },
                );
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

#[cfg(test)]
mod tests;
