use super::*;
use project::{Action, Cue, Event, Game, Trigger};

impl Editor {
    pub(super) fn events(&mut self, ui: &mut egui::Ui) {
        ui.heading("Events & cue queues");
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    self.project.events.len() < 256,
                    egui::Button::new("Add event"),
                )
                .clicked()
            {
                self.event = self.project.events.len();
                self.project.events.push(Event::default());
            }
            if ui
                .add_enabled(
                    self.event < self.project.events.len(),
                    egui::Button::new("Delete event"),
                )
                .clicked()
            {
                self.project.events.remove(self.event);
                self.event = self.event.saturating_sub(1);
            }
        });
        for (i, event) in self.project.events.iter().enumerate() {
            ui.selectable_value(&mut self.event, i, &event.name);
        }
        let Some(event) = self.project.events.get_mut(self.event) else {
            return;
        };
        ui.separator();
        ui.text_edit_singleline(&mut event.name);
        ui.horizontal(|ui| {
            ui.checkbox(&mut event.enabled, "Enabled");
            ui.checkbox(&mut event.once, "Run once");
        });
        let mut trigger = match event.trigger {
            Trigger::Start => 0,
            Trigger::AfterTicks(_) => 1,
            Trigger::Enter { .. } => 2,
            Trigger::Defeated(_) => 3,
            Trigger::Flag { .. } => 4,
        };
        let before = trigger;
        egui::ComboBox::from_id_salt("studio-trigger")
            .selected_text(
                [
                    "Mission starts",
                    "Elapsed time",
                    "Player enters region",
                    "Character defeated",
                    "Flag changes",
                ][trigger],
            )
            .show_ui(ui, |ui| {
                for (i, name) in [
                    "Mission starts",
                    "Elapsed time",
                    "Player enters region",
                    "Character defeated",
                    "Flag changes",
                ]
                .iter()
                .enumerate()
                {
                    ui.selectable_value(&mut trigger, i, *name);
                }
            });
        if trigger != before {
            let [x, _, z] = self.cursor.world();
            event.trigger = match trigger {
                0 => Trigger::Start,
                1 => Trigger::AfterTicks(30),
                2 => Trigger::Enter {
                    min: [x - 1000, -10_000, z - 1000],
                    max: [x + 1000, 1000, z + 1000],
                },
                3 => Trigger::Defeated(self.actor.max(1)),
                _ => Trigger::Flag {
                    name: "gate_open".into(),
                    value: true,
                },
            };
        }
        match &mut event.trigger {
            Trigger::AfterTicks(ticks) => {
                ticks_control(ui, ticks, "After");
            }
            Trigger::Enter { min, max } => {
                ui.small(
                    "World coordinates; Y increases downward. Region boundaries are inclusive.",
                );
                for axis in 0..3 {
                    ui.horizontal(|ui| {
                        ui.label(["X", "Y", "Z"][axis]);
                        ui.add(
                            egui::DragValue::new(&mut min[axis])
                                .range(-320_000..=320_000)
                                .speed(100),
                        );
                        ui.label("to");
                        ui.add(
                            egui::DragValue::new(&mut max[axis])
                                .range(-320_000..=320_000)
                                .speed(100),
                        );
                    });
                }
            }
            Trigger::Defeated(slot) => {
                ui.add(
                    egui::DragValue::new(slot)
                        .range(1..=self.project.mission.placements.len() - 1)
                        .prefix("Character "),
                );
            }
            Trigger::Flag { name, value } => flag(ui, name, value),
            Trigger::Start => (),
        }
        ui.small("Repeating regions and flags trigger on entry/change. Time and delays use 30 logic ticks per second.");
        ui.separator();
        ui.heading("Cue queue");
        ui.small("Each delay follows the previous cue. Simultaneous cues run in list order.");
        let mut remove = None;
        let mut swap = None;
        for (i, cue) in event.cues.iter_mut().enumerate() {
            ui.push_id(i, |ui| {
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.strong(format!("Cue {}", i + 1));
                        if i > 0 && ui.small_button("↑").clicked() {
                            swap = Some((i - 1, i));
                        }
                        if ui.small_button("Remove").clicked() {
                            remove = Some(i);
                        }
                    });
                    ticks_control(ui, &mut cue.delay, "Delay");
                    if let Some(command) = action(
                        ui,
                        &mut cue.action,
                        &self.sound_catalog,
                        &self.music_names,
                        &self.project.assets,
                    ) {
                        self.pending = Some(command);
                    }
                    if matches!(
                        cue.action,
                        Action::Sound { .. }
                            | Action::Music { .. }
                            | Action::StopMusic
                            | Action::CustomSound { .. }
                            | Action::CustomMusic { .. }
                    ) && ui.button("Audition cue").clicked()
                    {
                        self.pending = Some(Command::Preview(cue.action.clone()));
                    }
                });
            });
        }
        if let Some(i) = remove {
            event.cues.remove(i);
        } else if let Some((a, b)) = swap {
            event.cues.swap(a, b);
        }
        if ui
            .add_enabled(event.cues.len() < 128, egui::Button::new("Add cue"))
            .clicked()
        {
            event.cues.push(Cue {
                delay: 30,
                action: Action::Sound {
                    game: Game::Tenchu,
                    bank: 0,
                    id: 11,
                    volume: 100,
                    pan: 64,
                },
            });
        }
    }
}

fn ticks_control(ui: &mut egui::Ui, ticks: &mut u32, name: &str) {
    ui.horizontal(|ui| {
        ui.add(
            egui::DragValue::new(ticks)
                .range(0..=108_000)
                .prefix(format!("{name} "))
                .suffix(" ticks"),
        );
        ui.small(format!("{:.2} s", *ticks as f32 / 30.0));
    });
}

fn flag(ui: &mut egui::Ui, name: &mut String, value: &mut bool) {
    ui.text_edit_singleline(name);
    ui.checkbox(value, "Flag is set");
}

fn game(ui: &mut egui::Ui, game: &mut Game) {
    ui.horizontal(|ui| {
        ui.selectable_value(game, Game::Tenchu, "Tenchu");
        ui.selectable_value(game, Game::Tenchu2, "Tenchu 2");
    });
}

fn action(
    ui: &mut egui::Ui,
    action: &mut Action,
    sounds: &std::collections::BTreeMap<(Game, usize), Vec<(u16, String)>>,
    music: &[String],
    assets: &custom::Assets,
) -> Option<Command> {
    let mut command = None;
    let mut kind = match action {
        Action::Message { .. } => 0,
        Action::Sound { .. } => 1,
        Action::Music { .. } => 2,
        Action::StopMusic => 3,
        Action::Weather(_) => 4,
        Action::TimeOfDay(_) => 5,
        Action::Flag { .. } => 6,
        Action::Complete => 7,
        Action::Fail => 8,
        Action::CustomSound { .. } => 9,
        Action::CustomMusic { .. } => 10,
    };
    let before = kind;
    let names = [
        "Show message",
        "Sound effect",
        "Music",
        "Stop music",
        "Change weather",
        "Time of day",
        "Set flag",
        "Complete mission",
        "Fail mission",
        "Custom sound effect",
        "Custom music",
    ];
    egui::ComboBox::from_id_salt("action-kind")
        .selected_text(names[kind])
        .show_ui(ui, |ui| {
            for (i, name) in names.iter().enumerate() {
                ui.add_enabled_ui(
                    i < 9 || assets.values().any(|a| a.kind == custom::Kind::Audio),
                    |ui| {
                        ui.selectable_value(&mut kind, i, *name);
                    },
                )
                .response
                .on_disabled_hover_text("Import audio in the Assets workspace first");
            }
        });
    if kind != before {
        *action = match kind {
            0 => Action::Message {
                text: "Message".into(),
                ticks: 120,
            },
            1 => Action::Sound {
                game: Game::Tenchu,
                bank: 0,
                id: 11,
                volume: 100,
                pan: 64,
            },
            2 => Action::Music {
                game: Game::Tenchu,
                track: 0,
            },
            3 => Action::StopMusic,
            4 => Action::Weather(crate::settings::WeatherMode::Rain),
            5 => Action::TimeOfDay(720),
            6 => Action::Flag {
                name: "gate_open".into(),
                value: true,
            },
            7 => Action::Complete,
            8 => Action::Fail,
            9 => Action::CustomSound {
                asset: assets
                    .iter()
                    .find(|(_, a)| a.kind == custom::Kind::Audio)
                    .map_or(String::new(), |(id, _)| id.clone()),
                volume: 100,
                pan: 64,
            },
            _ => Action::CustomMusic {
                asset: assets
                    .iter()
                    .find(|(_, a)| a.kind == custom::Kind::Audio)
                    .map_or(String::new(), |(id, _)| id.clone()),
            },
        };
    }
    match action {
        Action::CustomMusic { asset } => custom::ui::audio_choice(ui, asset, assets),
        Action::CustomSound { asset, volume, pan } => {
            custom::ui::audio_choice(ui, asset, assets);
            ui.add(egui::Slider::new(volume, 0..=127).text("Volume"));
            ui.add(egui::Slider::new(pan, 0..=127).text("Pan"));
        }
        Action::Message { text, ticks } => {
            ui.add(egui::TextEdit::multiline(text).desired_rows(3));
            ticks_control(ui, ticks, "Display");
            *ticks = (*ticks).max(1);
        }
        Action::Sound {
            game: source,
            bank,
            id,
            volume,
            pan,
        } => {
            game(ui, source);
            ui.add(
                egui::DragValue::new(bank)
                    .range(0..=if *source == Game::Tenchu { 18 } else { 23 })
                    .prefix("Bank "),
            );
            ui.add(egui::DragValue::new(id).range(0..=2047).prefix("Sound ID "));
            if ui.button("Browse sounds in this bank").clicked() {
                command = Some(Command::BrowseSounds(*source, *bank));
            }
            if let Some(choices) = sounds.get(&(*source, *bank)) {
                egui::ComboBox::from_id_salt("sound-sample")
                    .selected_text(
                        choices
                            .iter()
                            .find(|(n, _)| n == id)
                            .map_or("Choose a sound…", |(_, label)| label.as_str()),
                    )
                    .show_ui(ui, |ui| {
                        for (value, label) in choices {
                            ui.selectable_value(id, *value, label);
                        }
                    });
            }
            ui.add(egui::Slider::new(volume, 0..=127).text("Volume"));
            ui.add(egui::Slider::new(pan, 0..=127).text("Pan"));
        }
        Action::Music {
            game: source,
            track,
        } => {
            game(ui, source);
            let names: Vec<_> = if *source == Game::Tenchu {
                music.iter().map(String::as_str).collect()
            } else {
                tenchu_assets::tenchu2::music::THEMES
                    .iter()
                    .map(|(_, name)| *name)
                    .collect()
            };
            *track = (*track).min(names.len().saturating_sub(1));
            egui::ComboBox::from_id_salt("music-track")
                .selected_text(names.get(*track).copied().unwrap_or("Choose music…"))
                .show_ui(ui, |ui| {
                    for (i, name) in names.iter().enumerate() {
                        ui.selectable_value(track, i, *name);
                    }
                });
        }
        Action::Weather(mode) => {
            egui::ComboBox::from_id_salt("cue-weather")
                .selected_text(mode.name())
                .show_ui(ui, |ui| {
                    for value in crate::settings::WeatherMode::ALL {
                        ui.selectable_value(mode, value, value.name());
                    }
                });
        }
        Action::TimeOfDay(time) => {
            ui.add(egui::Slider::new(time, 0..=1439).text("Minute of day"));
        }
        Action::Flag { name, value } => flag(ui, name, value),
        Action::StopMusic | Action::Complete | Action::Fail => (),
    }
    command
}
