mod subdivision;
#[cfg(test)]
mod tests;

use crate::{
    Result,
    math::Matrix,
    projection::{self, Point},
};
use tenchu_assets::{executable::Executable, model::Geometry};

const BYTES: [u32; 8] = [20, 32, 24, 40, 28, 40, 36, 52];

#[derive(Clone, Copy, Debug)]
pub struct Subdivision {
    depth: i32,
    extent: [i16; 2],
}

impl Subdivision {
    pub fn load(exe: &Executable) -> Result<Self> {
        let data = exe.bytes(0x8008f7c8, 8)?;
        Ok(Self {
            depth: i32::from_le_bytes(data[..4].try_into().unwrap()),
            extent: std::array::from_fn(|i| {
                i16::from_le_bytes(data[4 + i * 2..6 + i * 2].try_into().unwrap())
            }),
        })
    }

    pub fn count(&self, mesh: &Geometry, matrix: &Matrix, levels: u8) -> Result<u32> {
        let mut counter = subdivision::Counter::new(self, matrix, levels);
        for face in &mesh.faces {
            if face.flags & 1 == 0 {
                return Err("Lit adaptive geometry is not implemented yet".into());
            }
            let mut vertices = [[0; 3]; 4];
            for (out, corner) in vertices.iter_mut().zip(&face.corners) {
                *out = mesh.vertices[usize::from(corner.vertex)];
            }
            counter.root((face.mode & 0x1c) >> 2, vertices);
        }
        Ok(counter.bytes)
    }
}

pub fn draw_tmd(mesh: &Geometry, matrix: &Matrix, fog: bool) -> u32 {
    let projected: Vec<_> = mesh
        .vertices
        .iter()
        .map(|&v| projection::point(matrix, v))
        .collect();
    let mut bytes = 0;
    for face in &mesh.faces {
        if face.flags & 1 == 0 {
            break;
        }
        let points: [Point; 3] =
            std::array::from_fn(|i| projected[usize::from(face.corners[i].vertex)]);
        if points.iter().fold(0, |f, p| f | p.flags) & projection::DRAW_TMD_REJECT != 0
            || projection::area(points) <= 0
        {
            continue;
        }
        let depth = if fog {
            if (points[2].mac[0].wrapping_add(8192) as u32) >> 14 != 0 {
                continue;
            }
            let depth = face
                .corners
                .iter()
                .map(|c| projected[usize::from(c.vertex)].z)
                .max()
                .unwrap()
                >> 4;
            if depth <= 150 {
                continue;
            }
            u32::from(depth)
        } else {
            let sum: u32 = points.iter().map(|p| u32::from(p.z)).sum();
            ((sum >> 4) + (sum >> 2)) >> 4
        };
        if depth > 0 && depth < 1250 {
            bytes += BYTES[usize::from((face.mode & 0x1c) >> 2)];
        }
    }
    bytes
}

pub fn sdk_plain(mesh: &Geometry, matrix: &Matrix) -> Result<u32> {
    let mut bytes = 0;
    for face in &mesh.faces {
        if face.flags & 1 == 0 {
            return Err("Lit SDK geometry is not implemented yet".into());
        }
        let vertices = std::array::from_fn(|i| mesh.vertices[usize::from(face.corners[i].vertex)]);
        let (points, flags) = projection::triangle(matrix, vertices);
        if flags & 0x80000000 != 0 || projection::area(points) <= 0 {
            continue;
        }
        if face.corners.len() == 4
            && projection::point(matrix, mesh.vertices[usize::from(face.corners[3].vertex)]).flags
                & 0x80000000
                != 0
        {
            continue;
        }
        bytes += BYTES[usize::from((face.mode & 0x1c) >> 2)];
    }
    Ok(bytes)
}
