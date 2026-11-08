use crate::{
    Result,
    texture::{Image, Vram},
    u16_le, u32_le,
};

pub(super) fn image(data: &[u8]) -> Result<crate::frontend::Image> {
    let mut image = crate::frontend::Image::parse(data)?;
    enable_image_blending(&mut image);
    Ok(image)
}

pub(super) fn enable_image_blending(image: &mut crate::frontend::Image) {
    if image.palette_color(0).is_some()
        && !(0..256)
            .filter_map(|i| image.palette_color(i))
            .any(|v| v & 0x8000 != 0 && v & 0x7fff != 0)
    {
        for pixel in &mut image.pixels {
            if *pixel != 0 {
                *pixel |= 0x8000;
            }
        }
    }
}

pub(super) fn upload(vram: &mut Vram, data: &[u8]) -> Result<Image> {
    let image = vram.upload_tim(data)?;
    let flags = u32_le(data, 4)?;
    if flags & 8 != 0 {
        let count = if flags & 1 != 0 { 256 } else { 16 };
        let width = usize::from(u16_le(data, 16)?);
        let height = usize::from(u16_le(data, 18)?);
        if width * height < count {
            return Err("Incomplete Tenchu 2 texture palette".into());
        }
        let x = usize::from(u16_le(data, 12)?);
        let y = usize::from(u16_le(data, 14)?);
        let colors = crate::slice(data, 20, width * height * 2)?;
        // 80017600 enables blending for palettes without an explicit mixed mask.
        let enable = !colors[..count * 2].chunks_exact(2).any(|p| {
            let v = u16::from_le_bytes(p.try_into().unwrap());
            v & 0x8000 != 0 && v & 0x7fff != 0
        });
        // Palette uploads follow image uploads, including intentionally overlapping rectangles.
        for (index, pixel) in colors.chunks_exact(2).enumerate() {
            let v = u16::from_le_bytes(pixel.try_into().unwrap());
            let address = ((y + index / width) & 511) * 1024 + ((x + index % width) & 1023);
            vram.authored_colors.remove(&address);
            vram.words[address] = v;
            if enable && index < count && v != 0 && v & 0x8000 == 0 {
                vram.authored_colors.insert(address, v);
                vram.words[address] |= 0x8000;
            }
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlapping_background_palettes_upload_after_pixels_and_wrap() {
        let mut data = Vec::new();
        for value in [0x10u32, 9, 524] {
            data.extend(value.to_le_bytes());
        }
        for value in [896u16, 496, 256, 1] {
            data.extend(value.to_le_bytes());
        }
        for color in 0u16..256 {
            data.extend(color.to_le_bytes());
        }
        data.extend(268u32.to_le_bytes());
        for value in [896u16, 496, 128, 1] {
            data.extend(value.to_le_bytes());
        }
        for _ in 0..128 {
            data.extend(0xffffu16.to_le_bytes());
        }
        let mut vram = Vram::default();
        upload(&mut vram, &data).unwrap();
        for color in 0u16..256 {
            let address = 496 * 1024 + (896 + usize::from(color)) % 1024;
            assert_eq!(
                vram.words[address],
                if color == 0 { 0 } else { color | 0x8000 }
            );
            if color != 0 {
                assert_eq!(vram.authored_colors[&address], color);
            }
        }
    }

    #[test]
    fn palette_blending_preserves_transparent_black_and_authored_mixed_masks() {
        for (colors, expected) in [
            ([0u16, 0x421, 0x8000, 0x7fff], [0, 0x8421, 0x8000, 0xffff]),
            ([0, 0x421, 0x8421, 0x7fff], [0, 0x421, 0x8421, 0x7fff]),
        ] {
            for (mode, count) in [(0, 16), (1, 256)] {
                let mut data = Vec::new();
                for value in [0x10u32, mode | 8, 12 + count * 2] {
                    data.extend(value.to_le_bytes());
                }
                for value in [32u16, 480, count as u16, 1] {
                    data.extend(value.to_le_bytes());
                }
                for value in colors
                    .into_iter()
                    .chain(std::iter::repeat_n(0, count as usize - 4))
                {
                    data.extend(value.to_le_bytes());
                }
                data.extend(14u32.to_le_bytes());
                for value in [384u16, 0, 1, 1, if mode == 0 { 0x123 } else { 0x302 }] {
                    data.extend(value.to_le_bytes());
                }
                let mut vram = Vram::default();
                let image = upload(&mut vram, &data).unwrap();
                assert_eq!(&vram.words[480 * 1024 + 32..480 * 1024 + 36], &expected);
                let uv = if mode == 0 { [0, 0] } else { [1, 0] };
                assert_eq!(
                    vram.authored_texel(image.page, image.palette, uv),
                    colors[3]
                );
                assert_eq!(vram.texel(image.page, image.palette, uv), expected[3]);
                vram.upload_tim(&data).unwrap();
                assert!(vram.authored_colors.is_empty());
            }
        }
    }
}
