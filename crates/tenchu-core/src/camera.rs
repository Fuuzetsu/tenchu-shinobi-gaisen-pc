mod smoothing;
#[cfg(test)]
mod tests;

use crate::{
    Result,
    actor::Actor,
    ids::*,
    input::*,
    math::{Math, Matrix},
    random::Random,
    terrain::{self, Cursor},
};
use smoothing::Smooth;
use tenchu_assets::{
    camera::{CameraTables, Pose},
    executable::BodyTables,
    terrain::AreaMap,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum Mode {
    Normal = 0,
    Direction = 1,
    System = 2,
    Sight = 3,
    Critical = 4,
    Fight = 5,
    StickLeft = 6,
    StickRight = 7,
    Swim = 8,
    PeepLeft = 9,
    PeepRight = 10,
    Crouch = 11,
    Run = 12,
    Lock = 13,
    Fall = 14,
    Aim = 15,
    Knockback = 16,
    Hang = 17,
}

pub struct Environment<'a> {
    pub terrain: &'a AreaMap,
    pub cursor: &'a mut Cursor,
    pub math: &'a Math,
    pub body: &'a BodyTables,
}

#[derive(Clone, Debug)]
pub struct Camera {
    pub position: [i32; 3],
    pub reference: [i32; 3],
    pub target: [i32; 3],
    pub mode: Mode,
    pub old_mode: u8,
    pub direction: [i16; 2],
    pub snap_pending: bool,
    pub view: Matrix,
    smoothing: [Smooth; 2],
    tables: CameraTables,
}

fn angles(from: [i32; 3], to: [i32; 3], math: &Math) -> Result<[i16; 2]> {
    let [x, y, z] = std::array::from_fn(|i| to[i].wrapping_sub(from[i]));
    Ok([
        math.atan2(y, math.length([x, 0, z]))?,
        math.atan2(x.wrapping_neg(), z.wrapping_neg())?,
    ])
}

fn rotate_short(math: &Math, rotation: [i16; 3], point: [i16; 3]) -> [i16; 3] {
    Matrix {
        rotation: math.rotate_yxz(rotation),
        translation: [0; 3],
    }
    .transform(point)
    .map(|v| v.clamp(-32768, 32767) as i16)
}

fn terrain_pitch(owner: &Actor, env: &Environment) -> Result<i16> {
    let node = env.terrain.node(owner.map.area)?;
    let height = |p: [i32; 3]| -> Result<i32> {
        let [x1, z1, x2, z2] = node.bounds;
        let x = (p[0] / 10) as i16;
        let z = (p[2] / 10) as i16;
        let xs = i32::from(x2.wrapping_sub(x1).wrapping_add(1));
        let zs = i32::from(z2.wrapping_sub(z1).wrapping_add(1));
        let delta = match node.attributes & 0xc000 {
            0x4000 => (i32::from(x.wrapping_sub(x1)) * i32::from(node.dy)).checked_div(xs),
            0x8000 => (i32::from(z.wrapping_sub(z1)) * i32::from(node.dy)).checked_div(zs),
            _ => Some(0),
        }
        .ok_or("Zero camera slope span")?;
        Ok(i32::from(node.y.wrapping_add(delta as i16)) * 10)
    };
    let [sin, cos] = env.math.pair(owner.rotation[1]);
    let shifted = [
        owner.position[0].wrapping_sub(sin / 16),
        owner.position[1],
        owner.position[2].wrapping_sub(cos / 16),
    ];
    let delta = height(shifted)?.wrapping_sub(height(owner.position)?);
    if delta <= -700 {
        return Ok(0);
    }
    Ok(env.math.atan2(delta, 256)?.clamp(-341, 341))
}

impl Camera {
    #[cfg(test)]
    pub(crate) fn smoothing_state(&self) -> [[i16; 6]; 2] {
        std::array::from_fn(|i| self.smoothing[i].state())
    }

    pub(crate) fn snap_target(&mut self, fallback: [i32; 3], env: &mut Environment) -> Result<()> {
        let delta = std::array::from_fn(|i| {
            (self.reference[i] as i16).wrapping_sub(self.position[i] as i16)
        });
        let step = env.math.normalize_short(delta).map(|v| v / 32);
        self.target =
            terrain::passage(env.terrain, env.cursor, self.position, step, -1)?.unwrap_or(fallback);
        Ok(())
    }

    pub fn new(tables: CameraTables, start: [i32; 3], math: &Math) -> Result<Self> {
        let mut camera = Self {
            position: [start[0], start[1].wrapping_sub(10000), start[2]],
            reference: start,
            target: [0; 3],
            mode: Mode::Normal,
            old_mode: 0,
            direction: [0; 2],
            snap_pending: false,
            view: Matrix::default(),
            smoothing: tables.smoothing.map(Smooth::from),
            tables,
        };
        camera.update_view(math)?;
        Ok(camera)
    }

    fn update_view(&mut self, math: &Math) -> Result<()> {
        self.set_view(self.position, self.reference, math)
    }

    pub fn set_view(&mut self, position: [i32; 3], reference: [i32; 3], math: &Math) -> Result<()> {
        self.position = position;
        self.reference = reference;
        let mut maximum = 0;
        for value in position.into_iter().chain(reference) {
            maximum = maximum.max(
                value
                    .checked_abs()
                    .ok_or("Camera coordinate magnitude overflow")?,
            );
        }
        let shift = (32 - (maximum as u32).leading_zeros()).saturating_sub(15);
        let delta =
            std::array::from_fn(|i| (reference[i] >> shift).wrapping_sub(position[i] >> shift));
        let length = math.length(delta);
        self.view = Matrix::default();
        if length == 0 {
            return Ok(());
        }
        let horizontal = math.length([delta[0], 0, delta[2]]);
        let sin = delta[1].wrapping_shl(12) / length;
        let cos = horizontal.wrapping_shl(12) / length;
        let pitch = Matrix {
            rotation: [
                [4096, 0, 0],
                [0, cos as i16, sin.wrapping_neg() as i16],
                [0, sin as i16, cos as i16],
            ],
            ..Matrix::default()
        };
        self.view = self.view.compose(&pitch);
        if horizontal != 0 {
            let sin = (delta[0].wrapping_shl(12) / horizontal).wrapping_neg();
            let cos = delta[2].wrapping_shl(12) / horizontal;
            self.view = self.view.compose(&Matrix {
                rotation: [
                    [cos as i16, 0, sin as i16],
                    [0, 4096, 0],
                    [sin.wrapping_neg() as i16, 0, cos as i16],
                ],
                ..Matrix::default()
            });
        }
        self.view.translation = self.view.apply_large(position.map(i32::wrapping_neg));
        Ok(())
    }

    fn direction_mode(&mut self, owner: &Actor, math: &Math, mode: Mode) -> Result<()> {
        if !matches!(self.mode, Mode::Direction | Mode::Lock) {
            let yaw = i32::from(angles(self.position, self.reference, math)?[1]);
            self.direction = [
                0,
                ((yaw - i32::from(owner.rotation[1]) + 10240) % 4096 - 2048) as i16,
            ];
        }
        self.mode = Mode::Direction;
        self.old_mode = mode as u8;
        Ok(())
    }

    pub fn normal(&mut self, owner: &Actor, math: &Math) -> Result<()> {
        if owner.pad.held & L1 != 0 {
            self.direction_mode(owner, math, Mode::Direction)
        } else {
            self.mode = Mode::Normal;
            Ok(())
        }
    }

    pub fn set_mode(
        &mut self,
        mode: Mode,
        owner: &Actor,
        env: &mut Environment,
        rng: &mut Random,
    ) -> Result<()> {
        match mode {
            Mode::Normal => self.normal(owner, env.math)?,
            Mode::Aim => {
                self.direction = [0; 2];
                self.mode = Mode::Direction;
                self.old_mode = mode as u8;
            }
            Mode::Direction | Mode::Sight => self.direction_mode(owner, env.math, mode)?,
            Mode::Critical => {
                self.old_mode = (rng.next15() % 4) as u8;
                for _ in 0..4 {
                    self.old_mode = (self.old_mode + 1) % 4;
                    let pose = self.tables.critical[self.old_mode as usize];
                    let mut rotation = owner.rotation;
                    rotation[0] = rotation[0].wrapping_add(terrain_pitch(owner, env)?);
                    let matrix = Matrix {
                        rotation: env.math.rotate_yxz(rotation),
                        translation: owner.position,
                    };
                    let (time, _) = terrain::ground_ray(
                        env.terrain,
                        env.cursor,
                        env.math,
                        matrix.transform(pose[2]),
                        matrix.transform(pose[3]),
                        0,
                    )?;
                    if time >= 2048 {
                        self.mode = mode;
                        self.snap_pending = true;
                        return Ok(());
                    }
                }
                self.old_mode = 0;
            }
            _ => self.mode = mode,
        }
        Ok(())
    }

    fn anti_wall(
        &self,
        position: &mut [i32; 3],
        reference: [i32; 3],
        env: &mut Environment,
    ) -> Result<()> {
        let [pitch, yaw] = angles(*position, reference, env.math)?;
        let matrix = Matrix {
            rotation: env.math.rotate_yxz([pitch, yaw, 0]),
            translation: [0; 3],
        };
        let probes = self.tables.wall.map(|point| matrix.transform(point));
        let right = terrain::level(
            env.terrain,
            env.cursor,
            std::array::from_fn(|i| position[i].wrapping_add(probes[1][i])),
            0,
        )?;
        let left = terrain::level(
            env.terrain,
            env.cursor,
            std::array::from_fn(|i| position[i].wrapping_add(probes[0][i])),
            0,
        )?;
        let mask = u8::from(left <= position[1]) | (u8::from(right <= position[1]) << 1);
        if mask != 0 {
            let mut push = match mask {
                1 => matrix.transform([1000, 0, 0]),
                2 => matrix.transform([-1000, 0, 0]),
                _ => [0; 3],
            };
            let half = push.map(|v| v / 2);
            let test =
                std::array::from_fn(|i| position[i].wrapping_add(push[i]).wrapping_add(half[i]));
            if terrain::level(env.terrain, env.cursor, test, 0)? <= position[1] {
                push = half.map(|v| v / 2);
            }
            for i in 0..3 {
                position[i] = position[i].wrapping_add(push[i]);
            }
        }
        Ok(())
    }

    fn pose_delta(
        &mut self,
        owner: &Actor,
        pose: Pose,
        env: &mut Environment,
    ) -> Result<(i32, [[i32; 3]; 2])> {
        let mut rotation = owner.rotation;
        rotation[0] = rotation[0].wrapping_add(terrain_pitch(owner, env)?);
        let origin = [
            owner.position[0],
            owner.position[1].wrapping_sub(1550),
            owner.position[2],
        ];
        let matrix = Matrix {
            rotation: env.math.rotate_yxz(rotation),
            translation: origin,
        };
        let [a, b, c, d] = pose.map(|v| matrix.transform(v));
        let (time, mut position) = terrain::ground_ray(env.terrain, env.cursor, env.math, c, d, 0)?;
        let reference = std::array::from_fn(|i| {
            a[i].wrapping_add(b[i].wrapping_sub(a[i]).wrapping_mul(time) / 4096)
        });
        self.anti_wall(&mut position, reference, env)?;
        let delta = if self.snap_pending {
            self.snap_pending = false;
            [
                std::array::from_fn(|i| reference[i].wrapping_sub(self.reference[i])),
                std::array::from_fn(|i| position[i].wrapping_sub(self.position[i])),
            ]
        } else {
            [
                self.smoothing[0].delta(self.reference, reference, env.math)?,
                self.smoothing[1].delta(self.position, position, env.math)?,
            ]
        };
        Ok((time, delta))
    }

    fn wall_mode(&mut self, owner: &Actor, env: &mut Environment) -> Result<()> {
        let [left, right] = self
            .tables
            .stick
            .map(|v| rotate_short(env.math, [owner.rotation[0], owner.rotation[1], 0], v));
        let mut mask = 0;
        for (v, sign, bit) in [(left, 1, 1), (right, 1, 2), (left, -1, 8), (right, -1, 4)] {
            let point = [
                owner.position[0].wrapping_add(i32::from(v[0]) * sign),
                owner.position[1],
                owner.position[2].wrapping_add(i32::from(v[2]) * sign),
            ];
            if terrain::level(env.terrain, env.cursor, point, terrain::STEP_DOWN)? == terrain::NONE
            {
                mask |= bit;
            }
        }
        self.mode = match mask {
            4 => Mode::PeepRight,
            8 => Mode::PeepLeft,
            _ if mask & 3 == 1 => Mode::StickRight,
            _ if mask & 3 == 2 => Mode::StickLeft,
            _ => Mode::Normal,
        };
        Ok(())
    }

    fn follow(&mut self, owner: &Actor, env: &mut Environment) -> Result<[[i32; 3]; 2]> {
        match owner.status {
            12 => self.wall_mode(owner, env)?,
            11 => self.mode = Mode::Crouch,
            3 => self.mode = Mode::Swim,
            10 => self.mode = Mode::Hang,
            6 if owner.motion.action == MOT_CHASE => self.mode = Mode::Run,
            8 if owner.motion.action == MOT_STATE_CLIMB => self.mode = Mode::Run,
            16 if (MOT_DAMAGE_LAUNCH_BACK..=MOT_DAMAGE_DOWNED).contains(&owner.motion.action)
                || owner.motion.action == MOT_DAMAGE_GETUP =>
            {
                self.mode = Mode::Knockback
            }
            _ => {}
        }
        let index = match self.mode {
            Mode::StickLeft => Some(0),
            Mode::StickRight => Some(1),
            Mode::PeepLeft => Some(2),
            Mode::PeepRight => Some(3),
            Mode::Crouch => Some(4),
            Mode::Run => Some(5),
            Mode::Swim => Some(6),
            Mode::Knockback => Some(7),
            Mode::Hang => Some(9),
            _ => None,
        };
        let pose = if self.mode == Mode::Critical {
            *self
                .tables
                .critical
                .get(self.old_mode as usize)
                .ok_or("Invalid critical camera index")?
        } else {
            index.map_or(self.tables.ordinary, |i| self.tables.poses[i])
        };
        let (time, mut delta) = self.pose_delta(owner, pose, env)?;
        if self.mode == Mode::Knockback && time <= 2048 {
            delta = self.pose_delta(owner, self.tables.poses[8], env)?.1;
        }
        if matches!(
            self.mode,
            Mode::Swim | Mode::Crouch | Mode::Run | Mode::Knockback | Mode::Hang
        ) {
            self.mode = Mode::Normal;
        }
        Ok(delta)
    }

    fn look(&mut self, owner: &Actor, input: u16, env: &mut Environment) -> Result<[[i32; 3]; 2]> {
        let mut x = if input & RIGHT != 0 {
            45
        } else if input & LEFT != 0 {
            -45
        } else {
            0
        };
        let mut y = if input & DOWN != 0 {
            45
        } else if input & UP != 0 {
            -45
        } else {
            0
        };
        if self.old_mode == Mode::Sight as u8 {
            x /= 2;
            y /= 2;
        } else if owner.pad.held & L1 == 0 {
            self.normal(owner, env.math)?;
        }
        self.direction[0] = self.direction[0].wrapping_sub(y).clamp(-910, 910);
        self.direction[1] = self.direction[1].wrapping_add(x).clamp(-1024, 1024);
        let r = rotate_short(
            env.math,
            [
                owner.rotation[0].wrapping_add(self.direction[0]),
                owner.rotation[1].wrapping_add(self.direction[1]),
                owner.rotation[2],
            ],
            [0, 0, 1200],
        )
        .map(i32::from);
        let mut origin = [
            owner.position[0],
            owner.position[1].wrapping_sub(1550),
            owner.position[2],
        ];
        let outer = terrain::vector(env.terrain, env.cursor, origin, 1000, 0)?;
        if outer.level != terrain::NONE && outer.vector != 0 {
            let inner = terrain::vector(env.terrain, env.cursor, origin, 500, 0)?;
            let (amount, mask) = if inner.vector == 0 {
                (500, outer.vector)
            } else {
                (1000, inner.vector)
            };
            for (i, axis) in [0, 2].into_iter().enumerate() {
                origin[axis] = origin[axis].wrapping_add(
                    i32::from(env.body.reflect_move[mask as usize][i].signum()) * amount,
                );
            }
        }
        let [sin, cos] = env.math.pair(owner.rotation[1]);
        origin[0] = origin[0].wrapping_sub(sin / 6);
        origin[2] = origin[2].wrapping_sub(cos / 6);
        origin[1] = origin[1].wrapping_add(if r[1] > 0 { -r[1] } else { r[1] / 4 });
        Ok([
            std::array::from_fn(|i| {
                origin[i].wrapping_sub(r[i]).wrapping_sub(self.reference[i]) / 4
            }),
            std::array::from_fn(|i| {
                origin[i].wrapping_add(r[i]).wrapping_sub(self.position[i]) / 8
            }),
        ])
    }

    pub fn advance(
        &mut self,
        owner: &Actor,
        input: u16,
        env: &mut Environment,
        scene: &mut crate::scene::Scene,
    ) -> Result<()> {
        let [reference, position] = match self.mode {
            Mode::Direction => self.look(owner, input, env)?,
            Mode::Lock => [[0; 3]; 2],
            Mode::Fall => [
                std::array::from_fn(|i| owner.position[i].wrapping_sub(self.reference[i])),
                [0; 3],
            ],
            _ => {
                if owner.pad.held & L1 != 0 {
                    return self.direction_mode(owner, env.math, Mode::Direction);
                }
                scene.models[owner.root_model].attribute |= crate::scene::NO_CULL;
                self.follow(owner, env)?
            }
        };
        for i in 0..3 {
            self.reference[i] = self.reference[i].wrapping_add(reference[i]);
            self.position[i] = self.position[i].wrapping_add(position[i]);
        }
        self.update_view(env.math)?;
        Ok(())
    }
}
