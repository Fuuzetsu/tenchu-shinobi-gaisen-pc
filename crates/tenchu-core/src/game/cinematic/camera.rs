use super::*;
use crate::math::Position;

impl Game {
    fn cinematic_focus(&self, id: usize) -> Position {
        let actor = self.roster.get(id);
        Position::new(
            actor.position.x,
            actor
                .position
                .y
                .wrapping_sub(i32::from(self.mission.characters[actor.definition].height))
                .wrapping_add(300),
            actor.position.z,
        )
    }

    pub(super) fn cinematic_camera_command(&mut self, command: CameraCommand) -> Result<bool> {
        let mut position = self.camera.position;
        let mut reference = self.camera.reference;
        match command {
            CameraCommand::AroundTarget {
                yaw_offset,
                distance,
            } => {
                let target = self.roster.get(self.cinematic.target);
                let yaw = target.rotation[1].wrapping_add(yaw_offset);
                let delta = self.math.velocity(yaw, distance, 0);
                position = self.cinematic_focus(self.cinematic.target);
                position[0] = position[0].wrapping_add(i32::from(delta[0]));
                position[2] = position[2].wrapping_add(i32::from(delta[2]));
            }
            CameraCommand::Position(value) => position = value.map(|v| i32::from(v) * 100).into(),
            CameraCommand::PositionAtActor(kind) => {
                let Some(id) = self.roster.find(kind) else {
                    return Ok(true);
                };
                position = self.cinematic_focus(id);
                self.cinematic.target = id;
            }
            CameraCommand::LookAt(value) => reference = value.map(|v| i32::from(v) * 100).into(),
            CameraCommand::LookAtActor(kind) => {
                let Some(id) = self.roster.find(kind) else {
                    return Ok(false);
                };
                reference = self.cinematic_focus(id);
                self.cinematic.target = id;
            }
            CameraCommand::Unchanged => {}
        }
        self.camera.set_view(position, reference, &self.math)?;
        Ok(true)
    }

    pub(in crate::game) fn cinematic_camera(&mut self, input: u16) -> Result<()> {
        let mut position = self.camera.position;
        let mut reference = self.camera.reference;
        let x = position[0].wrapping_sub(reference[0]);
        let z = position[2].wrapping_sub(reference[2]);
        let mut length = self.math.length(Position::new(x, 0, z));
        let mut yaw = self.math.direction(x, z, 0)?;
        let speed = i32::from(self.cinematic.speed);
        match self.cinematic.pan {
            Pan::None => return Ok(()),
            Pan::Gameplay => {
                return self.camera.advance(
                    self.roster.get(self.player_id),
                    input,
                    &mut crate::camera::Environment {
                        terrain: &self.mission.terrain,
                        cursor: &mut self.cursor,
                        math: &self.math,
                        body: &self.body,
                    },
                    &mut self.scene,
                );
            }
            Pan::OrbitClockwise | Pan::OrbitCounterclockwise | Pan::In | Pan::Out => {
                match self.cinematic.pan {
                    Pan::OrbitClockwise => yaw = yaw.wrapping_add(self.cinematic.speed),
                    Pan::OrbitCounterclockwise => yaw = yaw.wrapping_sub(self.cinematic.speed),
                    Pan::In => length = length.wrapping_sub(speed),
                    Pan::Out => length = length.wrapping_add(speed),
                    _ => unreachable!(),
                }
                let delta = self.math.velocity(yaw, length as i16, 0);
                position[0] = reference[0].wrapping_add(i32::from(delta[0]));
                position[2] = reference[2].wrapping_add(i32::from(delta[2]));
            }
            Pan::Up => position[1] = position[1].wrapping_sub(speed),
            Pan::Down => position[1] = position[1].wrapping_add(speed),
            Pan::FollowTarget => reference = self.cinematic_focus(self.cinematic.target),
            Pan::Unchanged => (),
        }
        self.camera.set_view(position, reference, &self.math)
    }
}
