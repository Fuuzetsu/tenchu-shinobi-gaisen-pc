use crate::math::Position;
use crate::{Result, camera::Mode, game::Game, ids::ATTR_SUSPEND, math::Math};
use tenchu_assets::audio::voice::{Dialect, Sourced};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Sound {
    pub id: i16,
    pub volume: u8,
    pub pan: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(default)]
pub struct BurningSound {
    pub spatial: bool,
    pub volume: u16,
    pub range: u16,
}

impl Default for BurningSound {
    fn default() -> Self {
        Self {
            spatial: true,
            volume: 35,
            range: 6000,
        }
    }
}

impl BurningSound {
    pub fn sound(
        self,
        listener: Listener,
        math: &Math,
        position: Position,
    ) -> Result<Option<Sound>> {
        if self.volume == 0 {
            return Ok(None);
        }
        let mut sound = Sound::local(0x39);
        if self.spatial {
            let delta = position.wrapping_sub(listener.position);
            let distance = math.length(delta);
            let range = i32::from(self.range.max(1));
            if distance >= range {
                return Ok(None);
            }
            // Preserve retail panning; attenuate in three dimensions, including near the source.
            sound.pan = listener
                .sound(math, Some(position), 0x39)?
                .map_or(64, |s| s.pan);
            let gain = f64::from(range - distance) / f64::from(range);
            sound.volume = (126.0 * gain * gain) as u8;
        }
        sound.volume = (u32::from(sound.volume) * u32::from(self.volume.min(100)) / 100) as u8;
        Ok((sound.volume > 0).then_some(sound))
    }
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
        self.sounds_with_effects(self.effects.sounds.iter().copied())
    }

    pub fn sounds_with_effects(
        &self,
        effects: impl IntoIterator<Item = (Position, i16)>,
    ) -> Result<Vec<Sound>> {
        self.sounds_with_burning(effects, None)
    }

    pub fn sounds_with_burning(
        &self,
        effects: impl IntoIterator<Item = (Position, i16)>,
        burning: Option<BurningSound>,
    ) -> Result<Vec<Sound>> {
        Ok(self
            .audio_with_burning(effects, burning)?
            .into_iter()
            .map(|event| event.sound)
            .collect())
    }

    pub fn audio_with_burning(
        &self,
        effects: impl IntoIterator<Item = (Position, i16)>,
        burning: Option<BurningSound>,
    ) -> Result<Vec<Sourced<Sound>>> {
        let listener = Listener {
            position: self.player().position,
            yaw: self.player().rotation[1],
            direction: (self.camera.mode == Mode::Direction).then_some(self.camera.direction[1]),
        };
        self.audio_with_listener(listener, effects, burning)
    }

    pub fn audio_with_listener(
        &self,
        listener: Listener,
        effects: impl IntoIterator<Item = (Position, i16)>,
        burning: Option<BurningSound>,
    ) -> Result<Vec<Sourced<Sound>>> {
        let mut sounds = Vec::new();
        for &(id, mut sound) in &self.intelligence.voices {
            let Some(actor) = self.roster.actors.get(id).and_then(Option::as_ref) else {
                continue;
            };
            let voice = (id == self.player_id && sound & 0xf0 == 0)
                .then(|| {
                    let key = if sound <= 5 {
                        crate::game::moveset::SELECTED_WEAPON_VOICE
                    } else {
                        crate::game::moveset::SELECTED_VOICE
                    };
                    self.mission.moveset_voices.get(&key).or_else(|| {
                        self.moveset
                            .kind
                            .and_then(|kind| self.mission.moveset_voices.get(&kind))
                    })
                })
                .flatten();
            if sound & 0xf0 == 0 {
                if sound > 5 && (self.cinematic.frame || actor.attribute & ATTR_SUSPEND != 0) {
                    continue;
                }
                let weapon_program = actor
                    .sheathing
                    .as_ref()
                    .and_then(|s| s.weapon_program)
                    .filter(|_| {
                        sound <= 1
                            && matches!(
                                actor.motion.action,
                                crate::ids::MOT_STATE_DRAW | crate::ids::MOT_STATE_SHEATHE
                            )
                    });
                let program = if let Some(program) = weapon_program.or(actor.voice_program) {
                    program
                } else if (actor.kind as u16) < 2 {
                    5
                } else {
                    6 + self
                        .sound_appearance
                        .iter()
                        .position(|&kind| kind == actor.kind)
                        .unwrap_or(self.sound_appearance.len()) as i16
                };
                sound = if let Some(voice) = voice {
                    voice.cue(Dialect::Gaisen, sound as u8, actor.health == 0)
                } else {
                    sound | program << 4
                };
            }
            if let Some(sound) = listener.sound(
                &self.math,
                (id != self.player_id).then_some(actor.position),
                sound,
            )? {
                sounds.push(Sourced {
                    sound,
                    bank: voice.map(|v| v.bank.clone()),
                });
            }
        }
        for (position, sound) in effects {
            let position =
                (!matches!(sound, 0x0b | 0x0e | 0x27 | 0x39 | 0x41 | 0x43)).then_some(position);
            if let Some(sound) = listener.sound(&self.math, position, sound)? {
                sounds.push(sound.into());
            }
        }
        for &position in &self.effects.burning_sounds {
            let sound = if let Some(settings) = burning {
                settings.sound(listener, &self.math, position)?
            } else {
                listener.sound(&self.math, Some(position), 0x39)?
            };
            if let Some(sound) = sound {
                sounds.push(sound.into());
            }
        }
        Ok(sounds)
    }
}

#[cfg(test)]
mod tests;
