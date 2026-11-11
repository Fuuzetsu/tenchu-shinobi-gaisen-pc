use super::*;
use crate::settings::Settings;
use tenchu_core::game::Game;

impl Library {
    pub(super) fn load_equipment(&mut self, assets: &GameAssets<impl ReadAt>) -> Result<()> {
        let mut game = Game::new(
            MissionAssets::load(assets, 0, 0, 1)?,
            &Executable::new(&assets.executable)?,
            0,
            1,
        )?;
        game.events = Default::default();
        for _ in 0..30 {
            game.step(0)?;
        }
        let vram = Arc::new(game.mission.vram.clone());
        let mut known = BTreeMap::new();
        for body in &mut self.bodies {
            let Some(character) = &body.character else {
                continue;
            };
            if body.parts.len() != 15 {
                continue;
            }
            let equipment = if let Some(equipment) = known.get(&(character.weapon, character.width))
            {
                Arc::clone(equipment)
            } else {
                game.set_player_moveset(Some(character), &body.clips, Default::default())?;
                let equipment = Arc::new(game.player_equipment(&vram)?);
                known.insert((character.weapon, character.width), equipment.clone());
                equipment
            };
            body.equipment = Some(equipment);
        }
        Ok(())
    }

    pub fn weapon(&self, id: &str) -> Result<Option<&Body>> {
        if id.is_empty() {
            return Ok(None);
        }
        self.get(id)
            .filter(|body| body.equipment.is_some())
            .map(Some)
            .ok_or_else(|| format!("Weapon equipment unavailable: {id}"))
    }

    pub fn voice(&self, id: &str) -> Result<Option<&Arc<tenchu_assets::audio::voice::VoiceSet>>> {
        if id.is_empty() {
            return Ok(None);
        }
        self.get(id)
            .and_then(|body| body.moveset_voice())
            .map(Some)
            .ok_or_else(|| format!("Voice unavailable: {id}"))
    }

    pub fn equipment_choices(&self) -> Vec<(String, String)> {
        std::iter::once((String::new(), "Follow moveset".into()))
            .chain(
                self.bodies
                    .iter()
                    .filter(|body| body.equipment.is_some())
                    .map(|body| {
                        let name = if matches!(
                            body.resource.as_str(),
                            "tenchu2/actors/tatu2" | "tenchu2/actors/tatu2_m"
                        ) {
                            format!("{} · Izayoi sword", body.name)
                        } else if body.resource == "tenchu2/actors/yorojyu" {
                            format!("{} · Dual katana", body.name)
                        } else if body.resource == "tenchu2/actors/kagami" {
                            format!("{} · Sword", body.name)
                        } else {
                            body.name.clone()
                        };
                        (body.id.clone(), name)
                    }),
            )
            .collect()
    }

    pub fn voice_choices(&self) -> Vec<(String, String)> {
        std::iter::once((String::new(), "Follow moveset".into()))
            .chain(
                self.bodies
                    .iter()
                    .filter(|body| body.moveset_voice().is_some())
                    .map(|body| (body.id.clone(), body.name.clone())),
            )
            .collect()
    }

    pub fn apply_player(&self, game: &mut Game, settings: &Settings) -> Result<bool> {
        let moveset = self.moveset(settings)?;
        let weapon = self.weapon(&settings.weapon_model)?;
        let voice = self.voice(&settings.voice_model)?;
        let changed_moves = !game.moveset.matches(
            moveset.and_then(|b| b.moveset_character()).map(|c| c.kind),
            settings.model_moveset_options,
        );
        if changed_moves {
            if let Some(body) = moveset {
                body.apply_moveset(game, settings.model_moveset_options)?;
            } else {
                game.set_player_moveset(None, &[], settings.model_moveset_options)?;
            }
        }
        let equipment = weapon
            .and_then(|b| b.equipment.as_deref())
            .or_else(|| moveset.and_then(|b| b.imported_moveset.as_ref().map(|m| &m.equipment)));
        let changed_weapon = game.set_player_equipment(
            &settings.weapon_model,
            equipment,
            weapon.and_then(|b| b.moveset_voice()),
        )?;
        let changed_voice = game.moveset.voice_model != settings.voice_model;
        if changed_voice {
            game.moveset.voice_model.clone_from(&settings.voice_model);
            let voices = Arc::make_mut(&mut game.mission.moveset_voices);
            voices.remove(&tenchu_core::game::moveset::SELECTED_VOICE);
            if let Some(voice) = voice {
                voices.insert(tenchu_core::game::moveset::SELECTED_VOICE, voice.clone());
            }
        }
        Ok(changed_moves || changed_weapon || changed_voice)
    }
}

#[cfg(test)]
mod tests;
