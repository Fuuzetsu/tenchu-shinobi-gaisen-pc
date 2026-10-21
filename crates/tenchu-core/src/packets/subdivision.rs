use super::{BYTES, Subdivision};
use crate::{
    math::Matrix,
    projection::{self, Point},
};

type Vertex = [i16; 3];

pub(super) struct Counter<'a> {
    parameters: &'a Subdivision,
    matrix: &'a Matrix,
    levels: u8,
    pub bytes: u32,
}

fn midpoint(a: Vertex, b: Vertex) -> Vertex {
    std::array::from_fn(|i| ((i32::from(a[i]) + i32::from(b[i])) / 2) as i16)
}

fn average<const N: usize>(points: &[Point; N], scale: i32) -> i32 {
    ((points.iter().map(|p| i32::from(p.z)).sum::<i32>() * scale) >> 12).clamp(0, 65535)
}

enum Decision {
    Discard,
    Emit,
    Split,
}

impl<'a> Counter<'a> {
    pub fn new(parameters: &'a Subdivision, matrix: &'a Matrix, levels: u8) -> Self {
        Self {
            parameters,
            matrix,
            levels,
            bytes: 0,
        }
    }

    fn four(&self, vertices: [Vertex; 4]) -> ([Point; 4], u32, u32) {
        let (points, flags) =
            projection::triangle(self.matrix, [vertices[0], vertices[1], vertices[2]]);
        let last = projection::point(self.matrix, vertices[3]);
        ([points[0], points[1], points[2], last], flags, last.flags)
    }

    fn emit(&mut self, kind: u8) {
        self.bytes += BYTES[usize::from(kind)];
    }

    pub fn root(&mut self, kind: u8, vertices: [Vertex; 4]) {
        if kind & 2 != 0 {
            let (points, a, b) = self.four(vertices);
            if projection::area([points[0], points[1], points[2]]) <= 0 || a & b & 0x40000 != 0 {
                return;
            }
            if average(&points, 256) >= self.parameters.depth && (a | b) & 0x80000000 == 0 {
                self.emit(kind);
            } else {
                self.quad(kind, vertices, 0);
            }
        } else {
            let vertices = [vertices[0], vertices[1], vertices[2]];
            let (points, flags) = projection::triangle(self.matrix, vertices);
            if projection::area(points) <= 0 || flags & 0x40000 != 0 {
                return;
            }
            if average(&points, 341) >= self.parameters.depth && flags & 0x80000000 == 0 {
                self.emit(kind);
            } else {
                self.triangle(kind, vertices, 0);
            }
        }
    }

    fn decision<const N: usize>(&self, points: &[Point; N], flags: u32, level: u8) -> Decision {
        let low: [i16; 2] = std::array::from_fn(|i| points.iter().map(|p| p.xy[i]).min().unwrap());
        let high: [i16; 2] = std::array::from_fn(|i| points.iter().map(|p| p.xy[i]).max().unwrap());
        if (0..2).any(|i| high[i] < -[160, 120][i] || low[i] > [160, 120][i]) {
            return Decision::Discard;
        }
        if level == self.levels {
            return if flags & 0x7f85e000 == 0 {
                Decision::Emit
            } else {
                Decision::Discard
            };
        }
        if flags & 0x80000000 == 0
            && (0..2).all(|i| {
                i32::from(high[i]) - i32::from(low[i]) < i32::from(self.parameters.extent[i])
            })
        {
            Decision::Emit
        } else {
            Decision::Split
        }
    }

    fn edge(&mut self, kind: u8, vertices: [Vertex; 3]) {
        let (_, flags) = projection::triangle(self.matrix, vertices);
        let mask = if kind == 6 { 0x60000 } else { 0x80000000 };
        // The SDK emits these stitches even when their projected area is zero.
        if flags & mask == 0 {
            self.emit(kind & !2);
        }
    }

    fn quad(&mut self, kind: u8, v: [Vertex; 4], level: u8) {
        let (points, a, b) = self.four(v);
        match self.decision(&points, a | b, level) {
            Decision::Discard => (),
            Decision::Emit => self.emit(kind),
            Decision::Split => {
                let a = midpoint(v[0], v[1]);
                let b = midpoint(v[0], v[2]);
                let center = midpoint(v[0], v[3]);
                let c = midpoint(v[1], v[3]);
                let d = midpoint(v[3], v[2]);
                self.edge(kind, [v[0], v[1], a]);
                self.quad(kind, [v[0], a, b, center], level + 1);
                self.edge(kind, [v[1], v[3], c]);
                self.quad(kind, [v[1], a, c, center], level + 1);
                self.edge(kind, [v[3], v[2], d]);
                self.quad(kind, [v[3], d, c, center], level + 1);
                self.edge(kind, [v[2], v[0], b]);
                self.quad(kind, [v[2], d, b, center], level + 1);
            }
        }
    }

    fn triangle(&mut self, kind: u8, v: [Vertex; 3], level: u8) {
        let (points, flags) = projection::triangle(self.matrix, v);
        match self.decision(&points, flags, level) {
            Decision::Discard => (),
            Decision::Emit => self.emit(kind),
            Decision::Split => {
                let a = midpoint(v[0], v[1]);
                let b = midpoint(v[0], v[2]);
                let c = midpoint(v[1], v[2]);
                self.edge(kind, [v[0], v[1], a]);
                self.triangle(kind, [v[0], a, b], level + 1);
                self.triangle(kind, [c, a, b], level + 1);
                self.edge(kind, [v[1], v[2], c]);
                self.triangle(kind, [c, a, v[1]], level + 1);
                self.edge(kind, [v[2], v[0], b]);
                self.triangle(kind, [c, v[2], b], level + 1);
            }
        }
    }
}
