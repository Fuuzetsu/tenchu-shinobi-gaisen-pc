use super::*;
use crate::math::Position;

#[derive(Clone, Debug)]
pub struct Blood {
    pub hint: Option<usize>,
    pub position: Position,
    pub velocity: [i16; 3],
    pub scale: i32,
    pub rotation: i32,
    pub time: i16,
    pub brightness: u16,
    pub sprite: u8,
    pub mode: u8,
    pub gore: bool,
}

impl Effects {
    pub fn blood(
        &mut self,
        position: Position,
        count: i16,
        time: i16,
        rng: &mut Random,
        env: &mut Environment,
    ) -> Result<()> {
        terrain::level(env.terrain, env.cursor, position, 0)?;
        let hint = Some(env.cursor.area);
        for _ in 0..count {
            let slot = self.allocate();
            let sprite = (rng.next15() % 2) as u8;
            let scale = rng.next15() % 4096 + 8192;
            let rotation = (rng.next15() % 360) * 4096;
            let velocity = [
                rng.next15() % 120 - 60,
                rng.next15() % 60 - 120,
                rng.next15() % 120 - 60,
            ]
            .map(|v| v as i16);
            let half = i32::from(time) / 2;
            let range = i32::from(time) - half;
            let time = (if range > 0 {
                rng.next15() % range + half
            } else {
                half
            }) as i16;
            self.slots[slot] = Some(Effect::Blood(Blood {
                hint,
                position,
                velocity,
                scale,
                rotation,
                time,
                brightness: 128,
                sprite,
                mode: 0,
                gore: false,
            }));
        }
        Ok(())
    }

    pub fn gore(
        &mut self,
        parent: ModelId,
        position: [i16; 3],
        velocity: [i16; 3],
        clock: i32,
        rng: &mut Random,
        scene: &mut Scene,
    ) -> Result<()> {
        let matrix = scene.world(parent)?;
        let world_position = matrix.transform(position);
        let slot = self.allocate();
        let sprite = (rng.next15() % 2) as u8;
        let rotation = (rng.next15() % 360) * 4096;
        let velocity = Matrix {
            translation: Position::ZERO,
            ..matrix
        }
        .transform(velocity)
        .map(|v| v as i16);
        let time = (rng.next15() % 15 + 10) as i16;
        self.slots[slot] = Some(Effect::Blood(Blood {
            hint: None,
            position: world_position,
            velocity: velocity.to_array(),
            scale: 8192,
            rotation,
            time,
            brightness: 128,
            sprite,
            mode: 0,
            gore: true,
        }));
        if clock & 3 == 0 {
            self.impact_ex(Impact {
                parent: Some(parent),
                position: position.map(i32::from).into(),
                size: [8192; 2],
                color: [[128; 3]; 2],
                rotation: 0,
                speed: 80,
                sprite: 2,
                count: 0,
                time: 3,
            });
        }
        Ok(())
    }

    pub(super) fn update_blood(
        &mut self,
        blood: &mut Blood,
        rng: &mut Random,
        view: View,
        env: &mut Environment,
    ) -> Result<bool> {
        let mut alive = true;
        match blood.mode {
            3 => {
                blood.brightness = blood.brightness.wrapping_sub(5);
                if blood.brightness as i16 <= 0 {
                    blood.brightness = 0;
                    alive = false;
                }
                blood.position.y = blood.position.y.wrapping_add(i32::from(blood.velocity[1]));
                self.draw_blood(blood, view, true);
                return Ok(alive);
            }
            2 => {
                let previous = blood.time;
                blood.time = blood.time.wrapping_sub(1);
                if previous <= 0 {
                    blood.time = 128;
                    blood.mode += 1;
                }
            }
            1 => {
                blood.scale = blood.scale.wrapping_add(rng.next15() % 4096);
                let previous = blood.time;
                blood.time = blood.time.wrapping_sub(1);
                if previous <= 0 {
                    blood.mode += 1;
                    blood.time = (rng.next15() % 90) as i16;
                }
            }
            _ => {
                blood.velocity[1] = blood.velocity[1].wrapping_add(10);
                let floor = terrain::particle_floor(
                    env.terrain,
                    env.cursor,
                    blood.position,
                    &mut blood.hint,
                    0,
                )?;
                if blood.position.y >= floor {
                    blood.velocity = [0; 3];
                    if floor != terrain::NONE {
                        blood.position.y = floor;
                    } else {
                        blood.velocity[1] = (rng.next15() % 8 + 8) as i16;
                        blood.rotation = 0;
                        blood.sprite = blood.sprite.wrapping_add(2);
                        blood.scale = rng.next15() % (4096 / 2 - 4096 / 3) + 4096 / 3;
                    }
                    blood.mode = 1;
                    blood.time = (rng.next15() % 10) as i16;
                    self.sounds.push((blood.position, 0x37));
                } else {
                    let previous = blood.time;
                    blood.time = blood.time.wrapping_sub(1);
                    alive = previous > 0;
                }
                if blood.gore || env.clock & 1 != 0 {
                    let radius = if blood.gore { 60 } else { 80 };
                    let position = blood.position.map(|p| {
                        p.wrapping_sub(radius)
                            .wrapping_add(rng.next15() % (radius * 2))
                    });
                    let time = if blood.gore {
                        7
                    } else {
                        (rng.next15() % 10 + 10) as u8
                    };
                    self.bleed(position, blood.velocity.map(|v| v / 2), time, [127, 16, 23]);
                }
            }
        }
        for i in 0..3 {
            blood.position[i] = blood.position[i].wrapping_add(i32::from(blood.velocity[i]));
        }
        self.draw_blood(blood, view, false);
        Ok(alive)
    }

    fn draw_blood(&mut self, blood: &Blood, view: View, fading: bool) {
        let (xy, z) = view.project(blood.position);
        if z <= 36 {
            return;
        }
        let scale = i32::from((blood.scale.wrapping_mul(300) / i32::from(z)) as i16) + 1;
        for stain in [false, true].into_iter().take(if fading { 2 } else { 1 }) {
            let color = if stain {
                (blood.brightness as i16 / 2) as u8
            } else {
                blood.brightness as u8
            };
            self.drawings.push(Drawing {
                graphic: Graphic::Blood {
                    sprite: blood.sprite,
                    stain,
                    fading,
                },
                xy,
                scale: [scale; 2],
                color: [color; 3],
                rotation: blood.rotation,
                depth: (z >> 2).clamp(0, 1249),
            });
        }
    }
}
