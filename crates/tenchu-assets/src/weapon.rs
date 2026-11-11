use crate::{
    ReadAt, Result,
    executable::Executable,
    game::GameAssets,
    i16_le,
    model::{self, Geometry},
    short3, u32_le,
};
use std::{collections::BTreeMap, sync::Arc};

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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Hitbox {
    pub kind: i16,
    pub center: [i16; 3],
    pub radius: i16,
    pub trail: [[i16; 3]; 2],
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
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

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct Contact {
    pub part: usize,
    pub hitbox: Hitbox,
}

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct ImportedAttack {
    pub contacts: [Option<Contact>; 2],
    /// Next action with no horizontal direction, right, or left held.
    pub combo: [Option<i16>; 3],
    pub turn_on_exit: bool,
    pub holster: Option<[i16; 2]>,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct WeaponTransition {
    pub frame: i16,
    pub sound: bool,
}

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub enum Draw {
    Fixed,
    Single,
    Dual,
}

impl Draw {
    pub fn gaisen(kind: i16) -> Self {
        use kind::*;
        match kind {
            KODATI | JYUTE | EN | KATANA_2 => Self::Dual,
            KOZUKA | NINJA | KEITOU | KATANA_0 | HOUTOU | KATANA_1 => Self::Single,
            _ => Self::Fixed,
        }
    }

    pub fn swap<T>(self, slots: &mut [T; 5]) {
        if matches!(self, Self::Single | Self::Dual) {
            slots.swap(0, 2);
        }
        if matches!(self, Self::Dual) {
            slots.swap(1, 3);
        }
    }
}

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct ImportedModel {
    /// Disc-qualified identity; artwork packs use `source_id` independently.
    pub id: String,
    pub source_id: String,
    pub geometry: Vec<Geometry>,
    pub vram: Arc<crate::texture::Vram>,
}

#[derive(Clone)]
pub struct Attachment {
    pub part: usize,
    pub position: [i16; 3],
    pub melee: bool,
    /// Held right/left, stowed right/left, and always-visible accessory.
    pub slot: usize,
    pub model: Arc<ImportedModel>,
}

#[derive(Clone)]
pub struct Equipment {
    pub draw: Draw,
    pub attachments: Vec<Attachment>,
}

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Weapons {
    pub models: Arc<BTreeMap<i16, Vec<Geometry>>>,
    pub grips: Arc<BTreeMap<i16, Vec<Geometry>>>,
    pub names: BTreeMap<i16, String>,
    pub hitboxes: Vec<Hitbox>,
    pub attacks: Vec<Attack>,
    pub bow_timing: [[i16; 2]; 2],
    #[serde(default)]
    pub imported_attacks: BTreeMap<i16, ImportedAttack>,
    pub imported_weapon_frames: BTreeMap<i16, WeaponTransition>,
    pub imported_models: Arc<BTreeMap<i16, Arc<ImportedModel>>>,
}

impl Weapons {
    pub fn capture<C: crate::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        let Self {
            models,
            grips,
            names,
            hitboxes,
            attacks,
            bow_timing,
            imported_attacks,
            imported_weapon_frames,
            imported_models,
        } = self;
        out.shared_arc(models, true)?;
        out.shared_arc(grips, true)?;
        out.value(names)?;
        out.value(hitboxes)?;
        out.value(attacks)?;
        out.value(bow_timing)?;
        out.value(imported_attacks)?;
        out.value(imported_weapon_frames)?;
        out.shared_arc(imported_models, true)
    }

    pub fn load<S: ReadAt>(game: &GameAssets<S>, exe: &Executable) -> Result<Self> {
        Self::load_tables(
            game,
            exe,
            "HUMAN/WEAPON",
            [0x8008986c, 0x80089aac, 0x800891dc, 0x80097714],
        )
    }

    pub(crate) fn load_trial<S: ReadAt>(game: &GameAssets<S>, exe: &Executable) -> Result<Self> {
        Self::load_tables(
            game,
            exe,
            "TRIAL/HUMAN/WEAPON",
            [0x8008abb8, 0x8008adf8, 0x8008a528, 0x800971cc],
        )
    }

    fn load_tables<S: ReadAt>(
        game: &GameAssets<S>,
        exe: &Executable,
        prefix: &str,
        tables: [u32; 4],
    ) -> Result<Self> {
        let mut models = BTreeMap::new();
        let mut names = BTreeMap::new();
        for i in 0..256 {
            let row = exe.bytes(tables[0] + i * 12, 12)?;
            let kind = i16_le(row, 4)?;
            if kind == -1 {
                break;
            }
            let name = exe.string(u32_le(row, 0)?)?;
            names.insert(kind, name.to_owned());
            models.insert(
                kind,
                model::tmd_model(&game.asset(&format!("{prefix}/{name}.TMD"))?)?,
            );
            if i == 255 {
                return Err("Unterminated weapon model table".into());
            }
        }
        let mut hitboxes = Vec::new();
        for i in 0..256 {
            let row = exe.bytes(tables[1] + i * 24, 24)?;
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
            let row = exe.bytes(tables[2] + i * 16, 16)?;
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
                    grips: grips(&models).into(),
                    models: models.into(),
                    hitboxes,
                    attacks,
                    imported_attacks: BTreeMap::new(),
                    imported_weapon_frames: BTreeMap::new(),
                    imported_models: Arc::default(),
                    bow_timing: [
                        [
                            i16_le(exe.bytes(tables[3], 4)?, 0)?,
                            i16_le(exe.bytes(tables[3], 4)?, 2)?,
                        ],
                        [
                            i16_le(exe.bytes(tables[3] + 4, 4)?, 0)?,
                            i16_le(exe.bytes(tables[3] + 4, 4)?, 2)?,
                        ],
                    ],
                });
            }
        }
        Err("Unterminated attack table".into())
    }
}

fn grips(models: &BTreeMap<i16, Vec<Geometry>>) -> BTreeMap<i16, Vec<Geometry>> {
    [kind::NINJA, kind::KODATI]
        .into_iter()
        .filter_map(|kind| {
            let source = models.get(&kind)?;
            let span = std::array::from_fn::<_, 3, _>(|axis| {
                source
                    .iter()
                    .flat_map(|g| &g.vertices)
                    .map(|v| i32::from(v[axis]).abs())
                    .max()
                    .unwrap_or(0)
            });
            let axis = (0..3).max_by_key(|&axis| span[axis]).unwrap();
            let mut grip = source.clone();
            for geometry in &mut grip {
                geometry.faces.retain(|face| {
                    face.corners.iter().all(|c| {
                        i32::from(geometry.vertices[usize::from(c.vertex)][axis]).abs()
                            <= span[axis] / 2
                    })
                });
            }
            Some((kind, grip))
        })
        .collect()
}
