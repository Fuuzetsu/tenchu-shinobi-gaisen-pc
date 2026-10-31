mod grid;
mod library;
mod menu;

use crate::{source::DiscSource, storage};
use std::path::PathBuf;
use tenchu_assets::{
    Result,
    game::GameAssets,
    trial::{Mission, Objective, Point, Tables, Theme},
};
use tenchu_core::game::{Game, trial::Phase};

#[derive(Clone, Copy, PartialEq, Eq)]
enum Origin {
    Preset(usize),
    Saved(usize),
    Draft,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tab {
    Missions,
    Terrain,
    Actors,
    Patrols,
    Settings,
    Share,
}
impl Tab {
    const ALL: [Self; 6] = [
        Self::Missions,
        Self::Terrain,
        Self::Actors,
        Self::Patrols,
        Self::Settings,
        Self::Share,
    ];
    fn label(self) -> &'static str {
        match self {
            Self::Missions => "Missions",
            Self::Terrain => "Terrain",
            Self::Actors => "Characters",
            Self::Patrols => "Patrols",
            Self::Settings => "Settings",
            Self::Share => "Share",
        }
    }
}

pub enum Event {
    Play { preview: bool },
    Return,
    Close,
}

pub struct Editor {
    pub open: bool,
    pub notice: String,
    pub character: i16,
    pub preview: bool,
    pub eligible: bool,
    pub tables: Option<Tables>,
    pub theme: Option<Theme>,
    pub mission: Option<Mission>,
    library: library::Library,
    path: PathBuf,
    presets: Vec<Mission>,
    origin: Origin,
    scroll_selection: bool,
    tab: Tab,
    cursor: Point,
    brush: u8,
    rotation: u8,
    ornament: bool,
    actor: usize,
    actor_kind: i16,
    patrol_wait: i16,
    patrol_facing: i16,
    undo: Vec<Mission>,
    stroke: Option<Mission>,
    textures: Vec<Option<egui::TextureHandle>>,
    share: String,
    password: String,
    unlocked: bool,
    record_name: String,
    new_theme: usize,
    new_objective: Objective,
    pending: Option<Event>,
}
impl Editor {
    pub fn new(path: PathBuf) -> Self {
        Self {
            path,
            open: false,
            notice: String::new(),
            character: 0,
            preview: false,
            eligible: false,
            tables: None,
            theme: None,
            mission: None,
            library: Default::default(),
            presets: vec![],
            origin: Origin::Draft,
            scroll_selection: false,
            tab: Tab::Missions,
            cursor: Point { x: 16, z: 16 },
            brush: 0,
            rotation: 0,
            ornament: false,
            actor: 0,
            actor_kind: 0,
            patrol_wait: 0,
            patrol_facing: 0,
            undo: vec![],
            stroke: None,
            textures: vec![],
            share: String::new(),
            password: String::new(),
            unlocked: true,
            record_name: "Player".into(),
            new_theme: 0,
            new_objective: Objective::ReachLord,
            pending: None,
        }
    }
    pub fn enter(&mut self, assets: &GameAssets<DiscSource>) -> Result<()> {
        if self.tables.is_none() {
            let tables = Tables::load(assets)?;
            self.presets = tables.presets(assets)?;
            self.tables = Some(tables);
            match library::Library::load(&self.path) {
                Ok(saved) => self.library = saved,
                Err(e) => self.notice = e,
            }
            for (mission, records) in self.presets.iter_mut().zip(&self.library.records) {
                mission.records = records
                    .iter()
                    .map(|(ticks, name)| tenchu_assets::trial::Record {
                        ticks: *ticks,
                        name: name.clone(),
                    })
                    .collect();
            }
            let draft = self
                .library
                .draft
                .as_deref()
                .map(Mission::decode)
                .transpose()?;
            let (mission, origin) = draft.map_or_else(
                || (self.presets[0].clone(), Origin::Preset(0)),
                |m| (m, Origin::Draft),
            );
            self.select(mission, origin, assets)?;
            self.record_name.clone_from(&self.library.record_name);
        }
        self.open = true;
        Ok(())
    }
    fn select(
        &mut self,
        mission: Mission,
        origin: Origin,
        assets: &GameAssets<DiscSource>,
    ) -> Result<()> {
        let theme = Theme::load(assets, mission.theme)?;
        theme.validate(&mission)?;
        self.actor_kind = self.tables.as_ref().unwrap().appearance[mission.theme][2];
        self.brush = tenchu_assets::trial::DEFAULT_TILES[mission.theme];
        self.new_theme = mission.theme;
        self.new_objective = mission.objective;
        self.unlocked = mission.password.is_empty();
        self.password.clear();
        self.cursor = mission.placements[0].point;
        self.mission = Some(mission);
        self.origin = origin;
        self.theme = Some(theme);
        self.textures.clear();
        self.undo.clear();
        self.stroke = None;
        self.actor = 0;
        self.ornament = false;
        self.share.clear();
        Ok(())
    }
    fn edit(&mut self, before: Mission) {
        if self.mission.as_ref() == Some(&before) {
            return;
        }
        if self.undo.len() == 64 {
            self.undo.remove(0);
        }
        self.undo.push(before);
        self.mission.as_mut().unwrap().records.clear();
        if matches!(self.origin, Origin::Preset(_)) {
            self.origin = Origin::Draft;
        }
    }
    fn undo(&mut self) {
        if let Some(mission) = self.undo.pop() {
            self.mission = Some(mission);
            self.actor = self
                .actor
                .min(self.mission.as_ref().unwrap().placements.len() - 1);
        }
    }
    pub fn persist(&mut self) -> Result<()> {
        if let Some(mission) = &self.mission {
            self.library.draft = Some(mission.encode()?);
        }
        self.library.record_name.clone_from(&self.record_name);
        self.library.write(&self.path)
    }
    pub fn finished(&mut self, game: &Game) {
        self.open = true;
        let state = game.trial.as_ref().unwrap();
        let time = library::time(state.elapsed);
        self.notice = match state.phase {
            Phase::Success(_) => format!("Mission complete — {time}"),
            Phase::Failure(reason, _) => format!("{} — {time}", reason.label()),
            Phase::Playing => "Returned to editor; your map is unchanged".into(),
        };
        if matches!(state.phase, Phase::Success(_))
            && game.events.outcome == tenchu_core::game::events::Outcome::Complete
            && self.eligible
            && !self.preview
        {
            let mission = self.mission.as_mut().unwrap();
            if let Some(rank) = mission.record(state.elapsed, self.record_name.clone()) {
                self.notice.push_str(&format!(" · Record #{}", rank + 1));
                match self.origin {
                    Origin::Preset(index) => {
                        self.presets[index].records.clone_from(&mission.records);
                        self.library.records.resize(16, vec![]);
                        self.library.records[index] = mission
                            .records
                            .iter()
                            .map(|r| (r.ticks, r.name.clone()))
                            .collect();
                    }
                    Origin::Saved(index) => {
                        if let Ok(bytes) = mission.encode() {
                            self.library.missions[index] = bytes;
                        }
                    }
                    Origin::Draft => (),
                }
            }
        } else if matches!(state.phase, Phase::Success(_)) {
            self.notice.push_str(" · Practice: no time record");
        }
        if let Err(e) = self.persist() {
            self.notice.push_str(&format!(" · {e}"));
        }
    }
    fn name(&self, kind: i16) -> String {
        if kind == 0 {
            return "Player start".into();
        }
        if kind == 5 {
            return "Princess Kiku".into();
        }
        if kind == 6 {
            return "Lord Gohda".into();
        }
        if kind == 170 {
            return "Medicinal herb".into();
        }
        self.tables
            .as_ref()
            .unwrap()
            .characters
            .iter()
            .find(|c| c.kind == kind)
            .map_or_else(
                || format!("Character {kind}"),
                |c| format!("{} ({kind})", c.model),
            )
    }
    pub fn controller(
        &mut self,
        action: crate::campaign::Action,
        assets: &GameAssets<DiscSource>,
    ) -> Option<Event> {
        use crate::campaign::Action;
        match action {
            Action::Back => return Some(Event::Close),
            Action::Start => return Some(Event::Play { preview: false }),
            Action::Replay => self.undo(),
            Action::Up | Action::Down if self.tab == Tab::Missions => {
                self.next_mission(if matches!(action, Action::Up) { -1 } else { 1 }, assets);
            }
            Action::Up => self.cursor.z = (self.cursor.z - 1).max(0),
            Action::Down => self.cursor.z = (self.cursor.z + 1).min(31),
            Action::Left => self.cursor.x = (self.cursor.x - 1).max(0),
            Action::Right => self.cursor.x = (self.cursor.x + 1).min(31),
            Action::Confirm if self.tab == Tab::Missions => {
                return Some(Event::Play { preview: false });
            }
            Action::Confirm if self.unlocked => {
                self.use_cell(false);
                self.finish_stroke();
            }
            _ => (),
        }
        None
    }
    fn next_mission(&mut self, direction: i32, assets: &GameAssets<DiscSource>) {
        let current = match self.origin {
            Origin::Preset(index) => index,
            Origin::Saved(index) => self.presets.len() + index,
            Origin::Draft => 0,
        };
        let index = (current as i32 + direction)
            .rem_euclid((self.presets.len() + self.library.missions.len()) as i32)
            as usize;
        let (mission, origin) = if index < self.presets.len() {
            (Ok(self.presets[index].clone()), Origin::Preset(index))
        } else {
            let index = index - self.presets.len();
            (
                Mission::decode(&self.library.missions[index]),
                Origin::Saved(index),
            )
        };
        if let Err(e) = mission.and_then(|m| self.select(m, origin, assets)) {
            self.notice = e;
        }
        self.scroll_selection = true;
    }
    pub fn next_tab(&mut self, direction: i32) {
        let at = Tab::ALL.iter().position(|&t| t == self.tab).unwrap() as i32;
        self.tab = Tab::ALL[(at + direction).rem_euclid(6) as usize];
    }
    fn finish_stroke(&mut self) {
        if let Some(before) = self.stroke.take() {
            self.edit(before);
        }
    }
}

pub fn status(ctx: &egui::Context, game: &Game, preview: bool) {
    let setup = game.mission.trial.as_ref().unwrap();
    let state = game.trial.as_ref().unwrap();
    egui::Area::new("trial-status".into()).anchor(egui::Align2::CENTER_TOP, [0.0, 8.0]).interactable(false).show(ctx, |ui| {
        egui::Frame::new().fill(egui::Color32::from_black_alpha(170)).inner_margin(5.0).show(ui, |ui| {
            ui.label(format!("{}{} · Esc: editor · F5: retry", setup.mission.objective.label(), if setup.mission.stealth { " · Never be spotted" } else { "" }));
            if preview { ui.label("Preview · WASD / QE fly · hold right mouse to look · F2 returns to player"); }
            match state.phase {
                Phase::Success(_) => { ui.colored_label(egui::Color32::GOLD, "Mission complete"); }
                Phase::Failure(reason, _) => { ui.colored_label(egui::Color32::LIGHT_RED, reason.label()); }
                Phase::Playing => (),
            }
            if let Some((text, _)) = state.notice { ui.label(text); }
        });
    });
}
