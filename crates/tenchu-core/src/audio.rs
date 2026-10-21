use crate::math::Position;
use crate::{Result, camera::Mode, game::Game, ids::ATTR_SUSPEND, math::Math};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sound {
    pub id: i16,
    pub volume: u8,
    pub pan: u8,
}

impl Sound {
    pub fn local(id: i16) -> Self {
        Self {
            id,
            volume: if id == 0x12 { 62 } else { 126 },
            pan: 64,
        }
    }
}

#[derive(Clone, Copy)]
pub struct Listener {
    pub position: Position,
    pub yaw: i16,
    pub direction: Option<i16>,
}

impl Listener {
    pub fn sound(&self, math: &Math, position: Option<Position>, id: i16) -> Result<Option<Sound>> {
        let Some(position) = position else {
            return Ok(Some(Sound::local(id)));
        };
        let [dx, dy, dz] = std::array::from_fn(|i| position[i].wrapping_sub(self.position[i]));
        let distance = math.length(Position::new(dx, 0, dz));
        let dy = dy.wrapping_abs();
        if distance >= 18000 || dy >= 10000 {
            return Ok(None);
        }
        let (direction, volume) = if distance < 2000 && dy < 2000 {
            (0, 127)
        } else {
            let volume = (127 - distance.wrapping_shl(7) / 18000).wrapping_mul(10000 - dy) / 10000;
            let mut direction =
                i32::from(math.atan2(dx.wrapping_neg(), dz.wrapping_neg())?) - i32::from(self.yaw);
            if let Some(yaw) = self.direction {
                direction -= i32::from(yaw);
            }
            if direction > 2048 {
                direction = 4096 - direction;
            } else if direction <= -2048 {
                direction += 4096;
            }
            (direction, volume)
        };
        let packed = direction.wrapping_shl(8) | volume;
        let direction = (packed >> 8) as i16;
        let pan = if direction > 0 {
            64 + ((packed >> 8) & 1023) / 16
        } else if direction < 0 {
            64 - i32::from(direction.wrapping_abs() & 1023) / 16
        } else {
            64
        };
        Ok(Some(Sound {
            id,
            volume: (((packed & 127) * 127) >> 7) as u8,
            pan: pan as u8,
        }))
    }
}

impl Game {
    pub fn sounds(&self) -> Result<Vec<Sound>> {
        let listener = Listener {
            position: self.player().position,
            yaw: self.player().rotation[1],
            direction: (self.camera.mode == Mode::Direction).then_some(self.camera.direction[1]),
        };
        let mut sounds = Vec::new();
        for &(id, mut sound) in &self.intelligence.voices {
            let Some(actor) = self.roster.actors.get(id).and_then(Option::as_ref) else {
                continue;
            };
            if sound & 0xf0 == 0 {
                if sound > 5 && (self.cinematic.frame || actor.attribute & ATTR_SUSPEND != 0) {
                    continue;
                }
                let program = if (actor.kind as u16) < 2 {
                    5
                } else {
                    6 + self
                        .sound_appearance
                        .iter()
                        .position(|&kind| kind == actor.kind)
                        .unwrap_or(self.sound_appearance.len()) as i16
                };
                sound |= program << 4;
            }
            if let Some(sound) = listener.sound(
                &self.math,
                (id != self.player_id).then_some(actor.position),
                sound,
            )? {
                sounds.push(sound);
            }
        }
        for &(position, sound) in &self.effects.sounds {
            let position =
                (!matches!(sound, 0x0b | 0x0e | 0x27 | 0x39 | 0x41 | 0x43)).then_some(position);
            if let Some(sound) = listener.sound(&self.math, position, sound)? {
                sounds.push(sound);
            }
        }
        Ok(sounds)
    }
}

#[cfg(test)]
mod tests;
