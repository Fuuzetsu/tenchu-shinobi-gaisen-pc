use super::Rolling;
use crate::math::Position;
use crate::{Result, actor::Context};

#[derive(Clone, Debug, Default)]
pub struct Flight {
    pub start: Position,
    pub end: Position,
    pub control: Position,
    pub count: u8,
    pub total: u8,
    pub rolling: bool,
}

impl Flight {
    pub(super) fn new(
        start: Position,
        end: Position,
        width: i32,
        height: i32,
        speed: i32,
        ctx: &mut Context,
    ) -> Self {
        let distance = ctx.math.vector_distance(end.wrapping_sub(start));
        let total = if speed > 0 {
            ((distance / speed) as u8).max(1)
        } else {
            1
        };
        let width = distance.wrapping_mul(width / 2) / 4096;
        let height = distance.wrapping_mul(height / 2) / 4096;
        let midpoint: Position = Position::from_fn(|i| start[i].wrapping_add(end[i]) / 2);
        let x = midpoint[0].wrapping_add(if width.wrapping_mul(2) > 0 {
            ctx.random.next15() % (width * 2) - width
        } else {
            -width
        });
        let y = midpoint[1].wrapping_sub(if height - height / 2 > 0 {
            ctx.random.next15() % (height - height / 2) + height / 2
        } else {
            height / 2
        });
        let z = midpoint[2].wrapping_add(if width.wrapping_mul(2) > 0 {
            ctx.random.next15() % (width * 2) - width
        } else {
            -width
        });
        Self {
            start,
            end,
            control: Position::new(x, y, z),
            count: total - 1,
            total,
            rolling: false,
        }
    }

    pub(super) fn step(
        &mut self,
        position: &mut Position,
        rolling: &mut Rolling,
        ctx: &mut Context,
    ) -> Result<()> {
        if self.rolling {
            return rolling.step(position, ctx);
        }
        if self.total == 0 {
            return Err("Projectile arc has zero duration".into());
        }
        let q = 4096 - (i32::from(self.count) << 12) / i32::from(self.total);
        let square = q.wrapping_mul(q) / 4096;
        let weights = [4096 - q * 2 + square, q * 2 - square * 2, square];
        let next: Position = Position::from_fn(|i| {
            weights[0]
                .wrapping_mul(self.start[i])
                .wrapping_add(weights[1].wrapping_mul(self.control[i]))
                .wrapping_add(weights[2].wrapping_mul(self.end[i]))
                / 4096
        });
        if self.count == 0 {
            *rolling = Rolling {
                hint: None,
                velocity: std::array::from_fn(|i| next[i].wrapping_sub(position[i]) as i16),
                status: 0,
            };
            self.rolling = true;
        } else {
            self.count -= 1;
        }
        *position = next;
        Ok(())
    }
}
