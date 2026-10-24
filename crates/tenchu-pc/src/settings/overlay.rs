use super::{
    Settings,
    menu::{self, Item, Menu, Row},
};
use crate::debug::Debug;
use egui::{Color32, RichText, Ui};

pub enum Edit {
    Change(Item, i32),
    Preset(u16),
    Reset(Item),
    Coordinate(usize, i32),
    PlayerModel(String),
}

#[derive(Clone, Copy)]
pub struct State<'a> {
    pub settings: &'a Settings,
    pub debug: &'a Debug,
    pub paused: bool,
    pub lights: usize,
    pub samples: &'a [u32],
}

pub fn show(ctx: &egui::Context, menu: &mut Menu, state: State<'_>) -> Vec<Edit> {
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
        return edits;
    }
    let mut open = true;
    let screen = ctx.screen_rect();
    egui::Window::new("Settings & tools")
        .open(&mut open)
        .default_pos([24.0, 24.0])
        .default_width(610.0)
        .min_width(480.0)
        .max_width(screen.width() - 32.0)
        .resizable([true, false])
        .collapsible(false)
        .show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.weak(if paused || settings.pause_in_menu {
                    "Paused"
                } else {
                    "Game running"
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.weak("F1 / Esc: close · Tab / Shift+Tab: category");
                });
            });
            ui.separator();
            ui.horizontal_top(|ui| {
                ui.vertical(|ui| {
                    ui.set_width(126.0);
                    for (page, title) in menu::PAGES.iter().enumerate() {
                        if ui.selectable_label(menu.page == page, *title).clicked() {
                            menu.page(page as i32 - menu.page as i32);
                            ctx.memory_mut(|m| m.surrender_focus(ui.id()));
                        }
                    }
                });
                ui.add_space(10.0);
                ui.vertical(|ui| {
                    ui.set_min_width(335.0);
                    if menu.page == 10 {
                        ui.label("Model folder (optional)");
                        ui.add(
                            egui::TextEdit::singleline(&mut menu.model_directory)
                                .desired_width(f32::INFINITY)
                                .hint_text("Path to an exported or custom model pack"),
                        );
                        ui.weak("Choose a body below. Folder edits apply on Load / reload.");
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
                        ui.label(format!("{lights} active lights · {shadowed} shadowed"));
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
                        .max_height((screen.height() - 215.0).clamp(130.0, 360.0))
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
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
                                            let response =
                                                widget(ui, row, menu, samples, &mut edits);
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
                    ui.label(
                        RichText::new(rows[menu.row.min(rows.len() - 1)].help)
                            .small()
                            .color(Color32::from_rgb(167, 188, 204)),
                    );
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

fn widget(
    ui: &mut Ui,
    row: &Row,
    menu: &mut Menu,
    samples: &[u32],
    edits: &mut Vec<Edit>,
) -> egui::Response {
    let item = row.item;
    if item == Item::PlayerModel {
        let selected = menu
            .player_models
            .iter()
            .find(|(id, _)| id == &row.value)
            .map_or("Original / unavailable", |(_, label)| label.as_str());
        return egui::ComboBox::from_id_salt("player-model")
            .width(200.0)
            .selected_text(selected)
            .show_ui(ui, |ui| {
                for (id, label) in &menu.player_models {
                    if ui.selectable_label(id == &row.value, label).clicked() {
                        edits.push(Edit::PlayerModel(id.clone()));
                    }
                }
            })
            .response;
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
        && !matches!(item, Item::Antialiasing | Item::SurfaceDetail)
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
    if let Some((min, max, step, scale)) = range(item) {
        let current = if item == Item::TimeOfDay {
            clock_minutes(&row.value).unwrap_or(0.0)
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
                    } else {
                        value.to_string()
                    }
                })
                .custom_parser(move |text| {
                    if item == Item::TimeOfDay {
                        clock_minutes(text)
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
        Item::SurfaceDetail => strings(&["Off", "Bump", "Parallax"]),
        Item::Magnification => strings(&["Smooth", "Crisp", "Cubic", "Pixel"]),
        Item::Upscale => strings(&["Original", "Scale2x", "Scale4x"]),
        Item::ModelDetail => strings(&["Original", "Subtle", "Rounded"]),
        Item::SightStance => strings(&["Player", "Stand", "Sneak"]),
        Item::Speed => strings(&["25%", "50%", "100%", "200%", "400%", "Unlimited"]),
        Item::CutsceneSpeed => strings(&["1x", "2x", "4x", "8x", "16x", "Max"]),
        Item::LightCount => strings(&["0", "1", "2", "4", "8", "16"]),
        Item::ShadowCount => strings(&["0", "1", "2", "3", "4"]),
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
                            Edit::Preset(index as u16)
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

fn range(item: Item) -> Option<(f64, f64, f64, f64)> {
    let (min, max, step, scale) = match item {
        Item::TimeOfDay => (0, 1425, 15, 1),
        Item::TimeSpeed => (0, 120, 1, 1),
        Item::SunPath => (0, 345, 15, 1),
        Item::SunStrength | Item::MoonStrength => (0, 200, 5, 1),
        Item::DayAmbient | Item::NightAmbient | Item::DayRelighting => (0, 100, 5, 1),
        Item::BloodAmount => (0, 200, 10, 1),
        Item::BloodSeconds => (0, 60, 2, 1),
        Item::BloodMarks => (32, 1024, 32, 1),
        Item::Volume | Item::SurfaceStrength => (0, 100, 5, 1),
        Item::FogDistance | Item::SightDistance => {
            (if item == Item::FogDistance { 25 } else { 0 }, 400, 25, 1)
        }
        Item::Exposure => (25, 300, 25, 1),
        Item::FlightSpeed => (2, 100, 2, 1),
        Item::Fov => (20, 110, 5, 1),
        Item::LightRadius => (2, 50, 1, 1),
        Item::LightIntensity => (0, 400, 10, 1),
        Item::LightAmbient => (0, 100, 5, 1),
        Item::ShadowSoftness => (0, 8, 1, 1),
        Item::SkyTint => (0, 100, 10, 1),
        Item::DirectionalIntensity => (0, 200, 10, 1),
        Item::DirectionalAzimuth => (0, 345, 15, 1),
        Item::DirectionalElevation => (10, 85, 5, 1),
        Item::DirectionalRadius => (10, 60, 5, 1),
        Item::Bloom | Item::Occlusion | Item::Vignette | Item::Sharpen | Item::Haze => {
            (0, 100, 5, 1)
        }
        Item::BloomThreshold => (25, 300, 25, 1),
        Item::OcclusionRadius => (2, 20, 1, 100),
        Item::Contrast | Item::Gamma => (50, 150, 5, 1),
        Item::Saturation => (0, 200, 5, 1),
        Item::Temperature => (-50, 50, 5, 1),
        _ => return None,
    };
    Some((min as f64, max as f64, step as f64, scale as f64))
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
