use super::*;
use crate::hud::Sprite;

impl Game {
    pub(in crate::game) fn cinematic_text(&mut self) {
        if self.cinematic.caption != self.cinematic.displayed_caption {
            self.cinematic.caption_image = self
                .mission
                .font
                .prepare_caption(&self.cinematic.caption, &mut self.mission.vram);
        }
        self.cinematic
            .displayed_caption
            .clone_from(&self.cinematic.caption);
        self.cinematic.sprites.clear();
        self.drawing.bytes += 48;
        let text = &self.cinematic.caption;
        if let Some(image) = self.cinematic.caption_image {
            let mut sprite = Sprite::new(image, [-(image.size[0] as i16 / 2), 92]);
            sprite.origin = [0; 2];
            sprite.color = [255; 3];
            self.cinematic.sprites.push(sprite);
            self.drawing.bytes += 40;
        } else {
            let font = &self.mission.font;
            let width: i32 = text.iter().map(|&c| font.width(c)).sum();
            let mut x = -width / 2;
            let mut y = 92;
            for &code in text {
                if code == b'\n' {
                    x = -width / 2;
                    y += 16;
                } else {
                    let (image, nudge) = font.glyph(code);
                    let mut sprite = Sprite::new(image, [x as i16, (y as i16).wrapping_add(nudge)]);
                    sprite.origin = [0; 2];
                    sprite.color = [127; 3];
                    self.cinematic.sprites.push(sprite);
                    self.drawing.bytes += 52;
                    x += font.width(code);
                }
            }
        }
        for (i, &image) in self.mission.tanka.iter().enumerate() {
            if self.cinematic.tanka[i] {
                let shade = &mut self.cinematic.tanka_shade[i];
                if *shade as i8 >= 0 {
                    *shade = shade.wrapping_add(8);
                }
                let mut sprite =
                    Sprite::new(image, [(2 - i as i16) * 20 + 10, (i as i16 % 3) * 8 - 4]);
                if i == 5 {
                    sprite.xy[0] -= 8;
                    sprite.xy[1] = 40;
                }
                sprite.color = [*shade; 3];
                self.cinematic.sprites.push(sprite);
                self.drawing.bytes += 24;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    #[ignore = "requires TENCHU_DISC; checks the original single-quad subtitle cost and caption texture lifetime"]
    fn japanese_caption_packet_and_texture_survive_until_cleared() {
        let mut game = crate::oracle::game(1, 0);
        game.cinematic.caption = vec![0x93, 0x56, 0x92, 0x94];
        let revision = game.mission.vram.revision;
        for _ in 0..3 {
            game.drawing.bytes = 0;
            game.cinematic_text();
            assert_eq!(game.drawing.bytes, 88);
            assert_eq!(game.cinematic.sprites.len(), 1);
            let sprite = &game.cinematic.sprites[0];
            assert_eq!(sprite.xy, [-15, 92]);
            assert_eq!(sprite.image.word_rect(), [768, 496, 31, 15]);
            assert_eq!(game.mission.vram.revision, revision + 1);
        }
        game.cinematic.caption.clear();
        game.drawing.bytes = 0;
        game.cinematic_text();
        assert_eq!(game.drawing.bytes, 48);
        assert!(game.cinematic.sprites.is_empty());
        assert!(game.cinematic.displayed_caption.is_empty());
    }
}
