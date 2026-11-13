mod assets;
mod building;
pub(crate) mod campaign;
pub(crate) mod custom;
mod events;
mod example;
mod history;
mod project;
mod provenance;
mod showcase;
mod store;
#[cfg(test)]
mod tests;
mod ui;
mod viewport;

use crate::{debug::camera::Camera, source::DiscSource};
pub use events::{Observation, Runtime};
use history::History;
#[cfg(test)]
pub use project::Scenery;
pub use project::{Action, Game as AssetGame, Project};
pub(crate) use project::{Cue, Event};
use std::path::PathBuf;
use tenchu_assets::{
    Result,
    game::GameAssets,
    trial::{Mission, Objective, Point, Tables, Theme, Waypoint, editing},
};

pub enum Command {
    Open,
    Close,
    Rebuild,
    Play,
    Return,
    Preview(Action),
    BrowseSounds(AssetGame, usize),
    WeatherPreview(u16),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tool {
    Select,
    Paint,
    Erase,
    Patrol,
    Area,
    Stamp,
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Panel {
    Terrain,
    Actors,
    Scenery,
    Events,
    Environment,
    Mission,
    Library,
    Assets,
}

pub struct Editor {
    pub open: bool,
    pub testing: bool,
    pub project: Project,
    pub camera: Camera,
    pub notice: String,
    pub tables: Option<Tables>,
    pub theme: Option<Theme>,
    pub revision: u64,
    pub preview_revision: u64,
    pub runtime: Runtime,
    pub surfaces: Vec<crate::debug::terrain::Surface>,
    scenery_library: assets::SceneryLibrary,
    scenery_game: AssetGame,
    scenery_stage: usize,
    scenery_selection: Option<usize>,
    path: PathBuf,
    library: store::Library,
    selected_save: Option<usize>,
    presets: Vec<Mission>,
    history: History,
    panel: Panel,
    tool: Tool,
    cursor: Point,
    ground: bool,
    brush: u8,
    rotation: u8,
    brush_size: i16,
    actor: usize,
    actor_kind: i16,
    patrol_wait: i16,
    patrol_facing: i16,
    event: usize,
    search: String,
    share: String,
    password: String,
    unlocked: bool,
    grid: bool,
    guides: bool,
    zoom: f32,
    pan: egui::Vec2,
    last_cell: Option<Point>,
    textures: Vec<Option<egui::TextureHandle>>,
    pending: Option<Command>,
    new_theme: usize,
    new_objective: Objective,
    pub asset_names: Vec<(String, String)>,
    pub sound_catalog: std::collections::BTreeMap<(AssetGame, usize), Vec<(u16, String)>>,
    music_names: Vec<String>,
    selection: Option<(Point, Point)>,
    clipboard: Option<building::Stamp>,
    snap: i32,
    ephemeral: bool,
    default_graphics: crate::settings::Graphics,
    weather_preview_seconds: u16,
    environment_weather: bool,
    custom_kind: custom::Kind,
    campaign_cache: Option<campaign::Cache>,
    campaign_surface: Option<[usize; 2]>,
    campaign_cursor: [i32; 3],
    campaign_sources: Vec<(campaign::Source, String)>,
    campaign_prop: Option<usize>,
    campaign_pickup: Option<usize>,
    campaign_follow_terrain: bool,
    flight_keys: std::collections::HashSet<egui::Key>,
    pub(crate) campaign_slots: Vec<usize>,
}

impl Editor {
    pub fn new(path: PathBuf) -> Self {
        Self {
            open: false,
            testing: false,
            project: Project::new(Mission::new(0, Objective::ReachLord, 140).unwrap()),
            camera: Camera {
                position: glam::vec3(33_000.0, -28_000.0, -6_000.0),
                yaw: std::f32::consts::PI,
                pitch: -0.72,
                fov: 55.0,
            },
            notice: String::new(),
            tables: None,
            theme: None,
            revision: 0,
            preview_revision: u64::MAX,
            runtime: Runtime::default(),
            surfaces: vec![],
            scenery_library: Default::default(),
            scenery_game: AssetGame::Tenchu,
            scenery_stage: 0,
            scenery_selection: None,
            path,
            library: store::Library::default(),
            selected_save: None,
            presets: vec![],
            history: History::default(),
            panel: Panel::Terrain,
            tool: Tool::Paint,
            cursor: Point { x: 16, z: 16 },
            ground: true,
            brush: 0,
            rotation: 0,
            brush_size: 1,
            actor: 0,
            actor_kind: 140,
            patrol_wait: 0,
            patrol_facing: 0,
            event: 0,
            search: String::new(),
            share: String::new(),
            password: String::new(),
            unlocked: true,
            grid: false,
            guides: true,
            zoom: 1.0,
            pan: egui::Vec2::ZERO,
            last_cell: None,
            textures: vec![],
            pending: None,
            new_theme: 0,
            new_objective: Objective::ReachLord,
            asset_names: vec![],
            sound_catalog: Default::default(),
            music_names: vec![],
            selection: None,
            clipboard: None,
            snap: 500,
            ephemeral: false,
            default_graphics: Default::default(),
            weather_preview_seconds: 0,
            environment_weather: false,
            custom_kind: custom::Kind::Model,
            campaign_cache: None,
            campaign_surface: None,
            campaign_cursor: [0; 3],
            campaign_sources: vec![],
            campaign_prop: None,
            campaign_pickup: None,
            campaign_follow_terrain: true,
            flight_keys: Default::default(),
            campaign_slots: vec![],
        }
    }

    pub fn inherit_graphics(&mut self, graphics: &crate::settings::Graphics) {
        if self.ephemeral {
            return;
        }
        self.default_graphics = graphics.clone();
        if self.tables.is_none() {
            self.project.graphics = graphics.clone();
        }
    }

    fn new_project(&self, mission: Mission) -> Project {
        Project {
            graphics: self.default_graphics.clone(),
            ..Project::new(mission)
        }
    }

    pub fn enter(&mut self, assets: &GameAssets<DiscSource>) -> Result<()> {
        if self.tables.is_none() {
            let library = if self.ephemeral {
                store::Library::default()
            } else {
                store::Library::load(&self.path)?
            };
            let tables = Tables::load(assets)?;
            self.presets = tables.presets(assets)?;
            let music = tenchu_assets::audio::catalog::Catalog::load(
                &assets.executable,
                tenchu_assets::script::Language::Japanese,
            )?;
            self.music_names = (0..19)
                .map(|i| {
                    format!(
                        "{i:02} · {}",
                        music.music(i).map_or("Unavailable", |track| track
                            .path
                            .rsplit('/')
                            .next()
                            .unwrap_or(&track.path))
                    )
                })
                .collect();
            let project = library
                .draft
                .clone()
                .unwrap_or_else(|| self.project.clone());
            self.select(project, assets)?;
            self.library = library;
            self.tables = Some(tables);
        }
        Ok(())
    }

    fn select(&mut self, project: Project, assets: &GameAssets<DiscSource>) -> Result<()> {
        project.validate()?;
        let theme = Theme::load(assets, project.mission.theme)?;
        theme.validate(&project.mission)?;
        if let Some(campaign) = &project.campaign {
            self.load_campaign_cache(assets, &campaign.source, project.character)?;
            self.campaign_cursor = campaign.start;
        }
        self.zoom = 1.0;
        self.pan = egui::Vec2::ZERO;
        self.campaign_surface = None;
        self.campaign_prop = None;
        self.campaign_pickup = None;
        self.new_theme = project.mission.theme;
        self.new_objective = project.mission.objective;
        self.brush = tenchu_assets::trial::DEFAULT_TILES[project.mission.theme];
        self.unlocked = project.mission.password.is_empty();
        self.password.clear();
        self.cursor = project.mission.placements[0].point;
        if let Some(view) = project.views.first() {
            self.camera = view.camera();
        } else if let Some(campaign) = &project.campaign {
            self.camera.focus(
                glam::Vec3::from_array(campaign.start.map(|v| v as f32)),
                false,
            );
        } else if project.graphics.fog {
            let point = project.mission.placements[0].point;
            let mut target = point.world();
            target[1] = i32::from(theme.height(&project.mission, point.x, point.z, false)) * 1000;
            self.camera
                .focus(glam::Vec3::from_array(target.map(|v| v as f32)), false);
        }
        self.project = project;
        self.theme = Some(theme);
        self.textures.clear();
        self.history = History::default();
        self.actor = 0;
        self.scenery_selection = None;
        self.event = 0;
        self.selection = None;
        self.selected_save = None;
        self.revision = self.revision.wrapping_add(1);
        Ok(())
    }

    fn commit(&mut self) {
        if self.history.commit(&mut self.project) {
            self.revision = self.revision.wrapping_add(1);
        }
    }

    fn undo(&mut self, redo: bool) {
        let changed = if redo {
            self.history.redo(&mut self.project)
        } else {
            self.history.undo(&mut self.project)
        };
        if changed {
            self.actor = self.actor.min(self.project.actor_count() - 1);
            self.event = self.event.min(self.project.events.len().saturating_sub(1));
            self.revision = self.revision.wrapping_add(1);
        }
    }

    fn use_cell(&mut self, point: Point) {
        self.cursor = point;
        if !self.unlocked {
            return;
        }
        self.history.begin(&self.project);
        if self.tool == Tool::Select && self.panel == Panel::Scenery {
            if let Some(prop) = self
                .scenery_selection
                .and_then(|i| self.project.scenery.get_mut(i))
            {
                prop.position = point.world();
                prop.position[1] = i32::from(self.theme.as_ref().unwrap().height(
                    &self.project.mission,
                    point.x,
                    point.z,
                    false,
                )) * 1000;
            }
            return;
        }
        let mission = &mut self.project.mission;
        let theme = self.theme.as_ref().unwrap();
        let result = match self.tool {
            Tool::Area => {
                self.selection = Some((self.selection.map_or(point, |(a, _)| a), point));
                Ok(())
            }
            Tool::Stamp => self
                .clipboard
                .as_ref()
                .ok_or_else(|| "Copy a terrain area first".into())
                .and_then(|stamp| stamp.paste(mission, theme, point)),
            Tool::Paint | Tool::Erase => {
                let tile = if self.tool == Tool::Erase {
                    255
                } else {
                    self.brush | self.rotation << 6
                };
                let mut error = None;
                let half = (self.brush_size - 1) / 2;
                for z in point.z - half..point.z - half + self.brush_size {
                    for x in point.x - half..point.x - half + self.brush_size {
                        if (Point { x, z }).valid()
                            && let Err(e) =
                                editing::paint(mission, theme, Point { x, z }, !self.ground, tile)
                        {
                            error = Some(e);
                        }
                    }
                }
                error.map_or(Ok(()), Err)
            }
            Tool::Select => {
                if let Some(actor) = mission.placements.iter().position(|p| p.point == point) {
                    self.actor = actor;
                    self.panel = Panel::Actors;
                    Ok(())
                } else {
                    editing::move_actor(mission, theme, self.actor, point)
                }
            }
            Tool::Patrol => editing::patrol(
                mission,
                theme,
                self.actor,
                Waypoint {
                    point,
                    facing: self.patrol_facing,
                    wait: self.patrol_wait,
                },
            ),
        };
        if let Err(error) = result {
            self.notice = error;
        }
    }

    pub fn persist(&mut self) -> Result<()> {
        if self.ephemeral || self.tables.is_none() {
            return Ok(());
        }
        self.commit();
        self.project.validate()?;
        self.project.mark_saved();
        self.library.draft = Some(self.project.clone());
        self.library.write(&self.path)
    }

    pub fn record(&mut self, ticks: i32) -> Option<usize> {
        let rank = self
            .project
            .mission
            .record(ticks, self.library.record_name.clone());
        if let Some(index) = self.selected_save {
            self.library.projects[index] = self.project.clone();
        }
        rank
    }

    fn save(&mut self, copy: bool) -> Result<()> {
        self.commit();
        self.project.validate()?;
        self.project.mark_saved();
        let previous = self.library.clone();
        let selected = self.selected_save;
        if !copy && let Some(index) = selected {
            self.library.projects[index] = self.project.clone();
        } else {
            if self.library.projects.len() >= 128 {
                return Err(
                    "Project library is full; export a project before removing a copy".into(),
                );
            }
            self.selected_save = Some(self.library.projects.len());
            self.library.projects.push(self.project.clone());
        }
        if let Err(error) = self.persist() {
            self.library = previous;
            self.selected_save = selected;
            return Err(error);
        }
        self.notice = "Project saved".into();
        Ok(())
    }
}
