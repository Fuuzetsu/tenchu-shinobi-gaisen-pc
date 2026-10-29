use super::*;
use std::sync::Arc;

pub struct Model {
    pub source_kind: i16,
    pub vram: Arc<Vram>,
    pub clip_ids: BTreeMap<i16, i16>,
}

#[derive(Clone, Copy)]
pub struct Voice {
    pub stage: usize,
    pub character: i16,
    pub program: usize,
}

#[derive(Clone, Copy)]
pub struct Actor {
    pub definition: usize,
    pub model: i16,
}

#[derive(Clone, Copy)]
pub struct Reinforcement {
    pub actor: Option<Actor>,
    pub voice: Option<i16>,
    pub health_percent: u16,
}

impl MissionAssets {
    pub fn import_actor(
        &mut self,
        mut character: Character,
        parts: Vec<Part>,
        clips: &[Clip],
        vram: Arc<Vram>,
    ) -> Result<Actor> {
        let model = (256..i16::MAX)
            .find(|kind| !self.models.contains_key(kind))
            .ok_or("Too many imported actor models")?;
        let mut remap = BTreeMap::new();
        let mut next = i16::MIN;
        for clip in clips {
            while self.clips.iter().any(|c| c.id == next) {
                next = next.checked_add(1).ok_or("Too many imported motions")?;
            }
            let mut imported = clip.clone();
            remap.insert(imported.id, next);
            imported.id = next;
            self.clips.push(imported);
        }
        character.motions.retain_mut(|motion| {
            if let Some(&clip) = remap.get(&motion.clip) {
                motion.clip = clip;
                true
            } else {
                false
            }
        });
        self.imported_models.insert(
            model,
            Model {
                source_kind: character.kind,
                vram,
                clip_ids: remap
                    .into_iter()
                    .map(|(source, imported)| (imported, source))
                    .collect(),
            },
        );
        self.models.insert(model, parts);
        let definition = self.characters.len();
        self.characters.push(character);
        Ok(Actor { definition, model })
    }
}
