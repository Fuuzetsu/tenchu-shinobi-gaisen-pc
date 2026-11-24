use crate::{
    frontend::draw::{self, Artwork},
    settings::Graphics,
    visuals::pack::Pack,
};
use std::sync::Arc;
use tenchu_assets::{Result, frontend::Image};

pub(super) fn load<'a>(
    images: impl IntoIterator<Item = &'a Image>,
    graphics: &Graphics,
) -> Result<Artwork> {
    if !graphics.asset_pack {
        return Ok(Artwork::new());
    }
    let originals = images
        .into_iter()
        .map(|image| {
            let image = draw::pixels(image);
            (draw::identity(&image), image)
        })
        .collect();
    Ok(Pack::tenchu2_interface(originals)?.textures)
}

pub(super) fn swap_icons(
    original: &Image,
    artwork: &mut Artwork,
    x: [usize; 2],
    y: usize,
    size: [usize; 2],
) -> Image {
    let swapped = super::swap_menu_icons(original, x, y, size);
    if let Some(replacement) = artwork.get(&draw::identity(&draw::pixels(original))) {
        let mut replacement = (**replacement).clone();
        let [width, height] = replacement.size.map(|n| n as usize);
        let xs = x.map(|x| x * width / original.size[0]);
        let top = y * height / original.size[1];
        let bottom = (y + size[1]) * height / original.size[1];
        let length = size[0] * width / original.size[0];
        for row in top..bottom {
            for column in 0..length {
                for channel in 0..4 {
                    replacement.rgba.swap(
                        ((row * width + xs[0] + column) * 4) + channel,
                        ((row * width + xs[1] + column) * 4) + channel,
                    );
                }
            }
        }
        artwork.insert(
            draw::identity(&draw::pixels(&swapped)),
            Arc::new(replacement),
        );
    }
    swapped
}

pub(super) fn triangle_icon(
    original: &Image,
    artwork: &mut Artwork,
    position: [usize; 2],
    size: [usize; 2],
    buttons: &Image,
) -> Image {
    let mut pixels = original.pixels.clone();
    let icon_height = size[0] * (buttons.size[1] / 5) / buttons.size[0];
    let top = (size[1] - icon_height) / 2;
    for y in 0..size[1] {
        for x in 0..size[0] {
            pixels[(position[1] + y) * original.size[0] + position[0] + x] =
                if (top..top + icon_height).contains(&y) {
                    let sx = x * buttons.size[0] / size[0];
                    let sy =
                        buttons.size[1] * 4 / 5 + (y - top) * (buttons.size[1] / 5) / icon_height;
                    buttons.pixels[sy * buttons.size[0] + sx]
                } else {
                    0
                };
        }
    }
    let result = Image::from_pixels(original.size, pixels);
    if let Some(replacement) = artwork.get(&draw::identity(&draw::pixels(original))) {
        let mut replacement = (**replacement).clone();
        let source = draw::pixels(buttons);
        let source = artwork
            .get(&draw::identity(&source))
            .map_or(&source, AsRef::as_ref);
        let [width, height] = replacement.size.map(|n| n as usize);
        let [left, upper] = [
            position[0] * width / original.size[0],
            position[1] * height / original.size[1],
        ];
        let [right, bottom] = [
            (position[0] + size[0]) * width / original.size[0],
            (position[1] + size[1]) * height / original.size[1],
        ];
        let icon_top = (position[1] + top) * height / original.size[1];
        let icon_bottom = (position[1] + top + icon_height) * height / original.size[1];
        let [sw, sh] = source.size.map(|n| n as usize);
        for y in upper..bottom {
            for x in left..right {
                let target = ((y * width + x) * 4)..((y * width + x) * 4 + 4);
                if (icon_top..icon_bottom).contains(&y) {
                    let sx = (x - left) * sw / (right - left);
                    let sy = sh * 4 / 5 + (y - icon_top) * (sh / 5) / (icon_bottom - icon_top);
                    let offset = (sy * sw + sx) * 4;
                    replacement.rgba[target].copy_from_slice(&source.rgba[offset..offset + 4]);
                } else {
                    replacement.rgba[target].fill(0);
                }
            }
        }
        artwork.insert(
            draw::identity(&draw::pixels(&result)),
            Arc::new(replacement),
        );
    }
    result
}

#[cfg(test)]
mod tests;
