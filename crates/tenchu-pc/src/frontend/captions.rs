use super::raster::{Canvas, Sprite};
use tenchu_assets::{font::Font, frontend::Image, texture::Vram};

pub fn paint(canvas: &mut Canvas, text: &[u8], font: &mut Font, vram: &mut Vram) {
    if text.is_empty() {
        return;
    }
    if let Some(image) = font.prepare_caption(text, vram) {
        let image = Image::from_vram(vram, image);
        canvas.sprite(Sprite::new(&image, 0.0, 99.5));
    } else {
        let width: i32 = text.iter().map(|&c| font.width(c)).sum();
        let mut x = -width as f32 / 2.0;
        let mut y = 92.0;
        for &code in text {
            if code == b'\n' {
                x = -width as f32 / 2.0;
                y += 16.0;
                continue;
            }
            let (image, nudge) = font.glyph(code);
            let image = Image::from_vram(vram, image);
            canvas.sprite(Sprite::new(
                &image,
                x + image.size[0] as f32 / 2.0,
                y + f32::from(nudge) + 8.0,
            ));
            x += font.width(code) as f32;
        }
    }
}
