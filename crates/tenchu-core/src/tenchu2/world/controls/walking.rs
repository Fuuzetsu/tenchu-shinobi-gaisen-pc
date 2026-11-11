use super::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Walking {
    pub clip: i16,
    pub held: bool,
}

impl Walking {
    pub(super) fn motion(self, actor: &mut Actor, resources: &Resources) -> Result<()> {
        if !matches!(actor.motion.action, 3 | 0x103) {
            return Ok(());
        }
        let clip = if self.held {
            Some(self.clip)
        } else {
            resources
                .registry
                .clip(actor.definition_kind(), actor.motion.action)
        };
        if let Some(clip) = clip
            && actor.motion.clip != Some(clip)
        {
            actor
                .motion
                .select_clip(actor.motion.action, clip, &resources.clips)?;
            actor.update_velocity(Velocity::Motion, resources)?;
        }
        Ok(())
    }
}

impl World {
    pub fn set_player_walk(&mut self, clip: Option<i16>, assets: &Assets) -> Result<()> {
        if clip == self.controls.walking.map(|walking| walking.clip) {
            return Ok(());
        }
        if let Some(clip) = clip
            && !assets.resources.clips.get(&clip).is_some_and(|clip| {
                clip.duration > 0 && clip.forward_speed > 0 && !clip.bones.is_empty()
            })
        {
            return Err("Walking requires a moving, animated native T2 clip".into());
        }
        if let Some(walking) = self.controls.walking {
            Walking {
                held: false,
                ..walking
            }
            .motion(&mut self.actors[self.player].actor, &assets.resources)?;
            if self.script == 0
                && self.mission.input_allowed()
                && self.actors[self.player].behavior.programs.idle
                    != crate::tenchu2::ai::Program::Disabled
            {
                self.actors[self.player]
                    .actor
                    .sheathed_velocity(&assets.resources)?;
            }
        }
        self.controls.walking = clip.map(|clip| Walking { clip, held: false });
        Ok(())
    }
}
