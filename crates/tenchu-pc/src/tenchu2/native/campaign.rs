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

mod death;
#[cfg(test)]
mod tests;

pub enum Content {
    MissionSelection(Box<super::selection::Selection>),
    Prologue(Box<super::film::Prologue>),
    Equipment(Box<super::equipment::Selection>),
    Gameplay(Box<Session>),
    Introduction(Box<super::film::Introduction>),
    Movie(Box<super::film::Movie>),
    Story {
        story: Box<Story>,
        audio: Option<Arc<Clip>>,
    },
    Results(Box<super::results::Results>),
    Failed,
    Complete,
}

#[derive(Clone, Copy)]
pub enum Action {
    Continue,
    Retry,
    Quit,
    Equipment(super::equipment::Edit),
    MissionSelection(super::selection::Edit),
    SelectMission,
}

#[derive(Default)]
pub struct Update {
    pub sounds: Vec<tenchu_assets::audio::voice::Sourced<Sound>>,
    pub original_sounds: Vec<tenchu_assets::audio::voice::Sourced<tenchu_core::audio::Sound>>,
    pub stop_music: bool,
    pub music: Option<Arc<Clip>>,
    pub quit: bool,
}

pub struct Playback<S> {
    pub(super) disc: Disc<S>,
    campaign: Campaign,
    pub run: Run,
    pub content: Content,
    pub generation: u64,
    pub action: Option<Action>,
    loadout: Option<[u8; 21]>,
    previous_buttons: u16,
    blank: Vram,
    save: Option<super::save::SaveFile>,
    from_menu: bool,
    pub death: death::Hold,
    keep_all_active: bool,
    circle_action: tenchu_core::input::CircleAction,
    bleeding: tenchu_core::game::bleeding::Settings,
    gaisen: Option<super::gaisen::Source>,
    gaisen_water_depth: i16,
}

impl<S: ReadAt> Playback<S> {
    pub fn configure(&mut self, settings: &crate::settings::Settings) {
        self.death.configure(settings);
        self.keep_all_active = settings.keep_enemies_active;
        self.circle_action = settings.circle_action;
        self.bleeding = settings.bleeding;
        if !self.bleeding.enabled {
            self.run.carry.bleeding = None;
        }
        if let Content::Gameplay(session) = &mut self.content {
            session.world.set_bleeding(self.bleeding);
        }
    }

    pub fn load(
        disc: Disc<S>,
        sequence: usize,
        difficulty: u8,
        seed: u32,
        loadout: Option<[u8; 21]>,
        save: Option<super::save::SaveFile>,
    ) -> Result<Self> {
        let mut playback = Self::new(disc, Some(sequence), difficulty, seed, loadout, save)?;
        playback.load_current()?;
        Ok(playback)
    }

    pub fn menu(
        disc: Disc<S>,
        difficulty: u8,
        seed: u32,
        save: Option<super::save::SaveFile>,
    ) -> Result<Self> {
        let mut playback = Self::new(disc, None, difficulty, seed, None, save)?;
        playback.select_mission()?;
        Ok(playback)
    }

    pub fn gaisen(
        disc: Disc<S>,
        source: super::gaisen::Source,
        hero: usize,
        difficulty: u8,
        seed: u32,
        loadout: Option<[u8; 21]>,
        water_depth: i16,
    ) -> Result<Self> {
        let mut inventory = loadout.unwrap_or([0; 21]);
        inventory[0] = 255;
        let alternate = inventory[19] != 0;
        inventory[19] = 0;
        let mut session = source.session(
            &disc,
            tenchu_core::tenchu2::world::Options {
                hero,
                difficulty,
                seed,
                inventory: tenchu_core::tenchu2::inventory::Inventory::from_amounts(inventory),
            },
            alternate,
            water_depth,
        )?;
        let campaign = session.assets.campaign.clone();
        let mut run = Run::new(&campaign, hero, 0, difficulty, seed)?;
        run.prepare_area(&campaign)?;
        run.carry.inventory = inventory;
        run.carry.alternate = alternate;
        run.restore_area(&mut session.world, &session.assets, true)?;
        let mut playback = Self::state(disc, campaign, run, loadout, None);
        playback.gaisen = Some(source);
        playback.gaisen_water_depth = water_depth;
        playback.content = Content::Gameplay(Box::new(session));
        playback.generation = 1;
        if loadout.is_none() {
            playback.run.carry.inventory[0] = 0;
            playback.load_current()?;
        }
        Ok(playback)
    }

    fn new(
        disc: Disc<S>,
        sequence: Option<usize>,
        difficulty: u8,
        seed: u32,
        loadout: Option<[u8; 21]>,
        save: Option<super::save::SaveFile>,
    ) -> Result<Self> {
        let campaign = Campaign::load(&disc)?;
        let sequence =
            sequence.unwrap_or(campaign.starts[0][0].ok_or("Missing Japanese training mission")?);
        let hero = (0..3)
            .find(|&hero| campaign.mission(hero, sequence).is_some())
            .ok_or("Sequence is not part of a Japanese Tenchu 2 campaign")?;
        let mut run = Run::new(&campaign, hero, sequence, difficulty, seed)?;
        if let Some(save) = &save {
            run.progress = save.load()?;
        }
        Ok(Self::state(disc, campaign, run, loadout, save))
    }

    fn state(
        disc: Disc<S>,
        campaign: Campaign,
        run: Run,
        loadout: Option<[u8; 21]>,
        save: Option<super::save::SaveFile>,
    ) -> Self {
        Self {
            disc,
            campaign,
            run,
            content: Content::Complete,
            generation: 0,
            action: None,
            loadout,
            previous_buttons: 0,
            blank: Vram::default(),
            save,
            from_menu: false,
            death: Default::default(),
            keep_all_active: false,
            circle_action: Default::default(),
            bleeding: Default::default(),
            gaisen: None,
            gaisen_water_depth: tenchu_assets::tenchu2::collision::Gaisen::DEFAULT_WATER_DEPTH,
        }
    }

    fn select_mission(&mut self) -> Result<()> {
        self.death.clear();
        self.from_menu = true;
        self.loadout = None;
        self.content = Content::MissionSelection(Box::new(super::selection::Selection::load(
            &self.disc,
            self.run.progress.clone(),
        )?));
        self.generation += 1;
        Ok(())
    }

    pub fn vram(&self) -> &Vram {
        match &self.content {
            Content::Gameplay(session) => &session.assets.sprites.vram,
            Content::Story { story, .. } => &story.sprites.vram,
            _ => &self.blank,
        }
    }

    fn load_current(&mut self) -> Result<()> {
        self.death.clear();
        let descriptor = *self.run.descriptor(&self.campaign)?;
        self.content =
            match descriptor.kind {
                Kind::Gameplay(_) => {
                    if self.run.prepare_area(&self.campaign)? {
                        if let Some(items) = self.loadout.or_else(|| {
                            tenchu_core::tenchu2::equipment::training(descriptor.resource)
                        }) {
                            self.run.carry.inventory = items;
                            self.run.carry.alternate = items[19] != 0;
                            self.run.carry.inventory[19] = 0;
                        } else {
                            self.content =
                                Content::Equipment(Box::new(super::equipment::Selection::load(
                                    &self.disc,
                                    self.run.hero,
                                    if self.gaisen.is_some() {
                                        8
                                    } else {
                                        usize::from(descriptor.resource)
                                    },
                                    self.run.progress.stock[self.run.hero].clone(),
                                )?));
                            self.generation += 1;
                            return Ok(());
                        }
                    }
                    let mut session = if let Some(source) = &self.gaisen {
                        let mut session = source.session(
                            &self.disc,
                            tenchu_core::tenchu2::world::Options {
                                hero: self.run.hero,
                                difficulty: self.run.difficulty,
                                seed: self.run.random.seed,
                                inventory: tenchu_core::tenchu2::inventory::Inventory::from_amounts(
                                    self.run.carry.inventory,
                                ),
                            },
                            self.run.carry.alternate,
                            self.gaisen_water_depth,
                        )?;
                        self.run
                            .restore_area(&mut session.world, &session.assets, true)?;
                        session
                    } else {
                        Session::for_run(&self.disc, &self.run)?
                    };
                    session.world.set_bleeding(self.bleeding);
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
                    let values = self.run.enter_results(&self.campaign)?.clone();
                    Content::Results(Box::new(super::results::Results::load(&self.disc, values)?))
                }
                Kind::Introduction => Content::Introduction(Box::new(
                    super::film::Introduction::load(&self.disc, descriptor.resource)?,
                )),
                Kind::Movie => Content::Movie(Box::new(super::film::Movie::load(
                    &self.disc,
                    descriptor.resource,
                )?)),
            };
        self.generation += 1;
        if matches!(self.content, Content::Results(_))
            && let Some(save) = &self.save
        {
            save.write(&self.run.progress)?;
        }
        Ok(())
    }

    fn arrive(&mut self, destination: Destination) -> Result<()> {
        match destination {
            Destination::Sequence(_) => self.load_current(),
            Destination::Failed | Destination::Complete => {
                self.death.clear();
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
        let previous = self.previous_buttons;
        let pressed = buttons & !previous;
        self.previous_buttons = buttons;
        let action = self.action.take();
        if matches!(action, Some(Action::SelectMission)) {
            if self.gaisen.is_some() {
                return Ok(Update {
                    quit: true,
                    stop_music: true,
                    ..Default::default()
                });
            }
            self.select_mission()?;
            return Ok(Update {
                stop_music: true,
                ..Default::default()
            });
        }
        let mut update = Update::default();
        let mut choose_mission = false;
        let destination = match &mut self.content {
            Content::MissionSelection(selection) => {
                if selection.state.finished {
                    self.run.progress.clone_from(&selection.state.progress);
                    if selection.state.outcome
                        == Some(tenchu_core::tenchu2::selection::Outcome::Cancel)
                    {
                        update.quit = true;
                        None
                    } else {
                        let hero = selection.state.hero;
                        let sequence = self.campaign.starts[hero][selection.state.mission]
                            .ok_or("Missing Japanese mission start")?;
                        let mut run = Run::new(
                            &self.campaign,
                            hero,
                            sequence,
                            self.run.difficulty,
                            self.run.random.seed,
                        )?;
                        run.progress.clone_from(&self.run.progress);
                        run.random = self.run.random;
                        self.run = run;
                        if hero != 2 && self.run.progress.records[hero][0].time_tenths == 59999 {
                            self.content = Content::Prologue(Box::new(
                                super::film::Prologue::load(&self.disc)?,
                            ));
                            self.generation += 1;
                            update.stop_music = true;
                            None
                        } else {
                            Some(Destination::Sequence(sequence))
                        }
                    }
                } else {
                    update.sounds = selection
                        .step(buttons, action)?
                        .into_iter()
                        .map(Into::into)
                        .collect();
                    None
                }
            }
            Content::Equipment(selection) => {
                if selection.state.finished {
                    self.run.progress.stock[self.run.hero].clone_from(&selection.state.stock);
                    if selection.state.outcome
                        == Some(tenchu_core::tenchu2::equipment::Outcome::Cancel)
                    {
                        choose_mission = self.from_menu;
                        update.quit = !choose_mission;
                        None
                    } else {
                        self.run.carry.inventory = selection.state.inventory();
                        self.run.carry.alternate = selection.state.alternate();
                        Some(Destination::Sequence(self.run.sequence))
                    }
                } else {
                    update.sounds = selection
                        .step(buttons, action)
                        .into_iter()
                        .map(Into::into)
                        .collect();
                    None
                }
            }
            Content::Introduction(intro) => {
                use tenchu_core::tenchu2::introduction::{Outcome, Phase};
                if intro.state.phase == Phase::Finished {
                    if intro.state.outcome == Outcome::Quit {
                        update.quit = true;
                        None
                    } else {
                        Some(self.run.advance(&self.campaign, false)?)
                    }
                } else {
                    if intro.step(buttons) {
                        update.music = intro.audio.clone();
                    }
                    None
                }
            }
            Content::Prologue(intro) => {
                if intro.state.finished {
                    Some(Destination::Sequence(self.run.sequence))
                } else {
                    intro.step(buttons, matches!(action, Some(Action::Continue)));
                    None
                }
            }
            Content::Movie(movie) => {
                if movie.finished {
                    if movie.quit {
                        update.quit = true;
                        None
                    } else {
                        Some(self.run.advance(&self.campaign, false)?)
                    }
                } else {
                    movie.step(&self.disc, buttons)?;
                    None
                }
            }
            Content::Gameplay(session) => {
                session.world.keep_all_active = self.keep_all_active;
                session.circle_action = self.circle_action;
                session.world.set_bleeding(self.bleeding);
                self.death.begin(session, &self.run, &self.campaign)?;
                if self.death.waiting() {
                    if !self.death.enabled() || matches!(action, Some(Action::Continue)) {
                        let (run, destination) = self.death.finish().unwrap();
                        self.run = run;
                        Some(destination)
                    } else {
                        if self.death.live() {
                            update.sounds = session.observe_death()?;
                        }
                        session.frame.transition_fade = None;
                        None
                    }
                } else if matches!(session.world.mission.phase, Phase::Finished { .. }) {
                    Some(self.run.finish_area(&session.world, &self.campaign)?)
                } else {
                    update.sounds = session.step(buttons)?;
                    if let Some(scene) = &mut session.gaisen {
                        update.original_sounds = std::mem::take(&mut scene.audio.sounds);
                    }
                    self.death.begin(session, &self.run, &self.campaign)?;
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
            Content::Results(results) => {
                if let Some((action, remaining)) = &mut results.exit {
                    *remaining = remaining.saturating_sub(1);
                    if *remaining != 0 {
                        None
                    } else {
                        match action {
                            Action::Continue => Some(self.run.advance(&self.campaign, false)?),
                            Action::Retry => Some(self.run.retry(&self.campaign)?),
                            Action::Quit => {
                                choose_mission = self.from_menu;
                                update.quit = !choose_mission;
                                None
                            }
                            Action::Equipment(_)
                            | Action::MissionSelection(_)
                            | Action::SelectMission => None,
                        }
                    }
                } else {
                    let selected = action
                        .filter(|a| matches!(a, Action::Continue | Action::Retry | Action::Quit))
                        .or_else(|| {
                            if previous != 0 {
                                return None;
                            }
                            match buttons {
                                0x2000 => Some(Action::Continue),
                                8 => Some(Action::Retry),
                                0x4000 | 9 => Some(Action::Quit),
                                _ => None,
                            }
                        });
                    if let Some(action) = selected {
                        results.exit = Some((action, 30));
                        update.sounds.push(
                            Sound {
                                id: 2,
                                slot: 0,
                                note: 36,
                                volume: [127; 2],
                            }
                            .into(),
                        );
                    }
                    None
                }
            }
            Content::Failed | Content::Complete
                if matches!(action, Some(Action::Quit)) || pressed & 0x4000 != 0 =>
            {
                choose_mission = self.from_menu;
                update.quit = !choose_mission;
                None
            }
            Content::Failed | Content::Complete
                if matches!(action, Some(Action::Retry)) || pressed & (0x2000 | 8) != 0 =>
            {
                Some(self.run.retry(&self.campaign)?)
            }
            _ => None,
        };
        if choose_mission {
            update.stop_music = true;
            self.select_mission()?;
        } else if let Some(destination) = destination {
            update.stop_music = true;
            self.arrive(destination)?;
        }
        Ok(update)
    }
}
