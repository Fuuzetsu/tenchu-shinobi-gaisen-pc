use crate::render_mesh::Vertex;
use glam::Vec3;
use std::collections::BTreeMap;

// Cubic PN patches: Vlachos et al., https://doi.org/10.1145/364338.364387.
// Open/nonmanifold edges stay linear so independently animated parts keep their seams.
type Point = [u32; 3];

fn key(point: [f32; 3]) -> Point {
    point.map(|n| if n == 0.0 { 0 } else { n.to_bits() })
}

fn edge(a: Point, b: Point) -> [Point; 2] {
    if a < b { [a, b] } else { [b, a] }
}

pub fn prepare(vertices: &mut Vec<Vertex>, lighting: bool, detail: u16) {
    if !lighting && detail == 0 {
        return;
    }
    let faces: Vec<_> = vertices
        .chunks_exact(3)
        .map(|triangle| {
            let [a, b, c] = std::array::from_fn(|i| Vec3::from(triangle[i].position));
            (b - a).cross(c - a).normalize_or_zero()
        })
        .collect();
    let mut neighbors: BTreeMap<Point, Vec<usize>> = BTreeMap::new();
    let mut edges: BTreeMap<[Point; 2], Vec<usize>> = BTreeMap::new();
    for (face, triangle) in vertices.chunks_exact(3).enumerate() {
        for i in 0..3 {
            neighbors
                .entry(key(triangle[i].position))
                .or_default()
                .push(face);
            edges
                .entry(edge(
                    key(triangle[i].position),
                    key(triangle[(i + 1) % 3].position),
                ))
                .or_default()
                .push(face);
        }
    }
    let normals: BTreeMap<_, _> = neighbors
        .iter()
        .map(|(&point, indices)| {
            (
                point,
                indices
                    .iter()
                    .map(|&face| faces[face])
                    .sum::<Vec3>()
                    .normalize_or_zero(),
            )
        })
        .collect();
    if lighting {
        for (index, vertex) in vertices.iter_mut().enumerate() {
            if Vec3::from(vertex.normal).length_squared() > 0.5 {
                continue;
            }
            let face = faces[index / 3];
            vertex.normal = neighbors[&key(vertex.position)]
                .iter()
                .map(|&i| faces[i])
                .filter(|n| n.dot(face) >= 0.5)
                .sum::<Vec3>()
                .normalize_or_zero()
                .to_array();
        }
    }
    if detail == 0 {
        return;
    }
    let divisions = if detail == 1 { 2 } else { 4 };
    let strength = if detail == 1 { 0.35 } else { 0.7 };
    let mut curved = Vec::with_capacity(vertices.len() * divisions * divisions);
    for triangle in vertices.chunks_exact(3) {
        let points: [Vec3; 3] = std::array::from_fn(|i| Vec3::from(triangle[i].position));
        let mut tangent = [[Vec3::ZERO; 3]; 3];
        let mut sum = Vec3::ZERO;
        for i in 0..3 {
            for j in 0..3 {
                if i == j {
                    continue;
                }
                let normal = normals[&key(triangle[i].position)];
                let delta = points[j] - points[i];
                let adjacent = &edges[&edge(key(triangle[i].position), key(triangle[j].position))];
                let smooth =
                    adjacent.len() == 2 && faces[adjacent[0]].dot(faces[adjacent[1]]) > -0.5;
                tangent[i][j] = points[i]
                    + (delta
                        - if smooth {
                            normal * delta.dot(normal)
                        } else {
                            Vec3::ZERO
                        })
                        / 3.0;
                sum += tangent[i][j];
            }
        }
        let center = sum / 6.0 * 1.5 - (points[0] + points[1] + points[2]) / 6.0;
        let at = |x: usize, y: usize| {
            let u = x as f32 / divisions as f32;
            let v = y as f32 / divisions as f32;
            let weights = [1.0 - u - v, u, v];
            let flat = points
                .into_iter()
                .zip(weights)
                .map(|(p, w)| p * w)
                .sum::<Vec3>();
            let mut position = center * (6.0 * weights[0] * weights[1] * weights[2]);
            for i in 0..3 {
                position += points[i] * weights[i].powi(3);
                for j in 0..3 {
                    if i != j {
                        position += tangent[i][j] * (3.0 * weights[i].powi(2) * weights[j]);
                    }
                }
            }
            let mut vertex = triangle[0];
            vertex.position = flat.lerp(position, strength).to_array();
            vertex.normal = (0..3)
                .map(|i| Vec3::from(triangle[i].normal) * weights[i])
                .sum::<Vec3>()
                .normalize_or_zero()
                .to_array();
            vertex.color = std::array::from_fn(|axis| {
                (0..3).map(|i| triangle[i].color[axis] * weights[i]).sum()
            });
            vertex.uv =
                std::array::from_fn(|axis| (0..3).map(|i| triangle[i].uv[axis] * weights[i]).sum());
            vertex
        };
        for x in 0..divisions {
            for y in 0..divisions - x {
                curved.extend([at(x, y), at(x + 1, y), at(x, y + 1)]);
                if x + y + 1 < divisions {
                    curved.extend([at(x + 1, y), at(x + 1, y + 1), at(x, y + 1)]);
                }
            }
        }
    }
    *vertices = curved;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    fn mesh(points: &[[f32; 3]], faces: &[[usize; 3]]) -> Vec<Vertex> {
        faces
            .iter()
            .flatten()
            .map(|&i| Vertex {
                position: points[i],
                ..Vertex::zeroed()
            })
            .collect()
    }

    #[test]
    fn rounding_keeps_corners_and_open_seams_while_curving_closed_edges() {
        let points = [
            [1.0, 1.0, 1.0],
            [-1.0, -1.0, 1.0],
            [-1.0, 1.0, -1.0],
            [1.0, -1.0, -1.0],
        ];
        let mut closed = mesh(&points, &[[0, 1, 2], [0, 3, 1], [0, 2, 3], [1, 3, 2]]);
        prepare(&mut closed, true, 2);
        assert_eq!(closed.len(), 4 * 3 * 16);
        for point in points {
            assert!(closed.iter().any(|v| v.position == point));
        }
        let edge: Vec<_> = closed
            .iter()
            .filter(|v| {
                v.position[0].abs() < 0.00001
                    && v.position[1].abs() < 0.00001
                    && v.position[2] > 1.01
            })
            .collect();
        assert!(edge.len() >= 2);
        assert!(
            edge.iter()
                .all(|v| (v.position[2] - edge[0].position[2]).abs() < 0.00001)
        );
        let mut open = mesh(
            &[
                [-1.0, 0.0, 0.0],
                [1.0, 0.0, 0.0],
                [0.0, 1.0, 1.0],
                [0.0, -1.0, 1.0],
            ],
            &[[0, 1, 2], [1, 0, 3]],
        );
        prepare(&mut open, true, 2);
        for point in [
            [-0.5, 0.5, 0.5],
            [0.5, 0.5, 0.5],
            [-0.5, -0.5, 0.5],
            [0.5, -0.5, 0.5],
        ] {
            assert!(
                open.iter()
                    .any(|v| Vec3::from(v.position).distance(Vec3::from(point)) < 0.00001)
            );
        }
        assert!(open.iter().all(|v| Vec3::from(v.normal).is_finite()));
    }
}
