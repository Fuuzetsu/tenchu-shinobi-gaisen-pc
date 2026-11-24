use super::{AnimatedMaterial, Meshes, Renderer, Story, overlaps};
use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{tenchu2::story::Patch, texture::Vram};

#[cfg(test)]
mod tests;

pub(super) struct Faces {
    owner: i16,
    patches: Arc<Vec<Patch>>,
    base: Arc<Vram>,
    applied: Option<[Option<u8>; 2]>,
    materials: Vec<AnimatedMaterial>,
}

impl Faces {
    pub(super) fn new(
        id: i16,
        skin: &crate::tenchu2::native::models::story::Skin,
        story: &Story,
        meshes: &mut Meshes,
        renderer: &mut Renderer,
    ) -> Option<Self> {
        let patches = skin.faces.as_ref()?;
        let owner = story
            .resources
            .script
            .actors
            .iter()
            .find(|a| a.model == id && id > 290)
            .map_or(id, |a| a.texture_model);
        if !story.resources.faces.contains_key(&owner) {
            return None;
        }
        let regions: Vec<_> = meshes.materials().iter().map(|m| m.region).collect();
        if !regions
            .iter()
            .flatten()
            .any(|region| patches.iter().any(|patch| overlaps(*region, patch.rect())))
        {
            return None;
        }
        meshes.detach_textures();
        let mut regions = regions.into_iter().enumerate();
        let mut materials = Vec::new();
        meshes.bind(|material| {
            let (slot, region) = regions.next().unwrap();
            let Some(region) =
                region.filter(|region| patches.iter().any(|patch| overlaps(*region, patch.rect())))
            else {
                return renderer.bind_material(material);
            };
            let original = region.pixels(&skin.vram);
            let mut material = material.clone();
            material.id = format!("tenchu2/event/{}/override/{id}/{slot}", story.event);
            let handle = renderer.bind_material(&material);
            materials.push(AnimatedMaterial {
                region,
                base: material.image.clone(),
                material,
                previous: original.clone(),
                original,
            });
            handle
        });
        Some(Self {
            owner,
            patches: patches.clone(),
            base: skin.vram.clone(),
            applied: None,
            materials,
        })
    }

    pub(super) fn invalidate(&mut self) {
        self.applied = None;
    }

    pub(super) fn update(
        &mut self,
        states: &BTreeMap<i16, [Option<u8>; 2]>,
        renderer: &mut Renderer,
    ) {
        let indices = states.get(&self.owner).copied().unwrap_or_default();
        if self.applied == Some(indices) {
            return;
        }
        let mut vram = (*self.base).clone();
        for index in indices.into_iter().flatten() {
            if let Some(patch) = self.patches.get(usize::from(index)) {
                patch.apply(&mut vram);
            }
        }
        for material in &mut self.materials {
            material.update(material.region.pixels(&vram), renderer);
        }
        self.applied = Some(indices);
    }
}
