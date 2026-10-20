use crate::texture::{Image, Vram};

const GLYPHS: &[u8] = include_bytes!("../../data/japanese-font/glyphs.bin");

fn glyph(code: u16) -> Option<[u16; 15]> {
    let (glyphs, _) = GLYPHS.as_chunks::<32>();
    let index = glyphs
        .binary_search_by_key(&code, |row| u16::from_be_bytes([row[0], row[1]]))
        .ok()?;
    let row = &glyphs[index];
    Some(std::array::from_fn(|y| {
        u16::from_be_bytes([row[2 + y * 2], row[3 + y * 2]])
    }))
}

pub(super) fn caption(text: &[u8], star: &[u16; 15], vram: &mut Vram) -> Option<Image> {
    if !text.first().is_some_and(|c| c & 0x80 != 0) || !text.get(2).is_some_and(|c| c & 0x80 != 0) {
        return None;
    }
    for y in 496..511 {
        vram.words[y * 1024 + 768..y * 1024 + 1024].fill(0);
    }
    let mut width = 0;
    for pair in text.chunks_exact(2).take(16) {
        let code = u16::from_be_bytes([pair[0], pair[1]]);
        let rows = if code == 0x8199 {
            Some(*star)
        } else {
            glyph(code)
        };
        let Some(rows) = rows else { continue };
        let mut pixels = [[0u16; 16]; 15];
        for (y, row) in rows.iter().enumerate() {
            for x in 0..16 {
                if row & (0x8000 >> x) != 0 {
                    pixels[y][x] = 0x7fff;
                }
            }
        }
        for y in 1..14 {
            for x in 1..15 {
                if pixels[y][x] == 0 {
                    let neighbours = [
                        pixels[y - 1][x],
                        pixels[y][x - 1],
                        pixels[y + 1][x],
                        pixels[y][x + 1],
                    ];
                    if (0..4).any(|i| neighbours[i] == 0x7fff && neighbours[(i + 1) % 4] == 0x7fff)
                    {
                        pixels[y][x] = 0x1ce7;
                    }
                }
            }
        }
        for (y, row) in pixels.iter().enumerate() {
            let start = (496 + y) * 1024 + 768 + width;
            vram.words[start..start + 16].copy_from_slice(row);
        }
        width += 16;
    }
    vram.revision = vram.revision.wrapping_add(1);
    Some(Image {
        page: 0x11c,
        palette: 0,
        uv: [0, 240],
        size: [(width as u8).wrapping_sub(1).into(), 15],
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn caption_uses_the_reserved_strip_and_original_byte_limit() {
        let mut vram = Vram::default();
        vram.words.fill(0x1234);
        let mut text = vec![0x81, 0x99];
        text.extend_from_slice(&[0x93, 0x56, 0x92, 0x94]);
        let image = caption(&text, &[0xffff; 15], &mut vram).unwrap();
        assert_eq!(image.word_rect(), [768, 496, 47, 15]);
        assert!(
            vram.words[496 * 1024 + 768..496 * 1024 + 784]
                .iter()
                .all(|&p| p == 0x7fff)
        );
        assert!(
            vram.words[496 * 1024 + 816..497 * 1024]
                .iter()
                .all(|&p| p == 0)
        );
        assert!(vram.words[511 * 1024..].iter().all(|&p| p == 0x1234));
        let image = caption(&text.repeat(6), &[0xffff; 15], &mut vram).unwrap();
        assert_eq!(image.size, [255, 15]);
        assert!(caption(b"Latin text", &[0; 15], &mut vram).is_none());
        assert_eq!(vram.revision, 2);
    }
}
