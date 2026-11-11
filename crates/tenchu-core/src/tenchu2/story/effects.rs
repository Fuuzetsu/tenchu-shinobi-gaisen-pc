use crate::{
    Result,
    math::{Math, Position},
    random::Random,
    tenchu2::{
        effects::{self, Effect, blood::Spurt},
        terrain::Cursor,
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{collision::Collision, effects::Rules, story::Command};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Effects {
    pub tick: u64,
    pub particles: effects::Effects,
    pub frame: effects::Frame,
    pub cursor: Cursor,
}

impl Effects {
    pub fn dispatch(
        &mut self,
        command: &Command,
        math: &Math,
        rules: &Rules,
        terrain: &Collision,
        random: &mut Random,
    ) -> Result<()> {
        let Command::Effect {
            id,
            position,
            heading,
            value,
        } = *command
        else {
            return Ok(());
        };
        let position = Position::from(position);
        match id {
            2 | 29 => {
                self.particles.blood(
                    Spurt {
                        position,
                        yaw: heading,
                        speed: if id == 2 { 100 } else { 36 },
                        gravity: if id == 2 { 20 } else { 6 },
                    },
                    math,
                    rules,
                    random,
                )?;
                if let Some(Effect::Blood(blood)) = self.particles.entries.last_mut() {
                    // The source copies the fourth command word along with XYZ.
                    for drop in &mut blood.drops {
                        if drop.age < blood.lifetime {
                            drop.floor = value;
                        }
                    }
                }
            }
            3 => {
                self.particles.death_pool(
                    position.wrapping_sub(Position::new(0, 1000, 0)),
                    terrain,
                    &mut self.cursor,
                )?;
                if let Some(Effect::Pool(pool)) = self.particles.entries.last_mut() {
                    pool.lifetime = 3000;
                    if value == 2 {
                        pool.age = 60;
                    }
                }
            }
            4 | 25 | 26 | 34 => {
                let color = match id {
                    26 => [0, 96, 0],
                    34 => [128, 112, 112],
                    _ => [128, 0, 0],
                };
                for index in 0..if id == 34 { 2 } else { 4 } {
                    let direction = if id == 25 {
                        // The fourth source burst has a zero vector, not negative Z.
                        [[256, 0, 0], [-256, 0, 0], [0, 0, 256], [0; 3]][index]
                    } else if id == 34 {
                        [[256, 0, 0], [-256, 0, 0]][index]
                    } else {
                        math.velocity(heading, 4096, 0).map(|v| i32::from(v) >> 2)
                    };
                    self.particles.directional_blood(
                        position,
                        direction.into(),
                        color,
                        6,
                        rules,
                        random,
                    )?;
                }
            }
            _ => (),
        }
        Ok(())
    }
}
