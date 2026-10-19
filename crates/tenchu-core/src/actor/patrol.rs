#[derive(Clone, Debug)]
pub struct Point {
    pub position: [i32; 2],
    pub range: i16,
    pub pad: u16,
}

#[derive(Clone, Debug)]
pub struct Patrol {
    pub points: Vec<Point>,
    pub index: usize,
    pub count: i16,
}

impl Patrol {
    pub fn new(points: &[([i32; 3], i32)], home: [i32; 3]) -> Self {
        let mut points: Vec<_> = points
            .iter()
            .map(|(p, _)| Point {
                position: [p[0], p[2]],
                range: 1500,
                pad: 0,
            })
            .collect();
        points.push(Point {
            position: [home[0], home[2]],
            range: (home[1] / 100) as i16,
            pad: 0xffff,
        });
        Self {
            points,
            index: 0,
            count: 0,
        }
    }
}
use super::Actor;
use crate::{
    Result,
    ids::ATTR_WALL,
    input::{LEFT, RIGHT, UP},
    math::Math,
};

impl Actor {
    pub fn patrol_input(&mut self, math: &Math, turn: i16) -> Result<u16> {
        let Some(patrol) = &mut self.patrol else {
            return Ok(0);
        };
        let point = patrol
            .points
            .get(patrol.index)
            .ok_or("Patrol index outside route")?;
        let dx = point.position[0].wrapping_sub(self.position[0]);
        let dz = point.position[1].wrapping_sub(self.position[2]);
        let distance = math.square_root(dx.wrapping_mul(dx).wrapping_add(dz.wrapping_mul(dz)));
        let range = i32::from(point.range);
        if self.attribute & ATTR_WALL != 0 {
            patrol.count = -30;
        }
        let previous = patrol.count;
        patrol.count = patrol.count.wrapping_add(1);
        let mut input = UP;
        if previous > 0 {
            let direction = math.direction(dx, dz, self.rotation[1])?;
            if turn <= direction {
                input |= RIGHT;
            } else if i32::from(direction) <= -i32::from(turn) {
                input |= LEFT;
            }
            if i32::from(direction).abs() > 500 {
                input &= LEFT | RIGHT;
            }
        }
        if distance <= range {
            patrol.index += 1;
            let next = patrol
                .points
                .get(patrol.index)
                .ok_or("Unterminated patrol route")?;
            if next.pad == 0xffff {
                patrol.index = 0;
                return Ok(0xf000);
            }
            input |= next.pad;
        }
        Ok(input)
    }
}
