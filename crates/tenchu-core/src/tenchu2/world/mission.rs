use super::{Assets, World};
use crate::{
    Result,
    tenchu2::{
        ai::{Program, direction},
        audio::Sound,
        mission::{Input, Vitals},
    },
};

impl World {
    pub fn advance_mission(&mut self, assets: &Assets) -> Result<Option<Sound>> {
        let actor = &self.actors[self.player].actor;
        let update = self.mission.update(Input {
            tick: self.tick,
            position: actor.position,
            health: actor.health,
            state: actor.state,
            motion_loops: actor.motion.loops,
            killed: self.statistics.killed,
            detections: self.ai.detections,
            targets: self.mission.targets.map(|index| {
                index.map(|i| {
                    let actor = &self.actors[i].actor;
                    Vitals {
                        health: actor.health,
                        maximum: actor.max_health,
                    }
                })
            }),
        });
        if update.face_spotter
            && let Some(index) = self.ai.last_spotter
        {
            let player = self.actors[self.player].actor.position;
            let actor = &mut self.actors[index].actor;
            let delta = player.wrapping_sub(actor.position);
            actor.rotation.y = actor.rotation.y.wrapping_add(direction(
                &assets.resources.math,
                delta.x,
                delta.z,
                actor.rotation.y,
            )?);
            self.controls.camera_actor = index;
            self.update_camera(assets, true)?;
        }
        if update.began {
            self.actors[self.player].behavior.programs.idle = Program::Disabled;
            for agent in &mut self.actors {
                agent.actor.flags &= !0x10;
            }
        }
        if update.detection_sound && self.random.next15() % 3 == 0 {
            let note = 36 - (self.random.next15() % 4) as u8;
            let mut sound = self
                .audio
                .play(47, 0, 127, self.tick, &assets.resources.math);
            if let Some(sound) = &mut sound {
                sound.note = note;
            }
            return Ok(sound);
        }
        Ok(None)
    }
}
