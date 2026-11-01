use super::*;

impl Application {
    pub(super) fn edit_actor(&mut self, item: Item) -> Result<()> {
        if !self.campaign.playing() || self.demo.is_some() || self.benchmarks.run.is_some() {
            return Err("Start a mission before editing actors".into());
        }
        let id = self.debug.selected_actor;
        let game = &self.campaign.game;
        match item {
            Item::ActorPosition => {
                let actor = game
                    .roster
                    .actors
                    .get(id)
                    .and_then(Option::as_ref)
                    .ok_or("Actor no longer exists")?;
                self.debug.target = actor.position;
                self.menu.spawn_yaw =
                    (i32::from(actor.rotation[1]).rem_euclid(4096) * 360 / 4096) as i16;
                return Ok(());
            }
            Item::PickActorPosition => {
                self.debug.free_camera(game);
                self.debug.teleport_armed = false;
                self.debug.pick_actor_position = true;
                self.menu.open = false;
                return Ok(());
            }
            _ => (),
        }
        self.stop_edited_recording();
        let yaw = (i32::from(self.menu.spawn_yaw) * 4096 / 360) as i16;
        let game = &mut self.campaign.game;
        match item {
            Item::SpawnActor => {
                let view = self.view.as_mut().ok_or("Renderer is not ready")?;
                let library = view
                    .model_library
                    .as_ref()
                    .ok_or("Wait for the model library to finish loading")?;
                let source = crate::visuals::appearance::library::import_actor(
                    &mut game.mission,
                    library,
                    &self.menu.spawn_model,
                    &mut self.debug.actor_imports,
                )?;
                let think = tenchu_core::ai::programs::Programs::decode(source.think)?;
                let id = game.place_actor(
                    source.actor,
                    think,
                    self.debug.target,
                    yaw,
                    self.debug.snap_to_floor,
                )?;
                game.roster.get_mut(id).voice_program = source.voice;
                self.debug.selected_actor = id;
                self.debug.inspect_actor = true;
                self.debug.actors_edited = true;
                self.trial.eligible = false;
                view.refresh_actor_assets(game, &self.settings)?;
                if let Some(audio) = &mut self.audio {
                    audio.refresh_actor_bank(&self.assets, game)?;
                }
                self.menu.notice = format!("Placed actor #{id}. Edits last until retry.");
            }
            Item::MoveActor => {
                game.move_actor(id, self.debug.target, yaw, self.debug.snap_to_floor)?;
                self.menu.notice =
                    format!("Moved actor #{id}; its previous patrol route was cleared.");
            }
            Item::RemoveActor => {
                game.remove_actor(id)?;
                self.debug.selected_actor = game.player_id;
                self.menu.notice =
                    format!("Removed actor #{id}. Retry restores the original layout.");
            }
            _ => unreachable!(),
        }
        self.debug.actors_edited = true;
        self.trial.eligible = false;
        if let Some(view) = &mut self.view {
            view.reset_interpolation(game);
        }
        self.debug.update(game);
        Ok(())
    }
}
