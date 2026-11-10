use super::*;
use crate::frontend::{
    calligraphy,
    draw::Frame,
    raster::{Blend, Canvas, Sprite},
};
use tenchu_assets::frontend::Image;

pub(crate) struct Drawing {
    pub frame: Arc<Frame>,
    columns: Vec<Vec<Image>>,
}

impl Drawing {
    pub fn introduction(intro: &super::super::film::Introduction) -> Self {
        Self {
            frame: introduction(intro, None),
            columns: Vec::new(),
        }
    }

    pub fn prologue(intro: &super::super::film::Prologue) -> Self {
        Self {
            frame: prologue(intro, None),
            columns: Vec::new(),
        }
    }

    pub fn update_introduction(
        &mut self,
        intro: &super::super::film::Introduction,
        sharp_text: bool,
    ) {
        if sharp_text
            && self.columns.is_empty()
            && let Some(narration) = &intro.resources.narration
        {
            self.columns.push(calligraphy::sharpen(&narration.text));
        }
        self.frame = introduction(
            intro,
            self.columns
                .first()
                .filter(|_| sharp_text)
                .map(Vec::as_slice),
        );
    }

    pub fn update_prologue(&mut self, intro: &super::super::film::Prologue, sharp_text: bool) {
        if sharp_text && self.columns.is_empty() {
            self.columns = intro
                .resources
                .text
                .iter()
                .map(calligraphy::sharpen)
                .collect();
        }
        self.frame = prologue(intro, sharp_text.then_some(self.columns.as_slice()));
    }
}

fn introduction(intro: &super::super::film::Introduction, columns: Option<&[Image]>) -> Arc<Frame> {
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
            column(
                &mut canvas,
                &narration.text,
                15 - index,
                x,
                intensity,
                columns.map(|columns| &columns[15 - index]),
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

fn prologue(intro: &super::super::film::Prologue, columns: Option<&[Vec<Image>]>) -> Arc<Frame> {
    let mut canvas = Canvas::new(320);
    canvas.frame = Some(Frame::new([320, 240], Arc::default()));
    canvas.sprite(Sprite::new(&intro.resources.background, 0.0, 0.0));
    for &(index, x, intensity) in intro.state.frame.columns.iter().rev() {
        let sheet = usize::from(index >= 16);
        let image = &intro.resources.text[sheet];
        let index = if sheet == 0 { 15 - index } else { 19 - index };
        column(
            &mut canvas,
            image,
            index,
            x,
            intensity,
            columns.map(|columns| &columns[sheet][index]),
        );
    }
    canvas.fade((i32::from(intro.state.frame.fade) + 1) / 2);
    Arc::new(canvas.frame.unwrap())
}

fn column(
    canvas: &mut Canvas,
    original: &Image,
    index: usize,
    x: i16,
    intensity: u8,
    enhanced: Option<&Image>,
) {
    let sprite = if let Some(image) = enhanced {
        Sprite::new(image, f32::from(x), 0.0)
            .scale(
                1.0 / calligraphy::SCALE as f32,
                1.0 / calligraphy::SCALE as f32,
            )
            .filtered(true)
    } else {
        Sprite::new(original, f32::from(x), 0.0).region([index * 16, 0, 16, original.size[1]])
    };
    canvas.sprite(sprite.light(i32::from(intensity)).blend(Blend::Add));
}
