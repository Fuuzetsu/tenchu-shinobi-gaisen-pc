use crate::settings::{Graphics, WeatherMode};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use tenchu_assets::{Result, trial::Mission};

pub const FORMAT: &str = "tenchu-level-studio";
pub const MAX_BYTES: usize = 8 * 1024 * 1024;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Project {
    pub format: String,
    pub version: u32,
    #[serde(default)]
    pub build: super::provenance::Provenance,
    pub mission: Mission,
    pub character: i16,
    pub graphics: Graphics,
    pub actors: BTreeMap<usize, String>,
    pub scenery: Vec<Scenery>,
    pub events: Vec<Event>,
    #[serde(default)]
    pub stamps: Vec<super::building::Stamp>,
    #[serde(default)]
    pub views: Vec<super::building::View>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum Game {
    #[default]
    Tenchu,
    Tenchu2,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Scenery {
    pub name: String,
    pub game: Game,
    pub stage: usize,
    pub part: usize,
    pub position: [i32; 3],
    pub yaw: i16,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Event {
    pub name: String,
    pub enabled: bool,
    pub once: bool,
    pub trigger: Trigger,
    pub cues: Vec<Cue>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Trigger {
    Start,
    AfterTicks(u32),
    Enter { min: [i32; 3], max: [i32; 3] },
    Defeated(usize),
    Flag { name: String, value: bool },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Cue {
    pub delay: u32,
    pub action: Action,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    Message {
        text: String,
        ticks: u32,
    },
    Sound {
        game: Game,
        bank: usize,
        id: u16,
        volume: u8,
        pan: u8,
    },
    Music {
        game: Game,
        track: usize,
    },
    StopMusic,
    Weather(WeatherMode),
    TimeOfDay(u16),
    Flag {
        name: String,
        value: bool,
    },
    Complete,
    Fail,
}

impl Default for Event {
    fn default() -> Self {
        Self {
            name: "New event".into(),
            enabled: true,
            once: true,
            trigger: Trigger::Start,
            cues: vec![Cue {
                delay: 0,
                action: Action::Message {
                    text: "Your mission starts here.".into(),
                    ticks: 120,
                },
            }],
        }
    }
}

impl Project {
    pub fn new(mission: Mission) -> Self {
        Self {
            format: FORMAT.into(),
            version: 1,
            build: super::provenance::Provenance::new(),
            mission,
            character: 0,
            graphics: Graphics::default(),
            actors: BTreeMap::new(),
            scenery: vec![],
            events: vec![],
            stamps: vec![],
            views: vec![],
        }
    }

    pub fn validate(&self) -> Result<()> {
        if self.format != FORMAT || self.version != 1 {
            return Err("Unsupported Level Studio project format or version".into());
        }
        self.build.validate()?;
        self.mission.validate()?;
        if !(0..=1).contains(&self.character)
            || self.mission.title.len() > 512
            || self.mission.author.len() > 512
            || self.mission.password.len() > 512
            || self
                .mission
                .records
                .iter()
                .any(|record| record.name.len() > 512)
            || self.actors.len() > 10
            || self.scenery.len() > 512
            || self.events.len() > 256
            || self.stamps.len() > 32
            || self.views.len() > 16
        {
            return Err("Project exceeds the supported authoring limits".into());
        }
        for stamp in &self.stamps {
            stamp.validate()?;
        }
        for view in &self.views {
            view.validate()?;
        }
        for (&slot, model) in &self.actors {
            if slot >= self.mission.placements.len() || model.is_empty() || model.len() > 256 {
                return Err("Invalid actor asset reference".into());
            }
            if matches!(self.mission.placements[slot].kind, 5 | 6 | 170) {
                return Err("Objective characters must keep their original appearance".into());
            }
        }
        for prop in &self.scenery {
            if prop.name.len() > 512
                || prop.part > 65535
                || prop.stage >= if prop.game == Game::Tenchu { 19 } else { 24 }
                || prop
                    .position
                    .iter()
                    .any(|v| !(-320_000..=320_000).contains(v))
                || !(0..4096).contains(&prop.yaw)
            {
                return Err("Invalid scenery placement".into());
            }
        }
        for event in &self.events {
            if event.name.len() > 512 || event.cues.len() > 128 {
                return Err("Event name or cue queue is too large".into());
            }
            match &event.trigger {
                Trigger::AfterTicks(ticks) if *ticks > 108_000 => {
                    return Err("Event time exceeds one hour".into());
                }
                Trigger::Enter { min, max }
                    if (0..3).any(|i| min[i] > max[i] || min[i] < -320_000 || max[i] > 320_000) =>
                {
                    return Err("Invalid event region".into());
                }
                Trigger::Defeated(slot) if *slot == 0 || *slot >= self.mission.placements.len() => {
                    return Err("Event refers to a missing character".into());
                }
                Trigger::Flag { name, .. } => flag_name(name)?,
                _ => (),
            }
            let mut duration = 0u32;
            for cue in &event.cues {
                duration = duration
                    .checked_add(cue.delay)
                    .filter(|n| *n <= 108_000)
                    .ok_or("Event queue exceeds one hour")?;
                match &cue.action {
                    Action::Message { text, ticks }
                        if text.len() > 4096 || !(1..=108_000).contains(ticks) =>
                    {
                        return Err("Invalid message or display time".into());
                    }
                    Action::Sound {
                        game,
                        bank,
                        id,
                        volume,
                        pan,
                    } if *id > 2047
                        || *volume > 127
                        || *pan > 127
                        || *bank >= if *game == Game::Tenchu { 19 } else { 24 } =>
                    {
                        return Err("Invalid sound cue".into());
                    }
                    Action::Music { game, track }
                        if *track >= if *game == Game::Tenchu { 19 } else { 8 } =>
                    {
                        return Err("Invalid music cue".into());
                    }
                    Action::TimeOfDay(time) if *time >= 1440 => {
                        return Err("Time must be within a day".into());
                    }
                    Action::Flag { name, .. } => flag_name(name)?,
                    _ => (),
                }
            }
        }
        let tracks: std::collections::BTreeSet<_> = self
            .events
            .iter()
            .filter(|event| event.enabled)
            .flat_map(|event| &event.cues)
            .filter_map(|cue| {
                if let Action::Music { game, track } = cue.action {
                    Some((game, track))
                } else {
                    None
                }
            })
            .collect();
        if tracks.len() > 8 {
            return Err("A project supports up to eight distinct music tracks".into());
        }
        Ok(())
    }

    pub fn decode(bytes: &[u8]) -> Result<Self> {
        if bytes.len() > MAX_BYTES {
            return Err("Project file is too large".into());
        }
        let mut project: Self =
            serde_json::from_slice(bytes).map_err(|e| format!("Read project: {e}"))?;
        project.validate()?;
        project.graphics.validate();
        Ok(project)
    }

    pub fn mark_saved(&mut self) {
        self.build.last_saved_with = Some(super::provenance::Build::current());
    }

    pub fn encode(&self) -> Result<Vec<u8>> {
        self.validate()?;
        let mut saved = self.clone();
        saved.mark_saved();
        let bytes = serde_json::to_vec_pretty(&saved).map_err(|e| e.to_string())?;
        if bytes.len() > MAX_BYTES {
            return Err("Project file is too large".into());
        }
        Ok(bytes)
    }

    pub fn remove_actor(&mut self, slot: usize) -> Result<()> {
        if slot < self.mission.objective.reserved() || slot >= self.mission.placements.len() {
            return Err("Keep the player and objective characters".into());
        }
        if self
            .events
            .iter()
            .any(|e| e.trigger == Trigger::Defeated(slot))
        {
            return Err("Remove this character's event triggers before deleting it".into());
        }
        self.mission.placements.remove(slot);
        self.actors = std::mem::take(&mut self.actors)
            .into_iter()
            .filter_map(|(i, model)| (i != slot).then_some((i - usize::from(i > slot), model)))
            .collect();
        for event in &mut self.events {
            if let Trigger::Defeated(i) = &mut event.trigger
                && *i > slot
            {
                *i -= 1;
            }
        }
        Ok(())
    }
}

fn flag_name(name: &str) -> Result<()> {
    if name.is_empty() || name.len() > 128 {
        Err("Flags need a name of 1–128 bytes".into())
    } else {
        Ok(())
    }
}
