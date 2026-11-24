use crate::{
    Result,
    math::{Math, Matrix, Position},
    random::Random,
    tenchu2::{
        terrain::{self, Cursor, Query},
        visibility::View,
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::collision::Collision;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Kind {
    Rain,
    Snow,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Block {
    pub positions: [Position; 16],
    pub velocity: Position,
    pub color: [u8; 3],
    pub times: [i16; 16],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Weather {
    pub kind: Kind,
    pub blocks: Vec<Block>,
    pub stride: usize,
    pub enabled: bool,
    pub thunder: bool,
    pub thunder_time: i16,
    pub thunder_delay: i16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Line {
    pub points: [Position; 2],
    pub screen: [[i16; 2]; 2],
    pub depth: u16,
    pub color: [u8; 3],
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Frame {
    pub lines: Vec<Line>,
    pub sprites: Vec<super::sprites::Drawing>,
    pub flash: Option<u8>,
    pub thunder: bool,
}

impl Weather {
    pub fn for_map(map: usize, resource: u16, event: bool) -> Option<Self> {
        let kind = match map {
            15 if resource != 16 => Kind::Snow,
            5 | 6 | 8 | 15 => Kind::Rain,
            _ => return None,
        };
        Some(Self {
            kind,
            blocks: (0..8)
                .map(|i| Block {
                    positions: [Position::ZERO; 16],
                    velocity: Position::ZERO,
                    color: [0; 3],
                    times: std::array::from_fn(|j| {
                        let offset = (i + j) as i16;
                        match kind {
                            Kind::Snow => offset * 8 - 128,
                            Kind::Rain => {
                                let time = offset * 2 - 32;
                                if time <= 0 { -4 } else { time }
                            }
                        }
                    }),
                })
                .collect(),
            stride: 1,
            enabled: true,
            thunder: map == 8 && !event,
            thunder_time: 0,
            thunder_delay: 0,
        })
    }

    pub fn update(
        &mut self,
        view: View,
        map: &Collision,
        cursor: &mut Cursor,
        math: &Math,
        random: &mut Random,
    ) -> Result<Frame> {
        let mut frame = Frame::default();
        if !self.enabled {
            return Ok(frame);
        }
        let first = self.blocks.first().ok_or("Missing native weather blocks")?;
        let end = first.velocity.map(|v| (v as i16).wrapping_mul(-2));
        let color = first.color;
        if self.stride == 0 {
            return Err("Invalid native weather stride".into());
        }
        // The original advances the block pointer by one, but its counter by stride.
        let count = self.blocks.len().div_ceil(self.stride);
        for block in self.blocks.iter_mut().take(count) {
            let mut alive = false;
            for i in 0..16 {
                let previous = block.times[i];
                block.times[i] = previous.wrapping_sub(1);
                let time = block.times[i];
                if previous <= 0 && !(self.kind == Kind::Rain && time == -3) {
                    continue;
                }
                alive = true;
                let mut position = block.positions[i];
                if self.kind == Kind::Snow {
                    let [sin, cos] = math.pair((random.next15() & 4095) as i16);
                    block.velocity.x = -sin >> 7;
                    block.velocity.z = -cos >> 7;
                    position.x = position.x.wrapping_sub(block.velocity.x);
                    position.z = position.z.wrapping_sub(block.velocity.z);
                }
                if previous > 0 {
                    position.y = position
                        .y
                        .wrapping_sub(block.velocity.y.wrapping_mul(i32::from(time)));
                }
                let transform = Matrix {
                    translation: position,
                    ..Default::default()
                };
                if self.kind == Kind::Rain && previous > 0 {
                    let projection = view.matrix.compose(&transform);
                    let (start, z) = crate::visibility::project(&projection, [0; 3]);
                    let (finish, _) = crate::visibility::project(&projection, end.to_array());
                    let depth = z >> 4;
                    if depth != 0
                        && i32::from(depth) <= view.distance >> 4
                        && i32::from(finish[0]).abs() <= 192
                        && i32::from(finish[1]).abs() <= 152
                    {
                        frame.lines.push(Line {
                            points: [position, position.wrapping_add(end.map(i32::from))],
                            screen: [start, finish],
                            depth,
                            color,
                        });
                    }
                } else if let Some(depth) = view.actor(&transform) {
                    let (image, size, color) = match self.kind {
                        Kind::Rain => (33, 512, 40),
                        Kind::Snow => (32, 128, 128),
                    };
                    let scale = (size * 9600 / (i32::from(depth) * 4)) as i16;
                    frame.sprites.push(super::sprites::Drawing {
                        blood: None,
                        contact: None,
                        image,
                        position,
                        size: [size as i16; 2],
                        scale: [scale; 2],
                        color: [color; 3],
                        rotation: 0,
                        depth: depth as i16,
                        blend: super::surface::Blend::Add,
                    });
                }
            }
            if !alive {
                block.reset(self.kind, view.look, map, cursor, random)?;
            }
        }
        if self.thunder {
            if self.thunder_time == 0 {
                if random.next15() & 255 == 0 {
                    frame.flash = Some(128);
                    self.thunder_delay = (random.next15() & 31) as i16;
                    self.thunder_time = self.thunder_delay + 4;
                }
            } else {
                self.thunder_time = self.thunder_time.wrapping_sub(1);
                if self.thunder_time >= self.thunder_delay {
                    frame.flash = Some(
                        self.thunder_time
                            .wrapping_sub(self.thunder_delay)
                            .wrapping_mul(32)
                            .clamp(0, 255) as u8,
                    );
                } else if self.thunder_time == 0 {
                    frame.thunder = true;
                }
            }
        }
        Ok(frame)
    }
}

impl Block {
    fn reset(
        &mut self,
        kind: Kind,
        center: Position,
        map: &Collision,
        cursor: &mut Cursor,
        random: &mut Random,
    ) -> Result<()> {
        self.velocity = Position::new(0, if kind == Kind::Rain { 480 } else { 120 }, 0);
        if kind == Kind::Rain {
            self.color = [16, 24, 24];
        }
        for (i, position) in self.positions.iter_mut().enumerate() {
            self.times[i] = i as i16 * if kind == Kind::Rain { 2 } else { 8 };
            position.x = center
                .x
                .wrapping_add(random.next15() & 0x3fff)
                .wrapping_sub(8192);
            position.y = -0x40000000;
            position.z = center
                .z
                .wrapping_add(random.next15() & 0x3fff)
                .wrapping_sub(8192);
            if kind == Kind::Rain {
                cursor.current = None;
            }
            let floor = terrain::level(map, cursor, *position, Query::Surface)?;
            position.y = floor.unwrap_or(i32::MIN);
            if floor.is_none() {
                self.times[i] = -4;
            }
        }
        Ok(())
    }
}
