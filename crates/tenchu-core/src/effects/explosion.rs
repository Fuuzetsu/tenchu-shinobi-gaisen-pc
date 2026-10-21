use super::*;
use crate::math::Position;

#[derive(Clone, Debug)]
pub struct Explosion {
    pub position: Position,
    pub velocity: [i16; 3],
    pub rotation: i32,
    pub scale: i32,
    pub time: u8,
    pub mode: u8,
}

impl Effects {
    pub fn explosion(&mut self, position: Position, velocity: [i16; 3], rng: &mut Random) {
        let slot = self.allocate();
        self.slots[slot] = Some(Effect::Explosion(Explosion {
            position,
            velocity,
            rotation: (rng.next15() % 360) * 4096,
            scale: 4096,
            time: 5,
            mode: 0,
        }));
        self.scatter_bleeds(
            position,
            150,
            crate::effects::Burst {
                spread: 200,
                count: 20,
                lifetime: 30,
                color: [255, 255, 0],
            },
            rng,
        );
    }

    pub fn embers(
        &mut self,
        position: Position,
        power: [i16; 3],
        count: i16,
        rng: &mut Random,
    ) -> Result<()> {
        for _ in 0..count {
            if power.contains(&0) {
                return Err("Ember has zero velocity range".into());
            }
            let slot = self.allocate();
            let scale = rng.next15() % 4096 + 4096;
            let rotation = (rng.next15() % 360) * 4096;
            let power = power.map(i32::from);
            let velocity = [
                rng.next15() % power[0] - power[0] / 2,
                -(rng.next15() % power[1] + power[1] / 2),
                rng.next15() % power[2] - power[2] / 2,
            ]
            .map(|v| v as i16);
            let time = (rng.next15() % 15 + 15) as u8;
            self.slots[slot] = Some(Effect::Ember(Explosion {
                position,
                velocity,
                rotation,
                scale,
                time,
                mode: 0,
            }));
        }
        Ok(())
    }

    pub(super) fn update_explosion(
        &mut self,
        effect: &mut Explosion,
        ember: bool,
        view: View,
    ) -> bool {
        let mut color = 128;
        let mut alive = true;
        let image;
        if ember {
            if effect.mode == 0 {
                if effect.time == 0 {
                    effect.mode = 1;
                    effect.time = 30;
                }
            } else {
                color = ((u32::from(effect.time) << 7) / 30) as u8;
                alive = effect.time != 0;
            }
            effect.velocity[1] = effect.velocity[1].wrapping_add(5);
            effect.scale = effect.scale.wrapping_add(3072);
            image = 2;
        } else {
            image = match effect.mode {
                0 => {
                    if effect.time == 0 {
                        effect.time = 3;
                        effect.mode = 1;
                    } else {
                        effect.scale = effect.scale.wrapping_add(8192);
                        effect.rotation = effect.rotation.wrapping_add(100 * 4096);
                    }
                    0
                }
                1 => {
                    if effect.time == 0 {
                        effect.time = 5;
                        effect.mode = 2;
                    }
                    effect.scale = effect.scale.wrapping_add(8192);
                    effect.rotation = effect.rotation.wrapping_add(100 * 4096);
                    1
                }
                _ => {
                    color = ((u32::from(effect.time) << 7) / 5) as u8;
                    effect.scale = effect.scale.wrapping_sub(819);
                    effect.rotation = effect.rotation.wrapping_add(90 * 4096);
                    alive = effect.time != 0;
                    1
                }
            };
        }
        effect.time = effect.time.wrapping_sub(1);
        for i in 0..3 {
            effect.position[i] = effect.position[i].wrapping_add(i32::from(effect.velocity[i]));
        }
        self.sprite(
            Graphic::Explosion(image),
            effect.position,
            effect.scale,
            color,
            effect.rotation,
            view,
        );
        alive
    }
}
