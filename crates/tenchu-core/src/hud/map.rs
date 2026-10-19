use super::*;
use crate::{items::Items, scene::Scene};
use tenchu_assets::{hud::MapPlacement, stage::MissionAssets};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub points: [[i16; 2]; 2],
    pub color: [u8; 3],
}

#[derive(Debug, Default)]
pub struct Map {
    pub mode: u8,
    pub slide: i32,
    pub dim: Option<u8>,
    pub lines: Vec<Line>,
}

fn project(position: [i32; 3], placement: MapPlacement, math: &Math) -> [i16; 2] {
    let [sin, cos] = math.pair(placement.rotation as i16);
    let divisor = placement.divisor.max(1);
    let x = position[0] / divisor;
    let z = position[2] / divisor;
    let rotated = [
        x.wrapping_mul(cos).wrapping_add(z.wrapping_mul(sin)),
        x.wrapping_mul(sin).wrapping_sub(z.wrapping_mul(cos)),
    ];
    std::array::from_fn(|i| (rotated[i] / 4096).wrapping_add(placement.screen[i]) as i16)
}

impl Map {
    pub(crate) fn clear(&mut self) {
        self.dim = None;
        self.lines.clear();
    }

    fn marker(&mut self, xy: [i16; 2], color: [u8; 3]) {
        let [x, y] = xy;
        for direction in [-2, 2] {
            self.lines.push(Line {
                points: [
                    [x.wrapping_add(direction), y.wrapping_sub(2)],
                    [x.wrapping_sub(direction), y.wrapping_add(2)],
                ],
                color,
            });
        }
    }

    pub(crate) fn step(
        &mut self,
        held: bool,
        player: &Actor,
        mission: &MissionAssets,
        items: &Items,
        scene: &Scene,
        math: &Math,
        effects: &mut Effects,
        sprites: &mut Vec<Sprite>,
        drawing: &mut Drawing,
    ) {
        self.clear();
        if !held {
            self.mode = 0;
            return;
        }
        let shade = 160i32.wrapping_sub(self.slide) / 4;
        match self.mode {
            0 => {
                self.slide = 160;
                self.mode = 1;
                self.dim = Some(0);
                effects.sounds.push(([0; 3], 0x27));
            }
            1 => {
                let mut ghost = Sprite::new(mission.map_image, [self.slide as i16, 0]);
                ghost.color = [60; 3];
                ghost.depth = 2;
                drawing.bytes += ghost.bytes();
                sprites.push(ghost);
                self.dim = Some(shade as u8);
                self.slide = self.slide.wrapping_sub(40);
                if self.slide <= 0 {
                    self.mode += 1;
                }
            }
            _ => {
                self.slide = 0;
                self.dim = Some(shade as u8);
                self.marker(
                    project(
                        scene.models[player.root_model].local.translation,
                        mission.map_placement,
                        math,
                    ),
                    [20, 20, 200],
                );
                for item in &items.slots {
                    if item.processor.is_some() && item.kind == 8 && item.owner == Some(player.id) {
                        self.marker(
                            project(
                                scene.models[item.model].local.translation,
                                mission.map_placement,
                                math,
                            ),
                            [200, 20, 20],
                        );
                    }
                }
            }
        }
        let mut image = Sprite::new(mission.map_image, [self.slide as i16, 0]);
        image.depth = 1;
        drawing.bytes += 32 + image.bytes() + self.lines.len() as u32 * 16;
        sprites.push(image);
    }
}

#[cfg(test)]
mod tests;
