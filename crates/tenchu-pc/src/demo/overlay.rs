use super::Player;
use crate::{debug::Debug, settings::Settings};

pub enum Action {
    Pause,
    Previous,
    Next,
    Restart,
    Solo(usize),
    Selected,
    Exit,
}

pub fn show(
    ctx: &egui::Context,
    player: &mut Player,
    settings: &Settings,
    debug: &Debug,
    paused: bool,
) -> Option<Action> {
    let mut action = None;
    let clip = player.clip();
    if player.captions {
        let frontend = clip.frontend.is_some();
        let top = matches!(clip.shot, super::Shot::Cinematic | super::Shot::Player)
            || clip.artwork.is_some()
            || matches!(
                clip.feature,
                super::Feature::BloodSnow
                    | super::Feature::RainReflections
                    | super::Feature::RainPuddles
                    | super::Feature::RainHaze
                    | super::Feature::Experiment
            );
        let right = clip.feature == super::Feature::Experiment;
        egui::Area::new("demo-caption".into())
            .anchor(
                if right {
                    egui::Align2::RIGHT_TOP
                } else if top {
                    egui::Align2::LEFT_TOP
                } else {
                    egui::Align2::LEFT_BOTTOM
                },
                [
                    if right { -24.0 } else { 24.0 },
                    if top {
                        24.0
                    } else {
                        -24.0
                    },
                ],
            )
            .interactable(false)
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_black_alpha(215))
                    .corner_radius(8)
                    .inner_margin(16)
                    .show(ui, |ui| {
                        let width = if frontend {
                            140.0
                        } else if top {
                            460.0
                        } else {
                            680.0
                        };
                        ui.set_width((ctx.screen_rect().width() - 80.0).clamp(140.0, width));
                        ui.label(
                            egui::RichText::new(if frontend {
                                "TENCHU / INTERFACE".into()
                            } else {
                                format!(
                                "TENCHU / NATIVE SHOWCASE    {:02} / {:02}    MISSION {}",
                                player.index + 1,
                                player.playlist.len(),
                                clip.mission
                                )
                            })
                            .size(11.0)
                            .color(egui::Color32::from_rgb(120, 187, 210)),
                        );
                        ui.add_space(5.0);
                        ui.label(egui::RichText::new(&clip.title).size(if frontend { 18.0 } else { 25.0 }).strong());
                        ui.add_space(3.0);
                        ui.label(egui::RichText::new(&clip.description).size(15.0));
                        if player.video.is_some()
                            && matches!(clip.feature, super::Feature::Performance | super::Feature::Interface)
                        {
                            ui.weak("Offline export: host FPS/timings include capture and encoding. Video plays at 60 fps.");
                        }
                        ui.add_space(9.0);
                        let value = super::readout(player, settings, debug);
                        ui.label(
                            egui::RichText::new(value.label)
                                .size(14.0)
                                .color(egui::Color32::from_rgb(244, 211, 151)),
                        );
                        if let Some(value) = value.fraction {
                            let (rect, _) = ui
                                .allocate_exact_size(egui::vec2(260.0, 14.0), egui::Sense::hover());
                            let a = rect.left_center() + egui::vec2(5.0, 0.0);
                            let b = rect.right_center() - egui::vec2(5.0, 0.0);
                            let x = a.lerp(b, value.clamp(0.0, 1.0));
                            ui.painter()
                                .line_segment([a, b], (3.0, egui::Color32::from_gray(80)));
                            ui.painter().line_segment(
                                [a, x],
                                (3.0, egui::Color32::from_rgb(116, 197, 215)),
                            );
                            ui.painter().circle_filled(x, 5.0, egui::Color32::WHITE);
                        }
                        ui.add_space(5.0);
                        ui.add(
                            egui::ProgressBar::new(player.progress())
                                .desired_height(2.0)
                                .fill(egui::Color32::from_rgb(86, 146, 167)),
                        );
                    });
            });
    }
    if clip.feature == super::Feature::Experiment
        && player.progress() >= 0.375
        && let Some(camera) = debug.camera
    {
        let rect = ctx.screen_rect();
        let p = camera
            .matrix(rect.width() / rect.height())
            .project_point3(glam::Vec3::from(debug.target.map(|v| v as f32).to_array()));
        let center = egui::pos2(
            rect.left() + (p.x + 1.0) * rect.width() * 0.5,
            rect.top() + (1.0 - p.y) * rect.height() * 0.5,
        );
        let painter = ctx.layer_painter(egui::LayerId::new(
            egui::Order::Foreground,
            "demo-click".into(),
        ));
        let click =
            (player.progress() - 0.5).abs() < 0.025 || (player.progress() - 0.75).abs() < 0.025;
        let color = if click {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(116, 197, 215)
        };
        painter.circle_stroke(center, if click { 12.0 } else { 20.0 }, (2.0, color));
        painter.text(
            center + egui::vec2(0.0, -30.0),
            egui::Align2::CENTER_CENTER,
            if click { "CLICK" } else { "Teleport target" },
            egui::FontId::proportional(14.0),
            color,
        );
    }
    if player.transport {
        egui::Area::new("demo-transport".into())
            .anchor(
                if settings.show_performance {
                    egui::Align2::RIGHT_BOTTOM
                } else {
                    egui::Align2::RIGHT_TOP
                },
                [
                    -16.0,
                    if settings.show_performance {
                        -16.0
                    } else {
                        16.0
                    },
                ],
            )
            .show(ctx, |ui| {
                egui::Frame::new()
                    .fill(egui::Color32::from_black_alpha(210))
                    .corner_radius(6)
                    .inner_margin(8)
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.button("Previous").clicked() {
                                action = Some(Action::Previous);
                            }
                            if ui.button(if paused { "Resume" } else { "Pause" }).clicked() {
                                action = Some(Action::Pause);
                            }
                            if ui.button("Restart").clicked() {
                                action = Some(Action::Restart);
                            }
                            if ui.button("Next").clicked() {
                                action = Some(Action::Next);
                            }
                            if ui.selectable_label(player.library, "Clips [Tab]").clicked() {
                                player.library = !player.library;
                            }
                        });
                        ui.horizontal(|ui| {
                            ui.checkbox(&mut player.captions, "Captions [H]");
                            ui.checkbox(&mut player.repeat, "Loop");
                            ui.small("V: hide controls · Esc: exit");
                        });
                        if player.finished {
                            ui.label("Sequence complete. Restart or choose more clips.");
                        }
                    });
            });
    }
    if player.library {
        let mut open = true;
        egui::Window::new("Showcase clips")
            .open(&mut open)
            .default_width(590.0)
            .default_height(440.0)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.label("Choose a clip to play alone, or check several and play the selection.");
                ui.small(
                    "Space: pause · Left / Right: previous / next · Home: restart · F1: settings",
                );
                ui.horizontal(|ui| {
                    if ui.button("All").clicked() {
                        player.selected.fill(true);
                    }
                    if ui.button("None").clicked() {
                        player.selected.fill(false);
                    }
                    if ui
                        .add_enabled(
                            player.selected.iter().any(|v| *v),
                            egui::Button::new("Play selected"),
                        )
                        .clicked()
                    {
                        action = Some(Action::Selected);
                    }
                    if ui.button("Exit demo").clicked() {
                        action = Some(Action::Exit);
                    }
                });
                ui.separator();
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .max_height(400.0)
                    .show(ui, |ui| {
                        for (index, clip) in player.catalog.iter().enumerate() {
                            ui.horizontal(|ui| {
                                ui.checkbox(&mut player.selected[index], "");
                                if ui
                                    .button(&clip.title)
                                    .on_hover_text(&clip.description)
                                    .clicked()
                                {
                                    action = Some(Action::Solo(index));
                                }
                                ui.weak(format!(
                                    "{}s · M{} · {}",
                                    clip.seconds, clip.mission, clip.id
                                ));
                            });
                        }
                    });
            });
        player.library = open;
    }
    action
}
