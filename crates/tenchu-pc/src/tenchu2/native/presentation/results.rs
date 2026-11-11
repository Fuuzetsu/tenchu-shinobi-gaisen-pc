use crate::frontend::{
    draw::Frame,
    raster::{Blend, Canvas, Sprite},
};
use std::sync::Arc;
use tenchu_assets::Result;
use tenchu_assets::frontend::Image;

#[cfg(test)]
mod tests;

impl super::ContentDrawing {
    pub(super) fn results_backdrop(
        &self,
        renderer: &mut crate::renderer::Renderer,
    ) -> Result<Option<Arc<Image>>> {
        match self {
            Self::Results(drawing) => return Ok(drawing.backdrop.clone()),
            Self::Movie { frame, .. } => {
                return Ok(frame
                    .as_ref()
                    .map(|frame| Arc::new(grayscale(frame.size, &frame.rgba, frame.size[0] * 4))));
            }
            Self::Gameplay(_) | Self::Story(_) => {}
            _ => return Ok(None),
        }
        let [width, height] = renderer.size();
        let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("JP results backdrop"),
            size: wgpu::Extent3d {
                width,
                height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: renderer.format(),
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let target = texture.create_view(&Default::default());
        match self {
            Self::Gameplay(drawing) => {
                drawing.draw(&target, renderer)?;
                crate::fonts::capture(renderer, &texture, &drawing.texts);
            }
            Self::Story(drawing) => {
                drawing.draw(&target, renderer)?;
                crate::fonts::capture(renderer, &texture, &drawing.texts);
            }
            _ => unreachable!(),
        }
        let mut image = None;
        crate::capture::Readback::new(renderer, &texture).read(
            renderer,
            &texture,
            |pixels, stride| {
                image = Some(Arc::new(grayscale(
                    [width as usize, height as usize],
                    pixels,
                    stride,
                )));
                Ok(())
            },
        )?;
        Ok(image)
    }
}

fn grayscale([width, height]: [usize; 2], rgba: &[u8], stride: usize) -> Image {
    let crop_width = width.min(height * 4 / 3);
    let crop_height = height.min(width * 3 / 4);
    let left = (width - crop_width) / 2;
    let top = (height - crop_height) / 2;
    let mut pixels = Vec::with_capacity(320 * 240);
    for y in 0..240 {
        for x in 0..320 {
            let offset = (top + y * crop_height / 240) * stride + (left + x * crop_width / 320) * 4;
            let gray = rgba[offset..offset + 3].iter().copied().max().unwrap() >> 4;
            pixels.push(u16::from(gray) * 0x421);
        }
    }
    Image::from_pixels([320, 240], pixels)
}

pub(crate) struct Drawing {
    pub frame: Arc<Frame>,
    pub backdrop: Option<Arc<Image>>,
    pub text: Vec<crate::fonts::Text>,
    sharp_text: bool,
    circle_confirm: bool,
}

impl Drawing {
    pub fn new(results: &super::super::results::Results, backdrop: Option<Arc<Image>>) -> Self {
        let (frame, text) = build(results, backdrop.as_deref(), false, true);
        Self {
            frame,
            backdrop,
            text,
            sharp_text: false,
            circle_confirm: true,
        }
    }

    pub fn update(
        &mut self,
        results: &super::super::results::Results,
        sharp_text: bool,
        circle_confirm: bool,
    ) {
        if self.sharp_text == sharp_text && self.circle_confirm == circle_confirm {
            return;
        }
        (self.frame, self.text) = build(
            results,
            self.backdrop.as_deref(),
            sharp_text,
            circle_confirm,
        );
        self.sharp_text = sharp_text;
        self.circle_confirm = circle_confirm;
    }
}

fn build(
    results: &super::super::results::Results,
    backdrop: Option<&Image>,
    sharp_text: bool,
    circle_confirm: bool,
) -> (Arc<Frame>, Vec<crate::fonts::Text>) {
    let mut canvas = Canvas::new(320);
    canvas.frame = Some(Frame::new([320, 240], Arc::default()));
    canvas.sharp_text = sharp_text;
    if let Some(image) = backdrop {
        canvas.sprite(Sprite::new(image, 0.0, 0.0));
    }
    let panel;
    let panel = if circle_confirm {
        &results.resources.panel
    } else {
        // The two face-button icons are baked into result.tim's bottom legend.
        panel = super::swap_menu_icons(&results.resources.panel, [84, 195], 204, [16, 20]);
        &panel
    };
    append(&mut canvas, results, panel);
    (Arc::new(canvas.frame.unwrap()), canvas.text)
}

fn append(canvas: &mut Canvas, results: &super::super::results::Results, panel: &Image) {
    let images = &results.resources;
    let values = &results.values;
    canvas.sprite(Sprite::new(panel, 0.0, 0.0).light(64).blend(Blend::Add));
    let time = values.record.time_tenths;
    for (text, x) in [
        (format!("{:2}", time / 600), -11),
        (format!("{:02}", time % 600 / 10), 11),
        (format!("{}", time % 10), 31),
    ] {
        number(canvas, &images.numbers, &text, x, -76);
    }
    for (count, score, y) in [
        (
            values.summary.kills.stealth_kills,
            i32::from(values.summary.kills.stealth_kills) * 20,
            -45,
        ),
        (
            values.summary.kills.combat_kills,
            i32::from(values.summary.kills.combat_kills) * 5,
            -25,
        ),
        (
            values.summary.detections,
            400 - i32::from(values.summary.detections) * 30,
            -5,
        ),
        (
            values.summary.kills.innocents,
            i32::from(values.summary.kills.innocents) * -150,
            15,
        ),
    ] {
        number(canvas, &images.numbers, &format!("{count:3}"), 65, y);
        number(canvas, &images.numbers, &format!("{score:4}"), 105, y);
    }
    number(
        canvas,
        &images.numbers,
        &format!("{:5}", values.summary.effort),
        49,
        35,
    );
    number(
        canvas,
        &images.numbers,
        &format!("{:4}", values.record.score),
        105,
        60,
    );
    canvas.sprite(Sprite::new(&images.ranks, 46.0, 60.0).row(usize::from(values.record.rank), 7));
}

fn number(canvas: &mut Canvas, image: &Image, text: &str, x: i32, y: i32) {
    for (i, character) in text.bytes().enumerate() {
        let row = match character {
            b'0'..=b'9' => usize::from(character - b'0'),
            b'+' => 10,
            b'-' => 11,
            _ => continue,
        };
        canvas.label(
            Sprite::new(image, (x + i as i32 * 8) as f32, y as f32)
                .row(row, 12)
                .blend(Blend::Add),
            char::from(character).to_string(),
            egui::Color32::WHITE,
        );
    }
}
