use super::{collision::Collision, terrain::Cursor};
use crate::{
    Result,
    math::{Math, Matrix, Position, Vector3},
    random::Random,
    scene::{ModelId, Scene},
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{collision, level::objects};

mod contacts;
pub mod scenery;
mod triggers;
mod visibility;

// Imported T1 scenery mechanisms; native OSD kinds remain unchanged.
pub const GAISEN_PITFALL: i16 = 0x100;
pub const GAISEN_OPEN_PITFALL: i16 = 0x101;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Object {
    pub coordinate: ModelId,
    pub cell: usize,
    pub position: Position,
    pub yaw: i16,
    pub roll: i32,
    pub model: Option<usize>,
    pub kind: i16,
    pub link: i16,
    pub clock: i16,
    pub actor: Option<usize>,
    pub fall_yaw: Option<i16>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Trigger {
    pub position: Position,
    pub angle: i16,
    pub kind: i16,
    pub target: u16,
    pub clock: u16,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Environment {
    pub objects: Vec<Object>,
    pub triggers: Vec<Trigger>,
    pub static_phases: Vec<i16>,
    pub background_only: bool,
    pub fire: scenery::Fire,
    pub last_sound_vblank: u16,
    pub flame_frame: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Shot {
    Spear,
    Arrow,
    Smoke,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Debris {
    pub model: usize,
    pub position: Position,
    pub extent: Position,
    pub velocity: Position,
    pub speed: i16,
    pub gravity: i16,
    pub age: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Event {
    Explosion {
        position: Position,
        small: bool,
    },
    Pickup {
        position: Position,
        kind: u16,
    },
    Sound {
        position: Position,
        id: u8,
    },
    Rumble {
        actor: usize,
        kind: i16,
        frames: i16,
    },
    Shoot {
        kind: Shot,
        position: Position,
        velocity: Position,
    },
    Debris(Debris),
    Alert {
        position: Position,
        kind: u8,
    },
    Burn {
        actor: usize,
        interval: i16,
        duration: i16,
    },
}

pub trait Events {
    fn emit(&mut self, event: Event, random: &mut Random) -> Result<()>;
}

pub struct Context<'a, E> {
    pub map: &'a mut collision::Collision,
    pub cursor: &'a mut Cursor,
    pub collision: &'a mut Collision,
    pub scene: &'a mut Scene,
    pub math: &'a Math,
    pub random: &'a mut Random,
    pub events: &'a mut E,
}

pub struct SurfaceContext<'a, E> {
    pub map: &'a mut collision::Collision,
    pub cursor: &'a mut Cursor,
    pub math: &'a Math,
    pub random: &'a mut Random,
    pub events: &'a mut E,
}

pub struct SurfaceActor {
    pub index: usize,
    pub position: Position,
    pub height: i16,
    pub health: i16,
    pub visible: bool,
    pub focused: bool,
}

impl Environment {
    pub fn new(
        level: &tenchu_assets::tenchu2::level::Level,
        scene: &mut Scene,
        math: &Math,
        random: &mut Random,
    ) -> Result<Self> {
        let assets = &level.scene;
        let static_phases = assets
            .statics
            .iter()
            .map(|s| match s.kind {
                objects::StaticKind::Effect(_) => (random.next15() & 63) as i16,
                objects::StaticKind::Model(_) => s.rotation[0],
            })
            .collect();
        let objects = assets
            .dynamic
            .iter()
            .map(|s| {
                let mut position = Position::from(s.position);
                if let Some(model) = s.model {
                    let bounds = assets
                        .bounds
                        .get(model)
                        .ok_or("Missing Tenchu 2 object bounds")?;
                    let [sin, cos] = math.pair(s.facing.wrapping_add(1024));
                    position.x = position.x.wrapping_add(
                        (((sin >> 7) * i32::from(bounds.offset[2])) >> 5) as i16 as i32,
                    );
                    position.z = position.z.wrapping_add(
                        (((cos >> 7) * i32::from(bounds.offset[0])) >> 5) as i16 as i32,
                    );
                    position.y = position.y.wrapping_add(if s.kind == 9 {
                        500
                    } else {
                        i32::from(bounds.offset[1])
                    });
                }
                Ok(Object {
                    cell: level.grid.object_cell(position.to_array())?,
                    coordinate: scene.add(
                        None,
                        Matrix {
                            translation: position,
                            rotation: math.rotate_yxz([0, s.facing, 0]),
                        },
                    ),
                    position: s.position.into(),
                    yaw: s.facing,
                    roll: 0,
                    model: s.model,
                    kind: s.kind as i16,
                    link: s.trigger.map_or(0, |n| n as i16),
                    clock: 0,
                    actor: None,
                    fall_yaw: None,
                })
            })
            .collect::<Result<_>>()?;
        Ok(Self {
            objects,
            static_phases,
            triggers: assets
                .triggers
                .iter()
                .map(|s| Trigger {
                    position: s.position.into(),
                    angle: s.angle,
                    kind: s.kind,
                    target: s.target,
                    clock: 0,
                })
                .collect(),
            background_only: false,
            fire: scenery::Fire::default(),
            last_sound_vblank: 0,
            flame_frame: 3,
        })
    }

    pub fn enable_collision(
        &self,
        index: usize,
        bounds: &[objects::Bounds],
        collision: &mut Collision,
        math: &Math,
    ) -> Result<()> {
        let object = self.objects.get(index).ok_or("Missing Tenchu 2 object")?;
        if collision.index(object.coordinate).is_some() {
            return Ok(());
        }
        let model = object.model.ok_or("Missing Tenchu 2 object model")?;
        let bounds = bounds.get(model).ok_or("Missing Tenchu 2 object bounds")?;
        let (width, depth) = if object.kind == 5 {
            let [sin, cos] = math.pair(object.yaw);
            (
                (((i32::from(bounds.half_width) * cos) >> 12) as i16)
                    .wrapping_sub(((i32::from(bounds.half_depth) * sin) >> 12) as i16),
                (((i32::from(bounds.half_depth) * cos) >> 12) as i16)
                    .wrapping_add(((i32::from(bounds.half_width) * sin) >> 12) as i16),
            )
        } else {
            let extent = bounds.half_width.max(bounds.half_depth);
            (extent, extent)
        };
        let slot = collision.insert(object.coordinate)?;
        slot.flags = if matches!(object.kind, 5..=7 | 20 | GAISEN_PITFALL) {
            0x1200
        } else {
            0x1000
        };
        slot.size = Vector3::new(width, 300i16.wrapping_sub(bounds.top), depth);
        Ok(())
    }
}

impl<E: Events> Context<'_, E> {
    fn emit(&mut self, event: Event) -> Result<()> {
        self.events.emit(event, self.random)
    }
    fn sound(&mut self, position: Position, id: u8) -> Result<()> {
        self.emit(Event::Sound { position, id })
    }
    fn rumble(&mut self, actor: Option<usize>, kind: i16, frames: i16) -> Result<()> {
        if let Some(actor) = actor {
            self.emit(Event::Rumble {
                actor,
                kind,
                frames,
            })?;
        }
        Ok(())
    }
}
