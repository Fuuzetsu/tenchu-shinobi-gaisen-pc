use super::*;
use crate::hud::Sprite;

impl Game {
    pub(in crate::game) fn cinematic_text(&mut self) {
        self.cinematic
            .displayed_caption
            .clone_from(&self.cinematic.caption);
        self.cinematic.sprites.clear();
        self.drawing.bytes += 48;
        let text = &self.cinematic.caption;
        let kanji = text.first().is_some_and(|c| c & 0x80 != 0)
            && text.get(2).is_some_and(|c| c & 0x80 != 0);
        if !kanji {
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
