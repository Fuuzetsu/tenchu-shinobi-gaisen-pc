use super::*;
use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{model::Part, stage::MissionAssets, texture::Vram};

#[derive(Default)]
pub(super) struct SceneryLibrary {
    pub(super) sources: BTreeMap<(AssetGame, usize), Source>,
}

pub(super) struct Source {
    model_prefix: String,
    pub(super) parts: Vec<Part>,
    pub(super) names: Vec<String>,
    vram: Arc<Vram>,
}

impl SceneryLibrary {
    pub(super) fn load(
        &mut self,
        assets: &GameAssets<DiscSource>,
        game: AssetGame,
        stage: usize,
    ) -> Result<()> {
        if self.sources.contains_key(&(game, stage)) {
            return Ok(());
        }
        let source = match game {
            AssetGame::Tenchu => {
                let mission = MissionAssets::load(assets, stage, 0, 1)?;
                Source {
                    model_prefix: format!("stages/{}/objects", mission.stage.directory),
                    parts: Arc::unwrap_or_clone(mission.objects),
                    names: mission.object_names,
                    vram: Arc::new(mission.vram),
                }
            }
            AssetGame::Tenchu2 => {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let disc = crate::source::open_disc(
                        &crate::tenchu2::menu::disc_path()
                            .ok_or("Connect your Tenchu 2 disc from the game launcher")?,
                    )?;
                    let mut level = tenchu_assets::tenchu2::level::Level::load(&disc, stage)?;
                    level.shading.apply(level.objects.iter_mut());
                    Source {
                        model_prefix: format!("tenchu2/map{}/props", level.id),
                        names: (0..level.objects.len())
                            .map(|i| format!("Tenchu 2 · Area {stage:02} · Object {i}"))
                            .collect(),
                        parts: level.objects,
                        vram: Arc::new(level.vram),
                    }
                }
                #[cfg(target_arch = "wasm32")]
                return Err(
                    "Connect Tenchu 2 assets in the desktop game to browse this palette".into(),
                );
            }
        };
        self.sources.insert((game, stage), source);
        Ok(())
    }
}

impl Editor {
    pub(crate) fn add_scenery(
        &mut self,
        mission: &mut MissionAssets,
        assets: &GameAssets<DiscSource>,
    ) -> Result<Vec<crate::visuals::models::SceneryMaterial>> {
        let mut materials = vec![];
        let mut custom_parts = BTreeMap::new();
        let mut custom_vram = None;
        for (placed, prop) in self.project.scenery.iter().enumerate() {
            if let Some(model) = &prop.custom {
                let index = if let Some(&index) = custom_parts.get(model) {
                    index
                } else {
                    let id = format!("studio/custom/{placed}");
                    let (part, pack) = model.build(&self.project.assets, &id)?;
                    let index = mission.objects.len();
                    Arc::make_mut(&mut mission.objects).push(part);
                    mission.object_names.push(prop.name.clone());
                    let vram = custom_vram.get_or_insert_with(|| {
                        mission
                            .world_vram
                            .clone()
                            .unwrap_or_else(|| Arc::new(mission.vram.clone()))
                    });
                    materials.push(crate::visuals::models::SceneryMaterial {
                        custom: Some(pack),
                        part: index,
                        id,
                        label: prop.name.clone(),
                        vram: vram.clone(),
                    });
                    custom_parts.insert(model.clone(), index);
                    index
                };
                mission.placements.push(tenchu_assets::stage::Placement {
                    part: index,
                    cloned: true,
                    position: prop.position,
                    facing: i32::from(prop.yaw),
                });
                continue;
            }
            self.scenery_library.load(assets, prop.game, prop.stage)?;
            let source = &self.scenery_library.sources[&(prop.game, prop.stage)];
            let mut part = source
                .parts
                .get(prop.part)
                .ok_or("Scenery asset no longer exists")?
                .clone();
            if part.geometry.is_empty() {
                return Err("This scenery asset has no geometry".into());
            }
            part.parent = -1;
            part.child = -1;
            part.position = [0; 3];
            let index = mission.objects.len();
            Arc::make_mut(&mut mission.objects).push(part);
            mission.object_names.push(prop.name.clone());
            mission.placements.push(tenchu_assets::stage::Placement {
                part: index,
                cloned: true,
                position: prop.position,
                facing: i32::from(prop.yaw),
            });
            materials.push(crate::visuals::models::SceneryMaterial {
                custom: None,
                part: index,
                id: format!("{}/{}", source.model_prefix, prop.part),
                label: source.names[prop.part].clone(),
                vram: source.vram.clone(),
            });
        }
        Ok(materials)
    }

    pub(super) fn scenery(&mut self, ui: &mut egui::Ui, assets: &GameAssets<DiscSource>) {
        if self.project.campaign.is_some() {
            self.campaign_scenery(ui);
        }
        let cursor = self.world_cursor();
        ui.heading("Scenery from your discs");
        ui.collapsing("Browse disc assets", |ui| {
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.scenery_game, AssetGame::Tenchu, "Tenchu");
            ui.selectable_value(&mut self.scenery_game, AssetGame::Tenchu2, "Tenchu 2");
        });
        ui.add(
            egui::DragValue::new(&mut self.scenery_stage)
                .range(
                    0..=if self.scenery_game == AssetGame::Tenchu {
                        18
                    } else {
                        23
                    },
                )
                .prefix("Source area "),
        );
        if ui.button("Browse area assets").clicked()
            && let Err(error) =
                self.scenery_library
                    .load(assets, self.scenery_game, self.scenery_stage)
        {
            self.notice = error;
        }
        ui.small("Decorative scenery casts shadows and receives lighting. Build solid ground, walls and traps with the Terrain palette.");
        ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search scenery…"));
        if let Some(source) = self
            .scenery_library
            .sources
            .get(&(self.scenery_game, self.scenery_stage))
        {
            egui::ScrollArea::vertical()
                .id_salt("studio-scenery-palette")
                .max_height(200.0)
                .show(ui, |ui| {
                    for (i, name) in source.names.iter().enumerate().filter(|(i, name)| {
                        !source.parts[*i].geometry.is_empty()
                            && name.to_lowercase().contains(&self.search.to_lowercase())
                    }) {
                        if ui
                            .add_enabled(
                                self.project.scenery.len() < self.project.limits.scenery,
                                egui::Button::new(format!("+ {name}")),
                            )
                            .clicked()
                        {
                            let position = cursor;
                            self.campaign_prop = None;
                            self.scenery_selection = Some(self.project.scenery.len());
                            self.project.scenery.push(project::Scenery {
            custom: None,
                                name: name.clone(),
                                game: self.scenery_game,
                                stage: self.scenery_stage,
                                part: i,
                                position,
                                yaw: 0,
                            });
                            self.tool = Tool::Select;
                        }
                    }
                });
        }
        });
        ui.separator();
        ui.heading("Placed scenery");
        for (i, prop) in self.project.scenery.iter().enumerate() {
            if ui
                .selectable_label(self.scenery_selection == Some(i), &prop.name)
                .clicked()
            {
                self.scenery_selection = Some(i);
                self.campaign_prop = None;
                self.tool = Tool::Select;
            }
        }
        let Some(index) = self
            .scenery_selection
            .filter(|&i| i < self.project.scenery.len())
        else {
            return;
        };
        ui.horizontal_wrapped(|ui| {
            ui.label("Snap");
            for (step, label) in [
                (0, "Free"),
                (100, "0.1 m"),
                (500, "0.5 m"),
                (1000, "1 m"),
                (2000, "2 m"),
            ] {
                ui.selectable_value(&mut self.snap, step, label);
            }
        });
        ui.small("Arrows: nudge X/Z · Page Up/Down: height · End: ground on terrain");
        if ui.button("Ground on terrain  End").clicked() {
            self.ground_scenery();
        }
        let prop = &mut self.project.scenery[index];
        ui.separator();
        ui.text_edit_singleline(&mut prop.name);
        if let Some(model) = &mut prop.custom {
            custom::ui::model(ui, model, &self.project.assets);
        } else {
            ui.label(format!(
                "{} · source area {} · object {}",
                if prop.game == AssetGame::Tenchu {
                    "Tenchu"
                } else {
                    "Tenchu 2"
                },
                prop.stage,
                prop.part
            ));
        }
        ui.small(
            "Click or drag in 3D to place precisely; map view uses cell centres. Coordinates use 1000 units per metre.",
        );
        for (value, axis) in prop.position.iter_mut().zip(["X ", "Y ", "Z "]) {
            ui.add(
                egui::DragValue::new(value)
                    .speed(100)
                    .range(-320_000..=320_000)
                    .prefix(axis),
            );
        }
        ui.add(
            egui::DragValue::new(&mut prop.yaw)
                .range(0..=4095)
                .prefix("Yaw "),
        );
        ui.horizontal(|ui| {
            if ui.button("Rotate 90°").clicked() {
                self.project.scenery[index].yaw = (self.project.scenery[index].yaw + 1024) % 4096;
            }
            if ui
                .add_enabled(
                    self.project.scenery.len() < self.project.limits.scenery,
                    egui::Button::new("Duplicate"),
                )
                .clicked()
            {
                self.project
                    .scenery
                    .push(self.project.scenery[index].clone());
                self.scenery_selection = Some(self.project.scenery.len() - 1);
            }
            if ui.button("Delete").clicked() {
                self.project.scenery.remove(index);
                self.scenery_selection = None;
            }
        });
    }
}
