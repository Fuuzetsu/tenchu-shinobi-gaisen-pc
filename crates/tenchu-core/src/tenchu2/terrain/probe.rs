use super::{Cursor, Query, level};
use crate::{Result, math::Position};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::collision::{Collision, Rules};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Probe {
    pub floor: Option<i32>,
    pub height: i32,
    pub material: u16,
    pub blocked: u16,
    pub lower: u16,
    pub higher: u16,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Options {
    pub avoid_water: bool,
    pub ignore_steps: bool,
}

impl Probe {
    pub fn sample(
        &mut self,
        map: &Collision,
        cursor: &mut Cursor,
        rules: &Rules,
        position: Position,
        width: i16,
        options: Options,
    ) -> Result<()> {
        if self.lower != 0 || self.higher != 0 || self.height > 1999 {
            cursor.current = None;
        }
        self.floor = level(map, cursor, position, Query::Uncached)?;
        let category = usize::from(cursor.material >> 12 & 3);
        if rules.blocked[usize::from(self.material >> 12 & 3)][category] {
            self.floor = None;
        }
        self.material = cursor.material;
        self.lower = 0;
        self.higher = 0;
        let Some(floor) = self.floor else {
            self.height = 0;
            self.blocked = 15;
            return Ok(());
        };
        self.height = floor.wrapping_sub(position.y);
        self.blocked = 0;
        for (i, [dx, dz]) in rules.corners.into_iter().enumerate() {
            let corner = Position::new(
                position.x.wrapping_add(i32::from(dx) * i32::from(width)),
                position.y,
                position.z.wrapping_add(i32::from(dz) * i32::from(width)),
            );
            let height = level(map, cursor, corner, Query::Uncached)?
                .filter(|_| !(options.avoid_water && cursor.material & 0xf00 == 0x200));
            let Some(height) = height else {
                self.blocked |= 1 << i;
                continue;
            };
            if !options.ignore_steps
                && (rules.blocked[category][usize::from(cursor.material >> 12 & 3)]
                    || (self.height < i32::from(width) * 4
                        && floor.wrapping_sub(height) > 500
                        && (self.material | cursor.material) & 0x8000 == 0))
            {
                self.blocked |= 1 << i;
            } else if floor < height {
                self.lower |= 1 << i;
            } else if height < floor {
                self.higher |= 1 << i;
            }
        }
        Ok(())
    }
}
