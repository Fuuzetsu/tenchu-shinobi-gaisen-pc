use crate::{math::Matrix, visibility::divide};
#[cfg(test)]
mod tests;

pub const SUMMARY: u32 = 0x7f87e000;
pub const DRAW_TMD_REJECT: u32 = 0x01c66000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub xy: [i16; 2],
    pub z: u16,
    pub flags: u32,
    pub mac: [i32; 3],
}

fn mac0(value: i64) -> u32 {
    if value > i64::from(i32::MAX) {
        1 << 16
    } else if value < i64::from(i32::MIN) {
        1 << 15
    } else {
        0
    }
}

fn geometry(matrix: &Matrix, vertex: [i16; 3]) -> (Point, u32) {
    let mut flags = 0;
    let mut mac = [0; 3];
    for (axis, out) in mac.iter_mut().enumerate() {
        let mut sum = i64::from(matrix.translation[axis]) << 12;
        for (coefficient, value) in matrix.rotation[axis].into_iter().zip(vertex) {
            sum += i64::from(coefficient) * i64::from(value);
            if sum >= 1 << 43 {
                flags |= 1 << (30 - axis);
            }
            if sum < -(1 << 43) {
                flags |= 1 << (27 - axis);
            }
            sum = (sum << 20) >> 20;
        }
        *out = (sum >> 12) as i32;
        if !(-32768..=32767).contains(out) {
            flags |= 1 << (24 - axis);
        }
    }
    if !(0..=65535).contains(&mac[2]) {
        flags |= 1 << 18;
    }
    let z = mac[2].clamp(0, 65535) as u16;
    let ratio = divide(300, z);
    if z <= 150 {
        flags |= 1 << 17;
    }
    let xy = std::array::from_fn(|axis| {
        let scaled = i64::from(mac[axis].clamp(-32768, 32767)) * i64::from(ratio);
        flags |= mac0(scaled);
        let screen = scaled >> 16;
        if !(-1024..=1023).contains(&screen) {
            flags |= 1 << (14 - axis);
        }
        screen.clamp(-1024, 1023) as i16
    });
    (Point { xy, z, flags, mac }, ratio)
}

fn finish(flags: u32, ratio: u32) -> u32 {
    let cue = 0x2f282e0i64 - 0x7ef4 * i64::from(ratio);
    let flags = flags
        | mac0(cue)
        | if (0..=4096).contains(&(cue >> 12)) {
            0
        } else {
            1 << 12
        };
    flags | if flags & SUMMARY != 0 { 1 << 31 } else { 0 }
}

pub fn point(matrix: &Matrix, vertex: [i16; 3]) -> Point {
    let (mut point, ratio) = geometry(matrix, vertex);
    point.flags = finish(point.flags, ratio);
    point
}

pub fn triangle(matrix: &Matrix, vertices: [[i16; 3]; 3]) -> ([Point; 3], u32) {
    let projected = vertices.map(|v| geometry(matrix, v));
    let flags = finish(
        projected.iter().fold(0, |f, p| f | p.0.flags),
        projected[2].1,
    );
    (projected.map(|p| p.0), flags)
}

pub fn area(points: [Point; 3]) -> i32 {
    let [a, b, c] = points.map(|p| p.xy.map(i32::from));
    a[0] * (b[1] - c[1]) + b[0] * (c[1] - a[1]) + c[0] * (a[1] - b[1])
}
