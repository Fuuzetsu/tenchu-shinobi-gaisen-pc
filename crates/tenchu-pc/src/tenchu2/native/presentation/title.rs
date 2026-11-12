use super::super::{campaign::Action, title::Title};
use crate::frontend::{
    draw::{Artwork, Frame},
    raster::{Blend, Canvas, Sprite},
};
use std::sync::Arc;
use tenchu_core::tenchu2::title::Choice;

pub(crate) struct Drawing {
    pub frame: Arc<Frame>,
    tick: u32,
}

impl Drawing {
    pub fn new(title: &Title) -> Self {
        Self {
            frame: build(title),
            tick: title.state.tick,
        }
    }

    pub fn update(&mut self, title: &Title) {
        if self.tick != title.state.tick {
            self.frame = build(title);
            self.tick = title.state.tick;
        }
    }
}

fn build(title: &Title) -> Arc<Frame> {
    let mut canvas = Canvas::new(320);
    canvas.frame = Some(Frame::new([320, 240], Arc::new(Artwork::new())));
    let state = &title.state;
    for (i, descriptor) in title.resources.sprites.iter().enumerate() {
        if i >= 3 && !state.active {
            continue;
        }
        let [x, y] = descriptor.position.map(f32::from);
        let mut sprite = Sprite::new(
            &title.resources.images[descriptor.image],
            x - 160.0,
            y - 120.0,
        );
        if i >= 3 {
            sprite = sprite.row(descriptor.selected, descriptor.slices);
            if i - 3 != state.choice.index() {
                sprite = sprite.color([128, 0, 0]).blend(Blend::Add);
            }
        } else if i != 0 {
            sprite = sprite.color(state.color);
        }
        canvas.sprite(sprite);
    }
    canvas.fade((i32::from(state.fade).abs() + 1) / 2);
    Arc::new(canvas.frame.unwrap())
}

pub(super) fn controls(
    title: &Title,
    ctx: &egui::Context,
    confirm: &str,
    switch_games: bool,
) -> Option<Action> {
    let mut action = None;
    if title.editor_notice {
        egui::Window::new("Tenchu 2 mission editor")
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label("The Tenchu 2 mission editor has not been ported yet.");
                if ui.button("Back to title").clicked() {
                    action = Some(Action::Continue);
                }
            });
    } else if title.state.fade == 0 {
        let rect = rect(ctx);
        let scale = rect.height() / 240.0;
        if title.state.active {
            for (choice, descriptor) in Choice::ALL.into_iter().zip(&title.resources.sprites[3..]) {
                let [x, y] = descriptor.position.map(f32::from);
                let center = rect.min + egui::vec2(x, y) * scale;
                egui::Area::new(egui::Id::new(("jp-title-choice", choice.index())))
                    .movable(false)
                    .fixed_pos(center - egui::vec2(48.0, 7.0) * scale)
                    .show(ctx, |ui| {
                        let label = match choice {
                            Choice::Campaign => "Campaign",
                            Choice::Editor => "Mission editor (not yet ported)",
                            Choice::Options => "PC settings",
                        };
                        if ui
                            .allocate_response(egui::vec2(96.0, 14.0) * scale, egui::Sense::click())
                            .on_hover_text(label)
                            .clicked()
                        {
                            action = Some(Action::Title(choice));
                        }
                    });
            }
        }
    }
    egui::Area::new("jp-title-actions".into())
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -6.0])
        .show(ctx, |ui| {
            ui.spacing_mut().interact_size.y = 36.0;
            egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if switch_games && ui.button("◀ Shinobi Gaisen · Left / Right ▶").clicked()
                    {
                        action = Some(Action::Quit);
                    }
                    let label = if title.state.active {
                        match title.state.choice {
                            Choice::Campaign => "Campaign",
                            Choice::Editor => "Mission editor",
                            Choice::Options => "PC settings",
                        }
                    } else {
                        "Start"
                    };
                    if ui
                        .add_enabled(
                            title.state.fade == 0 && !title.editor_notice,
                            egui::Button::new(format!("{label} · Enter / {confirm}")),
                        )
                        .clicked()
                    {
                        action = Some(Action::Continue);
                    }
                });
            });
        });
    action
}

pub(super) fn rect(ctx: &egui::Context) -> egui::Rect {
    let screen = ctx.screen_rect();
    let scale = (screen.width() / 320.0).min((screen.height() - 52.0).max(1.0) / 240.0);
    egui::Rect::from_center_size(
        screen.center() - egui::vec2(0.0, 26.0),
        egui::vec2(320.0, 240.0) * scale,
    )
}
