//! Optional resources from the user's Tenchu 2 disc. See docs/cross-version-research.md.
pub mod characters;
pub mod collision;
pub mod combat;
pub mod commands;
mod edition;
pub use edition::Edition;
pub mod layout;
pub mod level;
pub mod loot;
mod math;
pub use math::math_tables;
mod model;
pub mod motions;
pub mod music;
pub mod registry;
mod volume;
pub mod weapons;
use crate::{ReadAt, Result, disc::Disc, i16_le, motion, slice, u32_le, weapon};
use serde::{Deserialize, Serialize};
use volume::{Volume, hash};

fn executable(disc: &Disc<impl ReadAt>) -> Result<(Edition, Vec<u8>)> {
    use sha2::{Digest, Sha256};
    let edition = Edition::detect(disc)?;
    let entry = disc.find("TENCHU2/GAME.EXE")?;
    if entry.size > 2 * 1024 * 1024 {
        return Err("Excessive Tenchu 2 executable size".into());
    }
    let bytes = disc.file(&entry).read(0, entry.size as usize)?;
    if format!("{:x}", Sha256::digest(&bytes)) != edition.addresses().game_hash {
        return Err(format!("Unsupported {} GAME.EXE revision", edition.name()));
    }
    Ok((edition, bytes))
}

// Gaisen's waist/torso/head, arms, legs and weapon anchors in T2 bone order.
const BONES: [usize; 15] = [0, 1, 2, 3, 4, 9, 10, 6, 7, 8, 12, 13, 14, 5, 11];
pub const CLIPS: [i16; 12] = [
    0xb8, 0xb9, 0x4bd, 0x4be, 0xba, 0xbb, 0x4bf, 0x4c0, 0xbc, 0xbd, 0x4c1, 0xbe,
];

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Move {
    pub clip: motion::Clip,
    pub attack: weapon::Attack,
    pub part: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub secondary_part: Option<usize>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Moves {
    pub format: String,
    pub attacks: Vec<Move>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rest_yaw: Option<i16>,
}

impl Moves {
    pub fn validate(&self) -> Result<()> {
        if self.format != "tenchu2-tatsumaru-v1" || self.attacks.len() != CLIPS.len() {
            return Err("Not a supported Tatsumaru moveset".into());
        }
        for (&id, entry) in CLIPS.iter().zip(&self.attacks) {
            let Move {
                clip,
                attack,
                part,
                secondary_part,
            } = entry;
            if clip.id != id
                || attack.clip != id
                || !(1..=120).contains(&clip.duration)
                || clip.bones.len() != BONES.len()
                || !matches!(part, 9 | 12 | 13 | 14)
                || secondary_part.is_some_and(|p| !matches!(p, 9 | 12 | 13 | 14) || p == *part)
                || !(1..=100).contains(&attack.power)
                || !(1..attack.end).contains(&attack.start)
                || attack.end > clip.duration
                || !(0..=clip.duration).contains(&attack.combo)
                || !(0..=clip.duration).contains(&attack.steering)
                || attack.trail_start != 0
                || attack.trail_end != 0
            {
                return Err(format!("Invalid Tatsumaru attack {id:#x}"));
            }
            for track in std::iter::once(&clip.root).chain(&clip.bones) {
                if track.is_empty()
                    || track.len() > 512
                    || track[0].time != 0
                    || track.last().unwrap().time < clip.duration
                    || track.last().unwrap().time > 240
                    || track.windows(2).any(|p| p[0].time >= p[1].time)
                {
                    return Err(format!("Invalid Tatsumaru motion track {id:#x}"));
                }
            }
        }
        Ok(())
    }
}

pub fn import(disc: &Disc<impl ReadAt>) -> Result<Moves> {
    let volume = Volume::open(disc)?;
    let cab = volume.read(r"o:\cdimage\game\cab\game", "T_00.cab")?;
    let motion = cab_member(&cab, r"o:\graphic\motion\bin\mot", "t_00.mot")?;
    let table = volume.read(r"o:\cdimage\game\weapon", "atktbl.dat")?;
    let mut offsets = Vec::new();
    for i in 0..1024 {
        let at = u32_le(motion, i * 4)? as usize;
        if at == 0 {
            break;
        }
        slice(motion, at, 16)?;
        offsets.push(at);
        if i == 1023 {
            return Err("Unterminated Tenchu 2 motion table".into());
        }
    }
    let mut attacks = Vec::new();
    for id in CLIPS {
        let at = offsets
            .iter()
            .copied()
            .find(|&at| i16_le(motion, at).ok() == Some(id))
            .ok_or_else(|| format!("Missing Tatsumaru clip {id:#x}"))?;
        let duration = i16_le(motion, at + 2)?;
        if !(15..=16).contains(&i16_le(motion, at + 4)?) || !(1..=120).contains(&duration) {
            return Err("Unexpected Tatsumaru skeleton or duration".into());
        }
        let track = |field| motion::track(motion, u32_le(motion, field)? as usize, duration);
        let speed = |offset| {
            i16_le(motion, at + offset)?
                .try_into()
                .map_err(|_| "Tatsumaru motion exceeds Gaisen velocity range".to_owned())
        };
        let clip = motion::Clip {
            id,
            duration,
            sweep: sweep(motion, at)?,
            forward_speed: speed(8)?,
            side_speed: speed(10)?,
            root: track(at + 12)?,
            bones: BONES
                .iter()
                .map(|&i| track(at + 16 + i * 4))
                .collect::<Result<_>>()?,
        };
        let row = table
            .chunks_exact(16)
            .find(|r| i16_le(r, 0).ok() == Some(id))
            .ok_or_else(|| format!("Missing Tatsumaru hit timing {id:#x}"))?;
        let mask = u32_le(row, 12)?;
        let parts: Vec<_> = BONES
            .iter()
            .enumerate()
            .filter_map(|(part, &bone)| (mask & (1 << bone) != 0).then_some(part))
            .collect();
        if !(1..=2).contains(&parts.len()) || parts.len() != mask.count_ones() as usize {
            return Err("Unsupported Tatsumaru striking limbs".into());
        }
        attacks.push(Move {
            clip,
            attack: weapon::Attack {
                clip: id,
                power: i16::from(row[2]),
                steering: i16::from(row[3]),
                combo: i16::from(row[4]),
                start: i16::from(row[5]),
                end: i16::from(row[6]),
                // Gaisen's blade ribbons are inappropriate for fists and feet.
                trail_start: 0,
                trail_end: 0,
            },
            part: parts[0],
            secondary_part: parts.get(1).copied(),
        });
    }
    let moves = Moves {
        format: "tenchu2-tatsumaru-v1".into(),
        attacks,
        rest_yaw: Some({
            let idle = offsets
                .iter()
                .copied()
                .find(|&at| i16_le(motion, at).ok() == Some(0xa2))
                .ok_or("Missing Tatsumaru idle motion")?;
            let root = u32_le(motion, idle + 16)? as usize;
            i16_le(motion, root + 2)?
        }),
    };
    moves.validate()?;
    Ok(moves)
}

fn sweep(data: &[u8], at: usize) -> Result<i8> {
    // T2 negates the stored blend length when selecting a motion.
    (-i32::from(i16_le(data, at + 6)?))
        .try_into()
        .map_err(|_| "Tenchu 2 motion blend exceeds Gaisen range".into())
}

fn cab_member<'a>(data: &'a [u8], directory: &str, name: &str) -> Result<&'a [u8]> {
    let count = u32_le(data, 0)? as usize;
    if count > 4096 {
        return Err("Excessive Tenchu 2 CAB entry count".into());
    }
    let rows = slice(data, 4, count * 12)?;
    for (i, row) in rows.chunks_exact(12).enumerate() {
        if u32_le(row, 4)? != hash(directory) || u32_le(row, 8)? != hash(name) {
            continue;
        }
        let at = 4usize
            .checked_add(u32_le(row, 0)? as usize)
            .ok_or("CAB offset overflow")?;
        let end = if i + 1 < count {
            4usize
                .checked_add(u32_le(rows, (i + 1) * 12)? as usize)
                .ok_or("CAB offset overflow")?
        } else {
            data.len()
        };
        return slice(data, at, end.checked_sub(at).ok_or("Reversed CAB offsets")?);
    }
    Err(format!("Missing Tenchu 2 CAB member {name}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn malformed_bundle_offsets_are_errors() {
        let mut data = vec![0; 28];
        data[..4].copy_from_slice(&2u32.to_le_bytes());
        data[4..8].copy_from_slice(&24u32.to_le_bytes());
        data[8..12].copy_from_slice(&hash("dir").to_le_bytes());
        data[12..16].copy_from_slice(&hash("name").to_le_bytes());
        data[16..20].copy_from_slice(&8u32.to_le_bytes());
        assert!(cab_member(&data, "dir", "name").is_err());
        data[16..20].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(cab_member(&data, "dir", "name").is_err());
        data[..4].copy_from_slice(&u32::MAX.to_le_bytes());
        assert!(cab_member(&data, "dir", "name").is_err());
    }
}
