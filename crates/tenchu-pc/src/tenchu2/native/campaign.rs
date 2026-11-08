use super::{Session, story::Story};
use std::sync::Arc;
use tenchu_assets::{
    ReadAt, Result,
    audio::stream::Clip,
    disc::Disc,
    tenchu2::campaign::{Campaign, Kind},
    texture::Vram,
};
use tenchu_core::tenchu2::{
    audio::Sound,
    campaign::{Destination, Run},
    mission::Phase,
};

#[cfg(test)]
mod tests;

pub enum Content {
    Gameplay(Box<Session>),
    Story {
        story: Box<Story>,
        audio: Option<Arc<Clip>>,
    },
    Results,
    Failed,
    Complete,
}

#[derive(Clone, Copy)]
pub enum Action {
    Continue,
    Retry,
}

#[derive(Default)]
pub struct Update {
    pub sounds: Vec<Sound>,
    pub stop_music: bool,
}

pub struct Playback<S> {
    disc: Disc<S>,
    campaign: Campaign,
    pub run: Run,
    pub content: Content,
    pub generation: u64,
    pub action: Option<Action>,
    loadout: [u8; 21],
    previous_buttons: u16,
    blank: Vram,
}

impl<S: ReadAt> Playback<S> {
    pub fn load(disc: Disc<S>, sequence: usize, seed: u32, loadout: [u8; 21]) -> Result<Self> {
        let campaign = Campaign::load(&disc)?;
        let hero = (0..3)
            .find(|&hero| campaign.mission(hero, sequence).is_some())
            .ok_or("Sequence is not part of a Japanese Tenchu 2 campaign")?;
        let run = Run::new(&campaign, hero, sequence, 2, seed)?;
        let mut playback = Self {
            disc,
            campaign,
            run,
            content: Content::Complete,
            generation: 0,
            action: None,
            loadout,
            previous_buttons: 0,
            blank: Vram::default(),
        };
        playback.load_current()?;
        Ok(playback)
    }

    pub fn vram(&self) -> &Vram {
        match &self.content {
            Content::Gameplay(session) => &session.assets.sprites.vram,
            Content::Story { story, .. } => &story.level.vram,
            _ => &self.blank,
        }
    }

    fn load_current(&mut self) -> Result<()> {
        let descriptor = *self.run.descriptor(&self.campaign)?;
        self.content = match descriptor.kind {
            Kind::Gameplay(_) => {
                if self.run.prepare_area(&self.campaign)? {
                    self.run.carry.inventory = self.loadout;
                }
                let session = Session::for_run(&self.disc, &self.run)?;
                Content::Gameplay(Box::new(session))
            }
            Kind::Event => {
                let mut story = Story::load(
                    &self.disc,
                    self.run.sequence,
                    descriptor.resource,
                    self.run.random.seed,
                )?;
                story.state.random = self.run.random;
                story.step(true)?;
                let audio = story
                    .resources
                    .audio
                    .as_ref()
                    .map(|track| Clip::load(&self.disc, track).map(Arc::new))
                    .transpose()?;
                Content::Story {
                    story: Box::new(story),
                    audio,
                }
            }
            Kind::Results => {
                self.run.enter_results(&self.campaign)?;
                Content::Results
            }
            Kind::Introduction | Kind::Movie => {
                return Err(format!(
                    "JP sequence {}: {:?} playback is not connected yet",
                    self.run.sequence, descriptor.kind
                ));
            }
        };
        self.generation += 1;
        Ok(())
    }

    fn arrive(&mut self, destination: Destination) -> Result<()> {
        match destination {
            Destination::Sequence(_) => self.load_current(),
            Destination::Failed | Destination::Complete => {
                self.content = if destination == Destination::Failed {
                    Content::Failed
                } else {
                    Content::Complete
                };
                self.generation += 1;
                Ok(())
            }
        }
    }

    pub fn step(&mut self, buttons: u16) -> Result<Update> {
        let pressed = buttons & !self.previous_buttons;
        self.previous_buttons = buttons;
        let action = self.action.take();
        let retry_on_confirm = matches!(self.content, Content::Failed | Content::Complete);
        let mut update = Update::default();
        let destination = match &mut self.content {
            Content::Gameplay(session) => {
                if matches!(session.world.mission.phase, Phase::Finished { .. }) {
                    Some(self.run.finish_area(&session.world, &self.campaign)?)
                } else {
                    update.sounds = session.step(buttons)?;
                    None
                }
            }
            Content::Story { story, .. } => {
                if let Some(ending) = story.state.ending.filter(|e| e.complete) {
                    self.run.random = story.state.random;
                    Some(self.run.advance(&self.campaign, ending.skip)?)
                } else {
                    story.step(true)?;
                    if story.state.ending.is_none()
                        && (story.state.finished(&story.resources.script)
                            || (story.state.frame > 0 && pressed & 0x6000 != 0))
                    {
                        let skip = pressed & 0x4000 != 0;
                        if pressed & 0x6000 != 0 {
                            story.state.presentation.tint = None;
                            story.state.presentation.warp = None;
                        }
                        let mut next = self.run.descriptor(&self.campaign)?.next;
                        if skip {
                            for _ in 0..self.campaign.sequences.len() {
                                let Some(index) = next else { break };
                                let row = self.campaign.sequences[index];
                                if row.kind != Kind::Event {
                                    break;
                                }
                                next = row.next;
                            }
                        }
                        story.state.begin_exit(
                            next.map(|i| self.campaign.sequences[i]),
                            skip,
                            story.resources.results_fade,
                        );
                        update.stop_music = true;
                    }
                    None
                }
            }
            Content::Results
                if matches!(action, Some(Action::Continue)) || pressed & 0x6000 != 0 =>
            {
                Some(self.run.advance(&self.campaign, false)?)
            }
            Content::Results | Content::Failed | Content::Complete
                if matches!(action, Some(Action::Retry))
                    || (retry_on_confirm && pressed & 0x6000 != 0) =>
            {
                Some(self.run.retry(&self.campaign)?)
            }
            _ => None,
        };
        if let Some(destination) = destination {
            update.stop_music = true;
            self.arrive(destination)?;
        }
        Ok(update)
    }
}
