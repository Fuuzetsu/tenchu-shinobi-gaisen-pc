use super::*;

impl Editor {
    pub(in crate::level_editor) fn campaign_projects(
        &mut self,
        ui: &mut egui::Ui,
        assets: &GameAssets<DiscSource>,
    ) {
        ui.collapsing("Campaign maps · T1 / T2", |ui| {
            ui.small("Open a disc map as an editable Studio project, including its terrain. The original disc stays unchanged.");
            if ui.button("Browse / refresh connected discs").clicked() {
                match campaign_sources(assets) { Ok(sources) => self.campaign_sources = sources, Err(e) => self.notice = e }
            }
            ui.add(egui::TextEdit::singleline(&mut self.search).hint_text("Find campaign area or sequence…"));
            let mut selected = None;
            egui::ScrollArea::vertical().id_salt("campaign-browser").max_height(240.0).show(ui, |ui| {
                for (source, name) in &self.campaign_sources {
                    if name.to_lowercase().contains(&self.search.to_lowercase()) && ui.button(name).clicked() { selected = Some(source.clone()); }
                }
            });
            if let Some(source) = selected && let Err(error) = self.open_campaign(assets, source) { self.notice = error; }
            ui.small("T2 maps need the matching connected disc edition. Campaign projects use Gaisen playtests and Studio events; retail scripts, cutscenes and T2 dynamic scenery are not imported.");
        });
    }

    pub(in crate::level_editor) fn campaign_terrain(&mut self, ui: &mut egui::Ui) {
        ui.heading("Campaign terrain");
        ui.small("Click a floor or roof in either view. Edit its elevation, ramp or solid area. Visible terrain and collision move together; the source footprint stays fixed.");
        let Some(key) = self.campaign_surface else {
            ui.label("Select a surface to begin.");
            return;
        };
        let Some(&original) = self.campaign_cache.as_ref().and_then(|c| c.nodes.get(&key)) else {
            return;
        };
        let campaign = self.project.campaign.as_mut().unwrap();
        let mut edit = campaign
            .terrain
            .iter()
            .find(|e| e.key == key)
            .cloned()
            .unwrap_or(Edit {
                key,
                original,
                node: original,
                geometry: true,
            });
        let before = edit.clone();
        ui.separator();
        ui.strong(format!("Surface {} / {}", key[0], key[1]));
        ui.small(format!(
            "Footprint: X {:.1}–{:.1} m · Z {:.1}–{:.1} m",
            f32::from(original.bounds[0]) / 100.0,
            f32::from(original.bounds[2]) / 100.0,
            f32::from(original.bounds[1]) / 100.0,
            f32::from(original.bounds[3]) / 100.0
        ));
        let t2 = campaign.source.t2();
        let mut elevation = -f32::from(edit.node.y) / 100.0;
        let mut rise = -f32::from(edit.node.dy) / 100.0;
        let mut slope = match (t2, edit.node.attributes & 0xc000) {
            (false, 0x4000) | (true, 0x8000) => 1,
            (false, 0x8000) | (true, 0xc000) => 2,
            _ => 0,
        };
        egui::Grid::new("campaign-terrain-fields")
            .num_columns(2)
            .spacing([12.0, 10.0])
            .show(ui, |ui| {
                ui.label("Elevation");
                if ui
                    .add(
                        egui::DragValue::new(&mut elevation)
                            .speed(0.1)
                            .range(-300.0..=300.0)
                            .suffix(" m"),
                    )
                    .changed()
                {
                    edit.node.y = (-elevation * 100.0).round() as i16;
                }
                ui.end_row();
                ui.label("Shape");
                egui::ComboBox::from_id_salt("campaign-slope")
                    .selected_text(["Flat", "Ramp along X", "Ramp along Z"][slope])
                    .show_ui(ui, |ui| {
                        for (i, label) in
                            ["Flat", "Ramp along X", "Ramp along Z"].iter().enumerate()
                        {
                            if ui.selectable_value(&mut slope, i, *label).changed() {
                                edit.node.attributes = (edit.node.attributes & !0xc000)
                                    | match (t2, i) {
                                        (_, 0) => 0,
                                        (false, 1) => 0x4000,
                                        (false, 2) | (true, 1) => 0x8000,
                                        _ => 0xc000,
                                    };
                                if i == 0 {
                                    edit.node.dy = 0;
                                }
                            }
                        }
                    });
                ui.end_row();
                ui.label("Ramp rise");
                if ui
                    .add_enabled(
                        slope != 0,
                        egui::DragValue::new(&mut rise)
                            .speed(0.1)
                            .range(-20.0..=20.0)
                            .suffix(" m"),
                    )
                    .changed()
                {
                    edit.node.dy = (-rise * 100.0).round() as i16;
                }
                ui.end_row();
            });
        ui.horizontal(|ui| {
            if ui.button("Raise 0.5 m").clicked() {
                edit.node.y = edit.node.y.saturating_sub(50).max(-30000);
            }
            if ui.button("Lower 0.5 m").clicked() {
                edit.node.y = edit.node.y.saturating_add(50).min(30000);
            }
        });
        ui.checkbox(&mut edit.geometry, "Move visible terrain with collision");
        ui.checkbox(
            &mut self.campaign_follow_terrain,
            "Keep characters / source props on surface",
        );
        ui.small("Clear this for invisible collision surfaces. Raised sections gain side walls using the source material.");
        ui.separator();
        ui.label("Solid area · click quarters to cut / restore");
        egui::Grid::new("campaign-quarters")
            .spacing([4.0, 4.0])
            .show(ui, |ui| {
                for z in 0..4 {
                    for x in 0..4 {
                        let bit = 1 << (z * 4 + x);
                        if ui
                            .add_enabled(
                                !edit.geometry || original.division & bit != 0,
                                egui::Button::new(if edit.node.division & bit != 0 {
                                    "Solid"
                                } else {
                                    "Hole"
                                })
                                .selected(edit.node.division & bit != 0)
                                .min_size(egui::vec2(42.0, 28.0)),
                            )
                            .clicked()
                        {
                            edit.node.division ^= bit;
                        }
                    }
                    ui.end_row();
                }
            });
        ui.horizontal(|ui| {
            if ui.button("Cut whole surface").clicked() {
                edit.node.division = 0;
            }
            if ui.button("Restore solid area").clicked() {
                edit.node.division = original.division;
            }
        });
        ui.collapsing("Collision attributes", |ui| {
            ui.small("Source flags control walls, water, hazards and slopes. Keep them unless you are intentionally changing collision behavior.");
            ui.add(egui::DragValue::new(&mut edit.node.attributes).hexadecimal(4,false,true).prefix("0x"));
        });
        if ui.button("Restore this surface from disc").clicked() {
            edit.node = original;
            campaign.edit_surface(edit, self.campaign_follow_terrain);
        } else if edit != before {
            campaign.edit_surface(edit, self.campaign_follow_terrain);
        }
        ui.small(format!("{} edited surfaces", campaign.terrain.len()));
    }

    pub(in crate::level_editor) fn campaign_characters(&mut self, ui: &mut egui::Ui) {
        ui.heading("Campaign characters");
        let cursor = self.campaign_cursor;
        let campaign = self.project.campaign.as_mut().unwrap();
        egui::ScrollArea::vertical()
            .id_salt("campaign-actors")
            .max_height(180.0)
            .show(ui, |ui| {
                if ui
                    .selectable_label(self.actor == 0, "0 · Player start")
                    .clicked()
                {
                    self.actor = 0;
                    self.tool = Tool::Select;
                }
                for (i, actor) in campaign.actors.iter().enumerate() {
                    if ui
                        .selectable_label(
                            self.actor == i + 1,
                            format!(
                                "{} · {}{}",
                                i + 1,
                                actor.name,
                                if actor.enabled { "" } else { " (disabled)" }
                            ),
                        )
                        .clicked()
                    {
                        self.actor = i + 1;
                        self.tool = Tool::Select;
                    }
                }
            });
        ui.separator();
        if self.actor == 0 {
            coordinates(ui, "start", &mut campaign.start);
            facing(ui, &mut campaign.facing);
        } else if let Some(actor) = campaign.actors.get_mut(self.actor - 1) {
            ui.text_edit_singleline(&mut actor.name);
            ui.checkbox(&mut actor.enabled, "Spawn in playtest");
            ui.checkbox(&mut actor.story, "Protected story character");
            if !actor.enabled {
                ui.small("Off-map source characters are retained but disabled. Move onto the map before enabling.");
            }
            coordinates(ui, "actor", &mut actor.spawn.position);
            facing(ui, &mut actor.spawn.facing);
            ui.small(format!("Source character kind: {}", actor.source_kind));
            ui.collapsing("Gaisen behavior", |ui| {
                ui.small("The source T2 appearance uses the Studio behavior adapter. Original T2 AI and campaign scripts are not simulated here.");
                let current = actor.spawn.think;
                egui::ComboBox::from_id_salt("campaign-think").selected_text(match current { 0x3454 => "Stand guard".into(), 0x3453=>"Patrol".into(), n=>format!("Source behavior 0x{n:04x}") }).show_ui(ui, |ui| {
                    ui.selectable_value(&mut actor.spawn.think, current, "Keep current");
                    ui.selectable_value(&mut actor.spawn.think, 0x3454, "Stand guard");
                    ui.selectable_value(&mut actor.spawn.think, 0x3453, "Patrol");
                });
            });
            ui.separator();
            ui.label("Patrol route");
            if ui
                .selectable_label(self.tool == Tool::Patrol, "Click terrain to add waypoints")
                .clicked()
            {
                self.tool = Tool::Patrol;
            }
            ui.horizontal(|ui| {
                if ui.button("Remove last waypoint").clicked() {
                    actor.spawn.patrol.pop();
                }
                if ui.button("Clear route").clicked() {
                    actor.spawn.patrol.clear();
                }
            });
            for (i, (p, wait)) in actor.spawn.patrol.iter_mut().enumerate() {
                ui.push_id(i, |ui| {
                    ui.collapsing(format!("{} · X {} / Z {}", i + 1, p[0], p[2]), |ui| {
                        coordinates(ui, "waypoint", p);
                        ui.add(
                            egui::DragValue::new(wait)
                                .range(0..=32767)
                                .prefix("Wait (source units) "),
                        );
                    });
                });
            }
        }
        ui.label("Click a marker to select; click terrain to move. F frames the selection.");
        let mut model = self
            .project
            .actors
            .get(&self.actor)
            .cloned()
            .unwrap_or_default();
        egui::ComboBox::from_id_salt("campaign-appearance")
            .width(ui.available_width())
            .selected_text(
                self.asset_names
                    .iter()
                    .find(|(id, _)| *id == model)
                    .map_or("Source appearance", |(_, n)| n.as_str()),
            )
            .show_ui(ui, |ui| {
                ui.selectable_value(&mut model, String::new(), "Gaisen source appearance");
                for (id, name) in &self.asset_names {
                    ui.selectable_value(&mut model, id.clone(), name);
                }
            });
        if model.is_empty() {
            self.project.actors.remove(&self.actor);
        } else {
            self.project.actors.insert(self.actor, model);
        }
        ui.separator();
        let kinds: Vec<_> = self
            .campaign_cache
            .as_ref()
            .into_iter()
            .flat_map(|c| c.mission.models.keys().copied())
            .filter(|k| *k > 6 && *k != 0xa9)
            .collect();
        if !kinds.contains(&self.actor_kind)
            && let Some(k) = kinds.first()
        {
            self.actor_kind = *k;
        }
        egui::ComboBox::from_id_salt("campaign-add-kind")
            .selected_text(format!("Kind {}", self.actor_kind))
            .show_ui(ui, |ui| {
                for k in kinds {
                    ui.selectable_value(&mut self.actor_kind, k, format!("Gaisen character {k}"));
                }
            });
        if ui
            .add_enabled(
                campaign.actors.len() < 64,
                egui::Button::new("Add character at cursor"),
            )
            .clicked()
        {
            campaign.actors.push(Actor {
                enabled: true,
                story: false,
                name: format!("Character {}", self.actor_kind),
                source_kind: self.actor_kind,
                spawn: Enemy {
                    slot: campaign.actors.len(),
                    kind: self.actor_kind,
                    think: 0x3454,
                    position: cursor,
                    facing: 0,
                    patrol: vec![],
                },
            });
            self.actor = campaign.actors.len();
            self.tool = Tool::Select;
        }
        if ui
            .add_enabled(
                self.actor > 0,
                egui::Button::new("Delete selected character"),
            )
            .clicked()
        {
            if let Err(e) = self.project.remove_actor(self.actor) {
                self.notice = e;
            } else {
                self.actor = 0;
            }
        }
    }

    pub(in crate::level_editor) fn campaign_scenery(&mut self, ui: &mut egui::Ui) {
        let campaign = self.project.campaign.as_mut().unwrap();
        let mut ground = false;
        ui.collapsing(format!("Original map props · {}", campaign.props.len()), |ui| {
            ui.small("These placements use the campaign's own object library. Solid walls and floors are edited separately under Terrain.");
            egui::ScrollArea::vertical().id_salt("campaign-props").max_height(180.0).show(ui, |ui| {
                for (i, p) in campaign.props.iter().enumerate() {
                    let name = self.campaign_cache.as_ref().and_then(|c| c.mission.object_names.get(p.part)).map_or("Object", String::as_str);
                    if ui.selectable_label(self.campaign_prop==Some(i), format!("{i} · {name}")).clicked() { self.campaign_prop=Some(i); self.scenery_selection=None; self.tool=Tool::Select; }
                }
            });
            if let Some(i) = self.campaign_prop.filter(|i| *i < campaign.props.len()) {
                let prop=&mut campaign.props[i];
                coordinates(ui,"source-prop",&mut prop.position);
                ui.add(egui::DragValue::new(&mut prop.facing).range(0..=4095).clamp_existing_to_range(false).prefix("Yaw "));
                ui.horizontal(|ui| {
                    if ui.button("Duplicate").clicked() && campaign.props.len()<8192 { campaign.props.push(campaign.props[i].clone()); self.campaign_prop=Some(campaign.props.len()-1); }
                    if ui.button("Delete").clicked() { campaign.props.remove(i); self.campaign_prop=None; }
                });
                if ui.button("Ground on terrain · End").clicked() { ground = true; }
                ui.small("Click terrain to move · F: frame · R: rotate · arrows: nudge · Ctrl+D: duplicate · Delete: remove");
            }
        });
        if ground {
            self.ground_scenery();
        }
    }

    pub(in crate::level_editor) fn campaign_prop_keys(&mut self, input: &egui::InputState) {
        let Some(i) = self.campaign_prop.filter(|i| {
            self.project
                .campaign
                .as_ref()
                .is_some_and(|c| *i < c.props.len())
        }) else {
            return;
        };
        if input.key_pressed(egui::Key::Delete)
            || input.modifiers.command && input.key_pressed(egui::Key::D)
        {
            self.history.begin(&self.project);
            let campaign = self.project.campaign.as_mut().unwrap();
            if input.key_pressed(egui::Key::Delete) {
                campaign.props.remove(i);
                self.campaign_prop = None;
            } else if campaign.props.len() < 8192 {
                campaign.props.push(campaign.props[i].clone());
                self.campaign_prop = Some(campaign.props.len() - 1);
            }
            self.commit();
        }
    }

    pub(in crate::level_editor) fn campaign_mission_ui(&mut self, ui: &mut egui::Ui) {
        let campaign = self.project.campaign.as_mut().unwrap();
        ui.heading("Campaign project");
        ui.label(campaign.source.label());
        egui::Grid::new("campaign-metadata")
            .num_columns(2)
            .spacing([12.0, 10.0])
            .show(ui, |ui| {
                ui.label("Title");
                ui.text_edit_singleline(&mut self.project.mission.title);
                ui.end_row();
                ui.label("Author");
                ui.text_edit_singleline(&mut self.project.mission.author);
                ui.end_row();
                ui.label("Edit password");
                ui.add(
                    egui::TextEdit::singleline(&mut self.project.mission.password).password(true),
                );
                ui.end_row();
            });
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.project.character, 0, "Rikimaru");
            ui.selectable_value(&mut self.project.character, 1, "Ayame");
        });
        ui.separator();
        ui.label("Playtest rules: Gaisen");
        ui.small("Create objectives, timed cues, success and failure conditions in Events & audio. Playtests never advance your campaign save. Source scripts, cutscenes and T2 dynamic scenery are not imported.");
        ui.collapsing(format!("Item pickups · {}", campaign.pickups.len()), |ui| {
            for (i, p) in campaign.pickups.iter().enumerate() {
                if ui
                    .selectable_label(
                        self.campaign_pickup == Some(i),
                        format!("{i} · item {}", p.kind),
                    )
                    .clicked()
                {
                    self.campaign_pickup = Some(i);
                }
            }
            if ui.button("Add pickup at cursor").clicked() && campaign.pickups.len() < 256 {
                campaign.pickups.push(Pickup {
                    kind: 0,
                    position: self.campaign_cursor,
                });
                self.campaign_pickup = Some(campaign.pickups.len() - 1);
            }
            if let Some(i) = self.campaign_pickup.filter(|i| *i < campaign.pickups.len()) {
                let p = &mut campaign.pickups[i];
                ui.add(
                    egui::DragValue::new(&mut p.kind)
                        .range(0..=24)
                        .prefix("Item kind "),
                );
                coordinates(ui, "pickup", &mut p.position);
                if ui.button("Delete pickup").clicked() {
                    campaign.pickups.remove(i);
                    self.campaign_pickup = None;
                }
            }
        });
    }
}

fn coordinates(ui: &mut egui::Ui, id: &str, p: &mut [i32; 3]) {
    egui::Grid::new(id)
        .num_columns(2)
        .spacing([12.0, 8.0])
        .show(ui, |ui| {
            for (v, label) in p.iter_mut().zip(["X", "Y", "Z"]) {
                ui.label(label);
                ui.add(
                    egui::DragValue::new(v)
                        .speed(100)
                        .range(-320000..=320000)
                        .clamp_existing_to_range(false),
                );
                ui.end_row();
            }
        });
    ui.small("1000 units = 1 metre · Y increases downward");
}
fn facing(ui: &mut egui::Ui, value: &mut i16) {
    ui.add(
        egui::DragValue::new(value)
            .range(0..=4095)
            .clamp_existing_to_range(false)
            .prefix("Facing "),
    );
}

fn campaign_sources(assets: &GameAssets<DiscSource>) -> Result<Vec<(Source, String)>> {
    let mut out = vec![];
    for stage in assets.stages()? {
        for layout in 1..=3 {
            out.push((
                Source::Tenchu {
                    stage: stage.id,
                    layout,
                },
                format!("T1 · {} · layout {layout}", stage.name),
            ));
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    for (edition, path) in crate::tenchu2::menu::known_discs()
        .into_iter()
        .filter(|(_, p)| p.exists())
    {
        let disc = crate::source::open_disc(&path)?;
        for layout in tenchu_assets::tenchu2::layout::Layout::available(&disc)? {
            if layout.map == 7 {
                continue;
            }
            let source = Source::Tenchu2 {
                map: layout.map,
                sequence: layout.sequence,
                edition,
            };
            out.push((source.clone(), source.label()));
        }
    }
    Ok(out)
}
