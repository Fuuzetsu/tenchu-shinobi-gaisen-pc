use super::*;
use std::sync::Arc;
use tenchu_assets::{
    audio::voice::VoiceSet,
    texture::Vram,
    weapon::{Attachment, Draw, Equipment, ImportedModel},
};

impl Game {
    pub fn player_equipment(&self, vram: &Arc<Vram>) -> Result<Equipment> {
        let actor = self.player();
        let kind = self.mission.characters[actor.definition].weapon;
        let draw = actor
            .imported_weapon_draw
            .unwrap_or_else(|| Draw::gaisen(kind));
        let mut slots = actor.weapons.clone();
        if actor.attribute & ATTR_WEAPON_DRAWN != 0 {
            draw.swap(&mut slots);
        }
        let attachments = slots
            .iter()
            .enumerate()
            .filter_map(|(slot, weapon)| weapon.as_ref().map(|weapon| (slot, weapon)))
            .map(|(slot, weapon)| {
                let model = &self.scene.models[weapon.model];
                let part = actor
                    .parts
                    .iter()
                    .position(|&part| Some(part) == model.parent)
                    .ok_or("Missing weapon attachment")?;
                let model_source = self
                    .mission
                    .weapons
                    .imported_models
                    .get(&weapon.kind)
                    .cloned()
                    .unwrap_or_else(|| {
                        Arc::new(ImportedModel {
                            id: format!("gaisen:weapons/{}", weapon.kind),
                            source_id: format!("weapons/{:04x}", weapon.kind),
                            geometry: self.mission.weapons.models[&weapon.kind].clone(),
                            vram: vram.clone(),
                        })
                    });
                Ok(Attachment {
                    part,
                    slot,
                    position: model.local.translation.to_array().map(|v| v as i16),
                    melee: weapon.imported_contact.unwrap_or_else(|| {
                        actor
                            .weapon_hitbox
                            .iter()
                            .flatten()
                            .any(|&i| self.mission.weapons.hitboxes[i].kind == weapon.kind)
                    }),
                    model: model_source,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        Ok(Equipment { draw, attachments })
    }

    pub fn set_player_equipment(
        &mut self,
        selection: &str,
        equipment: Option<&Equipment>,
        voice: Option<&Arc<VoiceSet>>,
    ) -> Result<bool> {
        if self.moveset.weapon_model == selection {
            return Ok(false);
        }
        if self.cinematic.active || self.player().scripted || self.player().health <= 0 {
            return Err("Change equipment with a living player outside a cutscene".into());
        }
        let actor = self.roster.get_mut(self.player_id);
        let mut ctx = Context {
            mission: &mut self.mission,
            math: &self.math,
            body: &self.body,
            cursor: &mut self.cursor,
            random: &mut self.random,
            effects: &mut self.effects,
            scene: &mut self.scene,
            collision: &mut self.collision,
            camera: &mut self.camera,
            drawing: &mut self.drawing,
        };
        actor.clear_attack(&mut ctx, 3)?;
        actor.rebuild_weapons(&mut ctx, &mut self.moveset.weapons)?;
        if let Some(equipment) = equipment {
            self.import_player_weapons(equipment)?;
        }
        self.moveset.weapon_model = selection.into();
        let voices = Arc::make_mut(&mut self.mission.moveset_voices);
        voices.remove(&SELECTED_WEAPON_VOICE);
        if !selection.is_empty()
            && let Some(voice) = voice
        {
            voices.insert(SELECTED_WEAPON_VOICE, voice.clone());
        }
        Ok(true)
    }
}
