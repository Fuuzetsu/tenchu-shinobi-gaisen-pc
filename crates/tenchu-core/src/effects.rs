use crate::math::Position;
mod blood;
mod fade;
pub use fade::Fade;
mod explosion;
mod impact;
mod lightning;
mod smoke;
mod snow;
mod texture;
pub use explosion::Explosion;
pub use snow::Snow;
pub use texture::{TextureMove, TextureScroll};
mod wire;
pub use blood::Blood;
pub use impact::Impact;
pub use wire::{FlyWire, Wire};

use crate::{
    Result,
    math::Matrix,
    random::Random,
    scene::{ModelId, Scene},
    terrain::{self, Cursor},
    visibility,
};

#[derive(Clone, Debug)]
pub struct Smoke {
    pub position: Position,
    pub velocity: [i16; 3],
    pub scale: i32,
    pub time: u8,
    pub change_at: u8,
    pub sprite: u8,
    pub rotation: i32,
}

#[derive(Clone, Debug)]
pub struct Splash {
    pub position: Position,
    pub size: [i16; 2],
    pub speed: u8,
    pub count: u8,
    pub mode: u8,
}

#[derive(Clone, Debug)]
pub struct Burst {
    pub spread: i16,
    pub count: i16,
    pub lifetime: i32,
    pub color: [u8; 3],
}

#[derive(Clone, Debug)]
pub struct Bleed {
    pub position: Position,
    pub velocity: [i16; 3],
    pub color: [u8; 3],
    pub time: u8,
    pub mode: u8,
}

#[derive(Clone, Debug)]
pub struct Flash {
    pub parent: Option<ModelId>,
    pub position: Position,
    pub size: i16,
    pub count: i16,
    pub mode: u8,
}

#[derive(Clone, Debug)]
pub enum Effect {
    Smoke(Smoke),
    Splash(Splash),
    Bleed(Bleed),
    Flash(Flash),
    Blood(Blood),
    Impact(Impact),
    FlyWire(FlyWire),
    Explosion(Explosion),
    Ember(Explosion),
    Fade(Fade),
    Snow(Snow),
    TextureScroll(TextureScroll),
}

pub struct Environment<'a> {
    pub terrain: &'a tenchu_assets::terrain::AreaMap,
    pub cursor: &'a mut Cursor,
    pub clock: i32,
    pub owner_position: Position,
    pub reference: Position,
}

#[derive(Clone, Copy)]
pub struct View {
    pub position: Position,
    pub matrix: Matrix,
}

impl View {
    fn project(&self, point: Position) -> ([i16; 2], i16) {
        let matrix = Matrix {
            translation: Position::ZERO,
            ..self.matrix
        };
        let point = std::array::from_fn(|i| {
            point[i].wrapping_sub(i32::from(self.position[i] as i16)) as i16
        });
        let (xy, z) = visibility::project(&matrix, point);
        (xy, (z >> 2) as i16)
    }

    pub(crate) fn sprite(
        &self,
        position: Position,
        graphic: Graphic,
        scale: i32,
        color: u8,
    ) -> Option<Drawing> {
        let (xy, depth) = self.project(position);
        if depth <= 36 {
            return None;
        }
        let scale = i32::from((scale.wrapping_mul(300) / i32::from(depth)) as i16).wrapping_add(1);
        Some(Drawing {
            graphic,
            xy,
            scale: [scale; 2],
            color: [color; 3],
            rotation: 0,
            depth: (depth >> 2).clamp(0, 1249),
        })
    }
}

#[derive(Clone, Debug)]
pub enum Graphic {
    Sprite {
        image: u8,
        blend: u8,
    },
    Smoke(u8),
    Explosion(u8),
    Splash,
    Bleed,
    Flash(i16),
    Blood {
        sprite: u8,
        stain: bool,
        fading: bool,
    },
    Impact(u8),
}

#[derive(Clone, Debug)]
pub struct Drawing {
    pub graphic: Graphic,
    pub xy: [i16; 2],
    pub scale: [i32; 2],
    pub color: [u8; 3],
    pub rotation: i32,
    pub depth: i16,
}

impl Drawing {
    pub fn bytes(&self) -> u32 {
        if matches!(self.graphic, Graphic::Bleed)
            || (self.scale.map(|v| v as i16) == [4096; 2] && self.rotation == 0)
        {
            24
        } else {
            40
        }
    }
}

#[derive(Clone, Debug)]
pub struct Shadow {
    pub position: Position,
    pub rotation: [i16; 3],
    pub scale: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Line {
    pub points: [[i16; 2]; 2],
    pub depth: i16,
    pub color: [u8; 3],
}

#[derive(Clone, Debug)]
pub struct Effects {
    pub slots: Vec<Option<Effect>>,
    pub drawings: Vec<Drawing>,
    pub shadows: Vec<Shadow>,
    pub wires: Vec<Wire>,
    pub lines: Vec<Line>,
    pub blood_pools: Vec<Shadow>,
    pub sounds: Vec<(Position, i16)>,
    pub fades: Vec<(i32, [u8; 3])>,
    pub texture_moves: Vec<TextureMove>,
    cursor: usize,
    visiting: Option<usize>,
}

impl Default for Effects {
    fn default() -> Self {
        Self {
            slots: vec![None; 201],
            drawings: Vec::new(),
            shadows: Vec::new(),
            wires: Vec::new(),
            lines: Vec::new(),
            blood_pools: Vec::new(),
            sounds: Vec::new(),
            fades: Vec::new(),
            texture_moves: Vec::new(),
            cursor: 0,
            visiting: None,
        }
    }
}

impl Effects {
    pub fn reset(&mut self) {
        for slot in &mut self.slots[..200] {
            if !matches!(slot, Some(Effect::TextureScroll(_))) {
                *slot = None;
            }
        }
    }

    fn sprite(
        &mut self,
        graphic: Graphic,
        position: Position,
        scale: i32,
        color: u8,
        rotation: i32,
        view: View,
    ) {
        let matrix = view.matrix.compose(&Matrix {
            translation: position,
            ..Matrix::default()
        });
        let Some(clip) = visibility::clip(&matrix, 0, [0; 3]) else {
            return;
        };
        let depth = clip.depth as i16 - 5;
        if depth < 1 {
            return;
        }
        let scale = ((scale >> 2).wrapping_mul(300) / i32::from(depth)) as i16;
        self.drawings.push(Drawing {
            graphic,
            xy: visibility::project(&matrix, [0; 3]).0,
            scale: [i32::from(scale); 2],
            color: [color; 3],
            rotation,
            depth,
        });
    }

    fn allocate(&mut self) -> usize {
        let mut i = self.cursor;
        for _ in 0..200 {
            i = (i + 1) % 200;
            if self.slots[i].is_none() && self.visiting != Some(i) {
                self.cursor = (i + 1) % 200;
                return i;
            }
        }
        200
    }

    pub fn smoke_burst(
        &mut self,
        position: Position,
        spread: u16,
        divisor: i16,
        count: i16,
        rng: &mut Random,
    ) {
        for _ in 0..count {
            let slot = self.allocate();
            let width = i32::from(spread as i16) * 2;
            let x = (if width > 0 { rng.next15() % width } else { 0 } - i32::from(spread)) as i16;
            let z = (if width > 0 { rng.next15() % width } else { 0 } - i32::from(spread)) as i16;
            let velocity = [x, -5, z];
            self.slots[slot] = Some(Effect::Smoke(Smoke {
                position: Position::from_fn(|i| position[i].wrapping_add(i32::from(velocity[i]))),
                velocity: velocity.map(|v| (i32::from(v) / i32::from(divisor)) as i16),
                scale: rng.next15() % 0x2000 + 0x1000,
                time: 15,
                change_at: (7 - rng.next15() % 15) as u8,
                sprite: 1,
                rotation: 0,
            }));
        }
    }

    pub fn splash(&mut self, position: Position, size: [i16; 2], speed: u8) {
        let slot = self.allocate();
        self.slots[slot] = Some(Effect::Splash(Splash {
            position,
            size,
            speed,
            count: 0,
            mode: 0,
        }));
    }

    pub fn flash(&mut self, parent: Option<ModelId>, position: Position, size: i16, time: i16) {
        let slot = self.allocate();
        self.slots[slot] = Some(Effect::Flash(Flash {
            parent,
            position,
            size,
            count: time,
            mode: 0,
        }));
    }

    pub fn scatter_bleeds(
        &mut self,
        position: Position,
        speed: i16,
        burst: Burst,
        rng: &mut Random,
    ) {
        let Burst {
            spread,
            count,
            lifetime: time,
            color,
        } = burst;
        for _ in 0..count {
            let jitter = |range: i16, rng: &mut Random| {
                let range = i32::from(range);
                if range > 0 {
                    rng.next15() % (range * 2) - range
                } else {
                    -range
                }
            };
            let position = position.map(|p| p.wrapping_add(jitter(spread, rng)));
            let velocity = std::array::from_fn(|_| jitter(speed, rng) as i16);
            let half = time / 2;
            let time = (if time - half > 0 {
                rng.next15() % (time - half) + half
            } else {
                half
            }) as u8;
            self.bleed(position, velocity, time, color);
        }
    }

    pub fn bleeds(
        &mut self,
        position: Position,
        velocity: [i16; 3],
        burst: Burst,
        rng: &mut Random,
    ) {
        let Burst {
            spread,
            count,
            lifetime: time,
            color,
        } = burst;
        for _ in 0..count {
            let width = i32::from(spread) * 2;
            let position = position.map(|p| {
                p.wrapping_add(if width > 0 {
                    rng.next15() % width - i32::from(spread)
                } else {
                    -i32::from(spread)
                })
            });
            let remaining = time.wrapping_sub(time / 8);
            let time = (if remaining > 0 {
                rng.next15() % remaining + time / 8
            } else {
                time / 8
            }) as u8;
            self.bleed(position, velocity, time, color);
        }
    }

    pub fn bleed(&mut self, position: Position, velocity: [i16; 3], time: u8, color: [u8; 3]) {
        let slot = self.allocate();
        self.slots[slot] = Some(Effect::Bleed(Bleed {
            position,
            velocity,
            color,
            time,
            mode: 0,
        }));
    }

    pub fn begin_tick(&mut self) {
        self.drawings.clear();
        self.shadows.clear();
        self.wires.clear();
        self.lines.clear();
        self.blood_pools.clear();
        self.sounds.clear();
        self.fades.clear();
        self.texture_moves.clear();
    }

    pub fn step(
        &mut self,
        rng: &mut Random,
        view: View,
        scene: &mut Scene,
        env: &mut Environment,
    ) -> Result<()> {
        for i in 0..200 {
            let Some(mut effect) = self.slots[i].take() else {
                continue;
            };
            self.visiting = Some(i);
            let result = self.update(&mut effect, rng, view, scene, env);
            self.visiting = None;
            match result {
                Ok(false) => {}
                Ok(true) => self.slots[i] = Some(effect),
                Err(error) => {
                    self.slots[i] = Some(effect);
                    return Err(error);
                }
            }
        }
        Ok(())
    }

    fn update(
        &mut self,
        effect: &mut Effect,
        rng: &mut Random,
        view: View,
        scene: &mut Scene,
        env: &mut Environment,
    ) -> Result<bool> {
        match effect {
            Effect::Snow(snow) => self.update_snow(snow, view, env),
            Effect::TextureScroll(scroll) => {
                self.texture_moves.push(scroll.step());
                Ok(true)
            }
            Effect::Fade(fade) => Ok(self.update_fade(fade, env.clock)),
            Effect::Smoke(smoke) => Ok(self.update_smoke(smoke, rng, view)),
            Effect::Bleed(bleed) => {
                let alive = bleed.mode != 0 || bleed.time != 0;
                if bleed.mode == 0 && bleed.time != 0 {
                    for i in 0..3 {
                        bleed.position[i] =
                            bleed.position[i].wrapping_add(i32::from(bleed.velocity[i]));
                    }
                    bleed.velocity[1] = bleed.velocity[1].wrapping_add(1);
                }
                bleed.time = bleed.time.wrapping_sub(1);
                let (xy, z) = view.project(bleed.position);
                if z > 36 {
                    self.drawings.push(Drawing {
                        graphic: Graphic::Bleed,
                        xy,
                        scale: [i32::from(900 / z + 1); 2],
                        color: bleed.color,
                        rotation: 0,
                        depth: (z >> 2).clamp(0, 1249),
                    });
                }
                Ok(alive)
            }
            Effect::Splash(splash) => {
                let (xy, z) = view.project(splash.position);
                if z <= 36 {
                    return Ok(true);
                }
                let mut scale = splash
                    .size
                    .map(|v| i32::from((i32::from(v) * 300 / i32::from(z)) as i16) + 1);
                if splash.mode == 0 {
                    splash.count = 0;
                    splash.mode = 1;
                    self.bleeds(
                        splash.position,
                        [0, -20, 0],
                        crate::effects::Burst {
                            spread: 100,
                            count: 6,
                            lifetime: 30,
                            color: [144, 152, 160],
                        },
                        rng,
                    );
                }
                let mut alive = true;
                if matches!(splash.mode, 1 | 2) {
                    if splash.speed == 0 {
                        return Err("Zero splash duration".into());
                    }
                    if splash.mode == 1 {
                        scale[1] = scale[1].wrapping_mul(i32::from(splash.count))
                            / i32::from(splash.speed);
                    } else {
                        scale[1] = scale[1]
                            .wrapping_mul(i32::from(splash.speed) - i32::from(splash.count))
                            / i32::from(splash.speed);
                        scale[0] /= 2;
                    }
                    splash.count = splash.count.wrapping_add(1);
                    if splash.count >= splash.speed {
                        if splash.mode == 1 {
                            splash.count = 0;
                            splash.mode = 2;
                        } else {
                            alive = false;
                        }
                    }
                }
                self.drawings.push(Drawing {
                    graphic: Graphic::Splash,
                    xy,
                    scale,
                    color: [128; 3],
                    rotation: 0,
                    depth: (z >> 2).clamp(0, 1249),
                });
                Ok(alive)
            }
            Effect::Flash(flash) => {
                let index = flash.count % 4;
                let mut color = 128;
                let mut alive = true;
                if flash.mode == 0 {
                    flash.count = flash.count.wrapping_sub(1);
                    if flash.count <= 0 {
                        flash.count = 128;
                        flash.mode = 1;
                    }
                } else if flash.mode == 1 {
                    color = flash.count as u8;
                    flash.count = flash.count.wrapping_sub(29);
                    alive = flash.count > 0;
                }
                let (xy, z) = if let Some(parent) = flash.parent {
                    let matrix = view.matrix.compose(&scene.world(parent)?);
                    let (xy, z) =
                        visibility::project(&matrix, flash.position.map(|v| v as i16).to_array());
                    (xy, (z >> 2) as i16)
                } else {
                    view.project(flash.position)
                };
                if z > 36 {
                    let scale = i32::from((i32::from(flash.size) * 300 / i32::from(z)) as i16) + 1;
                    self.drawings.push(Drawing {
                        graphic: Graphic::Flash(index),
                        xy,
                        scale: [scale; 2],
                        color: [color; 3],
                        rotation: 0,
                        depth: ((z - 50) >> 2).clamp(0, 1249),
                    });
                }
                Ok(alive)
            }
            Effect::Blood(blood) => self.update_blood(blood, rng, view, env),
            Effect::Impact(impact) => self.update_impact(impact, view, scene),
            Effect::FlyWire(wire) => Ok(self.update_wire(wire, rng, view, env.owner_position)),
            Effect::Explosion(effect) => Ok(self.update_explosion(effect, false, view)),
            Effect::Ember(effect) => Ok(self.update_explosion(effect, true, view)),
        }
    }
}

#[cfg(test)]
mod tests;
