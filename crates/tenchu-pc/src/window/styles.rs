use super::*;

impl Application {
    pub(super) fn switch_combat_style(&mut self) -> Result<String> {
        if self.recording.is_some() {
            return Err("Stop input recording before switching combat styles.".into());
        }
        if self.reverse.active
            || self.reverse.preview
            || self.demo.is_some()
            || self.benchmarks.run.is_some()
            || self.campaign.randomized.is_some()
        {
            return Err("Combat style switching is unavailable in rewind preview, demos, benchmarks and randomized runs.".into());
        }
        if !self.campaign.playing() || !settings::styles::gaisen_ready(&self.campaign.game) {
            return Err("Combat style unchanged: stand or move on the ground after the current action finishes.".into());
        }
        let view = self
            .view
            .as_mut()
            .ok_or("Wait for the game window before switching combat styles.")?;
        if view.model_library.is_none() {
            view.load_model_library(&self.assets, &self.campaign.game, &self.settings)?;
        }
        let notice = settings::styles::switch_gaisen(
            &mut self.campaign.game,
            &mut self.settings,
            view.model_library.as_ref().unwrap(),
        )?;
        view.refresh_actor_assets(&self.campaign.game, &self.settings)?;
        self.trial_record_check();
        if let Err(error) = self.settings_store.write(&self.settings) {
            return Ok(format!("{notice} · could not save preference: {error}"));
        }
        Ok(notice)
    }
}
