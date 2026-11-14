use super::*;
use crate::level_editor::previews::{Key, Previews};
use crate::level_editor::{Action, AssetGame, Command, Editor, Panel, Tool, project::Scenery};

impl Editor {
    #[cfg(not(target_os = "android"))]
    pub(crate) fn import_custom(&mut self, name: String, kind: Kind, bytes: Vec<u8>) -> Result<()> {
        if !self.open || !self.unlocked {
            return Err("Open and unlock a Studio project first".into());
        }
        let asset = Asset::import(name, kind, bytes)?;
        let mut assets = self.project.assets.clone();
        assets.insert(asset.id().into(), asset);
        validate(&assets)?;
        self.history.begin(&self.project);
        self.project.assets = assets;
        self.commit();
        self.notice = "Imported asset; its contents will be included in the exported map".into();
        self.panel = Panel::Assets;
        Ok(())
    }

    fn import_custom_file(&mut self) -> Result<()> {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                self.custom_kind.label(),
                match self.custom_kind {
                    Kind::Texture => &["png"][..],
                    Kind::Model => &["obj"][..],
                    Kind::Audio => &["wav", "ogg", "mp3", "flac"][..],
                },
            )
            .pick_file()
        {
            if std::fs::metadata(&path).map_err(|e| e.to_string())?.len() > FILE_LIMIT as u64 {
                return Err("Custom asset exceeds 32 MiB".into());
            }
            let name = path
                .file_name()
                .ok_or("Missing filename")?
                .to_string_lossy()
                .into_owned();
            self.import_custom(
                name,
                self.custom_kind,
                std::fs::read(&path).map_err(|e| e.to_string())?,
            )?;
        }
        #[cfg(target_arch = "wasm32")]
        crate::web::recording::start(crate::web::recording::Kind::StudioAsset(self.custom_kind))?;
        #[cfg(target_os = "android")]
        {
            return Err("Import individual assets on desktop or in the browser; Android can open bundled maps".into());
        }
        #[allow(unreachable_code)]
        Ok(())
    }

    pub(in crate::level_editor) fn custom_assets(&mut self, ui: &mut egui::Ui) {
        ui.heading("Custom assets");
        ui.small("Files travel inside your .tenchu map. Import a model and textures, then assign its materials in Scenery. Audio is available in event queues.");
        ui.horizontal_wrapped(|ui| {
            for kind in [Kind::Model, Kind::Texture, Kind::Audio] {
                ui.selectable_value(&mut self.custom_kind, kind, kind.label());
            }
        });
        if ui.button("Import file…").clicked()
            && let Err(error) = self.import_custom_file()
        {
            self.notice = error;
        }
        ui.small("PNG up to 2048² · triangulated OBJ · WAV / Ogg Vorbis / MP3 / FLAC");
        let bytes: usize = self.project.assets.values().map(Asset::bytes).sum();
        ui.label(format!(
            "{} assets · {:.1} / 64 MiB",
            self.project.assets.len(),
            bytes as f64 / 1048576.0
        ));
        let mut place = None;
        let mut remove = None;
        for (id, asset) in &mut self.project.assets {
            ui.push_id(id, |ui| {
                ui.group(|ui| {
                    match asset.kind {
                        Kind::Texture => {
                            self.previews.show(
                                ui,
                                Key::Texture(id.clone()),
                                egui::vec2(ui.available_width(), 120.0),
                            );
                        }
                        Kind::Model => {
                            self.previews.show(
                                ui,
                                Key::Model(Model {
                                    asset: id.clone(),
                                    scale: 1000,
                                    y_up: true,
                                    materials: Default::default(),
                                }),
                                egui::vec2(ui.available_width(), 120.0),
                            );
                        }
                        Kind::Audio => (),
                    }
                    ui.text_edit_singleline(&mut asset.name);
                    ui.small(format!(
                        "{} · {} · {:.1} KiB",
                        asset.kind.label(),
                        asset.description(),
                        asset.bytes() as f64 / 1024.0
                    ));
                    ui.horizontal_wrapped(|ui| {
                        if asset.kind == Kind::Model
                            && ui
                                .add_enabled(
                                    self.project.scenery.len() < self.project.limits.scenery,
                                    egui::Button::new("Place model"),
                                )
                                .clicked()
                        {
                            place = Some(id.clone());
                        }
                        if asset.kind == Kind::Audio {
                            if ui.button("Audition sound").clicked() {
                                self.pending = Some(Command::Preview(Action::CustomSound {
                                    asset: id.clone(),
                                    volume: 100,
                                    pan: 64,
                                }));
                            }
                            if ui.button("Audition music").clicked() {
                                self.pending = Some(Command::Preview(Action::CustomMusic {
                                    asset: id.clone(),
                                }));
                            }
                        }
                        if ui.button("Remove").clicked() {
                            remove = Some(id.clone());
                        }
                    });
                });
            });
        }
        if ui.button("Stop audio preview").clicked() {
            self.pending = Some(Command::Preview(Action::StopMusic));
        }
        if let Some(id) = remove {
            let asset = self.project.assets.remove(&id).unwrap();
            if let Err(error) = self.project.validate() {
                self.project.assets.insert(id, asset);
                self.notice =
                    format!("Remove this asset's scenery/material/cue references first: {error}");
            }
        }
        if let Some(id) = place {
            let asset = &self.project.assets[&id];
            let materials = asset
                .model()
                .unwrap()
                .iter()
                .map(|t| (t.material.clone(), None))
                .collect();
            self.campaign_prop = None;
            self.scenery_selection = Some(self.project.scenery.len());
            self.project.scenery.push(Scenery {
                custom: Some(Model {
                    asset: id,
                    scale: 1000,
                    y_up: true,
                    materials,
                }),
                name: asset.name.clone(),
                game: AssetGame::Tenchu,
                stage: 0,
                part: 0,
                position: self.world_cursor(),
                yaw: 0,
            });
            self.ground_scenery();
            self.panel = Panel::Scenery;
            self.tool = Tool::Select;
        }
    }
}

pub(in crate::level_editor) fn model(
    ui: &mut egui::Ui,
    model: &mut Model,
    assets: &Assets,
    previews: &mut Previews,
) {
    ui.label("Custom model");
    ui.add(
        egui::DragValue::new(&mut model.scale)
            .range(1..=100_000)
            .suffix(" mm per OBJ unit"),
    );
    ui.horizontal(|ui| {
        ui.label("OBJ vertical axis");
        ui.selectable_value(&mut model.y_up, true, "Y up");
        ui.selectable_value(&mut model.y_up, false, "Y down");
    });
    if let Ok(asset) = get(assets, &model.asset, Kind::Model)
        && let Ok(triangles) = asset.model()
    {
        for triangle in triangles {
            model
                .materials
                .entry(triangle.material.clone())
                .or_default();
        }
    }
    ui.collapsing("Material textures", |ui| {
        for (name, id) in &mut model.materials {
            ui.push_id(name, |ui| {
                ui.label(if name.is_empty() {
                    "Default material"
                } else {
                    name
                });
                if let Some(key) = id.as_ref() {
                    previews.show(ui, Key::Texture(key.clone()), egui::vec2(100.0, 80.0));
                }
                egui::ComboBox::from_id_salt("texture")
                    .selected_text(
                        id.as_ref()
                            .and_then(|i| assets.get(i))
                            .map_or("Vertex colors", |a| &a.name),
                    )
                    .show_ui(ui, |ui| {
                        ui.selectable_value(id, None, "Vertex colors");
                        for (key, asset) in assets.iter().filter(|(_, a)| a.kind == Kind::Texture) {
                            ui.horizontal(|ui| {
                                previews.show(
                                    ui,
                                    Key::Texture(key.clone()),
                                    egui::vec2(48.0, 48.0),
                                );
                                let response =
                                    ui.selectable_value(id, Some(key.clone()), &asset.name);
                                previews.tooltip(response, Key::Texture(key.clone()), &asset.name);
                            });
                        }
                    });
            });
        }
    });
}

pub(in crate::level_editor) fn audio_choice(ui: &mut egui::Ui, id: &mut String, assets: &Assets) {
    egui::ComboBox::from_id_salt("custom-audio")
        .selected_text(
            assets
                .get(id)
                .map_or("Import audio in Assets…", |a| &a.name),
        )
        .show_ui(ui, |ui| {
            for (key, asset) in assets.iter().filter(|(_, a)| a.kind == Kind::Audio) {
                ui.selectable_value(id, key.clone(), &asset.name);
            }
        });
}
