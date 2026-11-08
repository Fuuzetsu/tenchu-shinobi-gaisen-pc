pub mod aim;
pub mod arrow;
pub mod caltrop;
pub mod dart;
pub mod flight;
pub mod grapple;
pub mod grenade;
pub mod gun;
pub mod mine;
pub mod projectile;
pub mod rice;
pub mod sleep_bomb;
pub mod smoke;

use crate::{
    Result,
    math::{Math, Matrix, Position},
    scene::{ModelId, Scene},
    tenchu2::{collision::Collision, terrain::Cursor},
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Items {
    pub aim: aim::Aim,
    pub held: Option<u8>,
    pub entries: Vec<Entry>,
    free_models: Vec<(ModelId, i32, u8)>,
    /// Low four word-address bits of the original item pool, for recorded replays.
    pub pool_phase: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Entry {
    pub model: ModelId,
    pub state: State,
    unused_floor: i32,
    pub slot: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum State {
    Grapple(grapple::Grapple),
    Projectile(projectile::Projectile),
    Arrow(arrow::Arrow),
    Gunshot(gun::Gunshot),
    Grenade(grenade::Grenade),
    Mine(mine::Mine),
    Smoke(smoke::Smoke),
    Caltrop(caltrop::Caltrop),
    Rice(rice::Rice),
    Dart(dart::Dart),
    SleepBomb(sleep_bomb::SleepBomb),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Event {
    Splash(Position),
    Impact {
        position: Position,
        direction: Position,
    },
    Sound {
        position: Position,
        id: u8,
    },
    Alert(Position),
    Explosion {
        position: Position,
        kind: u8,
        contact: bool,
    },
    Smoke {
        position: Position,
        kind: u8,
    },
    Fuse(Position),
    SleepGas {
        position: Position,
        phase: i16,
    },
    Ribbon {
        model: ModelId,
        sample: Option<super::effects::ribbon::Sample>,
    },
}

pub struct Context<'a> {
    pub map: &'a tenchu_assets::tenchu2::collision::Collision,
    pub cursor: &'a mut Cursor,
    pub math: &'a Math,
    pub scene: &'a mut Scene,
    pub collision: &'a mut Collision,
    pub player: usize,
}

impl Items {
    pub fn grapple(&self) -> Option<&grapple::Grapple> {
        self.entries.iter().find_map(|e| match &e.state {
            State::Grapple(g) => Some(g),
            _ => None,
        })
    }

    pub fn grapple_mut(&mut self) -> Option<&mut grapple::Grapple> {
        self.entries.iter_mut().find_map(|e| match &mut e.state {
            State::Grapple(g) => Some(g),
            _ => None,
        })
    }

    pub fn insert(
        &mut self,
        mut state: State,
        scene: &mut Scene,
        collision: &mut Collision,
    ) -> Result<ModelId> {
        if self.entries.len() == 30 {
            self.remove(0, collision);
        }
        let (model, floor, slot) = self.free_models.pop().unwrap_or_else(|| {
            (
                scene.add(None, Matrix::default()),
                0,
                self.entries.len() as u8,
            )
        });
        scene.models[model].parent = None;
        match &mut state {
            State::SleepBomb(bomb) => {
                bomb.flight.floor = floor;
                scene.update(model, bomb.transform);
                bomb.register(model, collision)?;
            }
            State::Dart(dart) => {
                dart.flight.floor = floor;
                scene.update(
                    model,
                    Matrix {
                        translation: dart.flight.position,
                        ..Default::default()
                    },
                );
                dart.register(model, collision)?;
            }
            State::Rice(rice) => {
                if !rice.floor_initialized {
                    rice.flight.floor = floor;
                }
                scene.update(model, rice.transform);
                rice.register(model, collision)?;
            }
            State::Projectile(projectile) => {
                // The source allocator leaves the previous item's floor outside its cleared range.
                projectile.flight.floor = floor;
                scene.update(model, projectile.transform);
                projectile.register(model, collision)?;
            }
            State::Arrow(arrow) => {
                arrow.flight.floor = floor;
                scene.update(model, arrow.transform);
                arrow.register(model, collision)?;
            }
            State::Gunshot(shot) => {
                scene.update(
                    model,
                    Matrix {
                        translation: shot.position,
                        ..Default::default()
                    },
                );
                shot.register(model, collision)?;
            }
            State::Caltrop(caltrop) => {
                caltrop.flight.floor = floor;
                scene.update(
                    model,
                    Matrix {
                        translation: caltrop.flight.position,
                        ..Default::default()
                    },
                );
                caltrop.register(model, collision)?;
            }
            State::Grenade(grenade) => {
                grenade.flight.floor = floor;
                scene.update(model, grenade.transform);
                grenade.register(model, collision)?;
            }
            State::Mine(mine) => {
                scene.update(model, mine.transform);
                mine.register(model, collision)?;
            }
            State::Smoke(smoke) => {
                if smoke.age == 0 {
                    smoke.flight.floor = floor;
                }
                scene.update(model, smoke.transform);
                smoke.register(model, collision)?;
            }
            State::Grapple(_) => {}
        }
        self.entries.push(Entry {
            model,
            state,
            unused_floor: floor,
            slot,
        });
        Ok(model)
    }

    pub fn remove(&mut self, index: usize, collision: &mut Collision) {
        let entry = self.entries.remove(index);
        collision.remove(entry.model);
        let floor = match entry.state {
            State::Projectile(projectile) => projectile.flight.floor,
            State::Arrow(arrow) => arrow.flight.floor,
            State::Grenade(grenade) => grenade.flight.floor,
            State::Mine(mine) => mine.floor,
            State::Smoke(smoke) => smoke.flight.floor,
            State::Caltrop(caltrop) => caltrop.flight.floor,
            State::Rice(rice) => rice.flight.floor,
            State::Dart(dart) => dart.flight.floor,
            State::SleepBomb(bomb) => bomb.flight.floor,
            State::Grapple(_) | State::Gunshot(_) => entry.unused_floor,
        };
        self.free_models.push((entry.model, floor, entry.slot));
    }
}

pub fn wall_throw(
    mut origin: Position,
    actor: Position,
    mut direction: Position,
    action: i16,
) -> (Position, Position) {
    direction.y = 0;
    if action == 0x310 {
        direction.x = direction.x.wrapping_sub(direction.z >> 2);
        direction.z = direction.z.wrapping_add(direction.x >> 2);
    } else {
        direction.x = direction.x.wrapping_add(direction.z >> 2);
        direction.z = direction.z.wrapping_sub(direction.x >> 2);
    }
    origin.x = origin.x.wrapping_add(origin.x.wrapping_sub(actor.x));
    origin.z = origin.z.wrapping_add(origin.z.wrapping_sub(actor.z));
    (origin, direction)
}

fn facing(direction: Position, vertical: i32, math: &Math) -> [[i16; 3]; 3] {
    let up = if direction.x == 0 && direction.z == 0 {
        Position::new(0, 0, direction.y.wrapping_neg())
    } else {
        Position::new(0, vertical, 0)
    };
    let mut rotation = basis(direction, up, math);
    for row in &mut rotation {
        row[0] = row[0].wrapping_neg();
        row[2] = row[2].wrapping_neg();
    }
    rotation
}

fn basis(direction: Position, up: Position, math: &Math) -> [[i16; 3]; 3] {
    let cross = |a: Position, b: Position| {
        Position::from_fn(|i| {
            let j = (i + 1) % 3;
            let k = (i + 2) % 3;
            a[j].wrapping_mul(b[k])
                .wrapping_sub(a[k].wrapping_mul(b[j]))
                >> 12
        })
    };
    let side = math.normalize_vector(cross(up, direction));
    let up = math.normalize_vector(cross(direction, side));
    std::array::from_fn(|i| [side[i] as i16, up[i] as i16, direction[i] as i16])
}
