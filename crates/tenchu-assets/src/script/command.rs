use crate::Result;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Placement {
    pub position: [i16; 3],
    pub yaw: i16,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActorCommand {
    Activate { placement: Option<Placement> },
    Hide,
    Animate { motion: i16, loops: i16, next: i16 },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CameraCommand {
    AroundTarget { yaw_offset: i16, distance: i16 },
    Position([i16; 3]),
    PositionAtActor(i16),
    LookAt([i16; 3]),
    LookAtActor(i16),
    Unchanged,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Pan {
    #[default]
    None,
    Gameplay,
    OrbitClockwise,
    OrbitCounterclockwise,
    Up,
    Down,
    In,
    Out,
    FollowTarget,
    Unchanged,
}

impl Pan {
    pub fn from_source_id(id: i16) -> Self {
        match id {
            0 => Self::None,
            1 => Self::Gameplay,
            2 => Self::OrbitClockwise,
            3 => Self::OrbitCounterclockwise,
            4 => Self::Up,
            5 => Self::Down,
            6 => Self::In,
            7 => Self::Out,
            8 => Self::FollowTarget,
            _ => Self::Unchanged,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EffectCommand {
    Blood { position: [i16; 3], count: i16 },
    Fade { color: [u8; 3], duration: i16 },
    None,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Command {
    Sequence { id: i16, music: i16 },
    Wait { until: i16 },
    Actor { id: i16, action: ActorCommand },
    Camera(CameraCommand),
    Pan { motion: Pan, speed: i16 },
    Effect(EffectCommand),
    Caption { offset: Option<usize> },
}

impl Command {
    pub fn decode(operation: i16, id: i16, position: [i16; 3], parameter: i16) -> Result<Self> {
        Ok(match operation {
            0 => Self::Sequence {
                id,
                music: parameter,
            },
            1 => Self::Wait { until: id },
            2 => Self::Actor {
                id,
                action: ActorCommand::Activate {
                    placement: (parameter != -1).then_some(Placement {
                        position,
                        yaw: parameter,
                    }),
                },
            },
            3 => Self::Actor {
                id,
                action: if position[0] == -1 {
                    ActorCommand::Hide
                } else {
                    let [motion, loops, next] = position;
                    ActorCommand::Animate {
                        motion,
                        loops,
                        next,
                    }
                },
            },
            4 => Self::Camera(match id {
                0..=3 => CameraCommand::AroundTarget {
                    yaw_offset: id * 1024,
                    distance: if parameter == 0 { 3000 } else { parameter },
                },
                4 => CameraCommand::Position(position),
                5 => CameraCommand::PositionAtActor(parameter),
                _ => CameraCommand::Unchanged,
            }),
            5 => Self::Camera(if id == 0 {
                CameraCommand::LookAt(position)
            } else {
                CameraCommand::LookAtActor(parameter)
            }),
            6 => Self::Pan {
                motion: Pan::from_source_id(id),
                speed: if parameter == 0 { 20 } else { parameter },
            },
            7 => Self::Effect(match id {
                1 => EffectCommand::Blood {
                    position,
                    count: parameter,
                },
                3 => EffectCommand::Fade {
                    color: position.map(|v| v as u8),
                    duration: parameter,
                },
                _ => EffectCommand::None,
            }),
            8 => Self::Caption {
                offset: if id == -1 {
                    None
                } else {
                    Some(usize::try_from(id).map_err(|_| "Negative CVA text offset")?)
                },
            },
            _ => return Err(format!("Unknown CVA command {operation}")),
        })
    }
}
