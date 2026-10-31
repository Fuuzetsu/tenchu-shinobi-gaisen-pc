use super::{Mission, Tables, Theme};
use crate::{
    ReadAt, Result,
    executable::Executable,
    game::{GameAssets, Stage},
    hud::MapPlacement,
    model, motion,
    script::{Cutscenes, Language, Scripts},
    stage::MissionAssets,
    texture::Image,
};
use std::collections::BTreeMap;

pub struct Setup {
    pub mission: Mission,
    pub appearance: Vec<i16>,
    pub traps: Vec<u8>,
    pub ornaments: Vec<(super::Point, usize)>,
    pub collapsing: Vec<usize>,
    pub digits: Image,
    pub candles: Vec<Image>,
}

impl MissionAssets {
    pub fn load_trial<S: ReadAt>(
        assets: &GameAssets<S>,
        tables: &Tables,
        theme: &Theme,
        mission: &Mission,
        character: i16,
        random: impl FnMut() -> u16,
    ) -> Result<Self> {
        if !(0..=1).contains(&character) {
            return Err("Invalid trial player".into());
        }
        theme.validate(mission)?;
        let generated = theme.build(mission, tables, random)?;
        let exe = Executable::new(&tables.executable)?;
        let mut vram = theme.vram.clone();
        vram.upload_tpd(&assets.asset("TRIAL/IMAGE/COMMON.TPD")?)?;
        vram.upload_tpd(&assets.asset("TRIAL/IMAGE/TRIALCMN.TPD")?)?;
        vram.upload_tpd(&assets.asset(&format!("TRIAL/IMAGE/TRIAL{}.TPD", mission.theme + 1))?)?;
        let archive = assets.asset("IMAGE/IMAGES.ARC")?;
        let images: Vec<_> = (0..62)
            .map(|i| vram.upload_tim(model::arc_entry(&archive, i)?))
            .collect::<Result<_>>()?;
        let digits = vram.upload_tim(&assets.asset("TRIAL/IMAGE/TIME.TIM")?)?;
        let archive = assets.asset("TRIAL/IMAGE/ROUSOKU.ARC")?;
        let candles = (0..3)
            .map(|i| vram.upload_tim(model::arc_entry(&archive, i)?))
            .collect::<Result<_>>()?;
        let mut models = BTreeMap::new();
        for kind in mission
            .placements
            .iter()
            .map(|p| if p.kind == 0 { character } else { p.kind })
            .chain([0xa9])
            .chain(tables.appearance[mission.theme].iter().copied())
        {
            if models.contains_key(&kind) {
                continue;
            }
            let definition = tables
                .characters
                .iter()
                .find(|c| c.kind == kind)
                .ok_or("Unknown trial actor")?;
            models.insert(
                kind,
                model::archive(&assets.asset(&format!("TRIAL/HUMAN/{}.MAD", definition.model))?)?,
            );
        }
        let mut clips = Vec::new();
        for file in [
            "COMMON".to_owned(),
            if character == 0 { "RIKIMARU" } else { "AYAME" }.into(),
            format!("TRIAL{}", mission.theme + 1),
        ] {
            clips.extend(motion::parse(
                &assets.asset(&format!("TRIAL/HUMAN/MOTION/{file}.AMD"))?,
            )?);
        }
        let archive = assets.asset("IMAGE/MODELS.ARC")?;
        let common_models = (0..33)
            .filter(|i| !(22..=24).contains(i))
            .map(|i| Ok((i, model::tmd_model(model::arc_entry(&archive, i)?)?)))
            .collect::<Result<_>>()?;
        let font = crate::font::Font::load(assets, &mut vram)?;
        let traps = mission
            .map
            .iter()
            .enumerate()
            .map(|(i, cell)| {
                theme.tile(cell.ground).map_or(0, |tile| {
                    if tile.flags & 0x1000 != 0 {
                        0xff
                    } else if tile.flags & 0x2000 != 0 {
                        ((-theme.height(mission, (i % 32) as i16, (i / 32) as i16, false) as u8)
                            << 4)
                            | 0x80
                            | (cell.ground >> 6)
                    } else {
                        0
                    }
                })
            })
            .collect();
        let id = [8, 0, 1, 2, 9, 10, 3, 4, 5, 6, 7, 7][mission.theme];
        Ok(Self {
            trial: Some(Setup {
                mission: mission.clone(),
                appearance: tables.appearance[mission.theme].clone(),
                traps,
                collapsing: generated
                    .ornaments
                    .iter()
                    .filter_map(|(p, i)| {
                        theme
                            .tile(mission.cell(p.x, p.z).unwrap().ornament)
                            .filter(|t| t.flags & 0x2000 != 0)
                            .map(|_| *i)
                    })
                    .collect(),
                ornaments: generated.ornaments,
                digits,
                candles,
            }),
            stage: Stage {
                id,
                order: -1,
                name: mission.title.clone(),
                directory: format!("trial{}", mission.theme + 1),
                start: mission.placements[0].point.world(),
                facing: 0,
                reward: 0,
            },
            misc: crate::misc::Misc::load(assets, &[], &mut vram)?,
            map: vec![],
            map_image: images[0],
            map_placement: MapPlacement {
                divisor: 200,
                rotation: 0,
                screen: [0, 0],
            },
            objects: generated.objects,
            object_names: generated.names,
            placements: generated.placements,
            terrain: generated.collision,
            balma_terrain: None,
            vram,
            images,
            characters: tables.characters.clone(),
            models,
            imported_models: BTreeMap::new(),
            imported_voices: vec![],
            clips,
            common_motions: tables.common_motions.clone(),
            weapons: crate::weapon::Weapons::load_trial(assets, &exe)?,
            death_events: exe.death_events_at(0x80087c60)?,
            common_models,
            enemies: vec![],
            construction_actors: vec![],
            construction_pickups: vec![],
            pickups: vec![],
            disguise: tables.appearance[mission.theme][3],
            reinforcements: [tables.appearance[mission.theme][3]; 2],
            reinforcement_overrides: [None; 2],
            story: vec![],
            construction: vec![],
            layout: vec![],
            layout_index: 0,
            random_layouts: None,
            scripts: Scripts {
                language: Language::Japanese,
                events: vec![],
                cutscenes: Cutscenes::parse(vec![255, 255])?,
            },
            font,
            tanka: vec![],
        })
    }
}
