use super::{Edition, Event, Menu};
use crate::campaign::{Action, Campaign};
use tenchu_assets::{script::Language, tenchu2::music::THEMES};
use tenchu_core::ai::Difficulty;

impl Menu {
    pub fn refresh_discs(&mut self) {
        #[cfg(not(target_arch = "wasm32"))]
        {
            self.discs = super::known_discs()
                .into_iter()
                .map(|(edition, path)| (edition, path.to_string_lossy().into_owned()))
                .collect();
        }
        self.initialized = true;
    }

    pub fn navigate(&mut self, action: Action, campaign: &mut Campaign) -> (bool, Option<Event>) {
        if !self.initialized {
            self.refresh_discs();
        }
        let rows: &[usize] = if self.native() {
            &[7, 4, 5, 3, 11, 1, 12, 8]
        } else if self.selected == Some(Edition::Japan) {
            &[7, 4, 5, 2, 3, 6, 9, 10, 11, 1, 8]
        } else if self.selected.is_some() {
            &[7, 4, 5, 2, 3, 6, 9, 10, 1, 8]
        } else if cfg!(target_arch = "wasm32") {
            &[4, 5, 2, 3, 6, 1, 0]
        } else {
            &[7, 4, 5, 2, 3, 6, 1, 0, 8]
        };
        match action {
            Action::Up | Action::Down => {
                let index = rows.iter().position(|&r| r == campaign.cursor).unwrap_or(0);
                let delta = if matches!(action, Action::Down) {
                    1
                } else {
                    -1
                };
                campaign.cursor =
                    rows[(index as i32 + delta).rem_euclid(rows.len() as i32) as usize];
            }
            Action::Left | Action::Right | Action::Confirm | Action::Start => {
                let delta = if matches!(action, Action::Left) {
                    -1
                } else {
                    1
                };
                match campaign.cursor {
                    7 => {
                        let choices: Vec<_> = std::iter::once(None)
                            .chain(self.discs.iter().map(|(e, _)| Some(*e)))
                            .collect();
                        let selected = cycle(&choices, &self.selected, delta);
                        return (true, Some(Event::Select(selected)));
                    }
                    8 => return (true, Some(Event::Open)),
                    12 if self.native() && matches!(action, Action::Confirm | Action::Start) => {
                        return (true, Some(Event::MissionSelection));
                    }
                    1 if self.selected.is_some()
                        && matches!(action, Action::Confirm | Action::Start) =>
                    {
                        self.character = campaign.options.character;
                        return (true, Some(Event::Play));
                    }
                    4 if self.selected.is_some() => {
                        if !self.maps.is_empty() {
                            self.map = cycle(&self.maps, &self.map, delta);
                            self.select_map();
                        }
                    }
                    5 if self.selected.is_some() => {
                        let layouts: Vec<_> = self
                            .layouts
                            .iter()
                            .filter(|l| l.map == self.map)
                            .map(|l| Some(l.sequence))
                            .collect();
                        if !layouts.is_empty() {
                            self.sequence = cycle(&layouts, &self.sequence, delta);
                        }
                    }
                    6 if self.selected.is_some() => {
                        let choices: Vec<_> =
                            (0..self.music_tracks).map(Some).chain([None]).collect();
                        self.music = cycle(&choices, &self.music, delta);
                    }
                    9 => {
                        self.guard = cycle(
                            &[None, Some(0x10), Some(0x12), Some(0x11)],
                            &self.guard,
                            delta,
                        )
                    }
                    10 => self.source_models = !self.source_models,
                    11 => self.sandbox = !self.sandbox,
                    _ => return (false, None),
                }
            }
            _ => return (false, None),
        }
        (true, None)
    }

    fn select_map(&mut self) {
        self.sequence = self
            .layouts
            .iter()
            .find(|l| l.map == self.map)
            .map(|l| l.sequence);
    }

    pub fn show_launcher(&mut self, ctx: &egui::Context, campaign: &Campaign) -> Option<Event> {
        if !self.initialized {
            self.refresh_discs();
        }
        let mut event = None;
        let mut options = campaign.options;
        let width = (ctx.screen_rect().width() - 40.0).clamp(260.0, 560.0);
        egui::Window::new("Quick launcher · F6")
            .id("quick-launcher".into())
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false).resizable(false)
            .fixed_size(egui::vec2(width, (ctx.screen_rect().height() - 70.0).min(590.0)))
            .show(ctx, |ui| {
                ui.style_mut().wrap_mode = Some(egui::TextWrapMode::Wrap);
                egui::ScrollArea::vertical().show(ui, |ui| {
                    let selected = |row, name: &str| {
                        if campaign.cursor == row { format!("> {name}") } else { name.to_owned() }
                    };
                    if !cfg!(target_arch = "wasm32") {
                        ui.horizontal(|ui| {
                            ui.label(selected(7, "Game"));
                            let mut game = self.selected;
                            egui::ComboBox::from_id_salt("quick-game").selected_text(game.map_or("Shinobi Gaisen", Edition::name)).show_ui(ui, |ui| {
                                ui.selectable_value(&mut game, None, "Shinobi Gaisen");
                                for &(edition, _) in &self.discs {
                                    ui.selectable_value(&mut game, Some(edition), edition.name());
                                }
                            });
                            if game != self.selected { event = Some(Event::Select(game)); }
                        });
                    }
                    ui.separator();
                    if self.selected.is_some() {
                        if self.selected == Some(Edition::Japan) {
                            ui.checkbox(&mut self.sandbox, selected(11, "Use Gaisen scenery sandbox"));
                        }
                        if self.native() {
                            ui.label("Japanese gameplay, actors, voices and campaign progression");
                            ui.small("The quick start sequence determines the player character. Use the original mission selector to start from each mission's opening scene.");
                            if ui.button(selected(12, "Original JP mission selection…")).clicked() { event = Some(Event::MissionSelection); }
                        } else {
                            ui.label("Scenery sandbox · Gaisen gameplay and AI");
                            ui.small("Story events, cutscenes and mission objectives are not simulated here.");
                        }
                        ui.horizontal(|ui| {
                            ui.label(selected(4, "Area"));
                            let old = self.map;
                            egui::ComboBox::from_id_salt("quick-area").selected_text(format!("Area {}", self.map)).show_ui(ui, |ui| {
                                for &map in &self.maps { ui.selectable_value(&mut self.map, map, format!("Area {map}")); }
                            });
                            if old != self.map { self.select_map(); }
                        });
                        ui.horizontal(|ui| {
                            ui.label(selected(5, "Patrol layout"));
                            egui::ComboBox::from_id_salt("quick-sequence").selected_text(self.sequence.map_or_else(|| "None".into(), |s| format!("Sequence {s}"))).show_ui(ui, |ui| {
                                for layout in self.layouts.iter().filter(|l| l.map == self.map) {
                                    ui.selectable_value(&mut self.sequence, Some(layout.sequence), format!("Sequence {} · {} actors", layout.sequence, layout.actors.len()));
                                }
                            });
                        });
                    } else {
                        ui.horizontal(|ui| {
                            ui.label(selected(4, "Mission"));
                            egui::ComboBox::from_id_salt("quick-mission").selected_text(mission_name(campaign, options.mission)).show_ui(ui, |ui| {
                                for mission in 0..=10 { ui.selectable_value(&mut options.mission, mission, mission_name(campaign, mission)); }
                            });
                        });
                        ui.horizontal(|ui| {
                            ui.label(selected(5, "Layout"));
                            egui::ComboBox::from_id_salt("quick-layout").selected_text(layout_name(options.layout)).show_ui(ui, |ui| {
                                for layout in 0..=3 { ui.selectable_value(&mut options.layout, layout, layout_name(layout)); }
                            });
                        });
                    }
                    if !self.native() { ui.horizontal(|ui| {
                        ui.label(selected(2, "Character"));
                        ui.selectable_value(&mut options.character, 0, "Rikimaru");
                        ui.selectable_value(&mut options.character, 1, "Ayame");
                    }); }
                    ui.horizontal(|ui| {
                        ui.label(selected(3, "Difficulty"));
                        for difficulty in [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard] {
                            ui.selectable_value(&mut options.difficulty, difficulty, format!("{difficulty:?}"));
                        }
                    });
                    if self.selected.is_some() && !self.native() {
                        ui.horizontal(|ui| {
                            ui.label(selected(6, "Music"));
                            egui::ComboBox::from_id_salt("quick-music").selected_text(self.music.and_then(|i| THEMES.get(i)).map_or("Stage ambience (retail)", |&(_, name)| name)).show_ui(ui, |ui| {
                                for (i, &(_, name)) in THEMES.iter().take(self.music_tracks).enumerate() { ui.selectable_value(&mut self.music, Some(i), name); }
                                ui.selectable_value(&mut self.music, None, "Stage ambience (retail)");
                            });
                        });
                        ui.horizontal_wrapped(|ui| {
                            ui.label(selected(9, "Guards"));
                            for (kind, name) in [(None, "None"), (Some(0x10), "Sword"), (Some(0x12), "Spear"), (Some(0x11), "Archer")] {
                                ui.selectable_value(&mut self.guard, kind, name);
                            }
                        });
                        ui.checkbox(&mut self.source_models, selected(10, "Use Tenchu 2 guard models"));
                    } else if self.selected.is_none() {
                        ui.horizontal(|ui| {
                            ui.label(selected(6, "Language"));
                            let mut language = options.language.unwrap_or_default();
                            egui::ComboBox::from_id_salt("quick-language").selected_text(format!("{language:?}")).show_ui(ui, |ui| {
                                for language_option in [Language::English, Language::French, Language::Italian, Language::Japanese] {
                                    ui.selectable_value(&mut language, language_option, format!("{language_option:?}"));
                                }
                            });
                            if language != options.language.unwrap_or_default() { options.language = Some(language); }
                        });
                    }
                    ui.separator();
                    ui.horizontal_wrapped(|ui| {
                        if ui.button(selected(1, if self.selected.is_some() && !self.native() { "Explore area" } else { "Start mission" })).clicked() {
                            self.character = options.character;
                            event = Some(if self.selected.is_some() { Event::Play } else { Event::Gaisen(false) });
                        }
                        if self.selected.is_none() && ui.add_enabled(campaign.saved.is_some(), egui::Button::new(selected(0, "Continue saved game"))).clicked() {
                            event = Some(Event::Gaisen(true));
                        }
                        if !cfg!(target_arch = "wasm32") && ui.button(selected(8, "Connect another disc…")).clicked() { event = Some(Event::Open); }
                    });
                    ui.small("Arrows / D-pad: select and change · Confirm: start · Back: title · F1: settings · F7: randomizer");
                    if !self.notice.is_empty() { ui.small(&self.notice); }
                });
            });
        if options != campaign.options {
            return Some(Event::Options(options));
        }
        event
    }
}

fn cycle<T: Copy + PartialEq>(values: &[T], selected: &T, delta: i32) -> T {
    let index = values.iter().position(|v| v == selected).unwrap_or(0);
    values[(index as i32 + delta).rem_euclid(values.len() as i32) as usize]
}

fn mission_name(campaign: &Campaign, mission: usize) -> String {
    if mission == 0 {
        return "Training".into();
    }
    campaign
        .stages
        .iter()
        .find(|s| s.order == mission as i32)
        .map_or_else(
            || format!("Mission {mission}"),
            |s| format!("{mission}. {}", s.name),
        )
}

fn layout_name(layout: usize) -> String {
    if layout == 0 {
        "Random".into()
    } else {
        layout.to_string()
    }
}
