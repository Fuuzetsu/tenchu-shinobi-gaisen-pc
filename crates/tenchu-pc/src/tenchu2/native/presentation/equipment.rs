use super::super::{
    campaign::Action,
    equipment::{Edit, Selection},
};
use crate::frontend::{
    draw::Frame,
    raster::{Blend, Canvas, Sprite},
};
use std::sync::Arc;

pub(crate) struct Drawing {
    pub frame: Arc<Frame>,
    pub text: Vec<crate::fonts::Text>,
    tick: u32,
    sharp_text: bool,
}

impl Drawing {
    pub fn new(selection: &Selection) -> Self {
        let (frame, text) = build(selection, false);
        Self {
            frame,
            text,
            tick: selection.state.tick,
            sharp_text: false,
        }
    }

    pub fn update(&mut self, selection: &Selection, sharp_text: bool) {
        if self.tick != selection.state.tick || self.sharp_text != sharp_text {
            (self.frame, self.text) = build(selection, sharp_text);
            self.tick = selection.state.tick;
            self.sharp_text = sharp_text;
        }
    }
}

fn build(selection: &Selection, sharp_text: bool) -> (Arc<Frame>, Vec<crate::fonts::Text>) {
    let state = &selection.state;
    let images = &selection.images;
    let mut canvas = Canvas::new(320);
    canvas.sharp_text = sharp_text;
    canvas.frame = Some(Frame::new([320, 240], Arc::default()));
    canvas.sprite(Sprite::new(&images.background, 0.0, 0.0));
    for (i, image) in images.rings.iter().enumerate() {
        let [x, y] = selection.rules.cells[state.cursors[i]]
            .position
            .map(f32::from);
        let degrees = (state.tick.wrapping_mul(4 << (i + 1)) % 360) as f32;
        canvas.sprite(
            Sprite::new(image, x, y)
                .angle(degrees.to_radians())
                .blend(Blend::Add),
        );
    }
    for (index, cell) in selection.rules.cells.iter().enumerate().rev() {
        if state.stock.items[cell.item].is_none() {
            continue;
        }
        let [x, y] = cell.position.map(f32::from);
        canvas.sprite(
            Sprite::new(&images.items[cell.item], x, y).light(if state.selected == index {
                240
            } else {
                96
            }),
        );
    }
    for cell in selection.rules.cells.iter().rev() {
        if let Some(count) = state.stock.items[cell.item] {
            let [x, y] = cell.position.map(f32::from);
            quantity(&mut canvas, &images.digits, count, x + 10.0, y + 10.0);
        }
    }
    for (index, slot) in state.slots.iter().enumerate().rev() {
        let x = 130.0 - index as f32 * 24.0;
        if index != 0 {
            quantity(&mut canvas, &images.digits, slot.count, x + 10.0, 100.0);
        }
        canvas.sprite(Sprite::new(&images.items[slot.item], x, 90.0));
    }
    for (x, row) in [(44.0, state.remaining / 10), (54.0, state.remaining % 10)] {
        canvas.label(
            Sprite::new(&images.capacity, x, -73.0).row(usize::from(row), 10),
            row.to_string(),
            egui::Color32::WHITE,
        );
    }
    if state.stock.items[selection.rules.cells[state.selected].item].is_some() {
        if state.description >= 3 {
            let name = selection.rules.cells[state.selected].name;
            let [w, h] = images.names.size;
            canvas.sprite(Sprite::new(&images.names, -70.0, 45.0).region([
                name / 10 * (w / 2),
                name % 10 * (h / 10),
                w / 2,
                h / 10,
            ]));
        }
        canvas.text.push(crate::fonts::Text {
            position: egui::pos2(-128.0, 54.0),
            text: selection.rules.descriptions[state.description].clone(),
            color: egui::Color32::WHITE,
            size: 15.0,
            width: 256.0,
            centered: false,
        });
    }
    canvas.fade((i32::from(state.fade).abs() + 1) / 2);
    (Arc::new(canvas.frame.unwrap()), canvas.text)
}

fn quantity(
    canvas: &mut Canvas,
    image: &tenchu_assets::frontend::Image,
    value: u8,
    x: f32,
    y: f32,
) {
    let digits = value.to_string();
    if canvas.sharp_text {
        canvas.text.push(crate::fonts::Text {
            position: egui::pos2(x - 5.0 * (digits.len() - 1) as f32 - 2.0, y - 5.0),
            width: 5.0 * digits.len() as f32,
            text: digits,
            color: egui::Color32::WHITE,
            size: 6.75,
            centered: false,
        });
        return;
    }
    for (i, digit) in digits.bytes().enumerate() {
        let x = x - (digits.len() - i - 1) as f32 * 5.0;
        let source = [usize::from(digit - b'0') * 4, 0, 4, 5];
        canvas.sprite(Sprite::new(image, x + 1.0, y + 0.5).region(source).light(0));
        canvas.sprite(Sprite::new(image, x, y - 0.5).region(source));
    }
}

pub(super) fn controls(selection: &Selection, ctx: &egui::Context) -> Option<Action> {
    let mut action = None;
    if selection.state.fade != 0 || selection.state.finished {
        return None;
    }
    let rect = super::film::rect(ctx);
    let scale = rect.height() / 240.0;
    for (index, cell) in selection.rules.cells.iter().enumerate() {
        let position = rect.center()
            + egui::vec2(f32::from(cell.position[0]), f32::from(cell.position[1])) * scale;
        egui::Area::new(egui::Id::new(("jp-equipment-item", index)))
            .movable(false)
            .fixed_pos(position - egui::vec2(12.0, 12.0) * scale)
            .show(ctx, |ui| {
                if ui
                    .allocate_response(egui::vec2(24.0, 24.0) * scale, egui::Sense::click())
                    .clicked()
                {
                    action = Some(Action::Equipment(Edit::Select(index)));
                }
            });
    }
    egui::Area::new("jp-equipment-actions".into())
        .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -6.0])
        .show(ctx, |ui| {
            ui.set_max_width((rect.width() - 16.0).max(240.0));
            ui.spacing_mut().interact_size.y = 36.0;
            egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                ui.horizontal_wrapped(|ui| {
                    if ui.button("− Remove · Cross").clicked() {
                        action = Some(Action::Equipment(Edit::Remove));
                    }
                    if ui.button("+ Add · Circle").clicked() {
                        action = Some(Action::Equipment(Edit::Add));
                    }
                    if ui.button("Ready · Start / Enter").clicked() {
                        action = Some(Action::Continue);
                    }
                    if ui.button("Cancel").clicked() {
                        action = Some(Action::Quit);
                    }
                });
            });
        });
    action
}
