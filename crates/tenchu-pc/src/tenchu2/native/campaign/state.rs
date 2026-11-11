use super::*;
use serde::{Deserialize, Serialize};

const SIGNATURE: &str = "native-tenchu2-jp-campaign-1";

#[derive(Serialize, Deserialize)]
pub(in crate::tenchu2::native) enum Scene<G, S> {
    Gameplay(Box<G>),
    Story(Box<S>),
}

#[derive(Serialize, Deserialize)]
pub(in crate::tenchu2::native) struct State<C> {
    pub(super) signature: String,
    pub(super) run: Run,
    pub(super) content: C,
    action: Option<Action>,
    loadout: Option<[u8; 21]>,
    previous_buttons: u16,
    from_menu: bool,
    pub(super) death: death::Hold,
    keep_all_active: bool,
    circle_action: tenchu_core::input::CircleAction,
    bleeding: tenchu_core::game::bleeding::Settings,
    imported: bool,
    gaisen_water_depth: i16,
}

pub(in crate::tenchu2::native) type Owned =
    State<Scene<super::super::state::Owned, super::super::story::state::Owned>>;
pub(in crate::tenchu2::native) type Borrowed<'a> =
    State<Scene<super::super::state::Borrowed<'a>, super::super::story::state::Borrowed<'a>>>;

#[cfg(test)]
impl Borrowed<'_> {
    pub(in crate::tenchu2::native) fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        let Self {
            signature,
            run,
            content,
            action,
            loadout,
            previous_buttons,
            from_menu,
            death,
            keep_all_active,
            circle_action,
            bleeding,
            imported,
            gaisen_water_depth,
        } = self;
        out.value(signature)?;
        out.value(run)?;
        match content {
            Scene::Gameplay(session) => {
                out.value(&0u32)?;
                session.capture(out)?;
            }
            Scene::Story(story) => {
                out.value(&1u32)?;
                story.capture(out)?;
            }
        }
        out.value(action)?;
        out.value(loadout)?;
        out.value(previous_buttons)?;
        out.value(from_menu)?;
        out.value(death)?;
        out.value(keep_all_active)?;
        out.value(circle_action)?;
        out.value(bleeding)?;
        out.value(imported)?;
        out.value(gaisen_water_depth)
    }
}

impl<S: ReadAt> Playback<S> {
    pub(in crate::tenchu2::native) fn snapshot(&self) -> Result<Borrowed<'_>> {
        let Self {
            disc: _,
            campaign: _,
            run,
            content,
            generation: _,
            action,
            loadout,
            previous_buttons,
            blank: _,
            save: _,
            from_menu,
            death,
            keep_all_active,
            circle_action,
            bleeding,
            gaisen,
            gaisen_water_depth,
        } = self;
        let content = match content {
            Content::Gameplay(session) => Scene::Gameplay(Box::new(session.snapshot())),
            Content::Story { story, .. } => Scene::Story(Box::new(story.snapshot())),
            _ => return Err("Start a mission or 3D cutscene before saving a state".into()),
        };
        Ok(State {
            signature: SIGNATURE.into(),
            run: run.clone(),
            content,
            action: *action,
            loadout: *loadout,
            previous_buttons: *previous_buttons,
            from_menu: *from_menu,
            death: death.clone(),
            keep_all_active: *keep_all_active,
            circle_action: *circle_action,
            bleeding: *bleeding,
            imported: gaisen.is_some(),
            gaisen_water_depth: *gaisen_water_depth,
        })
    }

    #[cfg(test)]
    pub fn capture_state(&self) -> Result<Vec<u8>> {
        crate::savestate::encode(&self.snapshot()?)
    }

    #[cfg(test)]
    pub fn restore_state(&mut self, bytes: &[u8]) -> Result<()> {
        self.restore_snapshot(crate::savestate::decode(bytes)?, |_| Ok(()))
    }

    pub(in crate::tenchu2::native) fn restore_snapshot<T>(
        &mut self,
        saved: Owned,
        prepare: impl FnOnce(&Content) -> Result<T>,
    ) -> Result<T> {
        saved.validate(&self.campaign, self.gaisen.is_some())?;
        let State {
            run,
            content,
            action,
            loadout,
            previous_buttons,
            from_menu,
            death,
            keep_all_active,
            circle_action,
            bleeding,
            gaisen_water_depth,
            ..
        } = saved;
        let content = match content {
            Scene::Gameplay(session) => {
                Content::Gameplay(Box::new(session.restore(self.gaisen.as_ref())?))
            }
            Scene::Story(story) => {
                let story = story.restore(&self.disc, &self.campaign)?;
                let audio = match &self.content {
                    Content::Story {
                        story: previous,
                        audio,
                    } if previous.resources.audio == story.resources.audio => audio.clone(),
                    _ => story
                        .resources
                        .audio
                        .as_ref()
                        .map(|track| Clip::load(&self.disc, track).map(Arc::new))
                        .transpose()?,
                };
                Content::Story {
                    story: Box::new(story),
                    audio,
                }
            }
        };
        let prepared = prepare(&content)?;
        self.run = run;
        self.content = content;
        self.action = action;
        self.loadout = loadout;
        self.previous_buttons = previous_buttons;
        self.from_menu = from_menu;
        self.death = death;
        self.keep_all_active = keep_all_active;
        self.circle_action = circle_action;
        self.bleeding = bleeding;
        self.gaisen_water_depth = gaisen_water_depth;
        self.generation = self.generation.wrapping_add(1);
        Ok(prepared)
    }

    pub(in crate::tenchu2::native) fn restore_preferences(
        &self,
        settings: &mut crate::settings::Settings,
    ) {
        settings.wait_after_death = self.death.enabled();
        settings.live_death_scene = self.death.live();
        settings.keep_enemies_active = self.keep_all_active;
        settings.circle_action = self.circle_action;
        settings.bleeding = self.bleeding;
    }
}

fn validate_run(run: &Run, campaign: &Campaign) -> Result<()> {
    if !(1..=3).contains(&run.difficulty)
        || campaign.mission(run.hero, run.sequence) != Some(run.mission)
        || run.carry.bleeding.as_ref().is_some_and(|b| !b.valid())
        || run
            .progress
            .records
            .iter()
            .flatten()
            .any(|record| record.rank > 6)
        || run.progress.stock.iter().any(|stock| {
            stock.items.iter().flatten().any(|&amount| amount > 99)
                || stock.items[1..].iter().all(Option::is_none)
        })
    {
        return Err("Invalid native T2 campaign run, progress or supplies".into());
    }
    Ok(())
}

impl Owned {
    fn validate(&self, campaign: &Campaign, imported: bool) -> Result<()> {
        if self.signature != SIGNATURE || self.imported != imported {
            return Err(
                "State requires the matching native T2 campaign or imported T1 stage".into(),
            );
        }
        validate_run(&self.run, campaign)?;
        let mut bleeding = self.bleeding;
        bleeding.validate();
        if bleeding != self.bleeding
            || !(0..=16000).contains(&self.gaisen_water_depth)
            || self.gaisen_water_depth % 10 != 0
        {
            return Err("Invalid native T2 gameplay settings in state".into());
        }
        match &self.content {
            Scene::Gameplay(session) => {
                let (world, assets, _) = &session.core;
                if assets.hero != self.run.hero
                    || assets.sequence != self.run.sequence
                    || world.difficulty != self.run.difficulty
                    || assets
                        .level
                        .collision
                        .gaisen
                        .as_ref()
                        .is_some_and(|g| g.water_depth != self.gaisen_water_depth)
                    || postcard::to_allocvec(&assets.campaign).map_err(|e| e.to_string())?
                        != postcard::to_allocvec(campaign).map_err(|e| e.to_string())?
                {
                    return Err("Saved native T2 world belongs to a different campaign run".into());
                }
                if let Some((next, destination)) = &self.death.pending {
                    validate_run(next, campaign)?;
                    if world
                        .actors
                        .get(world.player)
                        .is_none_or(|agent| agent.actor.health != 0)
                        || next.hero != self.run.hero
                        || next.difficulty != self.run.difficulty
                        || matches!(destination, Destination::Sequence(sequence) if *sequence != next.sequence)
                    {
                        return Err("Invalid pending native T2 death transition".into());
                    }
                }
            }
            Scene::Story(story) => {
                if imported || story.sequence != self.run.sequence || self.death.waiting() {
                    return Err("Saved native T2 story belongs to a different campaign run".into());
                }
            }
        }
        Ok(())
    }
}
