mod events;
use super::*;
use egui::{Color32, RichText};

impl Editor {
    pub fn show(
        &mut self,
        ctx: &egui::Context,
        assets: &GameAssets<DiscSource>,
    ) -> Option<Command> {
        if !self.open {
            return None;
        }
        self.prepare_textures(ctx);
        if !self.ephemeral && !ctx.wants_keyboard_input() {
            let input = ctx.input(Clone::clone);
            {
                if input.modifiers.alt {
                    for (key, panel) in [
                        (egui::Key::Num1, Panel::Terrain),
                        (egui::Key::Num2, Panel::Actors),
                        (egui::Key::Num3, Panel::Scenery),
                        (egui::Key::Num4, Panel::Events),
                        (egui::Key::Num5, Panel::Environment),
                        (egui::Key::Num6, Panel::Mission),
                        (egui::Key::Num7, Panel::Library),
                    ] {
                        if input.key_pressed(key) {
                            self.panel = panel;
                        }
                    }
                }
                if input.key_pressed(egui::Key::G) {
                    self.guides = !self.guides;
                }
                if input.key_pressed(egui::Key::F5) {
                    self.pending = Some(Command::Play);
                }
                if self.unlocked && input.modifiers.command && input.key_pressed(egui::Key::Z) {
                    self.undo(input.modifiers.shift);
                }
                if self.unlocked && input.modifiers.command && input.key_pressed(egui::Key::Y) {
                    self.undo(true);
                }
                if input.modifiers.command
                    && input.key_pressed(egui::Key::S)
                    && let Err(e) = self.save(input.modifiers.shift)
                {
                    self.notice = e;
                }
                if self.unlocked && self.panel == Panel::Terrain {
                    for event in &input.events {
                        if let egui::Event::Paste(text) = event
                            && text.starts_with("tenchu-stamp-v1\n")
                        {
                            match building::Stamp::from_clipboard(text) {
                                Ok(stamp) => {
                                    self.clipboard = Some(stamp);
                                    self.tool = Tool::Stamp;
                                }
                                Err(error) => self.notice = error,
                            }
                        }
                    }
                    if input.modifiers.command && input.key_pressed(egui::Key::C)
                        || input.events.iter().any(|e| matches!(e, egui::Event::Copy))
                    {
                        self.copy_area(ctx);
                    }
                    if input.modifiers.command
                        && input.key_pressed(egui::Key::V)
                        && self.clipboard.is_some()
                    {
                        self.tool = Tool::Stamp;
                    }
                    if input.key_pressed(egui::Key::I) {
                        self.sample_cell();
                    }
                }
                if self.unlocked && self.panel == Panel::Scenery {
                    for (key, axis, direction) in [
                        (egui::Key::ArrowLeft, 0, -1),
                        (egui::Key::ArrowRight, 0, 1),
                        (egui::Key::ArrowUp, 2, -1),
                        (egui::Key::ArrowDown, 2, 1),
                        (egui::Key::PageUp, 1, -1),
                        (egui::Key::PageDown, 1, 1),
                    ] {
                        if input.key_pressed(key) {
                            self.nudge_scenery(axis, direction);
                        }
                    }
                    if input.key_pressed(egui::Key::End) {
                        self.ground_scenery();
                    }
                }
                if input.key_pressed(egui::Key::R) {
                    if self.tool == Tool::Stamp && self.unlocked {
                        if let Some(stamp) = &mut self.clipboard {
                            stamp.rotate();
                        }
                    } else if self.panel == Panel::Scenery && self.unlocked {
                        self.history.begin(&self.project);
                        if let Some(prop) = self
                            .scenery_selection
                            .and_then(|i| self.project.scenery.get_mut(i))
                        {
                            prop.yaw = (prop.yaw + 1024) % 4096;
                        }
                        self.commit();
                    } else {
                        self.rotation = (self.rotation + 1) % 4;
                    }
                }
                if input.key_pressed(egui::Key::F) {
                    let mut target = self.cursor.world();
                    if self.panel == Panel::Scenery
                        && let Some(prop) = self
                            .scenery_selection
                            .and_then(|i| self.project.scenery.get(i))
                    {
                        target = prop.position;
                    } else if self.panel == Panel::Actors {
                        let point = self.project.mission.placements[self.actor].point;
                        target = point.world();
                        target[1] = i32::from(self.theme.as_ref().unwrap().height(
                            &self.project.mission,
                            point.x,
                            point.z,
                            false,
                        )) * 1000;
                    }
                    self.camera
                        .focus(glam::Vec3::from_array(target.map(|v| v as f32)), false);
                }
                if input.key_pressed(egui::Key::Escape) {
                    self.tool = Tool::Select;
                }
                if input.key_pressed(egui::Key::Delete)
                    && self.panel == Panel::Actors
                    && self.unlocked
                {
                    self.history.begin(&self.project);
                    if let Err(e) = self.project.remove_actor(self.actor) {
                        self.notice = e;
                    } else {
                        self.actor = 0;
                    }
                    self.commit();
                }
                if self.panel == Panel::Scenery
                    && self.unlocked
                    && let Some(index) = self
                        .scenery_selection
                        .filter(|&i| i < self.project.scenery.len())
                    && (input.key_pressed(egui::Key::Delete)
                        || input.modifiers.command && input.key_pressed(egui::Key::D))
                {
                    self.history.begin(&self.project);
                    if input.key_pressed(egui::Key::Delete) {
                        self.project.scenery.remove(index);
                        self.scenery_selection = None;
                    } else if input.modifiers.command
                        && input.key_pressed(egui::Key::D)
                        && self.project.scenery.len() < 512
                    {
                        self.project
                            .scenery
                            .push(self.project.scenery[index].clone());
                        self.scenery_selection = Some(self.project.scenery.len() - 1);
                    }
                    self.commit();
                }
            }
        }
        egui::TopBottomPanel::top("studio-toolbar")
            .frame(
                egui::Frame::new()
                    .fill(crate::ui::theme::INK)
                    .inner_margin(10),
            )
            .show(ctx, |ui| {
                crate::ui::theme::style(ui);
                ui.horizontal_wrapped(|ui| {
                    ui.heading("Level Studio");
                    ui.separator();
                    if ui.button("Projects").clicked() {
                        self.panel = Panel::Library;
                    }
                    if ui.button("Save  Ctrl+S").clicked()
                        && let Err(e) = self.save(false)
                    {
                        self.notice = e;
                    }
                    if ui.button("Save copy").clicked()
                        && let Err(e) = self.save(true)
                    {
                        self.notice = e;
                    }
                    if ui
                        .add_enabled(
                            self.unlocked && self.history.can_undo(),
                            egui::Button::new("Undo"),
                        )
                        .clicked()
                    {
                        self.undo(false);
                    }
                    if ui
                        .add_enabled(
                            self.unlocked && self.history.can_redo(),
                            egui::Button::new("Redo"),
                        )
                        .clicked()
                    {
                        self.undo(true);
                    }
                    ui.separator();
                    if ui.button("▶ Playtest  F5").clicked() {
                        self.pending = Some(Command::Play);
                    }
                    if ui.button("Refresh preview").clicked() {
                        self.pending = Some(Command::Rebuild);
                    }
                    if ui.button("Exit studio").clicked() {
                        self.pending = Some(Command::Close);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    for (tool, label) in [
                        (Tool::Select, "Select / move"),
                        (Tool::Paint, "Paint"),
                        (Tool::Erase, "Erase"),
                        (Tool::Patrol, "Patrol"),
                        (Tool::Area, "Area"),
                        (Tool::Stamp, "Stamp"),
                    ] {
                        if ui.selectable_value(&mut self.tool, tool, label).clicked() {
                            self.panel = if matches!(
                                tool,
                                Tool::Paint | Tool::Erase | Tool::Area | Tool::Stamp
                            ) {
                                Panel::Terrain
                            } else {
                                Panel::Actors
                            };
                        }
                    }
                    ui.separator();
                    ui.selectable_value(&mut self.grid, false, "3D view");
                    ui.selectable_value(&mut self.grid, true, "Map view");
                    ui.checkbox(&mut self.guides, "Guides")
                        .on_hover_text("G toggles markers and regions for clean previews");
                    self.camera_views(ui);
                });
            });
        egui::TopBottomPanel::top("studio-sections")
            .frame(
                egui::Frame::new()
                    .fill(crate::ui::theme::PANEL)
                    .inner_margin(8),
            )
            .show(ctx, |ui| {
                crate::ui::theme::style(ui);
                let width = ((ui.available_width() - 6.0 * 8.0) / 7.0).max(90.0);
                ui.horizontal_wrapped(|ui| {
                    for (index, (panel, label)) in [
                        (Panel::Terrain, "Terrain"),
                        (Panel::Actors, "Characters"),
                        (Panel::Scenery, "Scenery"),
                        (Panel::Events, "Events & audio"),
                        (Panel::Environment, "Environment"),
                        (Panel::Mission, "Mission"),
                        (Panel::Library, "Projects"),
                    ]
                    .into_iter()
                    .enumerate()
                    {
                        if ui
                            .add_sized(
                                [width, 32.0],
                                egui::Button::new(label).selected(self.panel == panel),
                            )
                            .on_hover_text(format!("Alt+{}", index + 1))
                            .clicked()
                        {
                            self.panel = panel;
                        }
                    }
                });
            });
        egui::TopBottomPanel::bottom("studio-status").show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.label(RichText::new(&self.project.mission.title).color(crate::ui::theme::GOLD));
                ui.separator();
                ui.label(format!("Cell {}, {}", self.cursor.x, self.cursor.z));
                ui.separator();
                ui.small(if self.grid { "Scroll: zoom · middle drag: pan · click: edit · R: rotate" } else { "Right drag: look · right + WASD/QE: fly · middle drag: pan · scroll: dolly · F: frame selection" });
            });
            if !self.notice.is_empty() { ui.colored_label(Color32::GOLD, &self.notice); }
        });
        egui::SidePanel::left("studio-inspector")
            .default_width(350.0)
            .min_width(310.0)
            .max_width(470.0)
            .resizable(true)
            .show(ctx, |ui| {
                crate::ui::theme::style(ui);
                ui.spacing_mut().slider_width = (ui.available_width() - 210.0).clamp(70.0, 140.0);
                if !self.unlocked {
                    ui.label("This mission has an editing password");
                    ui.add(egui::TextEdit::singleline(&mut self.password).password(true));
                    if ui.button("Unlock").clicked() {
                        self.unlocked = self.password == self.project.mission.password;
                        if !self.unlocked {
                            self.notice = "Incorrect mission password".into();
                        }
                    }
                }
                egui::ScrollArea::vertical().show(ui, |ui| {
                    if self.panel == Panel::Library {
                        self.projects(ui, assets);
                    } else {
                        ui.add_enabled_ui(self.unlocked, |ui| {
                            self.history.begin(&self.project);
                            match self.panel {
                                Panel::Terrain => self.palette(ui),
                                Panel::Actors => self.characters(ui),
                                Panel::Scenery => self.scenery(ui, assets),
                                Panel::Events => self.events(ui),
                                Panel::Environment => self.environment(ui),
                                Panel::Mission => self.mission(ui),
                                Panel::Library => (),
                            }
                        });
                    }
                });
            });
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show(ctx, |ui| self.viewport(ui));
        if !ctx.input(|i| i.pointer.any_down()) && !ctx.wants_keyboard_input() {
            self.commit();
            self.last_cell = None;
        }
        self.pending.take()
    }

    fn palette(&mut self, ui: &mut egui::Ui) {
        ui.heading("Terrain palette");
        self.area_controls(ui);
        ui.small("I samples the tile under the cursor, including its rotation.");
        ui.horizontal(|ui| {
            for (ground, label) in [(true, "Ground"), (false, "Ornaments / traps")] {
                if ui
                    .selectable_value(&mut self.ground, ground, label)
                    .clicked()
                {
                    self.brush = self
                        .theme
                        .as_ref()
                        .unwrap()
                        .tiles
                        .iter()
                        .position(|t| t.is_ornament() != ground)
                        .unwrap_or(0) as u8;
                }
            }
        });
        ui.add(egui::Slider::new(&mut self.brush_size, 1..=8).text("Brush size"));
        ui.horizontal(|ui| {
            ui.label("Rotation");
            for n in 0..4 {
                ui.selectable_value(&mut self.rotation, n, format!("{}°", n as u16 * 90));
            }
        });
        ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Search assets…"));
        let theme = self.theme.as_ref().unwrap();
        for (i, tile) in theme.tiles.iter().enumerate().filter(|(_, t)| {
            t.is_ornament() != self.ground
                && t.name.to_lowercase().contains(&self.search.to_lowercase())
        }) {
            let button = if let Some(texture) = &self.textures[i] {
                egui::Button::image_and_text(
                    (texture.id(), egui::vec2(38.0, 38.0)),
                    format!("{i} · {}", tile.name),
                )
            } else {
                egui::Button::new(format!("{i} · {}", tile.name))
            };
            if ui
                .add_sized(
                    [ui.available_width(), 44.0],
                    button.selected(self.brush == i as u8),
                )
                .clicked()
            {
                self.brush = i as u8;
                self.tool = Tool::Paint;
            }
        }
    }

    fn characters(&mut self, ui: &mut egui::Ui) {
        ui.heading("Scene characters");
        for (i, actor) in self.project.mission.placements.iter().enumerate() {
            let name = if i == 0 {
                "Player".to_owned()
            } else {
                self.tables
                    .as_ref()
                    .unwrap()
                    .characters
                    .iter()
                    .find(|c| c.kind == actor.kind)
                    .map_or_else(|| format!("Character {}", actor.kind), |c| c.model.clone())
            };
            if ui
                .selectable_label(
                    self.actor == i,
                    format!("{i} · {name}  {}, {}", actor.point.x, actor.point.z),
                )
                .clicked()
            {
                self.actor = i;
                self.cursor = actor.point;
                self.tool = Tool::Select;
            }
        }
        ui.separator();
        egui::ComboBox::from_id_salt("studio-kind")
            .selected_text(format!("Character {}", self.actor_kind))
            .show_ui(ui, |ui| {
                for kind in &self.tables.as_ref().unwrap().appearance[self.project.mission.theme] {
                    if *kind > 6 && *kind != 170 {
                        let name = self
                            .tables
                            .as_ref()
                            .unwrap()
                            .characters
                            .iter()
                            .find(|c| c.kind == *kind)
                            .map_or_else(|| kind.to_string(), |c| c.model.clone());
                        ui.selectable_value(&mut self.actor_kind, *kind, name);
                    }
                }
            });
        if ui.button("Add character at cursor").clicked() {
            match editing::add_actor(
                &mut self.project.mission,
                self.theme.as_ref().unwrap(),
                self.cursor,
                self.actor_kind,
            ) {
                Ok(actor) => {
                    self.actor = actor;
                    self.tool = Tool::Select;
                }
                Err(e) => self.notice = e,
            }
        }
        let editable = self.actor >= self.project.mission.objective.reserved()
            || self.actor == 1
                && matches!(
                    self.project.mission.objective,
                    Objective::Assassinate | Objective::EliminateEnemies
                );
        if ui
            .add_enabled(editable, egui::Button::new("Replace character type"))
            .clicked()
        {
            self.project.mission.placements[self.actor].kind = self.actor_kind;
        }
        if ui
            .add_enabled(
                self.actor >= self.project.mission.objective.reserved(),
                egui::Button::new("Delete selected"),
            )
            .clicked()
        {
            if let Err(e) = self.project.remove_actor(self.actor) {
                self.notice = e;
            } else {
                self.actor = 0;
            }
        }
        ui.label("Select a character, then click a clear cell to move it.");
        let mut model = self
            .project
            .actors
            .get(&self.actor)
            .cloned()
            .unwrap_or_default();
        let appearance_allowed = !matches!(
            self.project.mission.placements[self.actor].kind,
            5 | 6 | 170
        );
        ui.add_enabled_ui(appearance_allowed, |ui| {
            egui::ComboBox::from_id_salt("studio-body")
                .width(250.0)
                .selected_text(
                    self.asset_names
                        .iter()
                        .find(|(id, _)| *id == model)
                        .map_or("Original appearance", |(_, label)| label.as_str()),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut model, String::new(), "Original appearance");
                    for (id, label) in &self.asset_names {
                        ui.selectable_value(&mut model, id.clone(), label);
                    }
                })
        });
        if !appearance_allowed {
            ui.small("Objective characters keep their original appearance and behavior.");
        }
        if model.is_empty() {
            self.project.actors.remove(&self.actor);
        } else {
            self.project.actors.insert(self.actor, model);
        }
        ui.separator();
        ui.heading("Patrol");
        ui.add_enabled_ui(self.actor != 0, |ui| {
            if ui.selectable_label(self.tool == Tool::Patrol, "Draw patrol on map").clicked() { self.tool = Tool::Patrol; }
            ui.add(egui::Slider::new(&mut self.patrol_wait, 0..=9).text("Wait (retail units)"));
            ui.horizontal(|ui| { for (n, name) in ["N", "E", "S", "W"].iter().enumerate() { ui.selectable_value(&mut self.patrol_facing, n as i16, *name); } });
            ui.small("Click adjacent cells. Open routes return along their path. Wait 0 passes through; other waits last value + 1 seconds.");
            let route = &mut self.project.mission.placements[self.actor].route;
            ui.horizontal(|ui| {
                if ui.button("Remove last").clicked() { route.pop(); if route.len() < 2 { route.clear(); } }
                if ui.button("Clear route").clicked() { route.clear(); }
            });
            ui.label(format!("{} / 64 waypoints", route.len()));
            for (i, point) in route.iter_mut().enumerate() {
                ui.push_id(i, |ui| ui.horizontal(|ui| {
                    ui.label(format!("{i}: {}, {}", point.point.x, point.point.z));
                    ui.add(egui::DragValue::new(&mut point.wait).range(0..=9).prefix("Wait "));
                    ui.add(egui::DragValue::new(&mut point.facing).range(0..=3).prefix("Face "));
                }));
            }
        });
    }

    fn environment(&mut self, ui: &mut egui::Ui) {
        ui.heading("Environment");
        ui.small("New maps start with your game settings. Changes here belong to this project.");
        if ui.button("Use my game settings").clicked() {
            self.project.graphics = self.default_graphics.clone();
        }
        let g = &mut self.project.graphics;
        ui.add_space(4.0);
        ui.label(RichText::new("Graphics preset").color(crate::ui::theme::GOLD));
        let mut preset = g.preset();
        egui::ComboBox::from_id_salt("studio-preset")
            .width(ui.available_width())
            .selected_text(crate::settings::presets::NAMES[preset as usize])
            .show_ui(ui, |ui| {
                for (i, name) in crate::settings::presets::NAMES.iter().enumerate() {
                    if i as u16 != crate::settings::presets::CUSTOM
                        && ui.selectable_value(&mut preset, i as u16, *name).changed()
                    {
                        g.set_preset(preset);
                    }
                }
            });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.environment_weather, false, "Lighting");
            ui.selectable_value(&mut self.environment_weather, true, "Weather");
        });
        if !self.environment_weather {
            egui::CollapsingHeader::new("Sun, moon & lighting")
                .default_open(true)
                .show(ui, |ui| {
                    ui.checkbox(&mut g.sky.enabled, "Sun and moon lighting");
                    egui::Grid::new("studio-lighting-fields")
                        .num_columns(2)
                        .spacing([12.0, 9.0])
                        .show(ui, |ui| {
                            ui.label("Time of day");
                            ui.add(
                                egui::Slider::new(&mut g.sky.time, 0..=1439).custom_formatter(
                                    |n, _| format!("{:02}:{:02}", n as u32 / 60, n as u32 % 60),
                                ),
                            );
                            ui.end_row();
                            slider(ui, "Sun heading", &mut g.sky.heading, 0..=359);
                            slider(ui, "Sun strength", &mut g.sky.sun, 0..=200);
                            slider(ui, "Night ambient", &mut g.sky.night_ambient, 0..=100);
                            slider(
                                ui,
                                "Directional light",
                                &mut g.lighting.directional.intensity,
                                0..=200,
                            );
                        });
                    ui.horizontal(|ui| {
                        ui.checkbox(&mut g.lighting.enabled, "Local lights");
                        ui.checkbox(&mut g.lighting.shadows, "Cast shadows");
                    });
                    ui.checkbox(&mut g.fog, "Fog");
                    ui.add_enabled_ui(g.fog, |ui| {
                        ui.add(
                            egui::Slider::new(&mut g.fog_distance, 25..=400).text("Fog distance"),
                        );
                    });
                });
        } else {
            egui::CollapsingHeader::new("Weather & surfaces")
                .default_open(true)
                .show(ui, |ui| {
                    egui::ComboBox::from_id_salt("studio-weather")
                        .width(ui.available_width())
                        .selected_text(g.weather.mode.name())
                        .show_ui(ui, |ui| {
                            for mode in crate::settings::WeatherMode::ALL {
                                ui.selectable_value(&mut g.weather.mode, mode, mode.name());
                            }
                        });
                    ui.add_enabled_ui(
                        g.weather.mode != crate::settings::WeatherMode::Original,
                        |ui| {
                            if let Some(seconds) = crate::settings::overlay::weather::preview(
                                ui,
                                &mut self.weather_preview_seconds,
                            ) {
                                self.pending = Some(Command::WeatherPreview(seconds));
                            }
                        },
                    );
                    ui.separator();
                    egui::Grid::new("studio-weather-fields")
                        .num_columns(2)
                        .spacing([12.0, 9.0])
                        .show(ui, |ui| {
                            slider(ui, "Precipitation", &mut g.weather.amount, 0..=100);
                            slider(ui, "Wind", &mut g.weather.wind, 0..=100);
                            slider(ui, "Wetness", &mut g.weather.wetness, 0..=100);
                            slider(ui, "Snow cover", &mut g.weather.snow, 0..=100);
                            slider(ui, "Lightning", &mut g.weather.lightning, 0..=100);
                            slider(ui, "Thunder", &mut g.weather.thunder, 0..=100);
                        });
                    ui.checkbox(&mut g.weather.storm_bolts, "Lightning bolts");
                });
        }
    }

    fn mission(&mut self, ui: &mut egui::Ui) {
        ui.heading("Mission settings");
        let mission = &mut self.project.mission;
        ui.label("Title");
        ui.text_edit_singleline(&mut mission.title);
        ui.label("Author");
        ui.text_edit_singleline(&mut mission.author);
        ui.label("Editing password (optional)");
        ui.add(egui::TextEdit::singleline(&mut mission.password).password(true));
        ui.checkbox(&mut mission.stealth, "Fail if spotted");
        objective(ui, &mut self.new_objective);
        if self.new_objective != mission.objective
            && ui.button("Change objective and reset characters").clicked()
        {
            mission.objective = self.new_objective;
            mission.reset_placements(self.tables.as_ref().unwrap().appearance[mission.theme][2]);
            for placement in &mission.placements {
                let cell =
                    &mut mission.map[placement.point.z as usize * 32 + placement.point.x as usize];
                cell.ornament = 255;
                if !self
                    .theme
                    .as_ref()
                    .unwrap()
                    .tile(cell.ground)
                    .is_some_and(|t| t.supports_actor())
                {
                    cell.ground = tenchu_assets::trial::DEFAULT_TILES[mission.theme];
                }
            }
            self.project.actors.clear();
            self.project
                .events
                .retain(|e| !matches!(e.trigger, project::Trigger::Defeated(_)));
            self.actor = 0;
        }
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.project.character, 0, "Rikimaru");
            ui.selectable_value(&mut self.project.character, 1, "Ayame");
        });
        ui.separator();
        ui.heading("Best times");
        for (i, record) in mission.records.iter().enumerate() {
            ui.label(format!(
                "{}. {} · {}",
                i + 1,
                crate::trial::library::time(record.ticks),
                record.name
            ));
        }
        ui.horizontal(|ui| {
            ui.label("Record name");
            ui.add(egui::TextEdit::singleline(&mut self.library.record_name).char_limit(9));
        });
        ui.small("Original rules and characters can set records. Events, imported characters or gameplay modifiers make a practice run. Campaign progress stays separate.");
    }

    fn projects(&mut self, ui: &mut egui::Ui, assets: &GameAssets<DiscSource>) {
        ui.heading("Projects");
        self.project.build.show(ui, self.project.version);
        if ui.button("Open T1 + T2 garden example").clicked()
            && let Err(error) = self.open_mixed_example(assets)
        {
            self.notice = error;
        }
        if ui.button("Open lighting & events example").clicked() {
            let mut project = self.new_project(self.presets[0].clone());
            project.mission.title = "Storm arrives".into();
            project.mission.password.clear();
            project.mission.records.clear();
            project.graphics.sky.enabled = true;
            project.graphics.sky.time = 1080;
            project.events = vec![project::Event {
                name: "Arrival and changing weather".into(),
                cues: vec![
                    project::Cue {
                        delay: 0,
                        action: Action::Message {
                            text: "A storm is approaching. Find your objective.".into(),
                            ticks: 120,
                        },
                    },
                    project::Cue {
                        delay: 90,
                        action: Action::Weather(crate::settings::WeatherMode::Rainstorm),
                    },
                    project::Cue {
                        delay: 0,
                        action: Action::TimeOfDay(1200),
                    },
                ],
                ..project::Event::default()
            }];
            if let Err(error) = self.save(false).and_then(|_| self.select(project, assets)) {
                self.notice = error;
            } else {
                self.panel = Panel::Events;
                self.notice = "Press F5: the arrival message appears immediately, then the weather changes after three seconds. Edit or extend the queue below.".into();
            }
        }
        egui::ComboBox::from_id_salt("studio-new-theme")
            .selected_text(tenchu_assets::trial::THEMES[self.new_theme])
            .show_ui(ui, |ui| {
                for (i, name) in tenchu_assets::trial::THEMES.iter().enumerate() {
                    ui.selectable_value(&mut self.new_theme, i, *name);
                }
            });
        objective(ui, &mut self.new_objective);
        if ui.button("Create new mission").clicked() {
            let result = self
                .save(false)
                .and_then(|_| {
                    Mission::new(
                        self.new_theme,
                        self.new_objective,
                        self.tables.as_ref().unwrap().appearance[self.new_theme][2],
                    )
                })
                .and_then(|m| self.select(self.new_project(m), assets));
            if let Err(e) = result {
                self.notice = e;
            } else {
                self.panel = Panel::Terrain;
            }
        }
        ui.small("Opening a mission saves your current work in this library.");
        let mut chosen = None;
        let mut remove = None;
        for (i, project) in self.library.projects.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui
                    .selectable_label(self.selected_save == Some(i), &project.mission.title)
                    .clicked()
                {
                    chosen = Some((project.clone(), Some(i)));
                }
                if ui
                    .small_button("Remove")
                    .on_hover_text("Remove this library copy; keep the current draft")
                    .clicked()
                {
                    remove = Some(i);
                }
            });
        }
        if let Some(index) = remove {
            let previous = self.library.clone();
            let selected = self.selected_save;
            self.library.projects.remove(index);
            self.selected_save =
                selected.and_then(|s| (s != index).then(|| s - usize::from(s > index)));
            if let Err(e) = self.persist() {
                self.library = previous;
                self.selected_save = selected;
                self.notice = e;
            }
            chosen = None;
        }
        ui.collapsing("Original challenges", |ui| {
            for (i, mission) in self.presets.iter().enumerate() {
                if ui
                    .button(format!("{:02} · {}", i + 1, mission.title))
                    .clicked()
                {
                    chosen = Some((self.new_project(mission.clone()), None));
                }
            }
        });
        if let Some((project, selected)) = chosen {
            if let Err(e) = self.save(false).and_then(|_| self.select(project, assets)) {
                self.notice = e;
            } else {
                self.selected_save = selected;
            }
        }
        ui.separator();
        ui.heading("Import / share");
        if ui.button("Import project / .msn / .mcs…").clicked()
            && let Err(e) = self.import_file(assets)
        {
            self.notice = e;
        }
        if ui.button("Export project…").clicked()
            && let Err(e) = self.export_file(false)
        {
            self.notice = e;
        }
        if ui.button("Export retail map only…").clicked()
            && let Err(e) = self.export_file(true)
        {
            self.notice = e;
        }
        ui.small("Projects contain references to your discs, never game assets. Retail .msn keeps the terrain, original characters, patrols and mission rules; Studio appearances, scenery, events and environment need a project.");
        if ui.button("Copy project").clicked() {
            match self.project.encode() {
                Ok(bytes) => {
                    self.share = String::from_utf8(bytes).unwrap();
                    ui.ctx().copy_text(self.share.clone());
                }
                Err(e) => self.notice = e,
            }
        }
        if ui.button("Copy retail share code").clicked() {
            match self.project.mission.encode() {
                Ok(bytes) => {
                    self.share = crate::trial::library::share(&bytes);
                    ui.ctx().copy_text(self.share.clone());
                }
                Err(e) => self.notice = e,
            }
        }
        if ui.button("Import pasted text").clicked() {
            let text = self.share.trim().to_owned();
            let bytes = if text.starts_with('{') {
                Ok(text.into_bytes())
            } else {
                crate::trial::library::unshare(&text)
            };
            if let Err(e) = bytes.and_then(|b| self.import(&b, assets)) {
                self.notice = e;
            }
        }
        ui.add(
            egui::TextEdit::multiline(&mut self.share)
                .desired_rows(6)
                .char_limit(project::MAX_BYTES)
                .hint_text("Paste a project or retail mission share code"),
        );
    }
}

fn objective(ui: &mut egui::Ui, objective: &mut Objective) {
    egui::ComboBox::from_id_salt("studio-objective")
        .selected_text(objective.label())
        .show_ui(ui, |ui| {
            for value in Objective::ALL {
                ui.selectable_value(objective, value, value.label());
            }
        });
}

fn slider(ui: &mut egui::Ui, label: &str, value: &mut u16, range: std::ops::RangeInclusive<u16>) {
    ui.label(label);
    ui.add(egui::Slider::new(value, range));
    ui.end_row();
}
