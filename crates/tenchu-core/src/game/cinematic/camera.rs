use super::*;

impl Game {
    fn cinematic_focus(&self, id: usize) -> [i32; 3] {
        let actor = self.roster.get(id);
        [
            actor.position[0],
            actor.position[1]
                .wrapping_sub(i32::from(self.mission.characters[actor.definition].height))
                .wrapping_add(300),
            actor.position[2],
        ]
    }

    pub(super) fn cinematic_camera_command(&mut self, command: Command) -> Result<bool> {
        let mut position = self.camera.position;
        let mut reference = self.camera.reference;
        if command.operation == 4 {
            match command.id {
                0..=3 => {
                    let target = self.roster.get(self.cinematic.target);
                    let yaw = target.rotation[1].wrapping_add(command.id * 1024);
                    let distance = if command.parameter == 0 {
                        3000
                    } else {
                        command.parameter
                    };
                    let delta = self.math.velocity(yaw, distance, 0);
                    position = self.cinematic_focus(self.cinematic.target);
                    position[0] = position[0].wrapping_add(i32::from(delta[0]));
                    position[2] = position[2].wrapping_add(i32::from(delta[2]));
                }
                4 => position = command.position.map(|v| i32::from(v) * 100),
                5 => {
                    let Some(id) = self.roster.find(command.parameter) else {
                        return Ok(true);
                    };
                    position = self.cinematic_focus(id);
                    self.cinematic.target = id;
                }
                _ => (),
            }
        } else if command.id == 0 {
            reference = command.position.map(|v| i32::from(v) * 100);
        } else {
            let Some(id) = self.roster.find(command.parameter) else {
                return Ok(false);
            };
            reference = self.cinematic_focus(id);
            self.cinematic.target = id;
        }
        self.camera.set_view(position, reference, &self.math)?;
        Ok(true)
    }

    pub(in crate::game) fn cinematic_camera(&mut self, input: u16) -> Result<()> {
        let mut position = self.camera.position;
        let mut reference = self.camera.reference;
        let x = position[0].wrapping_sub(reference[0]);
        let z = position[2].wrapping_sub(reference[2]);
        let mut length = self.math.length([x, 0, z]);
        let mut yaw = self.math.direction(x, z, 0)?;
        let speed = i32::from(self.cinematic.speed);
        match self.cinematic.pan {
            0 => return Ok(()),
            1 => {
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
            2 | 3 | 6 | 7 => {
                match self.cinematic.pan {
                    2 => yaw = yaw.wrapping_add(self.cinematic.speed),
                    3 => yaw = yaw.wrapping_sub(self.cinematic.speed),
                    6 => length = length.wrapping_sub(speed),
                    7 => length = length.wrapping_add(speed),
                    _ => unreachable!(),
                }
                let delta = self.math.velocity(yaw, length as i16, 0);
                position[0] = reference[0].wrapping_add(i32::from(delta[0]));
                position[2] = reference[2].wrapping_add(i32::from(delta[2]));
            }
            4 => position[1] = position[1].wrapping_sub(speed),
            5 => position[1] = position[1].wrapping_add(speed),
            8 => reference = self.cinematic_focus(self.cinematic.target),
            _ => (),
        }
        self.camera.set_view(position, reference, &self.math)
    }
}
