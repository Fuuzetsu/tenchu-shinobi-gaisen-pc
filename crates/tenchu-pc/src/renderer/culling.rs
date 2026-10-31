use crate::render_mesh::{self, Vertex};
use glam::{Mat4, Vec3, Vec3A, Vec4};
use std::ops::Range;

pub(super) struct Chunk {
    range: Range<u32>,
    bounds: Option<[Vec3; 2]>,
    foliage: Option<[Vec3; 2]>,
}

pub(super) fn update_chunks(vertices: &[Vertex], chunks: &mut Vec<Chunk>) -> Option<[Vec3; 2]> {
    chunks.clear();
    let mut scenery: Option<[Vec3A; 2]> = None;
    let mut start = 0;
    while start < vertices.len() {
        let limit = (start + 1536).min(vertices.len());
        let mut end = limit;
        // Keep distant actors from sharing a chunk that spans the space between them.
        for next in (start + 3..limit).step_by(3) {
            if Vec3A::from(vertices[next].position)
                .distance_squared(Vec3A::from(vertices[next - 1].position))
                > 5000.0_f32.powi(2)
            {
                end = next;
                break;
            }
        }
        let vertices = &vertices[start..end];
        let mut bounds: Option<[Vec3A; 2]> = None;
        let mut foliage = false;
        for v in vertices {
            let p = Vec3A::from(v.position);
            if v.flags & render_mesh::SCENERY != 0 {
                scenery = Some(scenery.map_or([p, p], |[a, b]| [a.min(p), b.max(p)]));
            }
            if v.flags
                & (render_mesh::SCREEN
                    | render_mesh::DEBUG
                    | render_mesh::ALPHA
                    | render_mesh::WATER)
                == 0
            {
                bounds = Some(bounds.map_or([p, p], |[a, b]| [a.min(p), b.max(p)]));
                foliage |= v.flags & render_mesh::FOLIAGE != 0;
            }
        }
        chunks.push(Chunk {
            range: start as u32..end as u32,
            bounds: bounds.map(|b| b.map(Vec3::from)),
            foliage: foliage.then(|| {
                let mut bounds = [Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)];
                for triangle in vertices
                    .chunks_exact(3)
                    .filter(|t| t.iter().any(|v| v.flags & render_mesh::FOLIAGE != 0))
                {
                    for v in triangle {
                        let p = Vec3::from(v.position);
                        bounds = [bounds[0].min(p), bounds[1].max(p)];
                    }
                }
                bounds
            }),
        });
        start = end;
    }
    scenery.map(|b| b.map(Vec3::from))
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) struct Rect {
    pub min: [u32; 2],
    pub max: [u32; 2],
}

impl Rect {
    pub fn full(size: u32) -> Self {
        Self {
            min: [0; 2],
            max: [size; 2],
        }
    }

    pub fn union(mut self, other: Self) -> Self {
        for axis in 0..2 {
            self.min[axis] = self.min[axis].min(other.min[axis]);
            self.max[axis] = self.max[axis].max(other.max[axis]);
        }
        self
    }

    fn projected(bounds: [Vec3; 2], matrix: Mat4, size: u32, padding: f32) -> Self {
        let [min, max] = [
            bounds[0] - Vec3::splat(padding),
            bounds[1] + Vec3::splat(padding),
        ];
        let mut low = glam::Vec2::splat(f32::INFINITY);
        let mut high = glam::Vec2::splat(f32::NEG_INFINITY);
        for i in 0..8 {
            let p = matrix
                * Vec3::new(
                    if i & 1 == 0 { min.x } else { max.x },
                    if i & 2 == 0 { min.y } else { max.y },
                    if i & 4 == 0 { min.z } else { max.z },
                )
                .extend(1.0);
            if !p.is_finite() || p.w <= 0.0 {
                return Self::full(size);
            }
            let pixel =
                (glam::Vec2::new(p.x, -p.y) / p.w * 0.5 + glam::Vec2::splat(0.5)) * size as f32;
            low = low.min(pixel);
            high = high.max(pixel);
        }
        Self {
            min: low
                .to_array()
                .map(|p| (p.floor() - 1.0).clamp(0.0, (size - 1) as f32) as u32),
            max: high
                .to_array()
                .map(|p| (p.ceil() + 1.0).clamp(1.0, size as f32) as u32),
        }
    }
}

pub(super) struct Frustum([Vec4; 6]);

pub(super) struct Volume {
    planes: Frustum,
    corners: [Vec3; 8],
}

impl Volume {
    pub fn new(matrix: Mat4) -> Self {
        let inverse = matrix.inverse();
        Self {
            planes: Frustum::new(matrix),
            corners: std::array::from_fn(|i| {
                inverse.project_point3(Vec3::new(
                    if i & 1 == 0 { -1.0 } else { 1.0 },
                    if i & 2 == 0 { -1.0 } else { 1.0 },
                    if i & 4 == 0 { 0.0 } else { 1.0 },
                ))
            }),
        }
    }

    pub fn intersects(&self, other: &Self) -> bool {
        [(self, other), (other, self)].into_iter().all(|(a, b)| {
            a.planes.0.iter().all(|plane| {
                let tolerance = plane.truncate().length() * 2.0;
                !b.corners
                    .iter()
                    .all(|p| plane.truncate().dot(*p) + plane.w < -tolerance)
            })
        })
    }
}

impl Frustum {
    pub fn new(matrix: Mat4) -> Self {
        let rows = matrix.transpose().to_cols_array_2d().map(Vec4::from);
        Self([
            rows[3] + rows[0],
            rows[3] - rows[0],
            rows[3] + rows[1],
            rows[3] - rows[1],
            rows[2],
            rows[3] - rows[2],
        ])
    }
    pub fn contains(&self, bounds: [Vec3; 2], padding: f32) -> bool {
        let center = (bounds[0] + bounds[1]) * 0.5;
        let extent = (bounds[1] - bounds[0]) * 0.5 + Vec3::splat(padding);
        self.0
            .iter()
            .all(|p| p.truncate().dot(center) + p.w + p.truncate().abs().dot(extent) >= -0.001)
    }
    pub fn ranges(&self, chunks: &[Chunk], wind: f32) -> (Vec<Range<u32>>, bool) {
        let mut ranges: Vec<Range<u32>> = Vec::new();
        let mut foliage = false;
        for chunk in chunks.iter().filter(|c| {
            c.bounds.is_some_and(|b| {
                self.contains(
                    b,
                    if c.foliage.is_some() {
                        wind.abs() * 100.0 + 2.0
                    } else {
                        2.0
                    },
                )
            })
        }) {
            foliage |= chunk.foliage.is_some();
            if let Some(previous) = ranges.last_mut().filter(|r| r.end == chunk.range.start) {
                previous.end = chunk.range.end;
            } else {
                ranges.push(chunk.range.clone());
            }
        }
        (ranges, foliage)
    }

    pub fn damage(
        &self,
        chunks: &[Chunk],
        wind: f32,
        dynamic: bool,
        matrix: Mat4,
        size: u32,
    ) -> Option<Rect> {
        let mut damage: Option<Rect> = None;
        for chunk in chunks {
            let bounds = if dynamic {
                chunk.bounds
            } else if wind != 0.0 {
                chunk.foliage
            } else {
                None
            };
            let Some(bounds) = bounds else {
                continue;
            };
            let padding = if chunk.foliage.is_some() {
                wind.abs() * 100.0 + 2.0
            } else {
                2.0
            };
            if !self.contains(bounds, padding) {
                continue;
            }
            let rect = Rect::projected(bounds, matrix, size, padding);
            damage = Some(damage.map_or(rect, |old| old.union(rect)));
        }
        damage
    }
}

pub(super) fn spatial(vertices: &[Vertex]) -> Vec<Vertex> {
    let mut triangles: Vec<_> = vertices
        .chunks_exact(3)
        .map(|v| [v[0], v[1], v[2]])
        .collect();
    triangles.sort_by_cached_key(|t| {
        let center = t.iter().map(|v| Vec3::from(v.position)).sum::<Vec3>() / 3.0;
        let cell = (center / 5000.0).floor().as_ivec3();
        (cell.x, cell.z, cell.y)
    });
    triangles.into_iter().flatten().collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn receiver_volumes_keep_containment_crossings_and_clip_boundaries() {
        let box_at = |position: Vec3, size: f32| {
            Volume::new(
                Mat4::orthographic_rh(-size, size, -size, size, -size, size)
                    * Mat4::from_translation(-position),
            )
        };
        let cube = box_at(Vec3::ZERO, 100.0);
        for other in [
            box_at(Vec3::ZERO, 200.0),
            box_at(Vec3::ZERO, 20.0),
            box_at(Vec3::new(150.0, 0.0, 0.0), 100.0),
            box_at(Vec3::new(200.0, 0.0, 0.0), 100.0),
        ] {
            assert!(cube.intersects(&other));
            assert!(other.intersects(&cube));
        }
        let far = box_at(Vec3::splat(400.0), 100.0);
        assert!(!cube.intersects(&far));
        assert!(!far.intersects(&cube));

        let projection = Mat4::perspective_rh(1.1, 16.0 / 9.0, 50.0, 250_000.0);
        let view = Volume::new(projection);
        for position in [
            Vec3::new(0.0, 0.0, -50.0),
            Vec3::new(0.0, 0.0, -250_000.0),
            Vec3::new(5000.0, 1000.0, -10_000.0),
        ] {
            let near = box_at(position, 100.0);
            assert!(view.intersects(&near));
            assert!(near.intersects(&view));
        }
        assert!(!view.intersects(&box_at(Vec3::new(0.0, 0.0, 1000.0), 100.0)));
    }

    #[test]
    fn shadow_frusta_keep_straddling_boxes_and_wind_at_the_edge() {
        let f = Frustum::new(Mat4::IDENTITY);
        assert!(f.contains([Vec3::new(-2.0, -2.0, -1.0), Vec3::splat(2.0)], 0.0));
        assert!(!f.contains([Vec3::new(2.0, 0.0, 0.2), Vec3::new(3.0, 0.2, 0.8)], 0.0));
        assert!(f.contains([Vec3::new(2.0, 0.0, 0.2), Vec3::new(3.0, 0.2, 0.8)], 1.1));
        assert!(!f.contains(
            [Vec3::new(-0.5, -0.5, -2.0), Vec3::new(0.5, 0.5, -1.0)],
            0.0
        ));
    }
}
