use super::*;
use crate::frontend::{
    draw::Frame,
    raster::{Blend, Canvas, Sprite},
};

pub(super) fn introduction(intro: &super::super::film::Introduction) -> Arc<Frame> {
    let mut canvas = Canvas::new(320);
    canvas.frame = Some(Frame::new([320, 240], Arc::default()));
    let resources = &intro.resources;
    if intro.state.frame.title {
        canvas.sprite(Sprite::new(&resources.subtitle, 0.0, 0.0).row(resources.section, 3));
        let h = resources.heading.size[1] / 3;
        canvas.sprite(
            Sprite::new(&resources.heading, 0.0, -50.0 - h as f32 / 4.0).region([
                0,
                resources.section * h,
                resources.heading.size[0],
                h / 2,
            ]),
        );
    } else if let Some(narration) = &resources.narration {
        canvas.sprite(Sprite::new(&narration.background, 0.0, 0.0));
        for &(index, x, intensity) in intro.state.frame.columns.iter().rev() {
            canvas.sprite(
                Sprite::new(&narration.text, f32::from(x), 0.0)
                    .region([(15 - index) * 16, 0, 16, narration.text.size[1]])
                    .light(i32::from(intensity))
                    .blend(Blend::Add),
            );
        }
    }
    canvas.fade((i32::from(intro.state.frame.fade) + 1) / 2);
    Arc::new(canvas.frame.unwrap())
}

pub(super) fn rect(ctx: &egui::Context) -> egui::Rect {
    let screen = ctx.screen_rect();
    let scale = (screen.width() / 320.0).min(screen.height() / 240.0);
    egui::Rect::from_center_size(screen.center(), egui::vec2(320.0, 240.0) * scale)
}

pub(super) fn prologue(intro: &super::super::film::Prologue) -> Arc<Frame> {
    let mut canvas = Canvas::new(320);
    canvas.frame = Some(Frame::new([320, 240], Arc::default()));
    canvas.sprite(Sprite::new(&intro.resources.background, 0.0, 0.0));
    for &(index, x, intensity) in intro.state.frame.columns.iter().rev() {
        let sheet = usize::from(index >= 16);
        let image = &intro.resources.text[sheet];
        let column = if sheet == 0 { 15 - index } else { 19 - index };
        canvas.sprite(
            Sprite::new(image, f32::from(x), 0.0)
                .region([column * 16, 0, 16, image.size[1]])
                .light(i32::from(intensity))
                .blend(Blend::Add),
        );
    }
    canvas.fade((i32::from(intro.state.frame.fade) + 1) / 2);
    Arc::new(canvas.frame.unwrap())
}
