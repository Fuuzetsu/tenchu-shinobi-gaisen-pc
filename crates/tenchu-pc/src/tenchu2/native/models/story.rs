use super::{Custom, Library, Pack};
use crate::settings::Settings;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};
use tenchu_assets::{Result, model::Part, tenchu2::story::Resources, texture::Vram};

#[cfg(test)]
mod tests;

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Override {
    selected: String,
    hero: usize,
    pub actors: BTreeSet<i16>,
    pub skin: Option<Skin>,
    pub custom: Option<Custom>,
    weapon_selected: String,
    pub equipment: BTreeMap<usize, Vec<Attachment>>,
}

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Attachment {
    pub bone: usize,
    pub position: [i16; 3],
    pub model: Arc<tenchu_assets::weapon::ImportedModel>,
}

#[derive(Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Skin {
    pub parts: Vec<Part>,
    pub vram: Arc<Vram>,
    pub sources: Vec<String>,
}

impl Override {
    pub(in crate::tenchu2::native) fn same_body(&self, other: &Self) -> Result<bool> {
        Ok(self.selected == other.selected
            && self.hero == other.hero
            && self.actors == other.actors
            && self.skin == other.skin
            && self.weapon_selected == other.weapon_selected
            && self.equipment == other.equipment
            && Custom::same_resources(self.custom.as_ref(), other.custom.as_ref())?)
    }

    pub(in crate::tenchu2::native) fn reuse_skin(&mut self, other: &mut Self) {
        std::mem::swap(&mut self.skin, &mut other.skin);
        std::mem::swap(&mut self.equipment, &mut other.equipment);
    }

    pub(in crate::tenchu2::native) fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        let Self {
            selected,
            hero,
            actors,
            skin,
            custom,
            weapon_selected,
            equipment,
        } = self;
        out.value(selected)?;
        out.value(hero)?;
        out.value(actors)?;
        out.value(&skin.is_some())?;
        if let Some(Skin {
            parts,
            vram,
            sources,
        }) = skin
        {
            out.shared(parts)?;
            out.shared_arc(vram, true)?;
            out.value(sources)?;
        }
        out.value(&custom.is_some())?;
        if let Some(custom) = custom {
            custom.capture(out)?;
        }
        out.value(weapon_selected)?;
        out.value(&equipment.len())?;
        for (slot, parts) in equipment {
            out.value(slot)?;
            out.value(&parts.len())?;
            for part in parts {
                out.value(&part.bone)?;
                out.value(&part.position)?;
                out.shared_arc(&part.model, true)?;
            }
        }
        Ok(())
    }

    pub fn current(&self, hero: usize, settings: &Settings) -> bool {
        self.selected == settings.player_model
            && self.weapon_selected == equipment_id(settings)
            && (self.selected.is_empty() && self.weapon_selected.is_empty() || self.hero == hero)
    }

    pub(crate) fn validate_state(&self, resources: &Resources) -> Result<()> {
        if self.hero >= 3
            || self
                .actors
                .iter()
                .any(|id| !resources.models.contains_key(id))
            || self.skin.as_ref().is_some_and(|skin| {
                skin.parts.len() != tenchu_assets::tenchu2::BONES.len()
                    || skin.vram.words.len() != 1024 * 512
            })
            || self.skin.is_some() && self.custom.is_some()
            || self.equipment.iter().any(|(&slot, parts)| {
                resources
                    .script
                    .actors
                    .get(slot)
                    .and_then(|track| resources.models.get(&track.model))
                    .is_none_or(|actor| {
                        !is_hero(&actor.name, self.hero)
                            || parts.iter().any(|p| {
                                p.bone >= actor.parts.len()
                                    || p.model.vram.words.len() != 1024 * 512
                            })
                    })
            })
        {
            return Err("Invalid saved cutscene player override".into());
        }
        if let Some(custom) = &self.custom {
            custom.validate_state()?;
        }
        Ok(())
    }

    pub fn sync(
        &mut self,
        resources: &Resources,
        hero: usize,
        library: &Library,
        bodies: &Arc<Pack>,
        settings: &Settings,
    ) -> Result<bool> {
        let selected = settings.player_model.as_str();
        let weapon_selected = equipment_id(settings);
        let custom = Custom::selected(selected, bodies)?;
        if self.selected == selected
            && self.weapon_selected == weapon_selected
            && self.hero == hero
            && Custom::same(self.custom.as_ref(), custom.as_ref())
        {
            return Ok(false);
        }
        let body = if selected.is_empty() || custom.is_some() {
            None
        } else {
            let body = library
                .get(selected)
                .ok_or("Selected model is unavailable")?;
            if body.parts.len() != tenchu_assets::tenchu2::BONES.len() {
                return Err("This model has no compatible humanoid rig".into());
            }
            Some(body)
        };
        let weapon = if settings.weapon_model.is_empty() {
            library.get(weapon_selected)
        } else {
            library.weapon(weapon_selected)?
        };
        let mut equipment = BTreeMap::new();
        if let Some(weapon) = weapon
            && let Some(gear) = &weapon.equipment
        {
            for (slot, track) in resources.script.actors.iter().enumerate() {
                let Some(actor) = resources.models.get(&track.model) else {
                    continue;
                };
                if !is_hero(&actor.name, hero)
                    || weapon.id.starts_with("tenchu2-jp:")
                        && weapon.resource == format!("tenchu2/actors/{}", actor.name)
                {
                    continue;
                }
                let drawn = resources
                    .attachments
                    .iter()
                    .any(|a| a.actor == slot && a.slot >= 3);
                let mut slots = [0, 1, 2, 3, 4];
                if drawn {
                    gear.draw.swap(&mut slots);
                }
                equipment.insert(
                    slot,
                    gear.attachments
                        .iter()
                        .filter(|a| [slots[0], slots[1], slots[4]].contains(&a.slot))
                        .map(|a| Attachment {
                            bone: tenchu_assets::tenchu2::BONES[a.part],
                            position: a.position,
                            model: a.model.clone(),
                        })
                        .collect(),
                );
            }
        }
        self.actors = resources
            .models
            .iter()
            .filter(|(_, model)| {
                (custom.is_some() || body.is_some())
                    && is_hero(&model.name, hero)
                    && !body.is_some_and(|body| {
                        body.id.starts_with("tenchu2-jp:")
                            && body.resource == format!("tenchu2/actors/{}", model.name)
                    })
            })
            .map(|(&id, _)| id)
            .collect();
        self.skin = body.map(|body| Skin {
            parts: body.parts.clone(),
            vram: body.vram.clone(),
            sources: body.sources().into_iter().map(|s| s.id).collect(),
        });
        self.custom = custom;
        self.selected = selected.into();
        self.hero = hero;
        self.weapon_selected = weapon_selected.into();
        self.equipment = equipment;
        Ok(true)
    }
}

fn equipment_id(settings: &Settings) -> &str {
    if !settings.weapon_model.is_empty() {
        &settings.weapon_model
    } else {
        settings.moveset_id().unwrap_or(&settings.player_model)
    }
}

fn is_hero(name: &str, hero: usize) -> bool {
    match hero {
        0 => matches!(
            name,
            "rikimaru" | "rikia" | "riki2" | "rikia2" | "riki3" | "rikia3"
        ),
        1 => matches!(name, "ayame" | "ayamea"),
        2 => matches!(name, "tatumaru" | "tatu2" | "tatu2_m" | "tatua" | "tatua_m"),
        _ => false,
    }
}
