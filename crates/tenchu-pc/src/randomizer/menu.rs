use super::*;
use egui::{Color32, RichText, Ui};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, path::PathBuf};

pub enum Event {
    Start(Box<Run>),
    End,
}

impl Event {
    pub fn start(run: Run) -> Self {
        Self::Start(Box::new(run))
    }
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Saved {
    setup: Run,
    current: Option<Run>,
}

pub struct Editor {
    pub open: bool,
    pub setup: Run,
    pub current: Option<Run>,
    pub notice: String,
    tab: usize,
    search: String,
    share: String,
    path: PathBuf,
}

impl Editor {
    pub fn new(path: PathBuf) -> Self {
        let mut editor = Self {
            open: false,
            setup: Run::new(Document::default()),
            current: None,
            notice: String::new(),
            tab: 0,
            search: String::new(),
            share: String::new(),
            path,
        };
        match crate::storage::read(&editor.path) {
            Ok(bytes) => match serde_json::from_slice::<Saved>(&bytes) {
                Ok(saved) if bytes.len() <= 131072 => {
                    editor.setup = saved.setup;
                    editor.current = saved.current;
                }
                _ => editor.notice = "Could not read randomizer preferences; using defaults".into(),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => (),
            Err(error) => editor.notice = format!("Randomizer preferences: {error}"),
        }
        editor
    }

    pub fn save(&mut self) {
        let saved = Saved {
            setup: self.setup.clone(),
            current: self.current.clone(),
        };
        if let Err(error) = serde_json::to_vec_pretty(&saved)
            .map_err(|e| e.to_string())
            .and_then(|bytes| crate::storage::atomic_write(&self.path, &bytes))
        {
            self.notice = format!("Could not save randomizer preferences: {error}");
        }
    }

    pub fn started(&mut self, run: &Run) {
        self.current = Some(run.clone());
        self.save();
        self.open = false;
    }

    pub fn show(
        &mut self,
        ctx: &egui::Context,
        catalog: Option<&Catalog>,
        active: Option<(&Run, &Plan)>,
    ) -> Option<Event> {
        if !self.open {
            return None;
        }
        let before = self.setup.clone();
        let mut open = true;
        let mut event = None;
        let screen = ctx.screen_rect();
        egui::Window::new("Randomizer")
            .open(&mut open)
            .collapsible(false)
            .default_size([750.0, 620.0])
            .min_width(480.0)
            .max_width((screen.width() - 24.0).max(480.0))
            .max_height((screen.height() - 30.0).max(250.0))
            .show(ctx, |ui| {
                ui.label(
                    "A separate seeded run. Normal campaign saves and visual preferences are kept.",
                );
                if let Some((run, plan)) = active {
                    ui.group(|ui| {
                        ui.label(
                            RichText::new(format!(
                                "Playing {} · round {} · mission {} · {}",
                                run.document.seed,
                                u64::from(run.round) + 1,
                                plan.mission,
                                run.document.fingerprint()
                            ))
                            .strong(),
                        );
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Retry same roll").clicked() {
                                event = Some(Event::start(run.clone()));
                            }
                            if ui.button("Next roll").clicked() {
                                match run.next() {
                                    Ok(next) => event = Some(Event::start(next)),
                                    Err(error) => self.notice = error,
                                }
                            }
                            if ui.button("Copy current run").clicked() {
                                self.share = run.export();
                                ctx.copy_text(self.share.clone());
                                self.tab = 6;
                            }
                            if ui.button("End randomizer").clicked() {
                                event = Some(Event::End);
                            }
                        });
                    });
                } else if let Some(run) = &self.current {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!(
                            "Saved run: {} · round {}",
                            run.document.seed,
                            u64::from(run.round) + 1
                        ));
                        if ui.button("Resume mission from start").clicked() {
                            event = Some(Event::start(run.clone()));
                        }
                    });
                }
                ui.separator();
                ui.horizontal_wrapped(|ui| {
                    for (index, label) in [
                        "Setup",
                        "Player",
                        "Enemies",
                        "Stages",
                        "Items",
                        "Preview",
                        "Share seed",
                    ]
                    .iter()
                    .enumerate()
                    {
                        if ui.selectable_label(self.tab == index, *label).clicked() {
                            self.tab = index;
                            self.search.clear();
                        }
                    }
                });
                ui.separator();
                let Some(catalog) = catalog else {
                    ui.spinner();
                    ui.label(
                        "Loading the full disc catalog… Browser assets may still be prefetching.",
                    );
                    return;
                };
                egui::ScrollArea::vertical()
                    .id_salt("randomizer-body")
                    .max_height((screen.height() - 300.0).clamp(180.0, 480.0))
                    .auto_shrink([false, false])
                    .show(ui, |ui| match self.tab {
                        0 => self.setup(ui),
                        1 => self.player(ui, catalog),
                        2 => self.enemies(ui, catalog),
                        3 => self.stages(ui, catalog),
                        4 => self.items(ui),
                        5 => preview(ui, &self.setup, catalog),
                        _ => self.share(ui, ctx, catalog),
                    });
                ui.separator();
                let valid = self.setup.plan(catalog);
                if let Err(error) = &valid {
                    ui.colored_label(Color32::from_rgb(240, 174, 120), error);
                }
                ui.horizontal_wrapped(|ui| {
                    if ui
                        .add_enabled(
                            valid.is_ok(),
                            egui::Button::new(if active.is_some() {
                                "Start edited setup (replaces this attempt)"
                            } else {
                                "Start randomized run"
                            }),
                        )
                        .clicked()
                    {
                        event = Some(Event::start(self.setup.clone()));
                    }
                    ui.weak(format!(
                        "Version 1 · {} · Esc / F7 closes",
                        self.setup.document.fingerprint()
                    ));
                });
                if !self.notice.is_empty() {
                    ui.colored_label(Color32::from_rgb(240, 193, 120), &self.notice);
                }
            });
        self.open &= open;
        if before != self.setup {
            self.save();
        }
        event
    }

    fn setup(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Seed phrase");
            ui.add(
                egui::TextEdit::singleline(&mut self.setup.document.seed)
                    .char_limit(128)
                    .desired_width(290.0),
            );
            if ui.button("New seed").clicked() {
                let time = web_time::SystemTime::now()
                    .duration_since(web_time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos();
                self.setup.document.seed = format!("ninja-{time:x}");
                self.setup.round = 0;
            }
        });
        ui.horizontal(|ui| {
            ui.label("Roll index (0 = first)");
            ui.add(egui::DragValue::new(&mut self.setup.round).range(0..=u32::MAX - 1));
        });
        ui.weak("The same seed, rules and roll reproduce the same start on desktop and browser. Changing one category does not reroll the others.");
        ui.separator();
        ui.horizontal_wrapped(|ui| {
            ui.label("Quick setups:");
            for name in ["Balanced", "Wild", "Boss patrols", "Scavenger"] {
                if ui.button(name).clicked() {
                    let mut rules = Rules::default();
                    match name {
                        "Wild" => {
                            rules.player = Player::Models;
                            rules.enemy_pool = Pool::all();
                            rules.replace_civilians = true;
                            rules.stages = Stages::Tour;
                            rules.health = Some(Range::new(25, 250));
                            rules.random_facing = true;
                            rules.item_types = Range::new(1, 12);
                        }
                        "Boss patrols" => {
                            rules.player = Player::Models;
                            rules.player_pool.groups = [Group::Bosses].into();
                            rules.enemy_pool.groups = [Group::Bosses].into();
                            rules.enemies = Enemies::Uniform;
                            rules.health = Some(Range::new(150, 250));
                            rules.item_types = Range::new(6, 10);
                        }
                        "Scavenger" => {
                            rules.item_types = Range::new(0, 1);
                            rules.item_count = Range::new(1, 2);
                            rules.stages = Stages::Tour;
                        }
                        _ => (),
                    }
                    self.setup.document.rules = rules;
                    self.notice = format!("{name} rules applied; the seed is unchanged");
                }
            }
        });
        let r = &mut self.setup.document.rules;
        ui.separator();
        choice(
            ui,
            "Language",
            &mut r.language,
            &[
                (Language::English, "English"),
                (Language::French, "French"),
                (Language::Italian, "Italian"),
                (Language::Japanese, "Japanese"),
            ],
        );
        choice(
            ui,
            "Difficulty",
            &mut r.difficulty,
            &[
                (Some(0), "Easy"),
                (Some(1), "Normal"),
                (Some(2), "Hard"),
                (None, "Random per roll"),
            ],
        );
        choice(
            ui,
            "Layout",
            &mut r.layout,
            &[(0, "Random"), (1, "1"), (2, "2"), (3, "3")],
        );
        range(ui, "Enemy sight distance (%)", &mut r.sight, 0, 400);
        ui.checkbox(
            &mut r.keep_active,
            "Keep enemies loaded and thinking across the map",
        );
        ui.weak("Graphics remain yours. F1 visual presets and retro mode still work.");
    }

    fn player(&mut self, ui: &mut Ui, catalog: &Catalog) {
        let r = &mut self.setup.document.rules;
        choice(
            ui,
            "Player selection",
            &mut r.player,
            &[
                (Player::Chosen, "Chosen protagonist"),
                (Player::Protagonists, "Random Rikimaru / Ayame"),
                (Player::Models, "Any allowed model"),
            ],
        );
        choice(
            ui,
            "Story / fallback protagonist",
            &mut r.character,
            &[(0, "Rikimaru"), (1, "Ayame")],
        );
        let mut random_health = r.health.is_some();
        if ui
            .checkbox(&mut random_health, "Randomize maximum HP")
            .changed()
        {
            r.health = random_health.then_some(Range::new(75, 150));
        }
        if let Some(health) = &mut r.health {
            range(ui, "Player HP", health, 1, 999);
        }
        ui.checkbox(&mut r.model_moveset, "Use the selected model's moveset");
        ui.checkbox(
            &mut r.player_movement,
            "Keep player walking, running, jumping and grappling",
        );
        ui.checkbox(
            &mut r.no_idle_taunts,
            "Disable idle taunts that lock movement",
        );
        ui.weak("Non-player bodies retain a protagonist's mission scripts and missing actions. Animal rigs and unusual movesets can look strange.");
        ui.separator();
        pool(ui, &mut self.search, &mut r.player_pool, catalog);
    }

    fn enemies(&mut self, ui: &mut Ui, catalog: &Catalog) {
        let r = &mut self.setup.document.rules;
        choice(
            ui,
            "Enemy selection",
            &mut r.enemies,
            &[
                (Enemies::Original, "Keep original bodies"),
                (Enemies::Mixed, "Roll each actor"),
                (Enemies::Uniform, "One model per mission"),
            ],
        );
        range(ui, "Enemy HP (%)", &mut r.enemy_health, 1, 400);
        ui.checkbox(&mut r.random_facing, "Randomize starting facing");
        ui.checkbox(&mut r.replace_civilians, "Also replace ordinary civilians");
        ui.checkbox(
            &mut r.protect_story,
            "Protect story actors, bosses and mission objectives",
        );
        if !r.protect_story {
            ui.colored_label(Color32::YELLOW, "Experimental: changing scripted actors can make a mission impossible. Retry / next / end remain available.");
        }
        ui.weak("Original positions and patrol routes remain. Replacements bring their body, motions, weapons and AI from any mission; story roles stay attached to their original slots.");
        ui.separator();
        pool(ui, &mut self.search, &mut r.enemy_pool, catalog);
    }

    fn stages(&mut self, ui: &mut Ui, catalog: &Catalog) {
        let r = &mut self.setup.document.rules;
        choice(
            ui,
            "Stage selection",
            &mut r.stages,
            &[
                (Stages::Chosen, "Chosen stage"),
                (Stages::Random, "Random per roll"),
                (Stages::Tour, "Shuffled tour (no repeats)"),
            ],
        );
        egui::ComboBox::from_label("Chosen stage")
            .selected_text(stage_name(catalog, r.mission))
            .show_ui(ui, |ui| {
                for (id, _) in &catalog.stages {
                    ui.selectable_value(&mut r.mission, *id, stage_name(catalog, *id));
                }
            });
        ui.weak("A tour visits every allowed stage once before shuffling again. Completing any stage offers the next roll, including mission 10.");
        ui.separator();
        ui.label("Allowed stages (uncheck to blacklist)");
        ui.horizontal(|ui| {
            if ui.small_button("All").clicked() {
                r.blocked_stages.clear();
            }
            if ui.small_button("None").clicked() {
                r.blocked_stages = catalog.stages.iter().map(|s| s.0).collect();
            }
            if ui.small_button("Missions 1–10").clicked() {
                r.blocked_stages = [0].into();
            }
        });
        for (id, _) in &catalog.stages {
            allowed(ui, &mut r.blocked_stages, *id, stage_name(catalog, *id));
        }
    }

    fn items(&mut self, ui: &mut Ui) {
        let r = &mut self.setup.document.rules;
        ui.checkbox(&mut r.equipment, "Randomize starting loadout");
        range(ui, "Different item types", &mut r.item_types, 0, 20);
        ui.weak("The guaranteed grapple is extra; it does not count toward this range.");
        range(ui, "Quantity per item", &mut r.item_count, 1, 99);
        ui.checkbox(
            &mut r.infinite_items,
            "Unlimited quantities for selected consumables",
        );
        ui.checkbox(&mut r.keep_grapple, "Guaranteed grapple (unlimited)");
        ui.checkbox(&mut r.pickups, "Randomize ordinary ground pickups");
        ui.weak("Armour is equipment, not a ground pickup. Special mission objects stay intact. Without a grapple some routes may be unreachable.");
        ui.separator();
        ui.label("Allowed items (uncheck to blacklist)");
        ui.horizontal(|ui| {
            if ui.small_button("All").clicked() {
                r.blocked_items.clear();
            }
            if ui.small_button("Grapple only").clicked() {
                r.blocked_items = (1..20).collect();
                r.item_types = Range::new(0, 0);
                r.pickups = false;
            }
        });
        egui::Grid::new("randomizer-items")
            .num_columns(2)
            .show(ui, |ui| {
                for (id, name) in crate::ui::ITEMS.iter().enumerate() {
                    allowed(ui, &mut r.blocked_items, id, *name);
                    if id % 2 == 1 {
                        ui.end_row();
                    }
                }
            });
    }

    fn share(&mut self, ui: &mut Ui, ctx: &egui::Context, catalog: &Catalog) {
        ui.label("Share the complete seed document, not just the phrase: it includes rules, exclusions, algorithm version and roll index.");
        ui.horizontal_wrapped(|ui| {
            if ui.button("Export setup / copy").clicked() {
                self.share = self.setup.export();
                ctx.copy_text(self.share.clone());
                self.notice = "Seed document copied; it is also available below".into();
            }
            if ui.button("Import text below").clicked() {
                match Run::import(&self.share).and_then(|run| {
                    run.document.validate(catalog)?;
                    Ok(run)
                }) {
                    Ok(run) => {
                        self.setup = run;
                        self.notice =
                            "Imported seed and all rules. Preview or start when ready.".into();
                    }
                    Err(error) => self.notice = error,
                }
            }
            #[cfg(not(target_arch = "wasm32"))]
            {
                if ui.button("Save file…").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("Randomizer seed", &["json"])
                        .set_file_name("tenchu-randomizer.json")
                        .save_file()
                {
                    match std::fs::write(path, self.setup.export()) {
                        Ok(()) => self.notice = "Seed file saved".into(),
                        Err(e) => self.notice = e.to_string(),
                    }
                }
                if ui.button("Open file…").clicked()
                    && let Some(path) = rfd::FileDialog::new()
                        .add_filter("Randomizer seed", &["json"])
                        .pick_file()
                {
                    match std::fs::read_to_string(path)
                        .map_err(|e| e.to_string())
                        .and_then(|text| Run::import(&text))
                        .and_then(|run| {
                            run.document.validate(catalog)?;
                            Ok(run)
                        }) {
                        Ok(run) => {
                            self.share = run.export();
                            self.setup = run;
                            self.notice = "Imported seed file".into();
                        }
                        Err(error) => self.notice = error,
                    }
                }
            }
        });
        ui.add(
            egui::TextEdit::multiline(&mut self.share)
                .code_editor()
                .desired_width(f32::INFINITY)
                .desired_rows(16)
                .char_limit(65536)
                .hint_text("Paste an exported randomizer JSON document here"),
        );
        ui.weak("You can select/copy/paste this text even if the browser blocks clipboard access. Share both the seed document and the game version for an exact gameplay replay.");
    }
}

fn choice<T: Copy + PartialEq>(ui: &mut Ui, label: &str, value: &mut T, choices: &[(T, &str)]) {
    egui::ComboBox::from_label(label)
        .selected_text(
            choices
                .iter()
                .find(|(v, _)| v == value)
                .map_or("Invalid", |(_, name)| *name),
        )
        .show_ui(ui, |ui| {
            for &(candidate, name) in choices {
                ui.selectable_value(value, candidate, name);
            }
        });
}

fn range(ui: &mut Ui, label: &str, value: &mut Range, min: u16, max: u16) {
    ui.horizontal(|ui| {
        ui.label(label);
        if ui
            .add(
                egui::DragValue::new(&mut value.min)
                    .range(min..=max)
                    .prefix("min "),
            )
            .changed()
        {
            value.max = value.max.max(value.min);
        }
        if ui
            .add(
                egui::DragValue::new(&mut value.max)
                    .range(min..=max)
                    .prefix("max "),
            )
            .changed()
        {
            value.min = value.min.min(value.max);
        }
    });
}

fn allowed<T: Ord + Clone>(
    ui: &mut Ui,
    blocked: &mut BTreeSet<T>,
    id: T,
    name: impl Into<egui::WidgetText>,
) {
    let mut enabled = !blocked.contains(&id);
    if ui.checkbox(&mut enabled, name).changed() {
        if enabled {
            blocked.remove(&id);
        } else {
            blocked.insert(id);
        }
    }
}

fn pool(ui: &mut Ui, search: &mut String, pool: &mut Pool, catalog: &Catalog) {
    ui.label(format!(
        "Eligible classes · {} allowed models across the full disc",
        pool.choices(catalog).len()
    ));
    ui.horizontal_wrapped(|ui| {
        for group in Group::ALL {
            let mut enabled = pool.groups.contains(&group);
            if ui.checkbox(&mut enabled, group.name()).changed() {
                if enabled {
                    pool.groups.insert(group);
                } else {
                    pool.groups.remove(&group);
                }
            }
        }
    });
    ui.horizontal(|ui| {
        ui.add(
            egui::TextEdit::singleline(search)
                .hint_text("Search model names…")
                .desired_width(280.0),
        );
        if ui.small_button("Allow all").clicked() {
            *pool = Pool::all();
        }
        if ui.small_button("Block all").clicked() {
            pool.blocked = catalog.actors.iter().map(|a| a.id.clone()).collect();
        }
    });
    ui.weak("Uncheck a model to blacklist it. Search only filters this list; it does not change the pool.");
    let query = search.to_lowercase();
    let mut entries: Vec<_> = catalog
        .actors
        .iter()
        .filter(|a| a.name.to_lowercase().contains(&query))
        .collect();
    entries.sort_by(|a, b| a.name.cmp(&b.name));
    egui::Grid::new("randomizer-actors")
        .num_columns(2)
        .spacing([18.0, 3.0])
        .show(ui, |ui| {
            for (i, entry) in entries.iter().enumerate() {
                ui.add_enabled_ui(pool.groups.contains(&entry.group), |ui| {
                    allowed(ui, &mut pool.blocked, entry.id.clone(), &entry.name);
                });
                if i % 2 == 1 {
                    ui.end_row();
                }
            }
        });
}

fn stage_name(catalog: &Catalog, id: usize) -> String {
    format!(
        "{} · {}",
        if id == 0 {
            "Training".into()
        } else {
            format!("Mission {id}")
        },
        catalog
            .stages
            .iter()
            .find(|s| s.0 == id)
            .map_or("Unknown", |s| s.1.as_str())
    )
}

fn preview(ui: &mut Ui, run: &Run, catalog: &Catalog) {
    match run.plan(catalog) {
        Ok(plan) => {
            ui.heading(stage_name(catalog, plan.mission));
            ui.label(format!(
                "Roll {} · layout {} · {} difficulty",
                u64::from(run.round) + 1,
                plan.layout,
                ["Easy", "Normal", "Hard"][plan.difficulty as usize]
            ));
            ui.label(format!(
                "Player: {} · story: {}",
                plan.player_model.as_ref().map_or(
                    if plan.character == 0 {
                        "Rikimaru"
                    } else {
                        "Ayame"
                    },
                    |id| catalog.name(id)
                ),
                if plan.character == 0 {
                    "Rikimaru"
                } else {
                    "Ayame"
                }
            ));
            ui.label(format!(
                "Maximum HP: {} · detection: {}%",
                plan.health.map_or("Original".into(), |hp| hp.to_string()),
                plan.sight
            ));
            ui.separator();
            ui.label("Starting equipment");
            for (i, &count) in plan.inventory.iter().enumerate().filter(|(_, n)| **n != 0) {
                ui.label(format!(
                    "{} × {}",
                    crate::ui::ITEMS[i],
                    if count == 255 {
                        "unlimited".into()
                    } else {
                        count.to_string()
                    }
                ));
            }
            ui.separator();
            ui.label(format!(
                "{} enemy models eligible · {}",
                run.document.rules.enemy_pool.choices(catalog).len(),
                if run.document.rules.protect_story {
                    "Story objectives protected"
                } else {
                    "Story actors may change"
                }
            ));
            ui.weak("Exact enemies and pickups are rolled from the mission's authored slots when it loads. Retrying retains them.");
        }
        Err(error) => {
            ui.colored_label(Color32::YELLOW, error);
        }
    }
}
