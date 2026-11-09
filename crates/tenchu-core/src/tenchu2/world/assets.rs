use crate::{Result, math::Math, tenchu2::actor::Resources};
use std::collections::BTreeMap;
use tenchu_assets::{
    ReadAt,
    disc::Disc,
    model::Part,
    tenchu2::{
        characters, collision, combat,
        commands::Commands,
        layout::Layout,
        level::Level,
        loot::Loot,
        math_tables, motions,
        registry::Registry,
        weapons::{self, Weapons},
    },
};

pub struct Assets {
    pub campaign: tenchu_assets::tenchu2::campaign::Campaign,
    pub ambient_interval: i16,
    pub items: tenchu_assets::tenchu2::items::Items,
    pub sprites: tenchu_assets::tenchu2::effects::Sprites,
    pub effect_rules: tenchu_assets::tenchu2::effects::Rules,
    pub camera: tenchu_assets::tenchu2::camera::Tables,
    pub hero: usize,
    pub sequence: usize,
    pub draw_distance: i32,
    pub resources: Resources,
    pub level: Level,
    pub models: BTreeMap<String, characters::Rig>,
    pub weapons: BTreeMap<String, Vec<Part>>,
    pub weapon_vram: tenchu_assets::texture::Vram,
    pub weapon_imports: BTreeMap<String, std::sync::Arc<tenchu_assets::weapon::ImportedModel>>,
    pub model_sources: BTreeMap<String, Vec<String>>,
    pub loot: Loot,
    pub terrain_rules: collision::Rules,
    pub combat_rules: combat::Rules,
    pub spline: [[i16; 4]; 33],
}

impl Assets {
    pub fn load(disc: &Disc<impl ReadAt>, hero: usize, layout: &Layout) -> Result<Self> {
        Self::equipped(disc, hero, layout, false)
    }

    pub fn equipped(
        disc: &Disc<impl ReadAt>,
        hero: usize,
        layout: &Layout,
        alternate: bool,
    ) -> Result<Self> {
        if hero >= 3 {
            return Err("Invalid Tenchu 2 hero".into());
        }
        let map = layout.map;
        let combat_rules = combat::Rules::load(disc)?;
        let mut registry = Registry::load(disc)?;
        if hero == 0 && layout.sequence > 93 {
            registry
                .actors
                .iter_mut()
                .find(|actor| actor.kind == 0)
                .ok_or("Missing Tenchu 2 Rikimaru definition")?
                .weapon = 0x1000;
        }
        if hero == 2 && map >= 12 {
            registry
                .actors
                .iter_mut()
                .find(|actor| actor.kind == 2)
                .ok_or("Missing Tenchu 2 Tatsumaru definition")?
                .model = if map < 14 { "tatu2_m" } else { "tatu2" }.into();
        }
        if map == 12 {
            registry
                .actors
                .iter_mut()
                .find(|actor| actor.kind == 0x65)
                .ok_or("Missing Tenchu 2 story actor")?
                .model = "mati_m".into();
        }
        if alternate {
            let player = registry
                .actors
                .iter_mut()
                .find(|actor| actor.kind == hero as i16)
                .ok_or("Missing Tenchu 2 hero definition")?;
            player.skill = 5;
            player.model = match hero {
                0 => "rikia",
                1 => "ayamea",
                _ if (12..14).contains(&map) => "tatua_m",
                _ => "tatua",
            }
            .into();
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
        let level = Level::japanese_sequence(disc, layout, false)?;
        let items = tenchu_assets::tenchu2::items::Items::load(disc)?;
        Ok(Self {
            campaign: tenchu_assets::tenchu2::campaign::Campaign::load(disc)?,
            ambient_interval: tenchu_assets::tenchu2::audio::interval(disc, map)?,
            items,
            sprites: tenchu_assets::tenchu2::effects::Sprites::load(disc)?,
            effect_rules: tenchu_assets::tenchu2::effects::Rules::load(disc)?,
            camera: tenchu_assets::tenchu2::camera::Tables::load(disc)?,
            hero,
            sequence: layout.sequence,
            draw_distance: level.draw_distance,
            resources: Resources {
                registry,
                clips: motions::for_map(disc, hero, map)?,
                weapons,
                commands: Commands::load(disc)?,
                math: Math {
                    tables: math_tables(disc)?,
                }
                .into(),
            },
            level,
            models,
            weapons: weapon_models,
            weapon_vram: weapons::textures(disc)?,
            weapon_imports: BTreeMap::new(),
            model_sources: BTreeMap::new(),
            loot: Loot::load(disc)?,
            terrain_rules: collision::Rules::load(disc)?,
            combat_rules,
            spline: motions::spline_basis(disc)?,
        })
    }
}
