use super::{
    Context, Environment, Event, Events, GAISEN_HEAT, GAISEN_OPEN_PITFALL, GAISEN_PITFALL, Object,
};
use crate::{
    Result,
    math::{Matrix, Position, Vector3},
    scene::ModelId,
    tenchu2::{ai::runtime::Agent, collision::BODY, visibility::View},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::{
    tenchu2::{
        effects::Sprites,
        level::{Level, objects::StaticKind},
    },
    texture::Image,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Fire {
    pub coordinate: Option<ModelId>,
    pub previous: i16,
}

impl Default for Fire {
    fn default() -> Self {
        Self {
            coordinate: None,
            previous: -1,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Model {
    pub source: Source,
    pub model: usize,
    pub stump: bool,
    pub position: Position,
    pub rotation: [i16; 3],
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Source {
    Static(usize),
    Object(usize),
}

impl Model {
    pub fn object(index: usize, object: &Object) -> Option<Self> {
        Some(Self {
            source: Source::Object(index),
            model: object.model?,
            stump: false,
            position: object.position,
            rotation: [
                if object.kind == 9 {
                    object.roll as i16
                } else {
                    0
                },
                object.yaw,
                if matches!(object.kind, GAISEN_PITFALL | GAISEN_OPEN_PITFALL) {
                    object.roll as i16
                } else {
                    0
                },
            ],
        })
    }
    pub fn poses(
        index: usize,
        object: &Object,
        yaw: i16,
        math: &crate::math::Math,
    ) -> [Option<Self>; 2] {
        let Some(mut base) = Self::object(index, object) else {
            return [None, None];
        };
        if object.kind != 14 || object.link == 0 || object.clock == 1 {
            return [Some(base), None];
        }
        if object.clock > 80 {
            return [None, None];
        }
        base.stump = true;
        let mut position = object.position;
        let step = i32::from(object.clock.min(15)) * 10;
        let [sin, cos] = math.pair(yaw.wrapping_add(1024));
        position.x = position.x.wrapping_sub((-sin * step) >> 12);
        position.z = position.z.wrapping_sub((-cos * step) >> 12);
        let clock = i32::from(object.clock);
        if clock < 31 {
            let fall = if clock < 10 {
                0
            } else {
                i32::from(((clock - 10) * (clock + 40)) as i16).min(1000)
            };
            position.y = position.y.wrapping_add(fall - 1000);
        }
        let roll = if clock < 20 {
            0
        } else if clock <= 60 {
            (((clock - 20) * ((clock - 20) / 2 + 5)) as i16).min(1024)
        } else {
            1024
        };

        let falling = Self {
            position,
            rotation: [0, yaw, roll],
            stump: false,
            ..base.clone()
        };
        [Some(base), Some(falling)]
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Sprite {
    pub image: Image,
    pub position: Position,
    pub scale: [i32; 2],
    pub color: u8,
    pub rotation: i32,
    pub blend: u8,
}

#[derive(Default, Serialize, Deserialize)]
pub struct Frame {
    pub models: Vec<Model>,
    pub sprites: Vec<Sprite>,
    pub flicker: Option<u8>,
}

pub struct Input<'a> {
    pub level: &'a Level,
    pub sprites: &'a Sprites,
    pub view: View,
    pub sequence: u16,
    pub vblank: u16,
    pub actors: &'a [Agent],
}

impl Environment {
    pub fn update_scenery(
        &mut self,
        input: &Input<'_>,
        c: &mut Context<'_, impl Events>,
    ) -> Result<Frame> {
        let mut frame = Frame::default();
        let mut flame_image = input.sprites.effects[15];
        flame_image.uv = input.sprites.effects[12 + usize::from(self.flame_frame & 3)].uv;
        if !matches!(input.level.id, 0 | 2 | 9 | 14 | 16 | 18) {
            frame.flicker = Some(((c.random.next15() & 31) + 32) as u8);
        }
        self.fire.contacts(input.actors, c)?;
        for (index, object) in input.level.scene.statics.iter().enumerate() {
            let position = Position::from(object.position);
            let phase = self
                .static_phases
                .get_mut(index)
                .ok_or("Missing Tenchu 2 scenery phase")?;
            match object.kind {
                StaticKind::Model(model)
                    if input.view.object(position, input.level.object_sizes[model]) =>
                {
                    frame.models.push(Model {
                        source: Source::Static(index),
                        model,
                        stump: false,
                        position,
                        rotation: [*phase, object.rotation[1], 0],
                    });
                }
                StaticKind::Effect(kind) if input.view.effect(position, kind, c.math) => match kind
                {
                    0..=2 | 6 => {
                        let size = if kind == 2 {
                            8192
                        } else if kind == 6 {
                            1536
                        } else {
                            4096
                        };
                        self.fire
                            .flame(position, matches!(kind, 1 | 2), size, phase, c)?;
                        frame
                            .sprites
                            .push(flame(flame_image, position, size, false));
                    }
                    3 | 5 => {
                        *phase = phase.wrapping_add(1) & 63;
                        let (image, size, color) = if kind == 3 {
                            let index = [3, 1, 4][(*phase as usize >> 3) % 3];
                            (
                                input.sprites.scenery[index],
                                (i32::from(*phase) << 7) + 1024,
                                240u8.wrapping_sub((*phase as u8).wrapping_mul(2)),
                            )
                        } else {
                            let index = [7, 6, 5][(*phase as usize >> 2) % 3];
                            (
                                input.sprites.scenery[index],
                                (i32::from(*phase) << 4) + 3082,
                                128u8.wrapping_sub((*phase as u8).wrapping_mul(2)),
                            )
                        };
                        let mut position = position;
                        position.y = position
                            .y
                            .wrapping_sub(i32::from(image.size[1]) * size / 256);
                        if kind == 3 {
                            position.y = position.y.wrapping_sub(i32::from(*phase) * 16);
                        }
                        frame.sprites.push(Sprite {
                            image,
                            position,
                            scale: [size; 2],
                            color,
                            rotation: i32::from(*phase) << 8,
                            blend: if kind == 3 { 3 } else { 1 },
                        });
                    }
                    4 => {
                        *phase = phase.wrapping_add(1) & 7;
                        let size = i32::from(*phase) * 128 + 1152;
                        frame.sprites.push(Sprite {
                            image: input.sprites.scenery[0],
                            position,
                            scale: [size * 2, size],
                            color: 192u8.wrapping_sub((*phase as u8 + 1) * 16),
                            rotation: 0,
                            blend: 1,
                        });
                    }
                    16 if input.sequence == 128 => {
                        *phase = phase.wrapping_add(1) & 127;
                        if *phase == 0 {
                            if c.random.next15() & 1 != 0 {
                                *phase = 63;
                            }
                            if *phase == 0 {
                                c.sound(position, 0x2e)?;
                                *phase = 1;
                            }
                        }
                    }
                    17 => {
                        *phase = phase
                            .wrapping_add(input.vblank.wrapping_sub(self.last_sound_vblank) as i16)
                            & 127;
                        self.last_sound_vblank = input.vblank;
                        if *phase > 23 {
                            c.sound(position, 0x2e)?;
                            *phase = 0;
                        }
                    }
                    _ => {}
                },
                _ => {}
            }
        }
        for index in 0..self.objects.len() {
            if !self.activate_object(index, input.level, &input.view, c.collision, c.math)? {
                continue;
            }
            let object = &mut self.objects[index];
            let model = object
                .model
                .ok_or("Missing Tenchu 2 visible object model")?;
            if object.kind == 14 && object.link != 0 {
                if object.clock > 80 {
                    object.clock = 0;
                    object.link = 0;
                    object.kind = -1;
                    object.model = None;
                    object.fall_yaw = None;
                    continue;
                }
                let yaw = *object.fall_yaw.get_or_insert_with(|| {
                    object
                        .actor
                        .and_then(|i| input.actors.get(i))
                        .map_or(object.yaw, |a| a.actor.rotation.y)
                });
                if object.clock == 30 {
                    c.sound(object.position, 0x2d)?;
                }
                frame.models.extend(
                    Model::poses(index, object, yaw, c.math)
                        .into_iter()
                        .flatten(),
                );
            } else {
                frame.models.push(
                    Model::object(index, object).ok_or("Missing Tenchu 2 visible object model")?,
                );
                if object.kind == 18 {
                    let offset = input.level.scene.bounds[model].offset[1];
                    let size = match offset {
                        -500 => 1024,
                        -99..=-1 => 768,
                        ..=-501 => 1536,
                        _ => 1280,
                    };
                    let shift = size == 1280 && object.roll != 0;
                    let mut position = c.scene.models[object.coordinate].local.translation;
                    position.y = object.position.y.wrapping_add(i32::from(offset) * 2 - 150);
                    self.fire
                        .flame(position, false, size, &mut object.clock, c)?;
                    frame
                        .sprites
                        .push(flame(flame_image, position, size, shift));
                }
            }
        }
        if self.triggers.get(1).is_some_and(|t| t.kind == GAISEN_HEAT) {
            self.heat_phases.resize(input.actors.len(), 0);
            for (agent, phase) in input.actors.iter().zip(&mut self.heat_phases) {
                let actor = &agent.actor;
                if actor.terrain.material & 0xfff == 0x401
                    && actor.terrain.floor == Some(actor.position.y)
                    && input.view.effect(actor.position, 0, c.math)
                {
                    self.fire.flame(actor.position, false, 4096, phase, c)?;
                }
            }
        }
        Ok(frame)
    }
}

fn flame(image: Image, mut position: Position, size: i32, shift: bool) -> Sprite {
    if size < 1024 {
        position.y = position.y.wrapping_add(30);
    } else if (1025..=1280).contains(&size) {
        position.y = position.y.wrapping_add(100);
        if shift {
            position.x = position.x.wrapping_add(50);
        }
    } else if size >= 2048 {
        position.y = position
            .y
            .wrapping_sub(i32::from(image.size[1]) * size / 256);
    }
    Sprite {
        image,
        position,
        scale: [size; 2],
        color: 128,
        rotation: 0,
        blend: 1,
    }
}

impl Fire {
    fn contacts(&mut self, actors: &[Agent], c: &mut Context<'_, impl Events>) -> Result<()> {
        let Some(coordinate) = self.coordinate else {
            return Ok(());
        };
        let Some(index) = c.collision.index(coordinate) else {
            return Ok(());
        };
        self.previous = index as i16;
        let contacts: Vec<_> = c
            .collision
            .contacts(coordinate)
            .filter_map(|contact| {
                let slot = c.collision.slot(contact.index)?;
                (slot.flags & BODY != 0).then_some(slot.owner).flatten()
            })
            .collect();
        for index in contacts {
            let actor = &actors
                .get(index)
                .ok_or("Missing Tenchu 2 fire contact actor")?
                .actor;
            if actor.kind < 3 || actor.flags & 0x40 != 0 {
                c.emit(Event::Burn {
                    actor: index,
                    interval: 10,
                    duration: 60,
                })?;
                c.sound(actor.position, 0x37)?;
            }
        }
        c.collision.remove(coordinate);
        Ok(())
    }

    fn flame(
        &mut self,
        position: Position,
        harmless: bool,
        size: i32,
        phase: &mut i16,
        c: &mut Context<'_, impl Events>,
    ) -> Result<()> {
        *phase = phase.wrapping_add(1) & !128;
        if harmless {
            return Ok(());
        }
        if self.previous == *phase >> 8 {
            *phase = 0;
        } else if *phase & 127 > 45
            && self
                .coordinate
                .is_none_or(|id| c.collision.index(id).is_none())
        {
            let coordinate = *self
                .coordinate
                .get_or_insert_with(|| c.scene.add(None, Matrix::default()));
            c.scene.update(
                coordinate,
                Matrix {
                    translation: position,
                    ..Matrix::default()
                },
            );
            let slot = c.collision.insert(coordinate)?;
            slot.flags = 0x5061;
            slot.size = if size == 4096 {
                Vector3::new(500, 300, 500)
            } else {
                Vector3::new(150, 100, 150)
            };
            let index = c
                .collision
                .index(coordinate)
                .ok_or("Missing Tenchu 2 fire collider")?;
            *phase = (*phase & 127) | ((index as i16) << 8);
        }
        Ok(())
    }
}
