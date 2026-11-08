use crate::{
    Result,
    math::{Math, Matrix, Position, Vector3},
    random::Random,
    tenchu2::{
        actor::Actor,
        terrain::{self, Query},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReturnPoint {
    pub position: Position,
    pub voice_clock: u32,
    pub return_clock: i32,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Resurrection {
    pub owner: usize,
    pub attacker: Option<usize>,
    pub age: i32,
    pub transform: Matrix,
    pub rotation: Vector3<i16>,
    pub roll_step: i16,
    pub floor: i32,
}

#[derive(Default)]
pub struct Update {
    pub alive: bool,
    pub fade: Option<u8>,
    pub smoke: Option<Position>,
    pub voice: Option<Position>,
    pub returned: bool,
}

pub fn destination(
    origin: Position,
    home: Position,
    markers: impl Iterator<Item = Position>,
) -> Position {
    let mut nearest = home;
    let mut distance = u32::MAX;
    for marker in markers {
        let dx = marker.x.wrapping_sub(origin.x) >> 8;
        let dz = marker.z.wrapping_sub(origin.z) >> 8;
        let squared = dx.wrapping_mul(dx).wrapping_add(dz.wrapping_mul(dz)) as u32;
        if squared < distance {
            nearest = marker;
            distance = squared;
        }
    }
    nearest
}

impl Resurrection {
    pub fn new(
        owner: usize,
        attacker: Option<usize>,
        actor: &Actor,
        context: &mut super::Context<'_>,
        random: &mut Random,
    ) -> Result<Self> {
        let position = actor.position.wrapping_sub(Position::new(0, 600, 0));
        let sign = if random.next15() & 1 != 0 { 1 } else { -1 };
        let rotation = Vector3::new(0, actor.rotation.y, 341 * sign);
        Ok(Self {
            owner,
            attacker,
            age: 0,
            transform: Matrix {
                translation: position,
                rotation: context.math.rotate_yxz(rotation.to_array()),
            },
            rotation,
            roll_step: 45 * sign,
            floor: terrain::level(context.map, context.cursor, position, Query::Surface)?
                .unwrap_or(position.y),
        })
    }

    pub fn advance(
        &mut self,
        actor: &mut Actor,
        point: &mut ReturnPoint,
        transition_ready: bool,
        math: &Math,
        random: &mut Random,
    ) -> Update {
        let mut update = Update {
            alive: true,
            ..Default::default()
        };
        if self.age < 0 {
            if transition_ready {
                point.return_clock = point.return_clock.wrapping_add(1);
            }
            if point.return_clock < 15 {
                update.fade = Some(255);
                update.voice = self
                    .voice(point, random)
                    .then_some(self.transform.translation);
                actor.position = point.position;
            } else {
                update.fade =
                    Some((self.age.wrapping_mul(4).wrapping_add(255) as i16).clamp(0, 255) as u8);
                if self.age < -63 {
                    update.alive = false;
                    point.voice_clock = 0;
                }
                self.age = self.age.wrapping_sub(2);
            }
            return update;
        }
        actor.health = actor.health.max(actor.max_health >> 1);
        actor.position = point.position;
        update.voice = self
            .voice(point, random)
            .then_some(self.transform.translation);
        self.age = self.age.wrapping_add(1);
        if self.transform.translation.y < self.floor.wrapping_sub(60) {
            self.transform.translation.y = self.transform.translation.y.wrapping_add(60);
            self.transform.rotation = math.rotate_yxz(self.rotation.to_array());
        } else {
            self.transform.translation.y = self.floor;
            if i32::from(self.rotation.z).abs() < 796 {
                self.rotation.z = self.rotation.z.wrapping_add(self.roll_step);
                self.transform.rotation = math.rotate_yxz(self.rotation.to_array());
            }
        }
        if self.age >= 109 {
            self.age = -1;
            point.return_clock = 0;
            update.returned = true;
            update.fade = Some(255);
        } else if self.age >= 46 {
            update.fade = Some(((self.age - 45).wrapping_mul(4) as i16).clamp(0, 255) as u8);
        } else if self.age == 15 {
            update.smoke = Some(
                self.transform
                    .translation
                    .wrapping_sub(Position::new(0, 500, 0)),
            );
        }
        update
    }

    fn voice(&self, point: &mut ReturnPoint, random: &mut Random) -> bool {
        if self.attacker.is_none() {
            return false;
        }
        let clock = point.voice_clock;
        point.voice_clock = clock.wrapping_add(1);
        clock & 15 == 15 && random.next15() & 3 == 0
    }
}
