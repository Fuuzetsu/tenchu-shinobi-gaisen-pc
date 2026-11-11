use super::appearance::skin::Influence;
use crate::render_mesh::Vertex;
use glam::Vec3;
use std::collections::{BTreeMap, BTreeSet};

type Point = [u32; 3];

// Loop masks, with pinned joints and bounded relaxation instead of limit-surface shrinkage.
// https://www.microsoft.com/en-us/research/publication/smooth-subdivision-surfaces-based-on-triangles/

fn key(p: [f32; 3]) -> Point {
    p.map(|v| if v == 0.0 { 0 } else { v.to_bits() })
}

fn edge(a: Point, b: Point) -> [Point; 2] {
    if a < b { [a, b] } else { [b, a] }
}

fn continuous(adjacent: &[(Point, Vec3)]) -> bool {
    adjacent.len() == 2 && adjacent[0].1.dot(adjacent[1].1) > -0.5
}

pub fn prepare(vertices: &mut Vec<Vertex>, levels: u16, maximum: f32, budget: &mut usize) {
    prepare_skinned(vertices, levels, maximum, budget, &mut Vec::new());
}

pub fn prepare_skinned(
    vertices: &mut Vec<Vertex>,
    levels: u16,
    maximum: f32,
    budget: &mut usize,
    skin: &mut Vec<Influence>,
) {
    refine(vertices, levels, maximum, budget, false, skin);
}

pub fn preserve_height(
    vertices: &mut Vec<Vertex>,
    maximum: f32,
    budget: &mut usize,
    skin: &mut Vec<Influence>,
) {
    refine(vertices, 3, maximum, budget, true, skin);
}

fn refine(
    vertices: &mut Vec<Vertex>,
    levels: u16,
    maximum: f32,
    budget: &mut usize,
    preserve_height: bool,
    skin: &mut Vec<Influence>,
) {
    let cost = vertices
        .len()
        .saturating_mul(4_usize.pow(u32::from(levels.min(3))));
    if levels == 0 || cost > *budget || vertices.is_empty() {
        return;
    }
    *budget -= cost;
    let min = vertices.iter().fold(Vec3::splat(f32::INFINITY), |p, v| {
        p.min(Vec3::from(v.position))
    });
    let max = vertices
        .iter()
        .fold(Vec3::splat(f32::NEG_INFINITY), |p, v| {
            p.max(Vec3::from(v.position))
        });
    let limit = (max - min).max_element() * maximum;
    for _ in 0..levels.min(3) {
        subdivide(vertices, limit, preserve_height, skin);
    }
    super::smooth::prepare(vertices, true, 0);
}

fn subdivide(
    vertices: &mut Vec<Vertex>,
    limit: f32,
    preserve_height: bool,
    skin: &mut Vec<Influence>,
) {
    let relax = |from: Vec3, to: Vec3| {
        let mut delta = (to - from) * 0.7;
        if preserve_height {
            delta.y = 0.0;
        }
        from + delta.clamp_length_max(limit)
    };
    let mut positions = BTreeMap::new();
    let mut neighbors: BTreeMap<Point, BTreeSet<Point>> = BTreeMap::new();
    let mut opposite: BTreeMap<[Point; 2], Vec<(Point, Vec3)>> = BTreeMap::new();
    for triangle in vertices.chunks_exact(3) {
        let keys: [_; 3] = std::array::from_fn(|i| key(triangle[i].position));
        let [a, b, c] = std::array::from_fn(|i| Vec3::from(triangle[i].position));
        let normal = (b - a).cross(c - a).normalize_or_zero();
        for i in 0..3 {
            positions.insert(keys[i], Vec3::from(triangle[i].position));
            neighbors
                .entry(keys[i])
                .or_default()
                .extend([keys[(i + 1) % 3], keys[(i + 2) % 3]]);
            opposite
                .entry(edge(keys[i], keys[(i + 1) % 3]))
                .or_default()
                .push((keys[(i + 2) % 3], normal));
        }
    }
    let pinned: BTreeSet<_> = opposite
        .iter()
        .filter(|(_, faces)| !continuous(faces))
        .flat_map(|(edge, _)| *edge)
        .collect();
    let corners: BTreeMap<_, _> = positions
        .iter()
        .map(|(&key, &p)| {
            let adjacent = &neighbors[&key];
            let target = if pinned.contains(&key) || adjacent.len() < 3 {
                p
            } else {
                let n = adjacent.len() as f32;
                let beta = (0.625 - (0.375 + 0.25 * (std::f32::consts::TAU / n).cos()).powi(2)) / n;
                p * (1.0 - n * beta) + adjacent.iter().map(|v| positions[v]).sum::<Vec3>() * beta
            };
            (key, relax(p, target))
        })
        .collect();
    let edges: BTreeMap<_, _> = opposite
        .iter()
        .map(|(&pair, adjacent)| {
            let a = positions[&pair[0]];
            let b = positions[&pair[1]];
            let center = (a + b) * 0.5;
            let target = if continuous(adjacent) {
                (a + b) * 0.375 + (positions[&adjacent[0].0] + positions[&adjacent[1].0]) * 0.125
            } else {
                center
            };
            (pair, relax(center, target))
        })
        .collect();
    let mut out = Vec::with_capacity(vertices.len() * 4);
    let mut out_skin = Vec::with_capacity(skin.len() * 4);
    for (face, triangle) in vertices.chunks_exact(3).enumerate() {
        let keys: [_; 3] = std::array::from_fn(|i| key(triangle[i].position));
        let p: [Vertex; 3] = std::array::from_fn(|i| Vertex {
            position: corners[&keys[i]].to_array(),
            normal: [0.0; 3],
            ..triangle[i]
        });
        let mid: [Vertex; 3] = std::array::from_fn(|i| {
            let a = triangle[i];
            let b = triangle[(i + 1) % 3];
            Vertex {
                position: edges[&edge(keys[i], keys[(i + 1) % 3])].to_array(),
                uv: std::array::from_fn(|j| (a.uv[j] + b.uv[j]) * 0.5),
                color: std::array::from_fn(|j| (a.color[j] + b.color[j]) * 0.5),
                normal: [0.0; 3],
                ..a
            }
        });
        if !skin.is_empty() {
            let p: [Influence; 3] = std::array::from_fn(|i| skin[face * 3 + i]);
            let mid: [Influence; 3] = std::array::from_fn(|i| {
                Influence::blend(
                    p,
                    std::array::from_fn(|j| if j == i || j == (i + 1) % 3 { 0.5 } else { 0.0 }),
                )
            });
            out_skin.extend([
                p[0], mid[0], mid[2], mid[0], p[1], mid[1], mid[2], mid[1], p[2], mid[0], mid[1],
                mid[2],
            ]);
        }
        out.extend([
            p[0], mid[0], mid[2], mid[0], p[1], mid[1], mid[2], mid[1], p[2], mid[0], mid[1],
            mid[2],
        ]);
    }
    *vertices = out;
    *skin = out_skin;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    #[test]
    fn back_to_back_panels_keep_their_flat_faces_and_straight_borders() {
        let points = [
            [-100.0, -100.0, 0.0],
            [0.0, -100.0, 0.0],
            [100.0, -100.0, 0.0],
            [-100.0, 100.0, 0.0],
            [0.0, 100.0, 0.0],
            [100.0, 100.0, 0.0],
        ];
        let front = [[0, 1, 3], [3, 1, 4], [1, 2, 4], [4, 2, 5]];
        let mut vertices: Vec<_> = front
            .into_iter()
            .flat_map(|t| [t, [t[2], t[1], t[0]]])
            .flatten()
            .map(|i| Vertex {
                position: points[i],
                ..Vertex::zeroed()
            })
            .collect();
        prepare(&mut vertices, 2, 0.025, &mut 10000);
        let area: f32 = vertices
            .chunks_exact(3)
            .map(|t| {
                let [a, b, c] = std::array::from_fn(|i| Vec3::from(t[i].position));
                (b - a).cross(c - a).length() * 0.5
            })
            .sum();
        assert!((area - 80000.0).abs() < 0.1, "panel area {area}");
        for p in points {
            assert!(vertices.iter().any(|v| v.position == p), "corner {p:?}");
        }
        assert!(vertices.iter().all(|v| v.position[2] == 0.0));
    }

    #[test]
    fn refinement_welds_material_seams_and_pins_open_joints() {
        let p = [
            [-100.0, 0.0, 0.0],
            [100.0, 0.0, 0.0],
            [0.0, -100.0, 100.0],
            [0.0, -100.0, -100.0],
        ];
        let mut vertices: Vec<_> = [[0, 1, 2], [1, 0, 3]]
            .into_iter()
            .enumerate()
            .flat_map(|(material, t)| {
                t.map(|i| Vertex {
                    position: p[i],
                    uv: [material as f32 + i as f32 * 0.1, 0.5],
                    material: material as u32 + 1,
                    ..Vertex::zeroed()
                })
            })
            .collect();
        let original = vertices.clone();
        prepare(&mut vertices, 2, 0.025, &mut 1000);
        assert_eq!(vertices.len(), original.len() * 16);
        for v in &original {
            assert!(
                vertices
                    .iter()
                    .any(|n| n.position == v.position && n.uv == v.uv && n.material == v.material)
            );
        }
        let left: BTreeSet<_> = vertices
            .iter()
            .filter(|v| v.material == 1)
            .map(|v| key(v.position))
            .collect();
        let right: BTreeSet<_> = vertices
            .iter()
            .filter(|v| v.material == 2)
            .map(|v| key(v.position))
            .collect();
        assert_eq!(left.intersection(&right).count(), 5);
        assert!(
            vertices
                .iter()
                .all(|v| Vec3::from(v.position).is_finite() && Vec3::from(v.normal).is_finite())
        );
        assert!(
            vertices
                .iter()
                .filter(|v| v.material == 1)
                .all(|v| v.uv[0] <= 0.2)
        );
        let mut limited = original.clone();
        prepare(&mut limited, 2, 0.025, &mut 10);
        assert_eq!(
            bytemuck::cast_slice::<_, u8>(&limited),
            bytemuck::cast_slice::<_, u8>(&original)
        );
    }
}
