use crate::{
    Result,
    math::{Math, Position},
    random::Random,
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Sound {
    pub id: u8,
    pub slot: u8,
    pub note: u8,
    pub volume: [u8; 2],
}

#[derive(Clone, Copy, Debug)]
pub struct Listener {
    pub actor: usize,
    pub position: Position,
    pub yaw: i16,
    pub distance: i32,
}

impl Listener {
    pub fn parameters(self, position: Option<Position>, math: &Math) -> Result<Option<(i32, u8)>> {
        let Some(position) = position else {
            return Ok(Some((0, 127)));
        };
        let delta = position.wrapping_sub(self.position);
        if delta.x.wrapping_abs() > 32767
            || delta.z.wrapping_abs() > 32767
            || delta.y.wrapping_abs() > 16383
        {
            return Ok(None);
        }
        let distance = math.length(delta) >> 4;
        if distance >= self.distance {
            return Ok(None);
        }
        let volume = 127 - distance * 127 / self.distance;
        let direction = (i32::from(math.atan2(delta.x.wrapping_neg(), delta.z.wrapping_neg())?)
            - i32::from(self.yaw))
            >> 4;
        Ok(Some((direction, volume as u8)))
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Audio {
    next_slot: u8,
    program_ticks: [i32; 16],
    last_base: i32,
    base_slot: u8,
    variant_slot: u8,
    #[serde(default)]
    extra_voice_ticks: std::collections::BTreeMap<i16, i32>,
}

impl Audio {
    pub fn extra_voice_ready(&mut self, kind: i16, cue: u8, tick: u32) -> bool {
        let previous = self.extra_voice_ticks.entry(kind).or_default();
        if kind >= 3 && (6..=9).contains(&cue) && (tick as i32).wrapping_sub(*previous) <= 29 {
            return false;
        }
        *previous = tick as i32;
        true
    }

    pub fn play(
        &mut self,
        id: u8,
        direction: i32,
        volume: u8,
        tick: u32,
        math: &Math,
    ) -> Option<Sound> {
        let program = usize::from(id >> 4);
        if program >= 5
            && (6..=9).contains(&(id & 15))
            && (tick as i32).wrapping_sub(self.program_ticks[program]) <= 29
        {
            return None;
        }
        let difference =
            ((math.pair(direction as i16)[0] * i32::from(volume)) >> 12).unsigned_abs() as u8;
        let volume = if direction & 2048 == 0 {
            [volume - difference, volume]
        } else {
            [volume, volume - difference]
        };
        let sound = Sound {
            id,
            slot: self.next_slot,
            note: 36,
            volume,
        };
        self.next_slot += 1;
        if self.next_slot > 23 {
            self.next_slot = 4;
        }
        self.program_ticks[program] = tick as i32;
        Some(sound)
    }

    pub fn ambient(
        &mut self,
        map: usize,
        interval: i16,
        tick: u32,
        random: &mut Random,
        math: &Math,
    ) -> Option<Sound> {
        if interval == 0 {
            return None;
        }
        let base = if interval < 0 {
            tick.is_multiple_of(30) && random.next15() % 10 == 0
        } else {
            let elapsed = (tick as i32).wrapping_sub(self.last_base);
            if elapsed < 1 || elapsed >= i32::from(interval) {
                self.last_base = tick as i32;
                true
            } else {
                false
            }
        };
        let (slot, id, volume, note) = if base {
            self.base_slot = 1 - self.base_slot;
            (self.base_slot, 0x2a, 127, 36)
        } else {
            if random.next15() % 300 != 0 {
                return None;
            }
            self.variant_slot = 1 - self.variant_slot;
            let (id, note) = if matches!(map, 21 | 23) {
                (
                    if random.next15() % 3 == 0 {
                        0x2f
                    } else {
                        0x2b + (random.next15() & 1) as u8
                    },
                    36,
                )
            } else {
                let note = 36 - (random.next15() % 4) as u8;
                (0x2b + (random.next15() & 1) as u8, note)
            };
            (self.variant_slot + 2, id, 63, note)
        };
        let saved = self.next_slot;
        self.next_slot = slot;
        let mut sound = self.play(id, 0, volume, tick, math);
        self.next_slot = saved;
        if let Some(sound) = &mut sound {
            sound.note = note;
        }
        sound
    }
}
