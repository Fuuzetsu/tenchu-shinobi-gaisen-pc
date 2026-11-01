use super::*;

impl Application {
    pub(super) fn record_rewind(&mut self) {
        if self.settings.rewind_seconds == 0
            || self.demo.is_some()
            || self.benchmarks.run.is_some()
            || self.trial.open
            || self.randomizer.open
            || !self.campaign.scene_visible()
            || self.faulted
            || !self.rewind.due(self.campaign.game.tick)
        {
            return;
        }
        let result = self.capture_rewind().and_then(|state| {
            self.rewind
                .push(self.campaign.game.tick, self.settings.rewind_seconds, state)
        });
        if let Err(error) = result {
            self.rewind.clear();
            self.settings.rewind_seconds = 0;
            self.state_notice(format!("Rewind disabled: {error}"));
        }
    }

    pub(super) fn rewind_step(&mut self) -> Result<()> {
        if self.settings.rewind_seconds == 0 {
            return Err("Enable rewind history under F1 → Simulation first".into());
        }
        let before = self.campaign.game.tick;
        let index = self
            .rewind
            .previous(before)
            .ok_or("No earlier rewind state available")?;
        let mut history = std::mem::take(&mut self.rewind);
        let result = self.restore_rewind(&history.entries[index].state);
        if result.is_ok() {
            history.branch(index);
        }
        self.rewind = history;
        result?;
        self.state_notice(format!(
            "Rewound {:.1}s · Ctrl+F12: back · F3: resume",
            (before - self.campaign.game.tick) as f64 / 30.0
        ));
        self.update_view();
        Ok(())
    }
}
