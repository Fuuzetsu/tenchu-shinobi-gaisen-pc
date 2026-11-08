use crate::{
    math::{Math, Matrix, Position},
    visibility::project,
};

#[derive(Clone, Copy, Debug)]
pub struct View {
    pub eye: Position,
    pub look: Position,
    pub focus: Position,
    pub matrix: Matrix,
    pub distance: i32,
}

impl View {
    pub fn actor(&self, transform: &Matrix) -> Option<u16> {
        let (screen, z) = project(&self.matrix.compose(transform), [0; 3]);
        let depth = z >> 4;
        (depth != 0
            && i32::from(depth) <= self.distance >> 4
            && i32::from(screen[0]).abs() <= 320
            && i32::from(screen[1]).abs() <= 240)
            .then_some(depth)
    }

    pub fn object(&self, position: Position, size: i32) -> bool {
        let delta = position.wrapping_sub(self.focus);
        if size < 700 && self.vertically_hidden(position, delta, 1500) {
            return false;
        }
        let distance = delta.into_iter().map(i32::wrapping_abs).max().unwrap();
        if distance > self.distance.wrapping_mul(3) / 4 {
            return false;
        }
        if distance <= 3000 {
            return true;
        }
        let point = position
            .wrapping_sub(self.eye)
            .wrapping_sub(Position::new(0, size, 0));
        let (screen, _) = project(
            &Matrix {
                rotation: self.matrix.rotation,
                translation: Position::ZERO,
            },
            point.map(|v| v as i16).to_array(),
        );
        i32::from(screen[0]).abs() <= 320 && i32::from(screen[1]).abs() <= 256
    }

    pub fn effect(&self, position: Position, kind: u16, math: &Math) -> bool {
        let delta = position.wrapping_sub(self.focus);
        if delta.into_iter().map(i32::wrapping_abs).max().unwrap()
            > self.distance.wrapping_mul(3) >> 2
        {
            return false;
        }
        if matches!(kind, 16 | 17) {
            return true;
        }
        if self.vertically_hidden(position, delta, 500) {
            return false;
        }
        let direction = self.look.wrapping_sub(self.eye).map(|v| v / 100);
        let delta = position.wrapping_sub(self.eye).map(|v| v / 100);
        let length =
            super::camera::length(delta, math).wrapping_mul(super::camera::length(direction, math));
        let length = if length == 0 { 1 } else { length };
        let dot = delta
            .into_iter()
            .zip(direction)
            .fold(0i32, |sum, (a, b)| sum.wrapping_add(a.wrapping_mul(b)));
        dot.wrapping_mul(10).wrapping_div(length) > 6
    }

    fn vertically_hidden(&self, position: Position, delta: Position, near: i32) -> bool {
        let pitch = self.look.y.wrapping_sub(self.eye.y);
        if pitch >= 1501 {
            position.y < self.eye.y
        } else if pitch <= -1501 {
            let nearby = delta.y.wrapping_abs() < 2200
                && delta.x.wrapping_abs() < near
                && delta.z.wrapping_abs() < near;
            !nearby && self.eye.y < position.y
        } else {
            false
        }
    }
}
