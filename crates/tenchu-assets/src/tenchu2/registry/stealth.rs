use crate::{Result, i16_le, slice, u32_le};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StealthEvent {
    pub frame: i16,
    pub effect: StealthEffect,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum StealthEffect {
    VictimVoice(u8),
    Blood { joint: usize, turn: i16 },
    AttackerCue(u8),
    HoldWeapon,
    ReleaseWeapon,
    FreezeRoot,
    Finish,
    FocusVictim,
}

pub(super) type Events = [[Vec<StealthEvent>; 7]; 3];

pub(super) fn read(data: &[u8], edition: super::Edition) -> Result<Events> {
    let addresses = edition.addresses();
    let mut events: Events = std::array::from_fn(|_| std::array::from_fn(|_| Vec::new()));
    for (hero, styles) in events.iter_mut().enumerate() {
        for (style, events) in styles.iter_mut().enumerate() {
            let pointer = u32_le(data, addresses.characters - 0xb4 + (hero * 7 + style) * 4)?;
            let offset = pointer
                .checked_sub(addresses.module_base)
                .ok_or("Invalid Tenchu 2 stealth event pointer")? as usize;
            for index in 0..256 {
                let row = slice(data, offset + index * 4, 4)?;
                let mut frame = i16_le(row, 0)?;
                if frame == -1 {
                    break;
                }
                if frame < 0 {
                    return Err("Invalid Tenchu 2 stealth event frame".into());
                }
                if edition == super::Edition::Europe {
                    frame = (i32::from(frame) * 5 / 6) as i16;
                    if events.last().is_some_and(|event| event.frame == frame) {
                        frame += 1;
                    }
                }
                let effect = match row[2] {
                    0 => StealthEffect::VictimVoice(row[3]),
                    1 => StealthEffect::Blood {
                        joint: usize::from(row[3] & 63),
                        turn: i16::from(row[3] >> 6) * 1024,
                    },
                    2 => StealthEffect::AttackerCue(row[3]),
                    4 => match row[3] & 15 {
                        1 => StealthEffect::HoldWeapon,
                        2 => StealthEffect::ReleaseWeapon,
                        3 => StealthEffect::FreezeRoot,
                        4 => StealthEffect::Finish,
                        5 => StealthEffect::FocusVictim,
                        _ => return Err("Unknown Tenchu 2 stealth control".into()),
                    },
                    _ => return Err("Unknown Tenchu 2 stealth event".into()),
                };
                events.push(StealthEvent { frame, effect });
            }
            if events.len() == 256 {
                return Err("Unterminated Tenchu 2 stealth events".into());
            }
        }
    }
    Ok(events)
}
