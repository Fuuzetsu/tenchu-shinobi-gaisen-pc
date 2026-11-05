use crate::visuals::{
    pixels,
    texture_recipe::{self, Filter},
};
use tenchu_assets::frontend::Image;

pub(super) const SCALE: usize = 4;

pub(super) fn sharpen(original: &Image) -> Vec<Image> {
    let [width, height] = original.size;
    // Columns scroll independently. Filtering across them leaks neighbouring ink.
    (0..width)
        .step_by(16)
        .map(|left| {
            let column_width = (width - left).min(16);
            let column = pixels::Image {
                size: [column_width as u32, height as u32],
                rgba: (0..height)
                    .flat_map(|y| {
                        original.pixels[y * width + left..][..column_width]
                            .iter()
                            .flat_map(|word| {
                                let value = ((word & 31) * 255 / 31) as u8;
                                [value, value, value, 255]
                            })
                    })
                    .collect(),
            };
            let size = [column_width * SCALE, height * SCALE];
            let smooth = texture_recipe::resize(&column, size.map(|v| v as u32), Filter::Lanczos);
            let pixels = smooth
                .rgba
                .chunks_exact(4)
                .map(|pixel| {
                    let ink = ((f32::from(pixel[0]) - 20.0) / 160.0).clamp(0.0, 1.0);
                    let value = (ink * ink * (3.0 - 2.0 * ink) * 31.0).round() as u16;
                    if value == 0 {
                        0
                    } else {
                        0x8000 | value | value << 5 | value << 10
                    }
                })
                .collect();
            Image::from_pixels(size, pixels)
        })
        .collect()
}
