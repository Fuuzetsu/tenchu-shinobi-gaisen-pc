//! TRIAL.EXE mission files and theme resources. See SOURCES.md.
mod mission;
mod setup;
pub use setup::Setup;
mod terrain;
pub use mission::{Cell, Mission, Objective, Placement, Point, Record, Waypoint};

use crate::{
    ReadAt, Result,
    executable::{Character, Executable, MotionRegistration},
    game::GameAssets,
    i16_le,
    model::{self, Geometry},
    short3,
    texture::{Image, Vram},
    u16_le, u32_le,
};
use sha2::{Digest, Sha256};

pub const THEMES: [&str; 12] = [
    "Dojo",
    "Merchant's estate",
    "Castle town",
    "Cave",
    "Mountain pass",
    "Samurai estate",
    "Temple",
    "Harbor",
    "Mountain road",
    "Castle",
    "Dark castle",
    "A.D. 2030",
];
pub const DEFAULT_TILES: [u8; 12] = [0, 0, 0, 0, 11, 0, 0, 1, 0, 0, 0, 0];
pub const EXECUTABLE_SHA256: &str =
    "f236f35ad8d4bed8c70f6cc1e69b62e7dfd27469e6bf1a838aa65932335d7230";

pub struct Tables {
    pub characters: Vec<Character>,
    pub common_motions: Vec<MotionRegistration>,
    pub appearance: Vec<Vec<i16>>,
    executable: Vec<u8>,
    vertices: Vec<[i16; 3]>,
    faces: Vec<Vec<TerrainFace>>,
}

struct TerrainFace {
    edge: usize,
    half: u8,
    texture: usize,
    vertices: [usize; 4],
}

impl Tables {
    pub fn load<S: ReadAt>(assets: &GameAssets<S>) -> Result<Self> {
        let entry = assets.disc.find("TENCHU/TRIAL.EXE")?;
        if entry.size != 555_008 {
            return Err("Unsupported TRIAL.EXE size".into());
        }
        let data = assets.disc.file(&entry).read(0, entry.size as usize)?;
        if format!("{:x}", Sha256::digest(&data)) != EXECUTABLE_SHA256 {
            return Err("Unsupported TRIAL.EXE; use an unmodified Shinobi Gaisen disc".into());
        }
        let exe = Executable::new(&data)?;
        let mut appearance = Vec::new();
        for theme in 0..12 {
            let address = u32_le(exe.bytes(0x80087dbc + (theme + 1) * 4, 4)?, 0)?;
            let mut kinds = Vec::new();
            for index in 0..64 {
                let kind = i16_le(exe.bytes(address + index * 2, 2)?, 0)?;
                if kind == -1 {
                    break;
                }
                kinds.push(kind);
            }
            appearance.push(kinds);
        }
        let vertices = exe
            .bytes(0x8008d720, 32 * 8)?
            .chunks_exact(8)
            .map(|row| short3(row, 0))
            .collect::<Result<_>>()?;
        let mut faces = Vec::new();
        for kind in 0..10 {
            let mut group = Vec::new();
            for row in exe
                .bytes(0x8008d820 + kind * 22 * 8, 22 * 8)?
                .chunks_exact(8)
            {
                let edge = i16_le(row, 0)?;
                if edge == -1 {
                    break;
                }
                group.push(TerrainFace {
                    edge: edge as usize,
                    half: row[2],
                    texture: row[3] as usize,
                    vertices: std::array::from_fn(|i| row[4 + i] as usize),
                });
            }
            faces.push(group);
        }
        Ok(Self {
            characters: exe.characters_at(0x80089dc0)?,
            common_motions: exe.motion_registry(0x80087df0)?,
            appearance,
            vertices,
            faces,
            executable: data,
        })
    }

    pub fn presets<S: ReadAt>(&self, assets: &GameAssets<S>) -> Result<Vec<Mission>> {
        (0..16)
            .map(|i| {
                Mission::decode(&assets.asset(&format!("TRIAL/PRESET/PRESET{i}.MSN"))?)
                    .map_err(|e| format!("Preset {}: {e}", i + 1))
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct Tile {
    pub flags: u16,
    pub textures: [i16; 10],
    pub ornament: Option<usize>,
    pub name: String,
}

impl Tile {
    pub fn kind(&self) -> usize {
        usize::from((self.flags >> 4) & 15) - 1
    }
    pub fn height(&self) -> i16 {
        [0, -1, -2, -3, -4, -1, -2, -99, 99, 0][self.kind()]
    }
    pub fn supports_actor(&self) -> bool {
        self.kind() < 5
    }
    pub fn is_ornament(&self) -> bool {
        self.flags & 0xc00 != 0
    }
}

pub struct Theme {
    pub tiles: Vec<Tile>,
    pub images: Vec<Image>,
    pub ornaments: Vec<Vec<Geometry>>,
    pub vram: Vram,
}

impl Theme {
    pub fn load<S: ReadAt>(assets: &GameAssets<S>, theme: usize) -> Result<Self> {
        if theme >= THEMES.len() {
            return Err("Unknown trial theme".into());
        }
        let bytes = assets.asset(&format!("TRIAL/THEME/THEME{theme}.LND"))?;
        if !bytes.len().is_multiple_of(24) || bytes.len() / 24 > 64 {
            return Err("Invalid trial tile definitions".into());
        }
        let mut vram = Vram::default();
        let images =
            vram.upload_tpd_images(&assets.asset(&format!("TRIAL/THEME/THEME{theme}.TPD"))?)?;
        let mut ornaments = Vec::new();
        let mut tiles = Vec::new();
        for row in bytes.chunks_exact(24) {
            let flags = u16_le(row, 0)?;
            if !(1..=10).contains(&((flags >> 4) & 15)) {
                return Err("Invalid trial terrain kind".into());
            }
            let mut textures = [-1; 10];
            let (ornament, name) = if flags & 0xc00 != 0 {
                let name = crate::text(&row[4..24])?;
                let index = ornaments.len();
                ornaments.push(model::tmd_model(
                    &assets.asset(&format!("TRIAL/THEME/{name}.TMD"))?,
                )?);
                (Some(index), name)
            } else {
                for (index, texture) in textures.iter_mut().enumerate() {
                    *texture = i16_le(row, 4 + index * 2)?;
                    if *texture < -1 || *texture as usize >= images.len() && *texture != -1 {
                        return Err("Trial face texture outside theme".into());
                    }
                }
                let name = [
                    "Ground",
                    "1 m ledge",
                    "2 m ledge",
                    "3 m ledge",
                    "4 m ledge",
                    "1 m ramp",
                    "2 m ramp",
                    "Wall",
                    "Pit",
                    "Water",
                ][usize::from((flags >> 4) & 15) - 1]
                    .to_owned();
                (None, name)
            };
            tiles.push(Tile {
                flags,
                textures,
                ornament,
                name,
            });
        }
        Ok(Self {
            tiles,
            images,
            ornaments,
            vram,
        })
    }

    pub fn tile(&self, encoded: u8) -> Option<&Tile> {
        (encoded != 255)
            .then(|| self.tiles.get(usize::from(encoded & 63)))
            .flatten()
    }

    pub fn height(&self, mission: &Mission, x: i16, z: i16, ornaments: bool) -> i16 {
        let Some(cell) = mission.cell(x, z) else {
            return -99;
        };
        let mut height = self.tile(cell.ground).map_or(0, Tile::height);
        if ornaments {
            height += self.tile(cell.ornament).map_or(0, Tile::height);
        }
        height.clamp(-99, 99)
    }
}
