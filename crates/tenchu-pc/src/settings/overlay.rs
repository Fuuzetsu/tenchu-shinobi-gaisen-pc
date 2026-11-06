mod actors;
mod rewind;
use super::{
    Settings,
    menu::{self, Item, Menu, Row},
};
use crate::debug::Debug;
use egui::{Color32, RichText, Ui};

pub enum Edit {
    Touch(crate::touch::Settings),
    Change(Item, i32),
    Preset(super::library::Choice),
    GameplayPreset(usize),
    Reset(Item),
    Coordinate(usize, i32),
    PlayerModel(String),
    MovesetModel(String),
    Benchmark,
    PresetLibrary,
    Controls,
    Randomizer,
    Diagnostics,
    SeekRewind(u64),
    ResumeRewind,
    PlayRewindForward,
    SelectRewind([u64; 2]),
    ExportRewind,
    ImportRewind,
    ImportUnarmed,
    ExportUnarmed,
    RecordedVisuals(bool),
}

#[derive(Clone, Copy)]
pub struct State<'a> {
    pub settings: &'a Settings,
    pub debug: &'a Debug,
    pub paused: bool,
    pub lights: usize,
    pub samples: &'a [u32],
}

pub fn show(
    ctx: &egui::Context,
    menu: &mut Menu,
    state: State<'_>,
    library: Option<&super::library::Library>,
    mut models: Option<&mut crate::visuals::appearance::picker::State<'_>>,
) -> Vec<Edit> {
    let State {
        settings,
        debug,
        paused,
        lights,
        samples,
    } = state;
    let mut edits = Vec::new();
    menu.hovered = None;
    if !menu.open {
        rewind::overlay(ctx, debug, &mut edits);
        return edits;
    }
    if menu.model_picker {
        egui::Modal::new("player-model-gallery".into()).show(ctx, |ui| {
            ui.set_width((ctx.screen_rect().width() - 48.0).min(640.0));
            ui.horizontal(|ui| {
                ui.heading("Player model");
                if ui.button("Done").clicked() {
                    menu.model_picker = false;
                }
            });
            if let Some(models) = &mut models {
                models.show(ui, menu, settings, &mut edits);
            }
        });
        return edits;
    }
    if menu.touch_controls {
        let mut value = settings.touch;
        egui::Modal::new("touch-settings".into()).show(ctx, |ui| {
            ui.set_max_width((ctx.screen_rect().width() - 48.0).min(440.0));
            ui.heading("Touch controls");
            ui.horizontal_wrapped(|ui| {
                for (mode, label) in [(crate::touch::Visibility::Auto, "Automatic"), (crate::touch::Visibility::Always, "Always"), (crate::touch::Visibility::Hidden, "Hidden")] {
                    ui.selectable_value(&mut value.visibility, mode, label);
                }
            });
            ui.add(egui::Slider::new(&mut value.size, 70..=140).text("Size %"));
            ui.add(egui::Slider::new(&mut value.opacity, 15..=90).text("Opacity %"));
            ui.checkbox(&mut value.left_handed, "Mirror gameplay controls");
            ui.label("Hold Move with one finger and actions with others. Enable camera-relative movement for the right camera stick. Settings remains accessible when controls are hidden.");
            if ui.button("Reset touch controls").clicked() { value = Default::default(); }
            if ui.button("Done").clicked() { menu.touch_controls = false; }
        });
        if value != settings.touch {
            edits.push(Edit::Touch(value));
        }
    }
    let mut open = true;
    let screen = ctx.screen_rect();
    let compact = screen.height() < 560.0 || screen.width() < 720.0;
    let body_height = (screen.height() - 205.0).max(100.0);
    let window = egui::Window::new("Settings & tools")
        .open(&mut open)
        .default_pos(if cfg!(target_os = "android") {
            [12.0, 64.0]
        } else {
            [24.0, 24.0]
        })
        .default_width(610.0)
        .min_width(480.0_f32.min(screen.width() - 32.0))
        .max_height(
            screen.height()
                - if cfg!(target_os = "android") {
                    80.0
                } else {
                    24.0
                },
        )
        .vscroll(true)
        .max_width(screen.width() - 32.0)
        .resizable([true, false])
        .collapsible(false);
    let window = if cfg!(target_os = "android") {
        window
            .fixed_pos([12.0, 64.0])
            .fixed_size([screen.width() - 40.0, (screen.height() - 112.0).max(120.0)])
    } else {
        window
    };
    window.show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.weak(if paused || settings.pause_in_menu {
                    "Paused"
                } else {
                    "Game running"
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak(if cfg!(target_os = "android") { "Settings / Back: close · Swipe lists to scroll" } else { "F1 / Esc: close · Tab / Shift+Tab: category" });
                });
            });
            if settings.retro {
                ui.label("Retro comparison is on. F8 restores your selected graphics; edits below are kept for that view.");
            }
            ui.separator();
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(if compact { 110.0 } else { 126.0 });
                    egui::ScrollArea::vertical().id_salt("settings-categories").max_height(if compact { body_height } else { (screen.height() - 140.0).max(100.0) }).show(ui, |ui| {
                    for (page, title) in menu::PAGES.iter().enumerate() {
                        if ui.selectable_label(menu.page == page, *title).clicked() {
                            menu.page(page as i32 - menu.page as i32);
                            ctx.memory_mut(|m| m.surrender_focus(ui.id()));
                        }
                    }
                    });
                });
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    egui::ScrollArea::vertical()
                        .id_salt(("settings-page", menu.page))
                        .scroll([false, compact])
                        .max_height(if compact { body_height } else { f32::INFINITY })
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                    ui.set_min_width(335.0);
                    if menu.page == 0 {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Benchmarks…").clicked() { edits.push(Edit::Benchmark); }
                            if ui.button("Presets / save as…").clicked() { edits.push(Edit::PresetLibrary); }
                            if ui.button("Controls…").clicked() { edits.push(Edit::Controls); }
                            if ui.button("Touch controls…").clicked() { menu.touch_controls = true; }
                            if ui.button("Randomizer… (F7)").clicked() { edits.push(Edit::Randomizer); }
                            if ui.button("Save diagnostic report…")
                                .on_hover_text("Save settings, version, hardware, recent inputs/timings and gameplay state as readable JSON. No game assets or raw memory. Personal paths are redacted; review before sharing.")
                                .clicked() { edits.push(Edit::Diagnostics); }
                            if ui.button("Import rewind recording…").clicked() { edits.push(Edit::ImportRewind); }
                        });
                        ui.separator();
                    }
                    if menu.page == 7 {
                        ui.label(&menu.artwork_status);
                        ui.horizontal_wrapped(|ui| {
                            ui.hyperlink_to("Download HD artwork", "https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/releases/tag/artwork-v3");
                            ui.hyperlink_to("Installation / updates", "https://github.com/Fuuzetsu/tenchu-shinobi-gaisen-pc/blob/master/docs/install.md#add-hd-artwork");
                        });
                        ui.separator();
                    }
                    if menu.page == 10 {
                        if compact && ui.button("Choose player model…").clicked() {
                            menu.model_picker = true;
                        }
                        ui.label("Model folder (optional)");
                        ui.add(
                            egui::TextEdit::singleline(&mut menu.model_directory)
                                .desired_width(f32::INFINITY)
                                .hint_text("Path to an exported or custom model pack"),
                        );
                        ui.weak("Choose a body below. Folder edits apply on Load / reload.");
                        ui.separator();
                    }
                    if menu.page == 20 {
                        ui.horizontal_wrapped(|ui| {
                            if ui.button("Import Tatsumaru moves…").clicked() { edits.push(Edit::ImportUnarmed); }
                            if ui.button("Export imported moves…").clicked() { edits.push(Edit::ExportUnarmed); }
                        });
                        ui.weak(if cfg!(any(target_arch = "wasm32", target_os = "android")) {
                            "Import a .t2moves file exported from the desktop port."
                        } else {
                            "Import your Tenchu 2 Europe CUE (with BIN beside it), or a .t2moves file."
                        });
                        ui.separator();
                    }
                    let rows = menu::rows(menu.page, settings, debug, paused);
                    if menu.page == 6 {
                        let light = &settings.graphics.lighting;
                        let shadowed = if light.enabled && light.shadows {
                            lights.min(light.shadow_count as usize)
                        } else {
                            0
                        };
                        ui.label(format!("{lights} scene lights · up to {} per region · {shadowed} shadowed", light.count));
                        if lights > shadowed {
                            ui.colored_label(
                                Color32::from_rgb(231, 179, 104),
                                "Unshadowed lights can shine through walls.",
                            );
                        }
                        ui.separator();
                    }
                    egui::ScrollArea::vertical()
                        .id_salt(("settings", menu.page))
                        .scroll([false, !compact])
                        .max_height(if compact { f32::INFINITY } else { (screen.height() - 215.0).clamp(130.0, 360.0) })
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                if menu.page == 4 {
                                    actors::show(ui, menu, models.as_deref(), &mut edits);
                                }
                                for (index, row) in rows.iter().enumerate() {
                                    ui.push_id(index, |ui| {
                                        ui.horizontal(|ui| {
                                            ui.set_min_height(25.0);
                                            let selected = menu.row == index;
                                            let text =
                                                RichText::new(row.label).color(if selected {
                                                    Color32::from_rgb(234, 198, 133)
                                                } else {
                                                    ui.visuals().text_color()
                                                });
                                            let label = ui
                                                .add_sized(
                                                    [190.0, 22.0],
                                                    egui::Label::new(text)
                                                        .halign(egui::Align::Min)
                                                        .wrap(),
                                                )
                                                .on_hover_text(row.help);
                                            let response = if row.item == Item::Preset && let Some(library) = library {
                                                preset_widget(ui, library, settings, &mut edits)
                                            } else {
                                                widget(ui, row, menu, samples, &mut edits)
                                            };
                                            let rect = label.rect.union(response.rect);
                                            if label.hovered() || response.hovered() {
                                                menu.hovered = Some(row.item);
                                            }
                                            if ctx.input(|i| i.pointer.delta() != egui::Vec2::ZERO)
                                                && (label.hovered() || response.hovered())
                                            {
                                                menu.row = index;
                                            }
                                            if menu.reveal_row && selected {
                                                ui.scroll_to_rect(rect, Some(egui::Align::Center));
                                                menu.reveal_row = false;
                                            }
                                            response
                                                .on_hover_text(format!(
                                                    "{}\nR or right-click: reset this value",
                                                    row.help
                                                ))
                                                .context_menu(|ui| {
                                                    if ui.button("Reset to default").clicked() {
                                                        edits.push(Edit::Reset(row.item));
                                                        ui.close_menu();
                                                    }
                                                });
                                        });
                                    });
                                }
                            });
                        });
                    ui.separator();
                    ui.add(
                        egui::Label::new(RichText::new(rows[menu.row.min(rows.len() - 1)].help)
                            .small()
                            .color(Color32::from_rgb(167, 188, 204))).wrap(),
                    );
                    if menu.page == 5 && (settings.rewind_seconds > 0 || debug.rewind.is_some()) {
                        if let Some(timeline) = debug.rewind {
                            rewind::timeline(ui, timeline, &mut edits);
                        } else {
                            ui.weak("Rewind history will fill during play.");
                        }
                        ui.weak(format!("{:.1} MiB stored", debug.rewind_bytes as f64 / 1048576.0));
                    }
                    if menu.page == 10 && !compact {
                        ui.separator();
                        if let Some(models) = &mut models {
                            models.show(ui, menu, settings, &mut edits);
                        }
                    }
                    });
                });
            });
            ui.separator();
            ui.horizontal(|ui| {
                if ui.small_button("Reset selected (R)").clicked() {
                    let rows = menu::rows(menu.page, settings, debug, paused);
                    edits.push(Edit::Reset(rows[menu.row].item));
                }
                ui.weak("Arrows / D-pad: select & change · Double-click numbers to type");
            });
            if !menu.notice.is_empty() {
                ui.label(
                    RichText::new(&menu.notice)
                        .small()
                        .color(Color32::from_rgb(231, 179, 104)),
                );
            }
        });
    menu.open &= open;
    edits
}

fn preset_widget(
    ui: &mut Ui,
    library: &super::library::Library,
    settings: &Settings,
    edits: &mut Vec<Edit>,
) -> egui::Response {
    let choices = library.choices();
    let current = library.current(settings);
    let name = choices
        .iter()
        .find(|(choice, _)| Some(*choice) == current)
        .map_or("Custom", |(_, name)| name.as_str());
    egui::ComboBox::from_id_salt("visual-preset")
        .width(143.0)
        .selected_text(name)
        .show_ui(ui, |ui| {
            for (choice, name) in choices {
                if ui.selectable_label(Some(choice) == current, name).clicked() {
                    edits.push(Edit::Preset(choice));
                }
            }
        })
        .response
}

fn widget(
    ui: &mut Ui,
    row: &Row,
    menu: &mut Menu,
    samples: &[u32],
    edits: &mut Vec<Edit>,
) -> egui::Response {
    let item = row.item;
    if item == Item::MovesetModel {
        let selected = menu
            .moveset_models
            .iter()
            .find(|(id, _)| id == &row.value)
            .map_or("Selected player body", |(_, name)| name.as_str());
        return egui::ComboBox::from_id_salt("moveset-source")
            .selected_text(selected)
            .width(ui.available_width().min(240.0))
            .truncate()
            .close_behavior(egui::PopupCloseBehavior::CloseOnClickOutside)
            .show_ui(ui, |ui| {
                ui.add(
                    egui::TextEdit::singleline(&mut menu.moveset_search)
                        .hint_text("Search movesets…")
                        .desired_width(220.0),
                );
                let query = menu.moveset_search.to_lowercase();
                for (id, name) in &menu.moveset_models {
                    if query
                        .split_whitespace()
                        .all(|word| name.to_lowercase().contains(word) || id.contains(word))
                        && ui.selectable_label(id == &row.value, name).clicked()
                    {
                        edits.push(Edit::MovesetModel(id.clone()));
                        ui.memory_mut(|memory| memory.close_popup());
                    }
                }
            })
            .response;
    }
    if item == Item::PlayerModel {
        let selected = menu
            .player_models
            .iter()
            .find(|(id, _)| id == &row.value)
            .map_or("Original / unavailable", |(_, label)| label.as_str());
        return ui
            .add_sized(
                [ui.available_width().min(240.0), 25.0],
                egui::Label::new(selected).truncate(),
            )
            .on_hover_text(selected);
    }
    if item.button() {
        let response = ui.button(if row.value == "Enter" {
            "Apply"
        } else {
            &row.value
        });
        if response.clicked() {
            edits.push(Edit::Change(item, 1));
        }
        return response;
    }
    if (row.value == "On" || row.value == "Off")
        && !matches!(
            item,
            Item::Antialiasing
                | Item::SurfaceDetail
                | Item::BloodHit
                | Item::BloodDrips
                | Item::BloodDeath
                | Item::BloodGround
        )
    {
        let mut on = row.value == "On";
        let response = ui.checkbox(&mut on, "");
        if response.changed() {
            edits.push(Edit::Change(item, 1));
        }
        return response;
    }
    if let Item::Target(axis) = item {
        if menu
            .edit
            .as_ref()
            .is_some_and(|(item, _)| *item == row.item)
        {
            let (_, text) = menu.edit.as_mut().unwrap();
            let response = ui.add(egui::TextEdit::singleline(text).desired_width(145.0));
            response.request_focus();
            if ui.input(|i| i.key_pressed(egui::Key::Escape)) {
                menu.edit = None;
                response.surrender_focus();
            } else if !response.gained_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                if let Ok(value) = text.parse::<i32>()
                    && (-320_000..=320_000).contains(&value)
                {
                    edits.push(Edit::Coordinate(axis, value));
                    menu.edit = None;
                    response.surrender_focus();
                } else {
                    menu.notice = "Enter a coordinate within +/-320000".into();
                }
            }
            return response;
        }
        let mut value = row.value.parse::<i32>().unwrap_or(0);
        let response = ui.add(
            egui::DragValue::new(&mut value)
                .speed(100.0)
                .range(-320_000..=320_000),
        );
        if response.changed() {
            edits.push(Edit::Coordinate(axis, value));
        }
        return response;
    }
    if let Some(slider) = item.slider() {
        let (min, max, step, scale) = (
            f64::from(slider.min),
            f64::from(slider.max),
            f64::from(slider.step),
            f64::from(slider.scale),
        );
        let current = if item == Item::TimeOfDay {
            clock_minutes(&row.value).unwrap_or(0.0)
        } else if item == Item::PoolingSpeed {
            number(&row.value) * 100.0
        } else {
            number(&row.value) / scale
        };
        let mut value = current;
        let response = ui.add(
            egui::Slider::new(&mut value, min..=max)
                .clamping(egui::SliderClamping::Edits)
                .step_by(step)
                .custom_formatter(move |v, _| {
                    let value = (v * scale).round() as i32;
                    if item == Item::TimeOfDay {
                        format!("{:02}:{:02}", value / 60, value % 60)
                    } else if item == Item::ReinforcementLimit && value == 0 {
                        "Unlimited".into()
                    } else if item == Item::PoolingSpeed {
                        format!("{:.2}x", v / 100.0)
                    } else {
                        value.to_string()
                    }
                })
                .custom_parser(move |text| {
                    if item == Item::TimeOfDay {
                        clock_minutes(text)
                    } else if item == Item::ReinforcementLimit
                        && text.trim().eq_ignore_ascii_case("unlimited")
                    {
                        Some(0.0)
                    } else if item == Item::PoolingSpeed {
                        text.trim_end_matches('x')
                            .parse::<f64>()
                            .ok()
                            .map(|v| v * 100.0)
                    } else {
                        text.parse().ok()
                    }
                }),
        );
        if response.changed() {
            let delta = ((value - current) / step).round() as i32;
            if delta != 0 {
                edits.push(Edit::Change(item, delta));
            }
        }
        return response;
    }
    let choices: Vec<String> = match item {
        Item::UnarmedStyle => [
            tenchu_core::game::sheathing::UnarmedStyle::Onikage,
            tenchu_core::game::sheathing::UnarmedStyle::Tatsumaru,
        ]
        .map(|s| s.name().into())
        .into(),
        Item::BehaviorCombat | Item::ActorCombat => (item == Item::ActorCombat)
            .then_some("Follow global")
            .into_iter()
            .chain(
                tenchu_core::ai::profiles::Combat::ALL
                    .into_iter()
                    .map(|p| p.name()),
            )
            .map(String::from)
            .collect(),
        Item::BehaviorIdle | Item::ActorIdle => (item == Item::ActorIdle)
            .then_some("Follow global")
            .into_iter()
            .chain(
                tenchu_core::ai::profiles::Idle::ALL
                    .into_iter()
                    .map(|p| p.name()),
            )
            .map(String::from)
            .collect(),
        Item::BloodHit | Item::BloodDrips | Item::BloodDeath | Item::BloodGround => {
            super::BloodStyle::ALL
                .iter()
                .map(|s| s.label().into())
                .collect()
        }
        Item::GameplayPreset => super::gameplay::PRESETS
            .iter()
            .map(|p| p.name.to_string())
            .collect(),
        Item::Preset => super::presets::NAMES[..super::presets::CUSTOM as usize]
            .iter()
            .map(|s| s.to_string())
            .collect(),
        Item::Filtering => strings(&[
            "Nearest",
            "Bilinear",
            "Trilinear",
            "2x aniso",
            "4x aniso",
            "8x aniso",
            "16x aniso",
        ]),
        Item::Weather => super::WeatherMode::ALL
            .iter()
            .map(|mode| mode.name().into())
            .collect(),
        Item::WaterDetail => strings(&["Original", "Curved", "Fine curves"]),
        Item::CharacterRefinement => strings(&["Original", "Refined", "Fine"]),
        Item::FoliageDetail => strings(&[
            "Original",
            "Fuller",
            "Dense",
            "Botanical",
            "Botanical dense",
        ]),
        Item::WeatherQuality => strings(&["Low", "Medium", "High"]),
        Item::SurfaceDetail => strings(&["Off", "Bump", "Parallax"]),
        Item::Magnification => strings(&["Smooth", "Crisp", "Cubic", "Pixel"]),
        Item::Upscale => strings(&["Original", "Scale2x", "Scale4x"]),
        Item::ModelDetail => strings(&["Original", "Subtle", "Rounded"]),
        Item::SightStance => strings(&["Player", "Stand", "Sneak"]),
        Item::Speed => strings(&["25%", "50%", "100%", "200%", "400%", "Unlimited"]),
        Item::CutsceneSpeed => strings(&["1x", "2x", "4x", "8x", "16x", "Max"]),
        Item::LightCount => strings(&["0", "1", "2", "4", "8", "16"]),
        Item::ShadowCount => strings(&["0", "1", "2", "3", "4", "6", "8"]),
        Item::LightResolution => strings(&["256", "512", "1024", "2048"]),
        Item::DirectionalResolution => strings(&["512", "1024", "2048", "4096"]),
        Item::DirectionalColor => strings(&["Neutral", "Cool moon", "Warm sun"]),
        Item::FogTint => strings(&["Original black", "Night blue", "Warm dusk", "Gray mist"]),
        Item::Antialiasing => samples
            .iter()
            .map(|n| {
                if *n == 1 {
                    "Off".into()
                } else {
                    format!("{n}x")
                }
            })
            .collect(),
        _ => Vec::new(),
    };
    if !choices.is_empty() {
        return egui::ComboBox::from_id_salt("choice")
            .width(143.0)
            .selected_text(&row.value)
            .show_ui(ui, |ui| {
                let current = choices.iter().position(|c| *c == row.value);
                for (index, choice) in choices.iter().enumerate() {
                    if ui
                        .selectable_label(current == Some(index), choice)
                        .clicked()
                    {
                        edits.push(if item == Item::Preset {
                            Edit::Preset(super::library::Choice::Builtin(index as u16))
                        } else if item == Item::GameplayPreset {
                            Edit::GameplayPreset(index)
                        } else {
                            Edit::Change(item, index as i32 - current.unwrap_or(0) as i32)
                        });
                    }
                }
            })
            .response;
    }
    ui.horizontal(|ui| {
        if ui.small_button("−").clicked() {
            edits.push(Edit::Change(item, -1));
        }
        ui.label(&row.value);
        if ui.small_button("+").clicked() {
            edits.push(Edit::Change(item, 1));
        }
    })
    .response
}

fn clock_minutes(text: &str) -> Option<f64> {
    if let Some((hours, minutes)) = text.trim().split_once(':') {
        let hours: u16 = hours.parse().ok()?;
        let minutes: u16 = minutes.parse().ok()?;
        (hours < 24 && minutes < 60).then_some(f64::from(hours * 60 + minutes))
    } else {
        text.parse().ok()
    }
}

fn strings(values: &[&str]) -> Vec<String> {
    values.iter().map(|v| (*v).into()).collect()
}
fn number(text: &str) -> f64 {
    text.trim_end_matches(|c: char| !c.is_ascii_digit())
        .parse()
        .unwrap_or(0.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn displaying_sliders_preserves_values_between_steps_or_outside_drag_range() {
        for (item, value) in [
            (Item::SkyTint, "15%"),
            (Item::SkyTint, "25%"),
            (Item::SkyTint, "55%"),
            (Item::DirectionalIntensity, "25%"),
            (Item::DirectionalAzimuth, "359 deg"),
            (Item::Exposure, "135%"),
            (Item::Temperature, "-8"),
            (Item::Volume, "37%"),
            (Item::TimeOfDay, "08:07"),
            (Item::TimeOfDay, "23:59"),
            (Item::PoolingSpeed, "0.25x"),
            (Item::PoolingSpeed, "10.00x"),
        ] {
            let ctx = egui::Context::default();
            let mut menu = Menu::default();
            let row = Row {
                item,
                label: "Test",
                value: value.into(),
                help: "",
            };
            for _ in 0..3 {
                let mut edits = Vec::new();
                let _ = ctx.run(Default::default(), |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        widget(ui, &row, &mut menu, &[1, 4], &mut edits);
                    });
                });
                assert!(edits.is_empty(), "Displaying {value} changed the setting");
            }
        }
    }

    #[test]
    fn off_msaa_opens_supported_modes_instead_of_behaving_like_a_toggle() {
        for (item, value) in [(Item::Fps, "Off"), (Item::Antialiasing, "Off")] {
            let ctx = egui::Context::default();
            let mut menu = Menu::default();
            let row = Row {
                item,
                label: "Test",
                value: value.into(),
                help: "",
            };
            let mut rect = egui::Rect::NOTHING;
            let mut edits = Vec::new();
            let input = |events| egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(640.0, 480.0),
                )),
                events,
                ..Default::default()
            };
            let _ = ctx.run(input(Vec::new()), |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    rect = widget(ui, &row, &mut menu, &[1, 4], &mut edits).rect;
                });
            });
            let point = rect.center();
            let _ = ctx.run(
                input(vec![
                    egui::Event::PointerMoved(point),
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed: true,
                        modifiers: egui::Modifiers::NONE,
                    },
                    egui::Event::PointerButton {
                        pos: point,
                        button: egui::PointerButton::Primary,
                        pressed: false,
                        modifiers: egui::Modifiers::NONE,
                    },
                ]),
                |ctx| {
                    egui::CentralPanel::default().show(ctx, |ui| {
                        widget(ui, &row, &mut menu, &[1, 4], &mut edits);
                    });
                },
            );
            if item == Item::Fps {
                assert!(matches!(edits.as_slice(), [Edit::Change(Item::Fps, 1)]));
            } else {
                assert!(edits.is_empty());
                assert!(ctx.memory(|m| m.any_popup_open()));
            }
        }
    }
}
