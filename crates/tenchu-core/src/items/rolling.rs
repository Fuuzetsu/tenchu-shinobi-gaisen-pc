use crate::math::Position;
use crate::{Result, actor::Context, terrain};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Status {
    #[default]
    Airborne,
    Water,
    Ground,
    Wall,
    Settled,
    OutsideTerrain,
}

impl Status {
    pub fn source_id(self) -> u8 {
        match self {
            Self::Airborne => 0,
            Self::Water => 1,
            Self::Ground => 2,
            Self::Wall => 3,
            Self::Settled => 4,
            Self::OutsideTerrain => 5,
        }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Rolling {
    pub hint: Option<tenchu_assets::terrain::Node>,
    pub velocity: [i16; 3],
    pub status: Status,
}

impl Rolling {
    pub(super) fn step(&mut self, position: &mut Position, ctx: &mut Context) -> Result<()> {
        if self.status == Status::Settled {
            return Ok(());
        }
        let previous = *position;
        *position = Position::from_fn(|i| position[i].wrapping_add(i32::from(self.velocity[i])));
        let floor = terrain::particle_floor(
            &ctx.mission.terrain,
            ctx.cursor,
            *position,
            &mut self.hint,
            0,
        )?;
        if floor >= position[1] {
            self.status = Status::Airborne;
            self.velocity[1] = self.velocity[1].wrapping_add(15);
            return Ok(());
        }
        *position = previous;
        let probe = terrain::vector(&ctx.mission.terrain, ctx.cursor, *position, 500, 0)?;
        if self.hint.is_none()
            && terrain::particle_floor(
                &ctx.mission.terrain,
                ctx.cursor,
                *position,
                &mut self.hint,
                0,
            )? == terrain::NONE
        {
            self.velocity = if self.status == Status::OutsideTerrain {
                std::array::from_fn(|_| (ctx.random.next15() % 1600 - 800) as i16)
            } else {
                [0, 250, 0]
            };
            self.status = Status::OutsideTerrain;
            return Ok(());
        }
        let reflect = ctx.body.reflect_move[usize::from(probe.vector)].map(i32::from);
        if probe.vector != 0 && probe.height > 500 {
            self.velocity[0] = (reflect[0] * (i32::from(self.velocity[0]).abs() / 4)) as i16;
            self.velocity[1] /= 2;
            self.velocity[2] = (reflect[1] * (i32::from(self.velocity[2]).abs() / 4)) as i16;
            self.status = Status::Wall;
        } else if probe.attribute & 4 != 0 {
            self.velocity[0] = (ctx.random.next15() % 20 - 10) as i16;
            self.velocity[2] = (ctx.random.next15() % 20 - 10) as i16;
            if self.velocity[1] > 20 {
                ctx.effects.splash(*position, [8192; 2], 4);
                self.status = Status::Water;
            }
            self.velocity[1] = (-i32::from(self.velocity[1]) / 8) as i16;
        } else {
            self.velocity[0] /= 2;
            self.velocity[2] /= 2;
            if probe.level != terrain::NONE && probe.height <= 1500 {
                position[1] = probe.level;
                if self.velocity[1] < 46 {
                    self.status = Status::Settled;
                    return Ok(());
                }
                self.velocity[0] = self.velocity[0]
                    .wrapping_add((reflect[0] * (ctx.random.next15() % 25 + 25)) as i16);
                self.velocity[2] = self.velocity[2]
                    .wrapping_add((reflect[1] * (ctx.random.next15() % 25 + 25)) as i16);
                self.velocity[1] = (-i32::from(self.velocity[1]).abs() / 2) as i16;
                self.status = Status::Ground;
            } else {
                self.velocity[1] =
                    (i32::from(self.velocity[1]).abs() / 2 + ctx.random.next15() % 25 + 25) as i16;
                self.status = Status::Wall;
            }
        }
        Ok(())
    }
}
