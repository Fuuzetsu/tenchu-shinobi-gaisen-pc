use crate::render_mesh::{self, Vertex};
use glam::{Mat4, Vec3, Vec4};
use std::ops::Range;

pub(super) struct Chunk {
    range: Range<u32>,
    bounds: Option<[Vec3; 2]>,
    foliage: bool,
}

pub(super) fn chunks(vertices: &[Vertex]) -> Vec<Chunk> {
    vertices
        .chunks(1536)
        .enumerate()
        .map(|(i, vertices)| {
            let mut bounds: Option<[Vec3; 2]> = None;
            let mut foliage = false;
            for v in vertices.iter().filter(|v| {
                v.flags
                    & (render_mesh::SCREEN
                        | render_mesh::DEBUG
                        | render_mesh::ALPHA
                        | render_mesh::WATER)
                    == 0
            }) {
                let p = Vec3::from(v.position);
                bounds = Some(bounds.map_or([p, p], |[a, b]| [a.min(p), b.max(p)]));
                foliage |= v.flags & render_mesh::FOLIAGE != 0;
            }
            let start = i as u32 * 1536;
            Chunk {
                range: start..start + vertices.len() as u32,
                bounds,
                foliage,
            }
        })
        .collect()
}

pub(super) struct Frustum([Vec4; 6]);
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
    pub fn ranges(&self, chunks: &[Chunk], wind: f32) -> Vec<Range<u32>> {
        let mut ranges: Vec<Range<u32>> = Vec::new();
        for chunk in chunks.iter().filter(|c| {
            c.bounds.is_some_and(|b| {
                self.contains(
                    b,
                    if c.foliage {
                        wind.abs() * 100.0 + 2.0
                    } else {
                        2.0
                    },
                )
            })
        }) {
            if let Some(previous) = ranges.last_mut().filter(|r| r.end == chunk.range.start) {
                previous.end = chunk.range.end;
            } else {
                ranges.push(chunk.range.clone());
            }
        }
        ranges
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
