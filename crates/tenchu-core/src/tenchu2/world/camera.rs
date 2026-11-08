use super::{Assets, World};
use crate::{
    Result,
    tenchu2::camera::{Context, Subject},
};

impl World {
    pub fn update_camera(&mut self, assets: &Assets, reset: bool) -> Result<()> {
        let index = self.controls.camera_actor;
        let actor = &self
            .actors
            .get(index)
            .ok_or("Missing Tenchu 2 camera actor")?
            .actor;
        let mut subject = Subject {
            actor,
            root: actor.position,
            target: None,
        };
        if actor.motion.action as u16 & 0xfff0 == 0x920 {
            subject.root = self.instances[index]
                .rig
                .root_position(actor, &mut self.scene)?;
            if let Some(target) = actor.target {
                let target_actor = &self
                    .actors
                    .get(target)
                    .ok_or("Missing Tenchu 2 camera target")?
                    .actor;
                subject.target = Some((
                    self.instances[target]
                        .rig
                        .root_position(target_actor, &mut self.scene)?,
                    target_actor.rotation,
                ));
            }
        }
        let mut context = Context {
            map: &self.terrain,
            cursor: &mut self.cursor,
            math: &assets.resources.math,
            tables: &assets.camera,
            hero: assets.hero,
            sequence: self.sequence,
        };
        if reset {
            self.camera.reset(&subject, &mut context, &mut self.random)
        } else {
            self.camera
                .advance(&subject, &mut context, &mut self.random)
        }
    }
}
