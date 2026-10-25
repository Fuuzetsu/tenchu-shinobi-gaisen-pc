use crate::render_mesh::Vertex;
use glam::Vec3;
use std::ops::Range;

#[derive(Clone, Copy)]
struct Bounds {
    min: Vec3,
    max: Vec3,
}

impl Bounds {
    fn overlaps(self, other: Self) -> bool {
        self.min.cmple(other.max).all() && self.max.cmpge(other.min).all()
    }
}

struct Node {
    bounds: Bounds,
    triangles: Range<usize>,
    children: Option<[usize; 2]>,
}

#[derive(Clone, Copy)]
pub struct Hit {
    pub position: Vec3,
    pub normal: Vec3,
    pub fraction: f32,
}

/// Presentation geometry only; never used by game collision or AI.
#[derive(Default)]
pub struct Surfaces {
    triangles: Vec<[Vertex; 3]>,
    nodes: Vec<Node>,
}

impl Surfaces {
    pub fn top(&self) -> f32 {
        self.nodes.first().map_or(0.0, |node| node.bounds.min.y)
    }

    pub fn exposed(&self, position: Vec3, velocity: Vec3) -> bool {
        let rise = (position.y - self.top() + 100.0).max(100.0);
        let start = position - velocity * (rise / velocity.y.max(0.001));
        self.cast(start, position).is_none()
    }

    pub fn new(vertices: &[Vertex]) -> Self {
        let mut out = Self {
            triangles: vertices
                .chunks_exact(3)
                .map(|t| [t[0], t[1], t[2]])
                .collect(),
            nodes: Vec::new(),
        };
        if !out.triangles.is_empty() {
            out.partition(0..out.triangles.len());
        }
        out
    }

    fn partition(&mut self, range: Range<usize>) -> usize {
        let mut bounds = Bounds {
            min: Vec3::splat(f32::INFINITY),
            max: Vec3::splat(f32::NEG_INFINITY),
        };
        for v in self.triangles[range.clone()].iter().flatten() {
            let p = Vec3::from(v.position);
            bounds.min = bounds.min.min(p);
            bounds.max = bounds.max.max(p);
        }
        let index = self.nodes.len();
        self.nodes.push(Node {
            bounds,
            triangles: range.clone(),
            children: None,
        });
        if range.len() > 12 {
            let size = bounds.max - bounds.min;
            let axis = if size.x > size.y && size.x > size.z {
                0
            } else if size.y > size.z {
                1
            } else {
                2
            };
            let middle = range.len() / 2;
            self.triangles[range.clone()].select_nth_unstable_by(middle, |a, b| {
                let center = |t: &[Vertex; 3]| t.iter().map(|v| v.position[axis]).sum::<f32>();
                center(a).total_cmp(&center(b))
            });
            let middle = range.start + middle;
            self.nodes[index].children = Some([
                self.partition(range.start..middle),
                self.partition(middle..range.end),
            ]);
        }
        index
    }

    fn visit(&self, bounds: Bounds, mut visit: impl FnMut(&[Vertex; 3])) {
        if self.nodes.is_empty() {
            return;
        }
        let mut stack = vec![0];
        while let Some(index) = stack.pop() {
            let node = &self.nodes[index];
            if !node.bounds.overlaps(bounds) {
                continue;
            }
            if let Some(children) = node.children {
                stack.extend(children);
            } else {
                for triangle in &self.triangles[node.triangles.clone()] {
                    visit(triangle);
                }
            }
        }
    }

    pub fn cast(&self, start: Vec3, end: Vec3) -> Option<Hit> {
        self.cast_excluding(start, end, 0)
    }
    pub fn cast_excluding(&self, start: Vec3, end: Vec3, excluded: u32) -> Option<Hit> {
        let mut nearest: Option<Hit> = None;
        self.visit(
            Bounds {
                min: start.min(end),
                max: start.max(end),
            },
            |triangle| {
                if triangle[0].flags & excluded == 0
                    && let Some(hit) = intersect(triangle, start, end)
                    && nearest.is_none_or(|old| hit.fraction < old.fraction)
                {
                    nearest = Some(hit);
                }
            },
        );
        nearest
    }

    pub fn project(&self, stamp: &Stamp) -> Vec<Vertex> {
        let extent = Vec3::splat(stamp.radius + stamp.depth);
        let mut out = Vec::new();
        self.visit(
            Bounds {
                min: stamp.center - extent,
                max: stamp.center + extent,
            },
            |triangle| {
                stamp.project(triangle, &mut out);
            },
        );
        out
    }
}

pub fn intersect(triangle: &[Vertex; 3], start: Vec3, end: Vec3) -> Option<Hit> {
    let [a, b, c] = triangle.map(|v| Vec3::from(v.position));
    let direction = end - start;
    let ab = b - a;
    let ac = c - a;
    let cross = direction.cross(ac);
    let determinant = ab.dot(cross);
    if determinant.abs() < 0.00001 {
        return None;
    }
    let inverse = determinant.recip();
    let offset = start - a;
    let u = offset.dot(cross) * inverse;
    let q = offset.cross(ab);
    let v = direction.dot(q) * inverse;
    let fraction = ac.dot(q) * inverse;
    if u < 0.0 || v < 0.0 || u + v > 1.0 || !(0.0..=1.0).contains(&fraction) {
        return None;
    }
    let mut normal = ab.cross(ac).normalize_or_zero();
    if normal.dot(direction) > 0.0 {
        normal = -normal;
    }
    Some(Hit {
        position: start + direction * fraction,
        normal,
        fraction,
    })
}

pub fn closest(triangle: &[Vertex; 3], point: Vec3) -> Vec3 {
    let [a, b, c] = triangle.map(|v| Vec3::from(v.position));
    let normal = (b - a).cross(c - a).normalize_or_zero();
    let projected = point - normal * (point - a).dot(normal);
    let edges = [(a, b), (b, c), (c, a)];
    if normal != Vec3::ZERO
        && edges
            .iter()
            .all(|&(a, b)| (b - a).cross(projected - a).dot(normal) >= -0.001)
    {
        return projected;
    }
    edges
        .map(|(a, b)| {
            let edge = b - a;
            a + edge * ((point - a).dot(edge) / edge.length_squared().max(0.0001)).clamp(0.0, 1.0)
        })
        .into_iter()
        .min_by(|a, b| {
            a.distance_squared(point)
                .total_cmp(&b.distance_squared(point))
        })
        .unwrap()
}

pub struct Stamp {
    pub center: Vec3,
    pub normal: Vec3,
    pub radius: f32,
    pub depth: f32,
    pub seed: u32,
    pub shape: Option<(Vec3, f32)>,
}

impl Stamp {
    pub fn project(&self, triangle: &[Vertex; 3], out: &mut Vec<Vertex>) {
        if triangle[0].flags & crate::render_mesh::WATER != 0 {
            return;
        }
        let [a, b, c] = triangle.map(|v| Vec3::from(v.position));
        let face_normal = (b - a).cross(c - a).normalize_or_zero();
        if face_normal.dot(self.normal).abs() < 0.3 || self.radius <= 0.0 {
            return;
        }
        let mut tangent = self.shape.map_or_else(
            || self.normal.any_orthonormal_vector(),
            |(forward, _)| (forward - self.normal * forward.dot(self.normal)).normalize_or_zero(),
        );
        if tangent.length_squared() < 0.5 {
            tangent = self.normal.any_orthonormal_vector();
        }
        let bitangent = self.normal.cross(tangent);
        let mut polygon = triangle.to_vec();
        let planes = std::array::from_fn::<_, 12, _>(|side| {
            let angle =
                (side as f32 + (self.seed % 32) as f32 / 32.0) * std::f32::consts::TAU / 12.0;
            let normal = tangent * angle.cos() + bitangent * angle.sin();
            let radius = self.radius
                * self.shape.map_or_else(
                    || 0.85 + 0.15 * ((side as u32 * 7 + self.seed) % 11) as f32 / 10.0,
                    |(_, width)| (angle.cos().powi(2) + (angle.sin() * width).powi(2)).sqrt(),
                );
            (normal, radius)
        });
        for (normal, radius) in planes {
            clip(&mut polygon, normal, normal.dot(self.center) + radius);
        }
        clip(
            &mut polygon,
            self.normal,
            self.normal.dot(self.center) + self.depth,
        );
        clip(
            &mut polygon,
            -self.normal,
            -self.normal.dot(self.center) + self.depth,
        );
        if polygon.len() < 3 {
            return;
        }
        let mut center = polygon[0];
        for (i, &vertex) in polygon.iter().enumerate().skip(1) {
            center = interpolate(center, vertex, 1.0 / (i + 1) as f32);
        }
        let finish = |mut v: Vertex| {
            let delta = Vec3::from(v.position) - self.center;
            let margin = planes
                .iter()
                .map(|(normal, radius)| radius - delta.dot(*normal))
                .fold(f32::INFINITY, f32::min);
            let depth = (self.depth - delta.dot(self.normal).abs()) / (self.depth * 0.4);
            v.opacity =
                (margin / (self.radius * 0.3)).clamp(0.0, 1.0) * depth.clamp(0.0, 1.0) * 0.92;
            v.position = (Vec3::from(v.position) + self.normal * 0.5).to_array();
            v.flags = (v.flags & !crate::render_mesh::EMITTER)
                | crate::render_mesh::ALPHA
                | crate::render_mesh::STAIN;
            v
        };
        for i in 0..polygon.len() {
            stamp_triangle(
                [center, polygon[i], polygon[(i + 1) % polygon.len()]],
                0,
                if self.shape.is_some() { 1 } else { 3 },
                &finish,
                out,
            );
        }
    }
}

// Sample the opacity field inside clipped triangles; interpolating only their
// boundary values creates pale wedges where a decal crosses terrain edges.
fn stamp_triangle(
    triangle: [Vertex; 3],
    depth: u8,
    max_depth: u8,
    finish: &impl Fn(Vertex) -> Vertex,
    out: &mut Vec<Vertex>,
) {
    let vertices = triangle.map(finish);
    let middle =
        std::array::from_fn::<_, 3, _>(|i| interpolate(triangle[i], triangle[(i + 1) % 3], 0.5));
    let center = interpolate(
        interpolate(triangle[0], triangle[1], 0.5),
        triangle[2],
        1.0 / 3.0,
    );
    let error =
        (finish(center).opacity - vertices.iter().map(|v| v.opacity).sum::<f32>() / 3.0).abs();
    let split = error > 0.04
        || middle.iter().enumerate().any(|(i, &v)| {
            (finish(v).opacity - (vertices[i].opacity + vertices[(i + 1) % 3].opacity) * 0.5).abs()
                > 0.04
        });
    if depth < max_depth && split {
        let [a, b, c] = triangle;
        let [ab, bc, ca] = middle;
        for triangle in [[a, ab, ca], [ab, b, bc], [ca, bc, c], [ab, bc, ca]] {
            stamp_triangle(triangle, depth + 1, max_depth, finish, out);
        }
    } else {
        out.extend(vertices);
    }
}

fn interpolate(a: Vertex, b: Vertex, t: f32) -> Vertex {
    let mut out = a;
    out.position = std::array::from_fn(|i| a.position[i] + (b.position[i] - a.position[i]) * t);
    out.color = std::array::from_fn(|i| a.color[i] + (b.color[i] - a.color[i]) * t);
    out.uv = std::array::from_fn(|i| a.uv[i] + (b.uv[i] - a.uv[i]) * t);
    out.normal = std::array::from_fn(|i| a.normal[i] + (b.normal[i] - a.normal[i]) * t);
    out
}

fn clip(polygon: &mut Vec<Vertex>, normal: Vec3, limit: f32) {
    let Some(&last) = polygon.last() else {
        return;
    };
    let distance = |v: Vertex| Vec3::from(v.position).dot(normal) - limit;
    let mut previous = last;
    let mut d0 = distance(previous);
    let mut clipped = Vec::with_capacity(polygon.len() + 1);
    for &next in polygon.iter() {
        let d1 = distance(next);
        if (d0 <= 0.0) != (d1 <= 0.0) {
            clipped.push(interpolate(previous, next, d0 / (d0 - d1)));
        }
        if d1 <= 0.0 {
            clipped.push(next);
        }
        previous = next;
        d0 = d1;
    }
    *polygon = clipped;
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    fn plane(y: f32) -> Vec<Vertex> {
        let points = [
            [-10., y, -10.],
            [10., y, -10.],
            [-10., y, 10.],
            [10., y, 10.],
        ];
        [0, 1, 2, 1, 3, 2]
            .map(|i| Vertex {
                position: points[i],
                uv: [points[i][0], points[i][2]],
                ..Vertex::zeroed()
            })
            .to_vec()
    }

    #[test]
    fn projected_stain_preserves_uvs_and_does_not_reach_parallel_surfaces() {
        let mut vertices = plane(0.);
        vertices.extend(plane(2.));
        let surface = Surfaces::new(&vertices);
        let hit = surface.cast(Vec3::NEG_Y * 20., Vec3::Y * 20.).unwrap();
        assert!(hit.position.length() < 0.001);
        assert_eq!(hit.normal, Vec3::NEG_Y);
        let stamp = Stamp {
            center: hit.position,
            normal: hit.normal,
            radius: 5.,
            depth: 0.2,
            seed: 1,
            shape: None,
        };
        let projected = surface.project(&stamp);
        assert!(!projected.is_empty());
        for v in projected {
            assert!((v.position[1] + 0.5).abs() < 0.001);
            assert!((v.uv[0] - v.position[0]).abs() < 0.001);
            assert!((v.uv[1] - v.position[2]).abs() < 0.001);
            assert!((0.0..=1.0).contains(&v.opacity));
        }
    }

    #[test]
    fn spatial_index_matches_exhaustive_segment_intersections() {
        let mut vertices = Vec::new();
        for i in 0..40 {
            vertices.extend(plane(i as f32 * 3.));
        }
        let surface = Surfaces::new(&vertices);
        for i in 0..60 {
            let start = Vec3::new(i as f32 - 30., -5., 0.);
            let end = Vec3::new(0., 130., i as f32 - 30.);
            let exhaustive = vertices
                .chunks_exact(3)
                .filter_map(|t| intersect(&[t[0], t[1], t[2]], start, end))
                .min_by(|a, b| a.fraction.total_cmp(&b.fraction));
            assert_eq!(
                surface.cast(start, end).map(|h| h.fraction),
                exhaustive.map(|h| h.fraction)
            );
        }
    }
}
