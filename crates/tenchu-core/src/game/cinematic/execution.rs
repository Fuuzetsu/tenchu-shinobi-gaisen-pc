use crate::ids::MOT_DEAD;
use tenchu_assets::script::{ActorCommand, CameraCommand, Command, EffectCommand, Pan, Placement};

/// Restore SA's execution shots without replacing localized captions or timing.
pub(super) fn commands(source: &[Command]) -> Vec<Command> {
    use CameraCommand::{LookAt, LookAtActor, Position};
    use Command::Camera;
    let mut out = Vec::with_capacity(source.len() + 8);
    let mut sequence = -1;
    let mut until = 0;
    let mut severed = false;
    let mut hero_shot = false;
    for &command in source {
        let mut command = command;
        if let Command::Sequence { id, .. } = command {
            sequence = id;
        }
        if let Command::Wait { until: frame } = command {
            until = frame;
        }
        if sequence == 20
            && let Command::Actor {
                id: 0x8f,
                action: ActorCommand::Activate { placement },
            } = &mut command
            && placement.is_some_and(|p| p.position == [-21, 0, -3])
        {
            *placement = Some(Placement {
                position: [-25, 0, -3],
                yaw: 1024,
            });
        }
        if sequence == 100 {
            match command {
                Command::Actor {
                    id: 0x8f,
                    action:
                        ActorCommand::Animate {
                            motion: MOT_DEAD, ..
                        },
                } => {
                    out.extend([
                        Camera(Position([-250, -60, -60])),
                        Camera(LookAtActor(0x8f)),
                        command,
                        Command::Wait { until },
                        Camera(Position([-290, 0, -50])),
                        Camera(LookAt([-290, -10, -20])),
                        Command::Pan {
                            motion: Pan::OrbitCounterclockwise,
                            speed: 5,
                        },
                    ]);
                    severed = true;
                    continue;
                }
                Command::Actor {
                    id: 0,
                    action: ActorCommand::Animate { motion: 265, .. },
                } if severed => {
                    out.extend([
                        command,
                        Camera(Position([-250, -60, -60])),
                        Camera(LookAtActor(0)),
                        Command::Pan {
                            motion: Pan::None,
                            speed: 20,
                        },
                    ]);
                    hero_shot = true;
                    continue;
                }
                Command::Caption { offset: Some(_) } if hero_shot => {
                    out.extend([
                        Camera(Position([-330, -30, 10])),
                        Camera(LookAt([-280, 0, -20])),
                        Command::Pan {
                            motion: Pan::Up,
                            speed: 1,
                        },
                    ]);
                    severed = false;
                    hero_shot = false;
                }
                Command::Camera(_)
                | Command::Pan { .. }
                | Command::Effect(EffectCommand::Blood { .. })
                    if severed =>
                {
                    continue;
                }
                Command::Effect(EffectCommand::Blood {
                    position: [-2900, -40, -200],
                    ..
                }) => {
                    command = Command::Effect(EffectCommand::Blood {
                        position: [-2900, -40, -200],
                        count: 60,
                    });
                }
                _ => (),
            }
        }
        out.push(command);
    }
    out
}
