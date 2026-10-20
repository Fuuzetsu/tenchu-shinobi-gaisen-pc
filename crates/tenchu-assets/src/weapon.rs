use crate::{
    ReadAt, Result,
    executable::Executable,
    game::GameAssets,
    i16_le,
    model::{self, Geometry},
    short3, u32_le,
};
use std::collections::BTreeMap;

pub mod kind {
    pub const CLAW: i16 = 0x01;
    pub const FIST: i16 = 0x02;
    pub const JAW: i16 = 0x03;
    pub const KODATI: i16 = 0x04;
    pub const JYUTE: i16 = 0x05;
    pub const EN: i16 = 0x07;
    pub const ANDON: i16 = 0x09;
    pub const JYURUR: i16 = 0x0a;
    pub const KOZUKA: i16 = 0x0c;
    pub const IKARI: i16 = 0x10;
    pub const BOU: i16 = 0x11;
    pub const SABRE: i16 = 0x12;
    pub const NINJA: i16 = 0x13;
    pub const KEITOU: i16 = 0x14;
    pub const KEITOUB: i16 = 0x15;
    pub const KATANA_0: i16 = 0x16;
    pub const HOUTOU: i16 = 0x19;
    pub const KATANA_1: i16 = 0x1c;
    pub const KATANA_2: i16 = 0x1f;
    pub const CROWR: i16 = 0x20;
    pub const YARI: i16 = 0x22;
    pub const KABUTUTI: i16 = 0x23;
    pub const SASUMATA: i16 = 0x24;
    pub const HALBERT: i16 = 0x25;
    pub const KON: i16 = 0x26;
    pub const NAGI: i16 = 0x27;
    pub const ENGETU: i16 = 0x28;
    pub const SEVEN: i16 = 0x29;
    pub const KATANAL: i16 = 0x2a;
    pub const SAYAL: i16 = 0x2b;
    pub const TUKAL: i16 = 0x2c;
    pub const TUKAANI: i16 = 0x2d;
    pub const TEPPO: i16 = 0x30;
    pub const GUN: i16 = 0x31;
    pub const YUMI: i16 = 0x32;
    pub const KATAYUMI: i16 = 0x35;
}

#[derive(Clone, Debug)]
pub struct Hitbox {
    pub kind: i16,
    pub center: [i16; 3],
    pub radius: i16,
    pub trail: [[i16; 3]; 2],
}

#[derive(Clone, Debug)]
pub struct Attack {
    pub clip: i16,
    pub power: i16,
    pub start: i16,
    pub end: i16,
    pub combo: i16,
    pub steering: i16,
    pub trail_start: i16,
    pub trail_end: i16,
}

pub struct Weapons {
    pub models: BTreeMap<i16, Vec<Geometry>>,
    pub names: BTreeMap<i16, String>,
    pub hitboxes: Vec<Hitbox>,
    pub attacks: Vec<Attack>,
    pub bow_timing: [[i16; 2]; 2],
}

impl Weapons {
    pub fn load<S: ReadAt>(game: &GameAssets<S>, exe: &Executable) -> Result<Self> {
        let mut models = BTreeMap::new();
        let mut names = BTreeMap::new();
        for i in 0..256 {
            let row = exe.bytes(0x8008986c + i * 12, 12)?;
            let kind = i16_le(row, 4)?;
            if kind == -1 {
                break;
            }
            let name = exe.string(u32_le(row, 0)?)?;
            names.insert(kind, name.to_owned());
            models.insert(
                kind,
                model::tmd_model(&game.asset(&format!("HUMAN/WEAPON/{name}.TMD"))?)?,
            );
            if i == 255 {
                return Err("Unterminated weapon model table".into());
            }
        }
        let mut hitboxes = Vec::new();
        for i in 0..256 {
            let row = exe.bytes(0x80089aac + i * 24, 24)?;
            let kind = i16_le(row, 22)?;
            if kind == -1 {
                break;
            }
            hitboxes.push(Hitbox {
                kind,
                center: short3(row, 0)?,
                radius: i16_le(row, 6)?,
                trail: [short3(row, 8)?, short3(row, 16)?],
            });
            if i == 255 {
                return Err("Unterminated weapon hitbox table".into());
            }
        }
        let mut attacks = Vec::new();
        for i in 0..256 {
            let row = exe.bytes(0x800891dc + i * 16, 16)?;
            let clip = i16_le(row, 0)?;
            attacks.push(Attack {
                clip,
                power: i16_le(row, 2)?,
                start: i16_le(row, 4)?,
                end: i16_le(row, 6)?,
                combo: i16_le(row, 8)?,
                steering: i16_le(row, 10)?,
                trail_start: i16_le(row, 12)?,
                trail_end: i16_le(row, 14)?,
            });
            if clip == -1 {
                return Ok(Self {
                    names,
                    models,
                    hitboxes,
                    attacks,
                    bow_timing: [
                        [
                            i16_le(exe.bytes(0x80097714, 4)?, 0)?,
                            i16_le(exe.bytes(0x80097714, 4)?, 2)?,
                        ],
                        [
                            i16_le(exe.bytes(0x80097718, 4)?, 0)?,
                            i16_le(exe.bytes(0x80097718, 4)?, 2)?,
                        ],
                    ],
                });
            }
        }
        Err("Unterminated attack table".into())
    }
}
