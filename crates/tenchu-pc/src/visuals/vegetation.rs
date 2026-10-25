mod bamboo;
pub mod botanical;
use super::{models::Material, pack::Pack};
use crate::render_mesh::{self, Vertex};
use glam::{Mat3, Vec3};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Off,
    Leaves,
    Blossoms,
    Autumn,
    Bamboo,
    Grass,
    Shrub,
    Trunk,
}

pub fn rules(pack: &Pack, artwork: bool) -> BTreeMap<String, Kind> {
    let mut rules: BTreeMap<_, _> =
        serde_json::from_str(include_str!("../../data/vegetation.json")).unwrap();
    if artwork {
        rules.extend(pack.vegetation.clone());
    }
    rules
}

fn point(position: [f32; 3]) -> [u32; 3] {
    position.map(|n| if n == 0.0 { 0 } else { n.to_bits() })
}

fn root(parents: &mut [usize], mut i: usize) -> usize {
    while parents[i] != i {
        parents[i] = parents[parents[i]];
        i = parents[i];
    }
    i
}

pub fn wind(vertices: &mut [Vertex], materials: &[Material], rules: &BTreeMap<String, Kind>) {
    let kinds: BTreeMap<_, _> = materials
        .iter()
        .enumerate()
        .filter_map(|(i, m)| {
            rules
                .get(&m.id)
                .filter(|&&kind| kind != Kind::Off)
                .map(|&kind| (i as u32 + 1, kind))
        })
        .collect();
    let (top, bottom) = vertices
        .iter()
        .filter(|v| kinds.contains_key(&v.material))
        .fold((f32::INFINITY, f32::NEG_INFINITY), |(a, b), v| {
            (a.min(v.position[1]), b.max(v.position[1]))
        });
    let height = (bottom - top).max(100.0);
    for vertex in vertices {
        if let Some(&kind) = kinds.get(&vertex.material) {
            let weight =
                (((bottom - vertex.position[1]) / height).clamp(0.0, 1.0) * 255.0).round() as u32;
            let weight = if kind == Kind::Trunk {
                weight / 4
            } else {
                weight
            };
            vertex.flags |= render_mesh::FOLIAGE | weight << render_mesh::WIND_WEIGHT_SHIFT;
            if kind != Kind::Trunk {
                vertex.flags |= render_mesh::FLUTTER;
            }
        }
    }
}

pub fn prepare(
    vertices: &mut Vec<Vertex>,
    materials: &[Material],
    rules: &BTreeMap<String, Kind>,
    detail: u16,
    budget: &mut usize,
) {
    let modern = detail >= 3;
    let dense = detail >= 4;
    let detail = detail.min(2);
    let mut groups: BTreeMap<Kind, Vec<Vertex>> = BTreeMap::new();
    let mut unchanged = Vec::new();
    for triangle in vertices.chunks_exact(3) {
        let kind = triangle[0]
            .material
            .checked_sub(1)
            .and_then(|i| materials.get(i as usize))
            .and_then(|m| rules.get(&m.id));
        if let Some(kind) = kind.filter(|k| **k != Kind::Off) {
            groups.entry(*kind).or_default().extend_from_slice(triangle);
        } else {
            unchanged.extend_from_slice(triangle);
        }
    }
    for (kind, mut group) in groups {
        if matches!(kind, Kind::Trunk | Kind::Shrub) {
            super::refine::prepare(
                &mut group,
                if kind == Kind::Shrub { detail } else { 1 },
                if kind == Kind::Shrub { 0.08 } else { 0.025 },
                budget,
            );
            if kind == Kind::Shrub {
                for vertex in &mut group {
                    vertex.flags |= render_mesh::FOLIAGE;
                }
            }
            if modern && kind == Kind::Shrub {
                group = botanical::canopy(&group, materials, kind, dense, budget);
            }
            unchanged.extend(group);
            continue;
        }
        let mut parents: Vec<_> = (0..group.len() / 3).collect();
        let mut shared: BTreeMap<[u32; 3], Vec<(usize, Vec3)>> = BTreeMap::new();
        for (i, t) in group.chunks_exact(3).enumerate() {
            let [a, b, c] = std::array::from_fn(|j| Vec3::from(t[j].position));
            let normal = (b - a).cross(c - a).normalize_or_zero();
            for v in t {
                let adjacent = shared.entry(point(v.position)).or_default();
                for &(j, other) in adjacent.iter() {
                    if normal.dot(other).abs() > 0.98 {
                        let a = root(&mut parents, i);
                        let b = root(&mut parents, j);
                        parents[b] = a;
                    }
                }
                adjacent.push((i, normal));
            }
        }
        let mut clusters: BTreeMap<usize, Vec<Vertex>> = BTreeMap::new();
        for (i, t) in group.chunks_exact(3).enumerate() {
            clusters
                .entry(root(&mut parents, i))
                .or_default()
                .extend_from_slice(t);
        }
        for mut cluster in clusters.into_values() {
            let min = cluster.iter().fold(Vec3::splat(f32::INFINITY), |p, v| {
                p.min(Vec3::from(v.position))
            });
            let max = cluster.iter().fold(Vec3::splat(f32::NEG_INFINITY), |p, v| {
                p.max(Vec3::from(v.position))
            });
            let center = (min + max) * 0.5;
            let span = (max - min).max_element();
            let [a, b, c] = std::array::from_fn(|i| Vec3::from(cluster[i].position));
            let normal = (b - a).cross(c - a).normalize_or_zero();
            let axis = if normal.y.abs() < 0.7 {
                Vec3::Y
            } else {
                (b - a).normalize_or_zero()
            };
            if modern && kind == Kind::Bamboo {
                unchanged.extend(bamboo::stalks(&cluster, materials, budget));
                continue;
            }
            if modern && kind != Kind::Grass {
                unchanged.extend(botanical::canopy(&cluster, materials, kind, dense, budget));
                continue;
            }
            let cost = cluster.len() * (usize::from(detail) + 1) * 4;
            if span < 20.0 || cost > *budget {
                unchanged.extend(cluster);
                continue;
            }
            *budget -= cost;
            super::smooth::curve(&mut cluster, true, 2, 0.0, 0.0);
            for vertex in &mut cluster {
                vertex.flags |= render_mesh::FOLIAGE | render_mesh::DOUBLE_SIDED;
            }
            let pivot = if kind == Kind::Grass {
                Vec3::new(center.x, max.y, center.z)
            } else {
                center
            };
            unchanged.extend_from_slice(&cluster);
            for layer in 0..detail {
                let sign = if layer == 0 { 1.0 } else { -1.0 };
                let rotation = Mat3::from_axis_angle(axis, sign * 0.46);
                for &v in &cluster {
                    let p = Vec3::from(v.position);
                    let rotated = pivot + rotation * (p - pivot) + normal * (sign * span * 0.025);
                    unchanged.push(Vertex {
                        position: (p + (rotated - p).clamp_length_max(span.min(6000.0) * 0.24))
                            .to_array(),
                        normal: (rotation * Vec3::from(v.normal)).to_array(),
                        ..v
                    });
                }
            }
        }
    }
    *vertices = unchanged;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    #[test]
    fn foliage_volume_keeps_connected_texture_tiles_together_and_obeys_budget() {
        let mut source = Vec::new();
        for material in 1..=2 {
            let x = (material - 1) as f32 * 100.0;
            let p = [
                [x, 0.0, 0.0],
                [x + 100.0, 0.0, 0.0],
                [x, -100.0, 0.0],
                [x + 100.0, -100.0, 0.0],
            ];
            source.extend([0, 1, 2, 1, 3, 2].map(|i| Vertex {
                position: p[i],
                uv: [(i % 2) as f32, (i / 2) as f32],
                material,
                ..Vertex::zeroed()
            }));
        }
        let materials: Vec<_> = ["a", "b"]
            .into_iter()
            .map(|id| Material {
                id: id.into(),
                region: None,
                image: std::sync::Arc::new(super::super::pixels::Image {
                    size: [1, 1],
                    rgba: vec![255; 4],
                }),
            })
            .collect();
        let rules = BTreeMap::from([("a".into(), Kind::Leaves), ("b".into(), Kind::Leaves)]);
        let mut foliage = source.clone();
        let mut budget = 1000;
        prepare(&mut foliage, &materials, &rules, 2, &mut budget);
        assert_eq!(foliage.len(), source.len() * 12);
        assert_eq!(foliage.len() + budget, 1000);
        for layer in foliage.chunks_exact(source.len() * 4) {
            let left: std::collections::BTreeSet<_> = layer
                .iter()
                .filter(|v| v.material == 1)
                .map(|v| point(v.position))
                .collect();
            let right: std::collections::BTreeSet<_> = layer
                .iter()
                .filter(|v| v.material == 2)
                .map(|v| point(v.position))
                .collect();
            assert_eq!(left.intersection(&right).count(), 3);
        }
        assert!(
            foliage
                .iter()
                .all(|v| v.uv.iter().all(|u| (0.0..=1.0).contains(u)))
        );
        let mut limited = source.clone();
        prepare(&mut limited, &materials, &rules, 2, &mut 0);
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(&limited),
            bytemuck::cast_slice::<_, u8>(&source)
        );
    }
}
