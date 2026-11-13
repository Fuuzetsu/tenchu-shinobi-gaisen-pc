use super::{
    Session,
    campaign::{Content, Playback},
    gaisen, models,
};
use crate::{
    randomizer::{Catalog, Enemies, Entry, Group, Plan, Run, Source},
    settings::Settings,
    visuals::{appearance::library::Library, pack::Pack},
};
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};
use tenchu_assets::{
    ReadAt, Result,
    disc::Disc,
    tenchu2::{campaign::Campaign, motions, registry::Registry},
};
use tenchu_core::{
    game::Game,
    tenchu2::{campaign::Run as CampaignRun, inventory::Inventory},
};

pub(super) struct Resources {
    pub catalog: Catalog,
    library: Library,
    donor: Option<Game>,
    campaign: Campaign,
    stages: Vec<tenchu_assets::game::Stage>,
    gaisen: Option<PathBuf>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(super) struct Active {
    pub run: Run,
    pub plan: Plan,
    pub health: i16,
}

pub(super) struct Previous {
    pub campaign: Campaign,
    pub run: CampaignRun,
    pub content: Content,
    pub gaisen: Option<gaisen::Source>,
    pub loadout: Option<[u8; 21]>,
    pub origin: super::campaign::Origin,
    pub death: super::campaign::death::Hold,
    pub keep_all_active: bool,
    pub previous_buttons: u16,
    pub action: Option<super::campaign::Action>,
}

#[derive(Default)]
pub(super) struct Runtime {
    pub resources: Option<Resources>,
    pub active: Option<Active>,
    pub previous: Option<Previous>,
}

impl Resources {
    pub fn context(
        &self,
        disc: &Disc<impl ReadAt>,
        active: &Active,
        water_depth: i16,
    ) -> Result<(Campaign, Option<gaisen::Source>)> {
        let source = self.source(&active.plan, active.run.document.rules.language.game())?;
        let campaign = if let Some(source) = &source {
            source
                .session(
                    disc,
                    tenchu_core::tenchu2::world::Options {
                        hero: active.plan.character as usize,
                        difficulty: active.plan.difficulty + 1,
                        seed: active.plan.seed,
                        inventory: Inventory::default(),
                    },
                    false,
                    water_depth,
                )?
                .assets
                .campaign
        } else {
            self.campaign.clone()
        };
        Ok((campaign, source))
    }

    pub fn load(disc: &Disc<impl ReadAt>, gaisen: Option<&Path>) -> Result<Self> {
        let (mut library, donor, stages) = if let Some(path) = gaisen {
            let assets = crate::startup::open(path)?;
            (
                Library::load(&assets)?,
                Some(models::donor(&assets)?),
                assets.stages()?,
            )
        } else {
            (Library::default(), None, Vec::new())
        };
        library.load_tenchu2(disc)?;
        let registry = Registry::load(disc)?;
        let wanted = registry
            .actors
            .iter()
            .flat_map(|a| a.motions.iter().map(|m| m.clip))
            .collect();
        let clips = motions::collect(disc, wanted)?;
        let mut actors: Vec<_> = library
            .bodies
            .iter()
            .filter_map(|body| {
                let kind = super::spawning::body::kind(&library, body).ok()?;
                let definition = registry.actor(kind)?;
                let player = body.parts.len() == tenchu_assets::tenchu2::BONES.len();
                let enemy = definition.health > 0
                    && definition
                        .motions
                        .iter()
                        .all(|m| clips.contains_key(&m.clip));
                (player || enemy).then(|| Entry {
                    id: body.id.clone(),
                    name: body.name.clone(),
                    player,
                    enemy,
                    group: match kind & 0xf0 {
                        0 if kind < 3 => Group::Players,
                        0 | 0xe0 | 0xf0 => Group::Bosses,
                        0x10 | 0x60 => Group::Civilians,
                        0x70..=0x90 => Group::Animals,
                        _ => Group::Guards,
                    },
                })
            })
            .collect();
        actors.sort_by(|a, b| a.id.cmp(&b.id));
        let campaign = Campaign::load(disc)?;
        let mut choices = Vec::new();
        for hero in 0..3 {
            for mission in 0..11 {
                if campaign.first_gameplay(hero, mission).is_some() {
                    choices.push((
                        hero * 11 + mission,
                        format!(
                            "T2 {} · {}",
                            Source::Tenchu2.heroes()[hero],
                            if mission == 0 {
                                "Training".into()
                            } else {
                                format!("Mission {mission}")
                            }
                        ),
                    ));
                }
            }
        }
        choices.extend(
            stages
                .iter()
                .map(|s| (100 + s.order as usize, format!("T1 · {}", s.name))),
        );
        choices.sort_by_key(|s| s.0);
        Ok(Self {
            catalog: Catalog {
                source: Source::Tenchu2,
                actors,
                stages: choices,
            },
            library,
            donor,
            campaign,
            stages,
            gaisen: gaisen.map(Path::to_owned),
        })
    }

    pub fn source(
        &self,
        plan: &Plan,
        language: tenchu_assets::script::Language,
    ) -> Result<Option<gaisen::Source>> {
        if plan.mission < 100 {
            return Ok(None);
        }
        let stage = self
            .stages
            .iter()
            .find(|s| s.order as usize + 100 == plan.mission)
            .ok_or("Connect the T1 disc used by this seed")?;
        Ok(Some(gaisen::Source::open(
            self.gaisen
                .as_ref()
                .ok_or("Connect the T1 disc used by this seed")?,
            stage.id,
            i16::from(plan.character == 1),
            plan.layout,
            language,
        )?))
    }

    pub fn apply(
        &mut self,
        disc: &Disc<impl ReadAt>,
        session: &mut Session,
        active: &Active,
    ) -> Result<()> {
        let rules = &active.run.document.rules;
        let document = &active.run.document;
        let round = active.run.round;
        let area = u32::from(session.world.sequence);
        let random = |domain: &str, index: usize| {
            document.random(round, &format!("area-{area}-{domain}"), index as u32)
        };
        let pool = rules.enemy_pool.choices_for(&self.catalog, true);
        for index in 0..session.world.actors.len() {
            let agent = &session.world.actors[index];
            if index == session.world.player
                || agent.actor.health == 0
                || agent.actor.max_health <= 0
            {
                continue;
            }
            if !rules.replace_civilians && matches!(agent.actor.kind & 0xf0, 0x10 | 0x60) {
                continue;
            }
            if rules.protect_story
                && (agent.behavior.profile.protected
                    || matches!(agent.actor.kind & 0xf0, 0 | 0xe0 | 0xf0)
                    || session.world.mission.targets.contains(&Some(index))
                    || session.world.script_hidden.contains(&index)
                    || matches!(
                        agent.behavior.programs.idle,
                        tenchu_core::tenchu2::ai::Program::PlayerOne
                            | tenchu_core::tenchu2::ai::Program::PlayerTwo
                    ))
            {
                continue;
            }
            if rules.enemies != Enemies::Original {
                let mut rng = if rules.enemies == Enemies::Uniform {
                    document.random(round, "enemy", 0)
                } else {
                    random("enemy", index)
                };
                let model = &pool[rng.index(pool.len())].id;
                session.replace_actor(disc, index, model, Some(&self.library))?;
            }
            let actor = &mut session.world.actors[index].actor;
            if rules.random_facing {
                actor.rotation.y = random("facing", index).index(4096) as i16;
            }
            let percentage = rules
                .enemy_health
                .sample(&mut random("enemy-health", index));
            actor.max_health =
                (i32::from(actor.max_health) * i32::from(percentage) / 100).clamp(1, 32767) as i16;
            if actor.health > 0 {
                actor.health = actor.max_health;
            }
            if let Some(scene) = &mut session.gaisen {
                scene.director.refresh_actor_health(index, actor.max_health);
            }
            session.world.publish(index, &session.assets);
        }
        if rules.pickups {
            let items: Vec<_> = Source::Tenchu2
                .items()
                .map(|(id, _)| id)
                .filter(|&i| Source::Tenchu2.consumable(i) && !rules.blocked_items.contains(&i))
                .collect();
            for (index, entry) in session.world.items.entries.iter_mut().enumerate() {
                if let tenchu_core::tenchu2::items::State::Pickup(pickup) = &mut entry.state
                    && Source::Tenchu2.consumable(usize::from(pickup.item))
                {
                    pickup.item = items[random("pickup", index).index(items.len())] as u8;
                }
            }
        }
        let settings = active.settings(&Settings::default());
        session.models.sync_optional(
            &mut session.world,
            &mut session.assets,
            self.donor.as_mut(),
            &self.library,
            &Arc::<Pack>::default(),
            &settings,
        )?;
        let actor = &mut session.world.actors[session.world.player].actor;
        if let Some(maximum) = active.plan.health {
            actor.max_health = maximum as i16;
        }
        actor.health = active.health;
        session.world.ai.sight_percent = active.plan.sight;
        session.world.keep_all_active = rules.keep_active;
        session.reset_interpolation()
    }
}

impl Active {
    pub fn same_source(&self, other: &Self) -> bool {
        if self.plan.mission < 100 && other.plan.mission < 100 {
            return true;
        }
        self.plan.mission == other.plan.mission
            && self.plan.character == other.plan.character
            && self.plan.layout == other.plan.layout
            && self.run.document.rules.language == other.run.document.rules.language
    }

    pub fn settings(&self, settings: &Settings) -> Settings {
        let mut settings = settings.clone();
        settings.player_model = self.plan.player_model.clone().unwrap_or_default();
        settings.moveset_model.clear();
        settings.model_moveset = self.run.document.rules.model_moveset;
        settings.model_moveset_options.player_movement = self.run.document.rules.player_movement;
        settings.model_moveset_options.no_idle_taunts = self.run.document.rules.no_idle_taunts;
        settings
    }

    pub fn replenish(&self, session: &mut Session) {
        if !self.run.document.rules.infinite_items {
            return;
        }
        let inventory = &mut session.world.actors[session.world.player].actor.inventory;
        for (id, &amount) in self.plan.inventory.iter().enumerate() {
            if amount == 255 && Source::Tenchu2.consumable(id) {
                if id == 20 {
                    inventory.resurrections = 255;
                } else {
                    inventory.counts[id] = 255;
                }
            }
        }
    }
}

impl<S: ReadAt> Playback<S> {
    pub fn start_randomizer(&mut self, run: Run) -> Result<()> {
        let resources = self
            .randomizer
            .resources
            .as_mut()
            .ok_or("Load the native randomizer catalog first")?;
        let plan = run.plan(&resources.catalog)?;
        let source = resources.source(&plan, run.document.rules.language.game())?;
        let mut amounts: [u8; 21] = plan
            .inventory
            .as_slice()
            .try_into()
            .map_err(|_| "Invalid native loadout")?;
        let alternate = amounts[19] != 0;
        amounts[19] = 0;
        let (mut campaign_run, mut session) = if let Some(source) = &source {
            let session = source.session(
                &self.disc,
                tenchu_core::tenchu2::world::Options {
                    hero: plan.character as usize,
                    difficulty: plan.difficulty + 1,
                    seed: plan.seed,
                    inventory: Inventory::from_amounts(amounts),
                },
                alternate,
                self.gaisen_water_depth,
            )?;
            let run = CampaignRun::new(
                &session.assets.campaign,
                plan.character as usize,
                0,
                plan.difficulty + 1,
                plan.seed,
            )?;
            (run, session)
        } else {
            let hero = plan.mission / 11;
            let sequence = resources
                .campaign
                .first_gameplay(hero, plan.mission % 11)
                .ok_or("Missing native mission gameplay")?;
            let mut run = CampaignRun::new(
                &resources.campaign,
                hero,
                sequence,
                plan.difficulty + 1,
                plan.seed,
            )?;
            run.prepare_area(&resources.campaign)?;
            run.carry.inventory = amounts;
            run.carry.alternate = alternate;
            let session = Session::for_run(&self.disc, &run)?;
            (run, session)
        };
        let campaign = session.assets.campaign.clone();
        campaign_run.prepare_area(&campaign)?;
        campaign_run.carry.inventory = amounts;
        campaign_run.carry.alternate = alternate;
        if source.is_some() {
            campaign_run.restore_area(&mut session.world, &session.assets, true)?;
        }
        let active = Active {
            health: plan.health.map_or(
                session.world.actors[session.world.player].actor.health,
                |h| h as i16,
            ),
            run,
            plan,
        };
        resources.apply(&self.disc, &mut session, &active)?;
        if self.randomizer.previous.is_none() {
            self.randomizer.previous = Some(Previous {
                campaign: self.campaign.clone(),
                run: self.run.clone(),
                content: std::mem::replace(&mut self.content, Content::Complete),
                gaisen: self.gaisen.take(),
                loadout: self.loadout,
                origin: self.origin,
                death: self.death.clone(),
                keep_all_active: self.keep_all_active,
                previous_buttons: self.previous_buttons,
                action: self.action,
            });
        }
        self.content = Content::Gameplay(Box::new(session));
        self.campaign = campaign;
        self.run = campaign_run;
        self.gaisen = source;
        self.loadout = None;
        self.origin = super::campaign::Origin::Quick;
        self.death.clear();
        self.action = None;
        self.previous_buttons = 0;
        self.randomizer.active = Some(active);
        self.generation = self.generation.wrapping_add(1);
        Ok(())
    }

    pub fn end_randomizer(&mut self) {
        if let Some(previous) = self.randomizer.previous.take() {
            self.campaign = previous.campaign;
            self.run = previous.run;
            self.content = previous.content;
            self.gaisen = previous.gaisen;
            self.loadout = previous.loadout;
            self.origin = previous.origin;
            self.death = previous.death;
            self.keep_all_active = previous.keep_all_active;
            self.action = previous.action;
            self.previous_buttons = previous.previous_buttons;
            self.randomizer.active = None;
            self.generation = self.generation.wrapping_add(1);
        }
    }

    pub(super) fn randomizer_step(
        &mut self,
        buttons: u16,
    ) -> Result<Option<super::campaign::Update>> {
        use super::campaign::Action;
        if self.randomizer.active.is_some() && matches!(self.action, Some(Action::SelectMission)) {
            self.end_randomizer();
            return Ok(Some(super::campaign::Update {
                stop_music: true,
                ..Default::default()
            }));
        }
        let Some(active) = &self.randomizer.active else {
            return Ok(None);
        };
        if self.death.waiting() {
            return Ok(None);
        }
        let mut action = self.action;
        if let Content::Results(results) = &self.content {
            action = results
                .exit
                .filter(|(_, remaining)| *remaining <= 1)
                .map(|(a, _)| a);
        } else if matches!(self.content, Content::Failed | Content::Complete) {
            let pressed = buttons & !self.previous_buttons;
            if action.is_none() {
                action = if pressed & (0x2000 | 8) != 0 {
                    Some(Action::Retry)
                } else if pressed & 0x4000 != 0 {
                    Some(Action::Quit)
                } else {
                    None
                };
            }
        }
        let run = match action {
            Some(Action::Continue)
                if matches!(self.content, Content::Results(_) | Content::Complete) =>
            {
                Some(active.run.next()?)
            }
            Some(Action::Retry) => Some(active.run.clone()),
            Some(Action::Quit | Action::SelectMission) => {
                self.end_randomizer();
                None
            }
            _ => return Ok(None),
        };
        if let Some(run) = run {
            self.start_randomizer(run)?;
        }
        self.action = None;
        Ok(Some(super::campaign::Update {
            stop_music: true,
            ..Default::default()
        }))
    }

    pub fn prepare_randomizer(&mut self, gaisen: Option<&Path>) -> Result<()> {
        if self.randomizer.resources.is_none() {
            self.randomizer.resources = Some(Resources::load(&self.disc, gaisen)?);
        }
        Ok(())
    }

    pub fn randomizer_catalog(&self) -> Option<&Catalog> {
        self.randomizer.resources.as_ref().map(|r| &r.catalog)
    }

    pub fn randomized(&self) -> Option<(&Run, &Plan)> {
        self.randomizer.active.as_ref().map(|a| (&a.run, &a.plan))
    }

    pub fn randomizer_models(&mut self, settings: &Settings, bodies: &Arc<Pack>) -> Result<bool> {
        let Some(active) = &self.randomizer.active else {
            return Ok(false);
        };
        let settings = active.settings(settings);
        let resources = self
            .randomizer
            .resources
            .as_mut()
            .ok_or("Load the native randomizer catalog first")?;
        match &mut self.content {
            Content::Gameplay(session) => {
                let changed = session.models.sync_optional(
                    &mut session.world,
                    &mut session.assets,
                    resources.donor.as_mut(),
                    &resources.library,
                    bodies,
                    &settings,
                )?;
                if changed {
                    session.reset_interpolation()?;
                }
                Ok(changed)
            }
            Content::Story { story, .. } => story.models.sync(
                &story.resources,
                self.run.hero,
                &resources.library,
                bodies,
                &settings,
            ),
            _ => Ok(false),
        }
    }
}

#[cfg(test)]
mod tests;
