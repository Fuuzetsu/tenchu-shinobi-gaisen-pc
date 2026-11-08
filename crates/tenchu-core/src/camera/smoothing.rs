use crate::math::Position;
use crate::{Result, math::Math};

#[cfg(test)]
#[path = "smoothing_tests.rs"]
mod tests;

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub(crate) struct Smooth {
    pub shift: i16,
    pub acceleration: i16,
    speed: i16,
    before: [i16; 3],
}

impl From<[i16; 6]> for Smooth {
    fn from([shift, acceleration, speed, x, y, z]: [i16; 6]) -> Self {
        Self {
            shift,
            acceleration,
            speed,
            before: [x, y, z],
        }
    }
}

impl Smooth {
    pub fn state(&self) -> [i16; 6] {
        [
            self.shift,
            self.acceleration,
            self.speed,
            self.before[0],
            self.before[1],
            self.before[2],
        ]
    }

    pub fn delta(&mut self, from: Position, to: Position, math: &Math) -> Result<Position> {
        self.delta_with_length(from, to, |v| math.vector_distance(v))
    }

    pub fn delta_with_length(
        &mut self,
        from: Position,
        to: Position,
        distance: impl Fn(Position) -> i32,
    ) -> Result<Position> {
        let delta = std::array::from_fn(|i| to[i].wrapping_sub(from[i]));
        let length = distance(delta.into());
        if length == 0 {
            return Ok(Position::ZERO);
        }
        let narrow = delta.map(|v| v as i16);
        let limit = length.wrapping_shr(self.shift as u32);
        let mut theta = (0..3).fold(0i32, |sum, i| {
            sum.wrapping_add(i32::from(narrow[i]) * i32::from(self.before[i]))
        });
        let mut lengths = distance(narrow.map(i32::from).into())
            .wrapping_mul(distance(self.before.map(i32::from).into()));
        if theta >= i32::MAX >> 12 {
            theta /= 4096;
            lengths /= 4096;
        }
        let alignment = if lengths == 0 {
            0
        } else {
            theta
                .wrapping_shl(12)
                .checked_div(lengths)
                .ok_or("Camera alignment division overflow")?
        };
        let speed = (i32::from(self.speed).wrapping_mul(alignment.wrapping_add(4096)) / 8192)
            .wrapping_add(i32::from(self.acceleration))
            .min(limit);
        self.speed = speed as i16;
        self.before = narrow;
        Ok(narrow
            .map(|v| i32::from(v).wrapping_mul(speed) / length)
            .into())
    }
}
