use super::*;
use egui::{Color32, RichText};

impl Editor {
    pub fn show(&mut self, ctx: &egui::Context, assets: &GameAssets<DiscSource>) -> Option<Event> {
        if !self.open {
            return None;
        }
        self.prepare_textures(ctx);
        #[cfg(target_arch = "wasm32")]
        {
            let imported = crate::web::take_trial_import();
            if !imported.is_empty() {
                match library::unshare(&imported).and_then(|b| self.import(&b, assets)) {
                    Ok(()) => (),
                    Err(e) => self.notice = e,
                }
            }
        }
        if !ctx.wants_keyboard_input() {
            if ctx.input(|i| i.key_pressed(egui::Key::F5)) {
                self.pending = Some(Event::Play { preview: false });
            }
            if ctx.input(|i| i.modifiers.command && i.key_pressed(egui::Key::Z)) {
                self.undo();
            }
            if ctx.input(|i| i.key_pressed(egui::Key::Tab)) {
                self.next_tab(if ctx.input(|i| i.modifiers.shift) {
                    -1
                } else {
                    1
                });
            }
            if self.tab == Tab::Missions {
                for (key, direction) in [(egui::Key::ArrowUp, -1), (egui::Key::ArrowDown, 1)] {
                    if ctx.input(|i| i.key_pressed(key)) {
                        self.next_mission(direction, assets);
                    }
                }
                if ctx.input(|i| i.key_pressed(egui::Key::Enter)) {
                    self.pending = Some(Event::Play { preview: false });
                }
            }
            if matches!(self.tab, Tab::Terrain | Tab::Actors | Tab::Patrols) {
                for (key, dx, dz) in [
                    (egui::Key::ArrowLeft, -1, 0),
                    (egui::Key::ArrowRight, 1, 0),
                    (egui::Key::ArrowUp, 0, -1),
                    (egui::Key::ArrowDown, 0, 1),
                ] {
                    if ctx.input(|i| i.key_pressed(key)) {
                        self.cursor.x = (self.cursor.x + dx).clamp(0, 31);
                        self.cursor.z = (self.cursor.z + dz).clamp(0, 31);
                    }
                }
                if ctx.input(|i| i.key_pressed(egui::Key::R)) {
                    self.rotation = (self.rotation + 1) % 4;
                }
                if ctx
                    .input(|i| i.key_pressed(egui::Key::Enter) || i.key_pressed(egui::Key::Delete))
                {
                    self.use_cell(ctx.input(|i| i.key_pressed(egui::Key::Delete)));
                    self.finish_stroke();
                }
            }
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_wrapped(|ui| {
                ui.heading("Mission editor");
                ui.separator();
                if ui.button("Play  F5").clicked() {
                    self.pending = Some(Event::Play { preview: false });
                }
                if ui.button("3D preview").clicked() {
                    self.pending = Some(Event::Play { preview: true });
                }
                egui::ComboBox::from_id_salt("trial-hero")
                    .selected_text(if self.character == 0 {
                        "Rikimaru"
                    } else {
                        "Ayame"
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut self.character, 0, "Rikimaru");
                        ui.selectable_value(&mut self.character, 1, "Ayame");
                    });
                if ui.button("Save").clicked()
                    && let Err(e) = self.save_copy(false)
                {
                    self.notice = e;
                }
                if ui.button("Save a copy").clicked()
                    && let Err(e) = self.save_copy(true)
                {
                    self.notice = e;
                }
                if ui
                    .add_enabled(!self.undo.is_empty(), egui::Button::new("Undo  Ctrl+Z"))
                    .clicked()
                {
                    self.undo();
                }
                if ui.button("Title  Esc").clicked() {
                    self.pending = Some(Event::Close);
                }
            });
            ui.horizontal_wrapped(|ui| {
                for tab in Tab::ALL {
                    ui.selectable_value(&mut self.tab, tab, tab.label());
                }
            });
            let mission = self.mission.as_ref().unwrap();
            ui.label(
                RichText::new(format!(
                    "{} · {} · {}{}",
                    mission.title,
                    tenchu_assets::trial::THEMES[mission.theme],
                    mission.objective.label(),
                    if mission.stealth {
                        " · Never be spotted"
                    } else {
                        ""
                    }
                ))
                .font(crate::fonts::id(15.0)),
            );
            if !self.unlocked {
                ui.horizontal(|ui| {
                    ui.label("Editing is password protected");
                    ui.add(
                        egui::TextEdit::singleline(&mut self.password)
                            .password(true)
                            .desired_width(140.0),
                    );
                    if ui.button("Unlock").clicked() {
                        if self.password == self.mission.as_ref().unwrap().password {
                            self.unlocked = true;
                            self.notice.clear();
                        } else {
                            self.notice = "Incorrect mission password".into();
                        }
                    }
                });
            }
            ui.separator();
            if !self.notice.is_empty() {
                ui.label(RichText::new(&self.notice).color(Color32::GOLD));
            }
            match self.tab {
                Tab::Missions => self.missions(ui, assets),
                Tab::Share => self.sharing(ui, assets),
                Tab::Settings => self.settings(ui, assets),
                _ => {
                    ui.columns(2, |columns| {
                        self.grid(&mut columns[0]);
                        egui::ScrollArea::vertical().id_salt("trial-tools").show(
                            &mut columns[1],
                            |ui| {
                                ui.add_enabled_ui(self.unlocked, |ui| match self.tab {
                                    Tab::Terrain => self.palette(ui),
                                    Tab::Actors => self.actors(ui),
                                    Tab::Patrols => self.patrols(ui),
                                    _ => (),
                                });
                            },
                        );
                    });
                }
            }
        });
        self.pending.take()
    }
    fn missions(&mut self, ui: &mut egui::Ui, assets: &GameAssets<DiscSource>) {
        ui.horizontal_wrapped(|ui| {
            ui.label("New mission:");
            theme_box(ui, &mut self.new_theme);
            objective_box(ui, &mut self.new_objective);
            if ui.button("Create").clicked() {
                let target = self.tables.as_ref().unwrap().appearance[self.new_theme][2];
                let result = Mission::new(self.new_theme, self.new_objective, target)
                    .and_then(|m| self.select(m, Origin::Draft, assets));
                match result {
                    Ok(()) => {
                        self.tab = Tab::Terrain;
                        self.notice =
                            "New draft. Paint terrain, place characters, then playtest.".into();
                    }
                    Err(e) => self.notice = e,
                }
            }
            if ui.button("Import file…").clicked()
                && let Err(e) = self.file_import(assets)
            {
                self.notice = e;
            }
        });
        ui.label("All sixteen disc challenges are available here. Playing trials does not change campaign progress.");
        ui.small("Save your edits before choosing another mission. Up / Down choose · Enter plays · Tab changes tools.");
        ui.columns(2, |columns| {
            columns[0].heading("Built-in challenges");
            let mut chosen = None;
            egui::ScrollArea::vertical().id_salt("trial-presets").max_height(columns[0].available_height()).show(&mut columns[0], |ui| {
                for (i, mission) in self.presets.iter().enumerate() {
                    let label = format!("{:02}  {}\n{} · {}", i + 1, mission.title, tenchu_assets::trial::THEMES[mission.theme], mission.objective.label());
                    let selected = self.origin == Origin::Preset(i);
                    let response = ui.add(egui::Button::new(RichText::new(label).font(crate::fonts::id(14.0))).selected(selected).min_size(egui::vec2(ui.available_width(), 45.0)));
                    if selected && self.scroll_selection { response.scroll_to_me(None); }
                    if response.clicked() { chosen = Some((mission.clone(), Origin::Preset(i))); }
                }
            });
            columns[1].heading("Your missions");
            let mut remove = None;
            egui::ScrollArea::vertical().id_salt("trial-saved").max_height((columns[1].available_height() - 210.0).max(100.0)).show(&mut columns[1], |ui| {
                if self.library.missions.is_empty() { ui.label("Save a draft to add it here. Use Share to exchange missions."); }
                for (i, bytes) in self.library.missions.iter().enumerate() {
                    if let Ok(mission) = Mission::decode(bytes) {
                        ui.horizontal(|ui| {
                            let selected = self.origin == Origin::Saved(i);
                            let response = ui.selectable_label(selected, RichText::new(&mission.title).font(crate::fonts::id(15.0)));
                            if selected && self.scroll_selection { response.scroll_to_me(None); }
                            if response.clicked() { chosen = Some((mission, Origin::Saved(i))); }
                            if ui.small_button("Remove").on_hover_text("Remove this library copy; the current draft is retained").clicked() { remove = Some(i); }
                        });
                    }
                }
            });
            if let Some(i) = remove {
                self.library.missions.remove(i);
                if let Origin::Saved(index) = self.origin { self.origin = if index == i { Origin::Draft } else { Origin::Saved(index - usize::from(index > i)) }; }
                if let Err(e) = self.persist() { self.notice = e; }
            }
            if let Some((mission, origin)) = chosen && let Err(e) = self.select(mission, origin, assets) { self.notice = e; }
            columns[1].separator();
            columns[1].heading("Best times for selected mission");
            let mission = self.mission.as_ref().unwrap();
            if mission.records.is_empty() { columns[1].label("No completed runs yet"); }
            for (i, r) in mission.records.iter().enumerate() {
                columns[1].label(format!("{}.  {}  {}", i + 1, library::time(r.ticks), r.name));
            }
            columns[1].horizontal(|ui| {
                ui.label("Record name:");
                ui.add(egui::TextEdit::singleline(&mut self.record_name).char_limit(9).desired_width(130.0));
            });
            columns[1].small("Stealth kill −15s · kill −5s · civilian +30s · spotted +10s. Modified gameplay is practice and does not set records.");
        });
        self.scroll_selection = false;
    }
    fn palette(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            if ui.selectable_label(!self.ornament, "Ground").clicked() {
                self.ornament = false;
                self.brush =
                    tenchu_assets::trial::DEFAULT_TILES[self.mission.as_ref().unwrap().theme];
            }
            if ui
                .selectable_label(self.ornament, "Ornaments / traps")
                .clicked()
            {
                self.ornament = true;
                if let Some(i) = self
                    .theme
                    .as_ref()
                    .unwrap()
                    .tiles
                    .iter()
                    .position(|t| t.is_ornament())
                {
                    self.brush = i as u8;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Rotation:");
            for i in 0..4 {
                ui.selectable_value(&mut self.rotation, i, format!("{}°", u16::from(i) * 90));
            }
        });
        ui.label("Drag to paint · right-drag to erase. Numbers mark height in metres; W marks impassable walls. Orange ornaments are traps.");
        let theme = self.theme.as_ref().unwrap();
        let columns = (ui.available_width() / 82.0).floor().max(1.0) as usize;
        egui::Grid::new("trial-palette")
            .num_columns(columns)
            .show(ui, |ui| {
                let mut n = 0;
                for (i, tile) in theme
                    .tiles
                    .iter()
                    .enumerate()
                    .filter(|(_, t)| t.is_ornament() == self.ornament)
                {
                    let label = format!("{}: {}", i, tile.name);
                    let button = if let Some(texture) = &self.textures[i] {
                        egui::Button::image_and_text(
                            (texture.id(), egui::vec2(44.0, 44.0)),
                            i.to_string(),
                        )
                    } else {
                        egui::Button::new(&label).min_size(egui::vec2(70.0, 44.0))
                    };
                    if ui
                        .add_sized([78.0, 52.0], button.selected(self.brush == i as u8))
                        .on_hover_text(label)
                        .clicked()
                    {
                        self.brush = i as u8;
                    }
                    n += 1;
                    if n % columns == 0 {
                        ui.end_row();
                    }
                }
            });
        if let Some(tile) = theme.tile(self.brush) {
            ui.label(format!("Selected: {} · {}", self.brush, tile.name));
        }
    }
    fn actor_list(&mut self, ui: &mut egui::Ui) {
        let mission = self.mission.as_ref().unwrap();
        for (i, p) in mission.placements.iter().enumerate() {
            if ui
                .selectable_label(
                    self.actor == i,
                    format!(
                        "{}: {} · {}, {}{}",
                        i,
                        self.name(p.kind),
                        p.point.x,
                        p.point.z,
                        if i < mission.objective.reserved() {
                            " · objective"
                        } else {
                            ""
                        }
                    ),
                )
                .clicked()
            {
                self.actor = i;
                self.cursor = p.point;
            }
        }
    }
    fn actors(&mut self, ui: &mut egui::Ui) {
        self.actor_list(ui);
        ui.separator();
        ui.label("Select a character, then click an empty map tile to move it. Right-click removes non-objective characters.");
        let mission = self.mission.as_ref().unwrap();
        let kinds = self.tables.as_ref().unwrap().appearance[mission.theme].clone();
        egui::ComboBox::from_id_salt("trial-actor-kind")
            .selected_text(self.name(self.actor_kind))
            .show_ui(ui, |ui| {
                for kind in kinds {
                    if kind > 6 && kind != 170 {
                        let name = self.name(kind);
                        ui.selectable_value(&mut self.actor_kind, kind, name);
                    }
                }
            });
        if ui.button("Add at cursor").clicked() {
            self.add_actor();
        }
        let allowed = self.actor >= self.mission.as_ref().unwrap().objective.reserved()
            || self.actor == 1
                && matches!(
                    self.mission.as_ref().unwrap().objective,
                    Objective::Assassinate | Objective::EliminateEnemies
                );
        if ui
            .add_enabled(allowed, egui::Button::new("Replace selected character"))
            .clicked()
        {
            let before = self.mission.as_ref().unwrap().clone();
            self.mission.as_mut().unwrap().placements[self.actor].kind = self.actor_kind;
            self.edit(before);
        }
    }
    fn patrols(&mut self, ui: &mut egui::Ui) {
        self.actor_list(ui);
        ui.separator();
        ui.label("Select an actor, then click adjacent tiles to extend its patrol. An open route walks back along the same path; return to the first tile to loop. Right-click removes the last point.");
        ui.add(egui::Slider::new(&mut self.patrol_wait, 0..=9).text("Wait at new point"));
        ui.horizontal(|ui| {
            ui.label("Look:");
            for (i, name) in ["North", "East", "South", "West"].into_iter().enumerate() {
                ui.selectable_value(&mut self.patrol_facing, i as i16, name);
            }
        });
        if self.actor == 0 {
            ui.label("The player has no patrol");
            return;
        }
        let route = &self.mission.as_ref().unwrap().placements[self.actor].route;
        ui.label(format!("{} / 64 waypoints", route.len()));
        for (i, p) in route.iter().enumerate() {
            ui.label(format!(
                "{i}: {}, {} · wait {} · facing {}",
                p.point.x, p.point.z, p.wait, p.facing
            ));
        }
        if ui.button("Clear patrol").clicked() {
            let before = self.mission.as_ref().unwrap().clone();
            self.mission.as_mut().unwrap().placements[self.actor]
                .route
                .clear();
            self.edit(before);
        }
    }
    fn settings(&mut self, ui: &mut egui::Ui, assets: &GameAssets<DiscSource>) {
        ui.add_enabled_ui(self.unlocked, |ui| {
            let before = self.mission.as_ref().unwrap().clone();
            let mission = self.mission.as_mut().unwrap();
            ui.label("Title (31 Shift-JIS bytes)"); ui.text_edit_singleline(&mut mission.title);
            ui.label("Author (17 Shift-JIS bytes)"); ui.text_edit_singleline(&mut mission.author);
            ui.label("Optional editing password (17 Shift-JIS bytes; not encryption)"); ui.add(egui::TextEdit::singleline(&mut mission.password).password(true));
            ui.checkbox(&mut mission.stealth, "Fail if spotted");
            ui.separator();
            objective_box(ui, &mut self.new_objective);
            if self.new_objective != mission.objective && ui.button("Change objective and reset character placements").clicked() {
                mission.objective = self.new_objective;
                mission.reset_placements(self.tables.as_ref().unwrap().appearance[mission.theme][2]);
                for p in &mission.placements { let cell = &mut mission.map[p.point.z as usize * 32 + p.point.x as usize]; cell.ornament = 255; if !self.theme.as_ref().unwrap().tile(cell.ground).is_some_and(|t| t.supports_actor()) { cell.ground = 0; } }
                self.actor = 0;
            }
            self.edit(before);
            ui.separator();
            theme_box(ui, &mut self.new_theme);
            ui.small("Tile definitions differ between themes. Changing theme creates a fresh map; save your current map first.");
            if self.new_theme != self.mission.as_ref().unwrap().theme && ui.button("Create fresh map with this theme").clicked() {
                let target = self.tables.as_ref().unwrap().appearance[self.new_theme][2];
                if let Err(e) = Mission::new(self.new_theme, self.new_objective, target).and_then(|m| self.select(m, Origin::Draft, assets)) { self.notice = e; }
            }
        });
    }
    fn sharing(&mut self, ui: &mut egui::Ui, assets: &GameAssets<DiscSource>) {
        ui.label("Exported .msn files use the original 7,484-byte mission format. Import also accepts a single 8 KiB memory-card block (.mcs). No game assets are included.");
        ui.horizontal(|ui| {
            if ui.button("Import file…").clicked()
                && let Err(e) = self.file_import(assets)
            {
                self.notice = e;
            }
            if ui.button("Export .msn…").clicked()
                && let Err(e) = self.file_export()
            {
                self.notice = e;
            }
            if ui.button("Copy share code").clicked() {
                match self.mission.as_ref().unwrap().encode() {
                    Ok(bytes) => {
                        self.share = library::share(&bytes);
                        ui.ctx().copy_text(self.share.clone());
                        self.notice = "Mission share code copied".into();
                    }
                    Err(e) => self.notice = e,
                }
            }
            if ui.button("Import pasted code").clicked() {
                match library::unshare(&self.share).and_then(|b| self.import(&b, assets)) {
                    Ok(()) => (),
                    Err(e) => self.notice = e,
                }
            }
        });
        ui.add(
            egui::TextEdit::multiline(&mut self.share)
                .desired_width(f32::INFINITY)
                .desired_rows(12)
                .char_limit(16400)
                .hint_text("Paste a mission share code here"),
        );
    }
}
fn theme_box(ui: &mut egui::Ui, theme: &mut usize) {
    egui::ComboBox::from_id_salt("trial-theme")
        .selected_text(tenchu_assets::trial::THEMES[*theme])
        .show_ui(ui, |ui| {
            for (i, name) in tenchu_assets::trial::THEMES.into_iter().enumerate() {
                ui.selectable_value(theme, i, name);
            }
        });
}
fn objective_box(ui: &mut egui::Ui, objective: &mut Objective) {
    egui::ComboBox::from_id_salt("trial-objective")
        .selected_text(objective.label())
        .show_ui(ui, |ui| {
            for option in Objective::ALL {
                ui.selectable_value(objective, option, option.label());
            }
        });
}
