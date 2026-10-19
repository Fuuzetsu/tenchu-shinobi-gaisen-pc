use crate::{
    ReadAt, Result,
    executable::{Character, Executable, MotionRegistration, StoryActor},
    game::{GameAssets, Stage},
    i16_le,
    model::{self, Part},
    motion::{self, Clip},
    slice,
    terrain::AreaMap,
    text,
    texture::Vram,
    u32_le,
};
use std::collections::BTreeMap;

#[derive(Clone, Debug)]
pub struct Placement {
    pub part: usize,
    pub position: [i32; 3],
    pub facing: i32,
}

#[derive(Clone, Debug)]
pub struct Enemy {
    pub slot: usize,
    pub kind: i16,
    pub think: i16,
    pub position: [i32; 3],
    pub facing: i16,
    pub patrol: Vec<([i32; 3], i32)>,
}

pub struct MissionAssets {
    pub stage: Stage,
    pub map: Vec<Part>,
    pub objects: Vec<Part>,
    pub placements: Vec<Placement>,
    pub terrain: AreaMap,
    pub vram: Vram,
    pub characters: Vec<Character>,
    pub models: BTreeMap<i16, Vec<Part>>,
    pub clips: Vec<Clip>,
    pub common_motions: Vec<MotionRegistration>,
    pub weapons: crate::weapon::Weapons,
    pub enemies: Vec<Enemy>,
    pub story: Vec<StoryActor>,
    pub construction: Vec<u8>,
    pub layout: Vec<u8>,
}

fn point(data: &[u8], at: usize) -> Result<[i32; 3]> {
    let row = slice(data, at, 12)?;
    Ok([
        u32_le(row, 0)? as i32,
        u32_le(row, 4)? as i32,
        u32_le(row, 8)? as i32,
    ])
}

impl MissionAssets {
    pub fn load<S: ReadAt>(
        game: &GameAssets<S>,
        id: usize,
        character: i16,
        layout: usize,
    ) -> Result<Self> {
        if !(0..=1).contains(&character) || !(1..=3).contains(&layout) {
            return Err("Invalid character or layout".into());
        }
        let stage = game
            .stages()?
            .into_iter()
            .find(|s| s.id == id)
            .ok_or("Unknown stage")?;
        let asset = |name: &str| game.asset(&format!("STAGE/{}/{name}", stage.directory));
        let mut vram = Vram::default();
        vram.upload_tpd(&asset("TIM.TPD")?)?;
        vram.upload_tpd(&game.asset("IMAGE/COMMON.TPD")?)?;
        let player = if character == 0 { "RIKIMARU" } else { "AYAME" };
        vram.upload_tpd(&game.asset(&format!("HUMAN/{player}.TPD"))?)?;
        let map = model::archive(&asset("MAP.MAD")?)?;
        if map.iter().any(|p| p.parent >= 0) {
            return Err("Unexpected parented map geometry".into());
        }
        let objects = model::archive(&asset("OBJECTS.MAD")?)?;
        let construction = asset("STAGE.CON")?;
        if !construction.len().is_multiple_of(32) {
            return Err("Partial CON record".into());
        }
        let mut originals = BTreeMap::new();
        let mut object = 0;
        let mut area = "STAGE.ACM".to_string();
        let mut placements = Vec::new();
        for (i, row) in construction.chunks_exact(32).enumerate() {
            match i16_le(row, 0)? {
                0 => area = format!("{}.ACM", text(&row[4..16])?),
                2 => {
                    let name = text(&row[4..16])?;
                    let part = if name.is_empty() {
                        *originals
                            .get(&(i16_le(row, 2)? as usize))
                            .ok_or("Invalid CON clone")?
                    } else {
                        let index = object;
                        originals.insert(i, index);
                        object += 1;
                        index
                    };
                    if part >= objects.len() {
                        return Err("CON object outside MAD archive".into());
                    }
                    placements.push(Placement {
                        part,
                        position: point(row, 16)?,
                        facing: u32_le(row, 28)? as i32,
                    });
                }
                _ => {}
            }
        }
        let terrain = AreaMap::parse(asset(&area)?)?;
        let exe = Executable::new(&game.executable)?;
        let characters = exe.characters()?;
        let mut story = exe.story_actors(id)?;
        for actor in &mut story {
            actor.kind = match actor.kind {
                -1 => {
                    if character == 0 {
                        3
                    } else {
                        2
                    }
                }
                -2 => {
                    if character == 0 {
                        6
                    } else {
                        5
                    }
                }
                kind => kind,
            };
        }
        let layout = asset(&format!("IE_{layout}"))?;
        slice(&layout, 0, 7000)?;
        let mut enemies = Vec::new();
        for slot in 0..30 {
            let p = slice(&layout, slot * 136, 136)?;
            let kind = i16_le(p, 0)?;
            if kind == -1 {
                continue;
            }
            let count = i16_le(p, 4)?;
            if !(0..=7).contains(&count) {
                return Err("Invalid patrol length".into());
            }
            let mut patrol = Vec::new();
            for i in 0..count as usize {
                patrol.push((point(p, 24 + i * 16)?, u32_le(p, 36 + i * 16)? as i32));
            }
            enemies.push(Enemy {
                slot,
                kind,
                think: i16_le(p, 2)?,
                position: point(p, 8)?,
                facing: i16_le(p, 20)?,
                patrol,
            });
        }
        let mut models = BTreeMap::new();
        for kind in std::iter::once(character)
            .chain(enemies.iter().map(|e| e.kind))
            .chain(story.iter().map(|s| s.kind))
        {
            if models.contains_key(&kind) {
                continue;
            }
            let character = characters
                .iter()
                .find(|c| c.kind == kind)
                .ok_or_else(|| format!("Unknown actor kind {kind} in stage {}", stage.directory))?;
            models.insert(
                kind,
                model::archive(&game.asset(&format!("HUMAN/{}.MAD", character.model))?)?,
            );
        }
        let mut clips = Vec::new();
        for file in [
            "COMMON".into(),
            player.to_owned(),
            format!("STAGE{}", id + 1),
        ] {
            clips.extend(
                motion::parse(&game.asset(&format!("HUMAN/MOTION/{file}.AMD"))?)
                    .map_err(|e| format!("{file}.AMD: {e}"))?,
            );
        }
        Ok(Self {
            stage,
            map,
            objects,
            placements,
            terrain,
            vram,
            characters,
            models,
            clips,
            common_motions: exe.common_motions()?,
            weapons: crate::weapon::Weapons::load(game, &exe)?,
            enemies,
            story,
            construction,
            layout,
        })
    }

    pub fn triangle_count(&self) -> usize {
        let count = |p: &Part| {
            p.geometry
                .iter()
                .flat_map(|g| &g.faces)
                .map(|f| f.corners.len() - 2)
                .sum::<usize>()
        };
        self.map.iter().map(count).sum::<usize>()
            + self
                .placements
                .iter()
                .map(|p| count(&self.objects[p.part]))
                .sum::<usize>()
    }
}
