use super::{
    actor::Actor,
    terrain::{self, Cursor, Query},
};
use crate::{
    Result,
    camera::smoothing::Smooth,
    math::{Math, Matrix, Position, Vector3},
    random::Random,
};
use serde::{Deserialize, Serialize};
use tenchu_assets::{
    camera::Pose,
    tenchu2::{camera::Tables, collision::Collision},
};

mod geometry;
use geometry::avoid_walls;
mod follow;

pub struct Subject<'a> {
    pub actor: &'a Actor,
    pub root: Position,
    pub target: Option<(Position, Vector3<i16>)>,
}

pub struct Context<'a> {
    pub map: &'a Collision,
    pub cursor: &'a mut Cursor,
    pub math: &'a Math,
    pub tables: &'a Tables,
    pub hero: usize,
    pub sequence: u16,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Camera {
    pub eye: Position,
    pub look: Position,
    pub roll: i32,
    pub mode: i16,
    pub previous: i16,
    pub direction: [i16; 2],
    pub snap: bool,
    pub underwater: bool,
    pub view: Matrix,
    counter: i32,
    orbit: i32,
    death_style: i16,
    held_yaw: i16,
    held_action: i16,
    history: [[i32; 3]; 3],
    smoothing: [Smooth; 2],
}

impl Camera {
    pub fn new(tables: &Tables) -> Self {
        Self {
            eye: Position::ZERO,
            look: Position::ZERO,
            roll: 0,
            mode: 0,
            previous: 0,
            direction: [0; 2],
            snap: true,
            underwater: false,
            view: Matrix::default(),
            counter: 0,
            orbit: 0,
            death_style: 0,
            held_yaw: 0,
            held_action: 0,
            history: [[0; 3]; 3],
            smoothing: tables.smoothing.map(Smooth::from),
        }
    }

    pub fn reset(
        &mut self,
        subject: &Subject<'_>,
        c: &mut Context<'_>,
        random: &mut Random,
    ) -> Result<()> {
        self.underwater = false;
        self.mode = 0;
        self.snap = true;
        self.roll = 0;
        self.eye = subject.actor.position;
        self.look = Position::new(0, self.eye.y, self.eye.z);
        self.advance(subject, c, random)
    }

    fn update_view(&mut self, math: &Math) -> Result<()> {
        self.view = crate::camera::view_matrix(self.eye, self.look, self.roll, math)?;
        Ok(())
    }

    fn smooth(&mut self, target: [Position; 2], action: i16, math: &Math) -> Result<[Position; 2]> {
        let [eye, look] = target;
        if self.snap {
            return Ok([eye.wrapping_sub(self.eye), look.wrapping_sub(self.look)]);
        }
        match (self.mode, action) {
            (2, _) => self.smoothing[1].shift = 3,
            (4, 0x211) => self.smoothing[1].shift = 1,
            (1, 0x104) => {
                self.smoothing[0].shift = 1;
                self.smoothing[1].shift = 2;
                for filter in &mut self.smoothing {
                    filter.acceleration = 24;
                }
            }
            _ => {}
        }
        let mut look = self.smoothing[0].delta_with_length(self.look, look, |v| length(v, math))?;
        let eye = self.smoothing[1].delta_with_length(self.eye, eye, |v| length(v, math))?;
        if self.mode == 14 {
            look.y = target[1].y.wrapping_sub(self.look.y) >> 1;
        }
        match (self.mode, action) {
            (2, _) | (4, 0x211) => self.smoothing[1].shift = 4,
            (1, 0x104) => {
                self.smoothing[0].shift = 2;
                self.smoothing[1].shift = 4;
                for filter in &mut self.smoothing {
                    filter.acceleration = 14;
                }
            }
            _ => {}
        }
        Ok([eye, look])
    }

    fn solve(
        &mut self,
        subject: &Subject<'_>,
        rotation: Vector3<i16>,
        pose: Pose,
        c: &mut Context<'_>,
    ) -> Result<[Position; 2]> {
        let [near_look, far_look, mut near_eye, mut far_eye] =
            self.endpoints(subject, rotation, pose, true, c)?;
        let (ratio, mut eye) = if self.mode == 15 {
            (4096, far_eye)
        } else {
            self.clip(subject.actor.position, &mut near_eye, &mut far_eye, c)?
        };
        let mut look = Position::from_fn(|i| {
            near_look[i]
                .wrapping_add(far_look[i].wrapping_sub(near_look[i]).wrapping_mul(ratio) >> 12)
        });
        if ratio > 1023 {
            avoid_walls(&mut eye, 500, c)?;
        }
        c.cursor.current = None;
        if let Some(floor) = terrain::level(c.map, c.cursor, eye, Query::Uncached)?
            && c.cursor.material & 0xf00 == 0
            && c.cursor.current.is_some_and(|id| {
                i32::from(c.map.cells[id.cell].nodes[id.node].y) * 10 != subject.actor.position.y
            })
            && floor.wrapping_sub(eye.y) < 701
        {
            look.y = look.y.wrapping_add(500);
            eye.y = eye.y.wrapping_sub(1000);
        }
        if self.mode == 15 {
            return Ok([eye, look]);
        }
        let mut delta = self.smooth([eye, look], subject.actor.motion.action, c.math)?;
        if !self.snap {
            for (axis, history) in self.history.iter_mut().enumerate() {
                history.rotate_left(1);
                history[2] = delta[0][axis];
                delta[0][axis] = history[1].wrapping_add(
                    history[0]
                        .wrapping_sub(history[1].wrapping_mul(2))
                        .wrapping_add(history[2])
                        >> 2,
                );
            }
        }
        Ok(delta)
    }

    fn stealth(
        &mut self,
        subject: &Subject<'_>,
        preferred: bool,
        c: &mut Context<'_>,
        random: &mut Random,
    ) -> Result<[Position; 2]> {
        let action = usize::from(subject.actor.motion.action as u16 & 7);
        let poses = c
            .tables
            .stealth
            .get(action)
            .ok_or("Invalid Tenchu 2 stealth camera action")?;
        let hero = if c.hero < 3 { c.hero } else { 0 };
        if preferred && random.next15() & 1 != 0 {
            let pose = poses[hero];
            let [_, _, mut near, mut far] =
                self.endpoints(subject, subject.actor.rotation, pose, true, c)?;
            if self.clip(subject.actor.position, &mut near, &mut far, c)?.0 >= 4096 {
                return self.solve(subject, subject.actor.rotation, pose, c);
            }
        }
        let start = random.next15() as usize % 7;
        for i in 0..7 {
            let pose = c.tables.stealth[(start + i) % 7][3];
            let [_, _, mut near, mut far] =
                self.endpoints(subject, subject.actor.rotation, pose, false, c)?;
            if self.clip(subject.actor.position, &mut near, &mut far, c)?.0 >= 4096 {
                return self.solve(subject, subject.actor.rotation, pose, c);
            }
        }
        self.solve(
            subject,
            subject.actor.rotation,
            if preferred {
                poses[3]
            } else {
                c.tables.stealth[2][3]
            },
            c,
        )
    }
}

fn length(vector: Position, math: &Math) -> i32 {
    if vector.into_iter().any(|v| v.wrapping_abs() >= 32768) {
        math.length(vector.map(|v| v >> 8)).wrapping_shl(8)
    } else {
        math.length(vector)
    }
}

fn rotate(vector: [i16; 3], rotation: Vector3<i16>, math: &Math) -> Vector3<i16> {
    Matrix {
        rotation: math.rotate_yxz(rotation.to_array()),
        translation: Position::ZERO,
    }
    .transform(vector)
    .map(|v| v.clamp(-32768, 32767) as i16)
}
