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

#[cfg(test)]
mod tests;
