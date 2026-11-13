use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Limits {
    pub characters: usize,
    pub props: usize,
    pub pickups: usize,
    pub terrain: usize,
    pub scenery: usize,
    pub events: usize,
    pub cues: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            characters: 64,
            props: 8192,
            pickups: 256,
            terrain: 8192,
            scenery: 512,
            events: 256,
            cues: 128,
        }
    }
}

impl Limits {
    pub const MAX: Self = Self {
        characters: 256,
        props: 16384,
        pickups: 4096,
        terrain: 32768,
        scenery: 8192,
        events: 4096,
        cues: 1024,
    };

    pub fn validate(self, project: &Project) -> Result<()> {
        let campaign = project.campaign.as_ref();
        for (name, used, limit, maximum) in [
            (
                "Campaign characters",
                campaign.map_or(0, |c| c.actors.len()),
                self.characters,
                Self::MAX.characters,
            ),
            (
                "Source props",
                campaign.map_or(0, |c| c.props.len()),
                self.props,
                Self::MAX.props,
            ),
            (
                "Pickups",
                campaign.map_or(0, |c| c.pickups.len()),
                self.pickups,
                Self::MAX.pickups,
            ),
            (
                "Terrain edits",
                campaign.map_or(0, |c| c.terrain.len()),
                self.terrain,
                Self::MAX.terrain,
            ),
            (
                "Scenery",
                project.scenery.len(),
                self.scenery,
                Self::MAX.scenery,
            ),
            (
                "Events",
                project.events.len(),
                self.events,
                Self::MAX.events,
            ),
            (
                "Cues per event",
                project
                    .events
                    .iter()
                    .map(|e| e.cues.len())
                    .max()
                    .unwrap_or(0),
                self.cues,
                Self::MAX.cues,
            ),
            (
                "Ornaments",
                if campaign.is_none() {
                    project
                        .mission
                        .map
                        .iter()
                        .filter(|c| c.ornament != 255)
                        .count()
                } else {
                    0
                },
                project.mission.limits.ornaments,
                1024,
            ),
            (
                "Patrol points",
                campaign.map_or(0, |c| {
                    c.actors
                        .iter()
                        .map(|a| a.spawn.patrol.len())
                        .max()
                        .unwrap_or(0)
                }),
                project.mission.limits.patrol,
                tenchu_assets::trial::Limits::MAX.patrol,
            ),
        ] {
            if used > limit || limit > maximum {
                return Err(format!(
                    "{name}: {used} used / {limit} allowed (maximum {maximum}). Adjust project Limits."
                ));
            }
        }
        Ok(())
    }
}

impl Editor {
    pub(super) fn limits_summary(&mut self, ui: &mut egui::Ui) {
        let (characters, limit, objects, object_limit, label) =
            if let Some(c) = &self.project.campaign {
                (
                    c.actors.len(),
                    self.project.limits.characters,
                    c.props.len(),
                    self.project.limits.props,
                    "source props",
                )
            } else {
                (
                    self.project.mission.placements.len(),
                    self.project.mission.limits.characters,
                    self.project
                        .mission
                        .map
                        .iter()
                        .filter(|c| c.ornament != 255)
                        .count(),
                    self.project.mission.limits.ornaments,
                    "ornaments",
                )
            };
        if ui.small_button(format!("Limits · characters {characters}/{limit} · {label} {objects}/{object_limit} · scenery {}/{}", self.project.scenery.len(), self.project.limits.scenery))
            .on_hover_text("Inspect counts and raise this project's authoring limits").clicked() {
            self.limits_open = true;
        }
    }

    pub(super) fn limits_window(&mut self, ctx: &egui::Context) {
        let mut open = self.limits_open;
        egui::Window::new("Project limits").open(&mut open).default_width(540.0).resizable(true).show(ctx, |ui| {
            ui.label("Limits belong to this map and travel with its .tenchu file. Raise them as needed; larger scenes take more memory and rendering time.");
            ui.add_enabled_ui(self.unlocked, |ui| {
                self.history.begin(&self.project);
                ui.horizontal(|ui| {
                    if ui.button("Default limits").clicked() {
                        let mut next = self.project.clone();
                        next.limits = Limits::default();
                        next.mission.limits = Default::default();
                        match next.validate() { Ok(()) => self.project = next, Err(_) => self.notice = "Remove excess objects before restoring default limits; nothing was deleted.".into() }
                    }
                    if ui.button("Extended limits").clicked() {
                        self.project.limits = Limits::MAX;
                        self.project.mission.limits = tenchu_assets::trial::Limits::MAX;
                    }
                });
                ui.separator();
                egui::Grid::new("studio-limits").num_columns(3).spacing([24.0, 8.0]).show(ui, |ui| {
                    ui.strong("Category"); ui.strong("Used"); ui.strong("Limit"); ui.end_row();
                    let project = &mut self.project;
                    let limits = &mut project.limits;
                    let trial = &mut project.mission.limits;
                    if let Some(c) = &project.campaign {
                        row(ui, "Characters (plus player)", c.actors.len(), &mut limits.characters, 0, Limits::MAX.characters);
                        row(ui, "Source props", c.props.len(), &mut limits.props, 0, Limits::MAX.props);
                        row(ui, "Pickups", c.pickups.len(), &mut limits.pickups, 0, Limits::MAX.pickups);
                        row(ui, "Edited surfaces", c.terrain.len(), &mut limits.terrain, 0, Limits::MAX.terrain);
                    } else {
                        row(ui, "Characters incl. player", project.mission.placements.len(), &mut trial.characters, 3, tenchu_assets::trial::Limits::MAX.characters);
                        row(ui, "Terrain ornaments", project.mission.map.iter().filter(|c| c.ornament != 255).count(), &mut trial.ornaments, 0, 1024);
                    }
                    let patrol = project.campaign.as_ref().map_or_else(
                        || project.mission.placements.iter().map(|p| p.route.len()).max().unwrap_or(0),
                        |c| c.actors.iter().map(|a| a.spawn.patrol.len()).max().unwrap_or(0));
                    row(ui, "Longest patrol", patrol, &mut trial.patrol, 2, tenchu_assets::trial::Limits::MAX.patrol);
                    row(ui, "Added scenery", project.scenery.len(), &mut limits.scenery, 0, Limits::MAX.scenery);
                    row(ui, "Events", project.events.len(), &mut limits.events, 0, Limits::MAX.events);
                    row(ui, "Largest cue queue", project.events.iter().map(|e| e.cues.len()).max().unwrap_or(0), &mut limits.cues, 0, Limits::MAX.cues);
                });
                self.commit();
            });
            ui.separator();
            let bytes: usize = self.project.assets.values().map(|a| a.bytes()).sum();
            ui.small(format!("Bundled assets: {}/128 · {:.1}/64 MiB. Individual files: 32 MiB; decoded data: 192 MiB; textures: 2048²; models: 100,000 triangles each, 500,000 across distinct placed variants.", self.project.assets.len(), bytes as f64 / 1048576.0));
            ui.small(format!("Saved views: {}/16 · Stamps: {}/32 · Terrain grid: 32 × 32. These storage and format bounds are fixed.", self.project.views.len(), self.project.stamps.len()));
            ui.small("Audio: 8 distinct disc music tracks and 16 simultaneous custom effects. Project library: 128 maps. Undo history: 128 edits.");
            ui.small("Retail .msn export stays within 10 characters, 64 ornaments and 64 patrol points. Extended maps use .tenchu export. Original gameplay activation rules still apply during playtests.");
        });
        self.limits_open = open;
    }
}

fn row(
    ui: &mut egui::Ui,
    label: &str,
    used: usize,
    limit: &mut usize,
    minimum: usize,
    maximum: usize,
) {
    ui.label(label);
    ui.label(used.to_string());
    ui.add(
        egui::DragValue::new(limit)
            .range(used.max(minimum)..=maximum)
            .speed(1),
    );
    ui.end_row();
}
