use crate::{Result, math::Math, tenchu2::actor::Resources};
use std::collections::BTreeMap;
use tenchu_assets::{
    ReadAt,
    disc::Disc,
    model::Part,
    tenchu2::{
        characters, collision, combat,
        commands::Commands,
        level::Level,
        loot::Loot,
        math_tables, motions,
        registry::Registry,
        weapons::{self, Weapons},
    },
};

pub struct Assets {
    pub resources: Resources,
    pub level: Level,
    pub models: BTreeMap<String, characters::Rig>,
    pub weapons: BTreeMap<String, Vec<Part>>,
    pub loot: Loot,
    pub terrain_rules: collision::Rules,
    pub combat_rules: combat::Rules,
    pub spline: [[i16; 4]; 33],
}

impl Assets {
    pub fn load(disc: &Disc<impl ReadAt>, hero: usize, map: usize) -> Result<Self> {
        let combat_rules = combat::Rules::load(disc)?;
        let mut registry = Registry::load(disc)?;
        if map == 12 {
            registry
                .actors
                .iter_mut()
                .find(|actor| actor.kind == 0x65)
                .ok_or("Missing Tenchu 2 story actor")?
                .model = "mati_m".into();
        }
        let weapons = Weapons::load(disc)?;
        let mut models = BTreeMap::new();
        let mut weapon_models = BTreeMap::new();
        for &kind in std::iter::once(&(hero as i16))
            .chain(registry.cast(hero, map).ok_or("Invalid Tenchu 2 cast")?)
        {
            let actor = registry
                .actor(kind)
                .ok_or("Missing Tenchu 2 actor definition")?;
            if !models.contains_key(&actor.model) {
                models.insert(actor.model.clone(), characters::rig(disc, &actor.model)?);
            }
            for name in weapons
                .parts(actor.weapon)
                .filter_map(|part| part.model.as_ref())
            {
                if !weapon_models.contains_key(name) {
                    weapon_models.insert(name.clone(), weapons::model(disc, name)?);
                }
            }
        }
        Ok(Self {
            resources: Resources {
                registry,
                clips: motions::for_map(disc, hero, map)?,
                weapons,
                commands: Commands::load(disc)?,
                math: Math {
                    tables: math_tables(disc)?,
                },
            },
            level: Level::load(disc, map)?,
            models,
            weapons: weapon_models,
            loot: Loot::load(disc)?,
            terrain_rules: collision::Rules::load(disc)?,
            combat_rules,
            spline: motions::spline_basis(disc)?,
        })
    }
}
