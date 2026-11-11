use super::{Custom, Library, Pack};
use std::{collections::BTreeSet, sync::Arc};
use tenchu_assets::{Result, model::Part, tenchu2::story::Resources, texture::Vram};

#[derive(Default)]
pub struct Override {
    selected: String,
    hero: usize,
    pub actors: BTreeSet<i16>,
    pub skin: Option<Skin>,
    pub custom: Option<Custom>,
}

pub struct Skin {
    pub parts: Vec<Part>,
    pub vram: Arc<Vram>,
    pub sources: Vec<String>,
}

impl Override {
    pub fn sync(
        &mut self,
        resources: &Resources,
        hero: usize,
        library: &Library,
        bodies: &Arc<Pack>,
        selected: &str,
    ) -> Result<bool> {
        let custom = Custom::selected(selected, bodies)?;
        if self.selected == selected
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
        Ok(true)
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
