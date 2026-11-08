use crate::{
    Result,
    texture::{Image, Vram},
    u16_le, u32_le,
};

pub(super) fn image(data: &[u8]) -> Result<crate::frontend::Image> {
    let mut image = crate::frontend::Image::parse(data)?;
    let count = if u32_le(data, 4)? & 1 != 0 { 256 } else { 16 };
    if image.palette_color(0).is_some()
        && !(0..count)
            .filter_map(|i| image.palette_color(i))
            .any(|v| v & 0x8000 != 0 && v & 0x7fff != 0)
    {
        for pixel in &mut image.pixels {
            if *pixel != 0 {
                *pixel |= 0x8000;
            }
        }
    }
    Ok(image)
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
        let start = usize::from(u16_le(data, 14)?) * 1024 + usize::from(u16_le(data, 12)?);
        let colors = vram
            .words
            .get_mut(start..start + count)
            .ok_or("Tenchu 2 texture palette outside VRAM")?;
        // 80017600 enables blending for palettes without an explicit mixed mask.
        if !colors.iter().any(|&v| v & 0x8000 != 0 && v & 0x7fff != 0) {
            for (index, v) in colors.iter_mut().enumerate() {
                if *v != 0 && *v & 0x8000 == 0 {
                    vram.authored_colors.insert(start + index, *v);
                    *v |= 0x8000;
                }
            }
        }
    }
    Ok(image)
}

#[cfg(test)]
mod tests {
    use super::*;

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
