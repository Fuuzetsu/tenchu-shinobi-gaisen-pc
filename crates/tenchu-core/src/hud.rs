use crate::math::Position;
use crate::{Result, actor::Actor, drawing::Drawing, effects::Effects, input::*, math::Math};
use tenchu_assets::{executable::Executable, hud::LifeStyle, texture::Image};
pub mod map;

pub(crate) struct Context<'a> {
    pub mission: &'a tenchu_assets::stage::MissionAssets,
    pub math: &'a Math,
    pub effects: &'a mut Effects,
    pub drawing: &'a mut Drawing,
}

#[derive(Clone, Debug)]
pub struct Sprite {
    pub image: Image,
    pub origin: [i16; 2],
    pub xy: [i16; 2],
    pub scale: [i16; 2],
    pub rotation: i32,
    pub color: [u8; 3],
    pub blend: u8,
    pub depth: u8,
}

impl Sprite {
    pub fn new(image: Image, xy: [i16; 2]) -> Self {
        Self {
            origin: image.size.map(|n| (n >> 1) as i16),
            image,
            xy,
            scale: [4096; 2],
            rotation: 0,
            color: [128; 3],
            blend: 0,
            depth: 0,
        }
    }

    fn bytes(&self) -> u32 {
        if self.rotation == 0 && self.scale == [4096; 2] {
            24
        } else {
            40
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
pub struct LifeBar {
    pub actor: usize,
    pub health: i16,
    pub maximum: i16,
    pub time: i16,
}

#[derive(Debug)]
pub struct Hud {
    pub map: map::Map,
    pub selected: Option<usize>,
    pub cursor: usize,
    pub cursor_rotation: i32,
    pub strain_phase: u16,
    pub bars: [LifeBar; 5],
    pub sprites: Vec<Sprite>,
    styles: [LifeStyle; 2],
}

impl Hud {
    pub fn new(exe: &Executable) -> Result<Self> {
        Ok(Self {
            map: map::Map::default(),
            selected: Some(0),
            cursor: 0,
            cursor_rotation: 0,
            strain_phase: 0,
            bars: [LifeBar::default(); 5],
            sprites: Vec::new(),
            styles: tenchu_assets::hud::life_styles(exe)?,
        })
    }

    pub fn request_life(&mut self, actor: usize, health: i16, maximum: i16) {
        let mut free = None;
        for (i, bar) in self.bars.iter().enumerate() {
            if bar.time < 1 {
                free.get_or_insert(i);
            } else if bar.actor == actor {
                free = Some(i);
                break;
            }
        }
        if let Some(i) = free {
            self.bars[i] = LifeBar {
                actor,
                health,
                maximum: maximum.max(1),
                time: if health == 0 { 100 } else { 300 },
            };
        }
    }

    fn number(&mut self, image: Image, mut xy: [i16; 2], mut number: i32) {
        let mut sprite = Sprite::new(image, xy);
        sprite.image.size[0] = 4;
        loop {
            sprite.xy = xy;
            sprite.image.uv[0] = image.uv[0].wrapping_add((number % 10 * 4) as u8);
            self.sprites.push(sprite.clone());
            xy[0] = xy[0].wrapping_sub(6);
            number /= 10;
            if number == 0 {
                break;
            }
        }
    }

    fn life(
        &mut self,
        images: &[Image],
        xy: [i16; 2],
        health: i16,
        maximum: i16,
        style: usize,
        clock: i32,
    ) {
        let style = self.styles[style];
        self.number(
            images[51],
            std::array::from_fn(|i| xy[i].wrapping_add(style.number[i])),
            i32::from(health),
        );
        let mut frame = Sprite::new(images[usize::from(style.images[0])], xy);
        frame.origin = [0; 2];
        frame.rotation = style.rotation;
        frame.blend = 1;
        frame.depth = 1;
        self.sprites.push(frame);
        let mut fill = Sprite::new(images[usize::from(style.images[1])], xy);
        fill.image.size[1] = (i32::from(style.base)
            + i32::from(style.scale) * i32::from(health) / i32::from(maximum.max(1)))
            as u16;
        fill.origin = [0; 2];
        fill.rotation = style.rotation;
        fill.blend = 2;
        fill.color = [if maximum / 4 < health || clock & 1 == 0 {
            128
        } else {
            230
        }; 3];
        self.sprites.push(fill);
    }

    fn strain(
        &mut self,
        ratio: i32,
        clock: i32,
        images: &[Image],
        math: &Math,
        effects: &mut Effects,
    ) {
        if ratio == i32::MAX || ratio > 20000 {
            return;
        }
        let (image, ratio) = if ratio == 0 {
            (48, 0)
        } else if ratio < -20000 {
            (49, 0)
        } else if ratio < 0 {
            if clock % 30 == 0 {
                effects.sounds.push((Position::ZERO, 0x0e));
            }
            (47, 0)
        } else {
            self.number(images[51], [-100, 100], (20000 - ratio) / 200);
            (46, ratio)
        };
        let delta = 20000 - ratio;
        self.strain_phase = self.strain_phase.wrapping_add((delta / 32) as u16);
        let mut sprite = Sprite::new(images[image], [-134, 92]);
        sprite.blend = 2;
        sprite.color = [(math.pair(self.strain_phase as i16)[0] * 96 / 4096 + 127) as u8; 3];
        sprite.scale = [((delta << 11) / 20000) as i16 + 2048; 2];
        self.sprites.push(sprite);
    }

    pub(crate) fn step(&mut self, player: &Actor, strain: i32, clock: i32, ctx: &mut Context) {
        let Context {
            mission,
            math,
            effects,
            drawing,
        } = ctx;
        let images = &mission.images;
        self.sprites.clear();
        if player.pad.held & TRIANGLE == 0 {
            let step = if player.pad.trigger & R2 != 0 {
                25
            } else if player.pad.trigger & L2 != 0 {
                1
            } else {
                0
            };
            if step != 0 {
                let previous = self.cursor;
                loop {
                    self.cursor = (self.cursor + step) % 26;
                    // Retail lets the aiming flag occupy an invisible selection slot.
                    let available = if self.cursor == 25 {
                        player.aiming
                    } else {
                        player.inventory[self.cursor] != 0
                    };
                    if available || self.cursor == previous {
                        break;
                    }
                }
                effects.sounds.push((Position::ZERO, 0x0b));
            }
        }
        self.selected = None;
        let mut x = 140;
        for kind in 0..25 {
            let count = player.inventory[kind];
            if count == 0 {
                continue;
            }
            if count != 255 {
                self.number(images[51], [x + 22, 100], i32::from(count));
            }
            let mut icon = Sprite::new(images[if kind < 20 { 20 + kind } else { 15 }], [x, 92]);
            if self.cursor == kind {
                let mut cursor = Sprite::new(images[50], [x, 92]);
                self.cursor_rotation = self.cursor_rotation.wrapping_sub(6 * 4096);
                cursor.rotation = self.cursor_rotation;
                cursor.blend = 2;
                cursor.depth = 1;
                self.sprites.push(cursor);
                self.selected = Some(kind);
            } else {
                icon.scale = [4096 * 2 / 3; 2];
            }
            self.sprites.push(icon);
            x -= 20;
        }
        self.life(
            images,
            [-148, 105],
            player.health,
            player.max_health,
            0,
            clock,
        );
        for i in 0..self.bars.len() {
            let bar = self.bars[i];
            if bar.time > 0 {
                self.life(
                    images,
                    [i as i16 * 60 - 140, -90],
                    bar.health,
                    bar.maximum,
                    1,
                    clock,
                );
                self.bars[i].time -= 1;
            }
        }
        self.strain(strain, clock, images, math, effects);
        drawing.bytes += self.sprites.iter().map(Sprite::bytes).sum::<u32>();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; checks selection through the reserved slot and life-bar pool ordering"]
    fn reserved_inventory_slot_and_existing_life_bar_keep_their_source_priority() {
        let mut game = crate::oracle::game(0, 0);
        let player = game.roster.get_mut(game.player_id);
        player.inventory.fill(0);
        player.inventory[0] = 255;
        player.inventory[2] = 5;
        player.aiming = true;
        game.step(R2).unwrap();
        assert_eq!((game.hud.cursor, game.hud.selected), (25, None));
        game.step(0).unwrap();
        game.step(L2).unwrap();
        assert_eq!((game.hud.cursor, game.hud.selected), (0, Some(0)));
        let rotation = game.hud.cursor_rotation;
        game.roster.get_mut(game.player_id).inventory[0] = 0;
        game.step(0).unwrap();
        assert_eq!((game.hud.cursor, game.hud.selected), (0, None));
        assert_eq!(game.hud.cursor_rotation, rotation);
        game.step(L2).unwrap();
        assert_eq!((game.hud.cursor, game.hud.selected), (2, Some(2)));

        for actor in 0..5 {
            game.hud.request_life(actor, 20, 100);
        }
        game.hud.bars[0].time = 0;
        game.hud.request_life(3, 0, 0);
        assert_eq!(game.hud.bars[0].time, 0);
        assert_eq!(
            (
                game.hud.bars[3].health,
                game.hud.bars[3].maximum,
                game.hud.bars[3].time
            ),
            (0, 1, 100)
        );
        game.hud.request_life(6, 20, 100);
        game.hud.request_life(7, 20, 100);
        assert_eq!(game.hud.bars.map(|b| b.actor), [6, 1, 2, 3, 4]);
    }
}
