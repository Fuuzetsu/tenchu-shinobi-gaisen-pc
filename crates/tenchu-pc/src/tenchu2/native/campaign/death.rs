use super::{Action, Campaign, Content, Destination, Playback, Run, Session};
use crate::settings::Settings;
use tenchu_assets::{ReadAt, Result};
use tenchu_core::tenchu2::mission::{Outcome, Phase};

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub struct Hold {
    enabled: bool,
    live: bool,
    pub(super) pending: Option<(Run, Destination)>,
}

impl Hold {
    pub fn configure(&mut self, settings: &Settings) {
        self.enabled = settings.wait_after_death;
        self.live = settings.live_death_scene;
    }

    pub fn enabled(&self) -> bool {
        self.enabled
    }

    pub fn live(&self) -> bool {
        self.live
    }

    pub fn waiting(&self) -> bool {
        self.pending.is_some()
    }

    pub fn frozen(&self) -> bool {
        self.waiting() && !self.live
    }

    pub(in crate::tenchu2::native) fn clear(&mut self) {
        self.pending = None;
    }

    pub(super) fn begin(
        &mut self,
        session: &mut Session,
        run: &Run,
        campaign: &Campaign,
    ) -> Result<()> {
        if !self.enabled
            || self.waiting()
            || session.world.actors[session.world.player].actor.health != 0
        {
            return Ok(());
        }
        let phase = session.world.mission.phase;
        let (Phase::Ending {
            outcome: Outcome::Dead,
            finished_at,
            ..
        }
        | Phase::Finished {
            outcome: Outcome::Dead,
            finished_at,
        }) = phase
        else {
            return Ok(());
        };
        if matches!(phase, Phase::Ending { .. })
            && session.world.actors[session.world.player]
                .actor
                .motion
                .loops
                >= 0
        {
            return Ok(());
        }
        let mut next = run.clone();
        session.world.mission.phase = Phase::Finished {
            outcome: Outcome::Dead,
            finished_at,
        };
        let result = next.finish_area(&session.world, campaign);
        session.world.mission.phase = phase;
        self.pending = Some((next, result?));
        session.frame.transition_fade = None;
        Ok(())
    }

    pub(super) fn finish(&mut self) -> Option<(Run, Destination)> {
        self.pending.take()
    }
}

impl<S: ReadAt> Playback<S> {
    pub fn confirm(&mut self) {
        self.action = if self.death.waiting() {
            Some(Action::Continue)
        } else {
            match self.content {
                Content::MissionSelection(_)
                | Content::Prologue(_)
                | Content::Equipment(_)
                | Content::Results(_) => Some(Action::Continue),
                Content::Failed | Content::Complete => Some(Action::Retry),
                _ => None,
            }
        };
    }

    pub fn controller_pause(&mut self, pressed: u16, circle_confirm: bool) -> bool {
        use tenchu_core::input::{CIRCLE, CROSS, START};
        if self.death.waiting() {
            let confirm = if circle_confirm { CIRCLE } else { CROSS };
            if pressed & (confirm | START) != 0 {
                self.confirm();
            }
            false
        } else {
            matches!(self.content, Content::Gameplay(_)) && pressed & START != 0
        }
    }

    pub fn death_controls(&mut self, ctx: &egui::Context, photo_mode: bool) {
        if self.death.waiting() && !photo_mode {
            egui::Area::new("jp-death-continue".into())
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -12.0])
                .show(ctx, |ui| {
                    ui.spacing_mut().interact_size.y = 36.0;
                    if ui.button("Continue · Enter / Confirm / Start").clicked() {
                        self.confirm();
                    }
                });
        }
    }
}
