use super::*;

impl Actor {
    pub(super) fn walking_motion(&mut self, ctx: &Context, world: &ActionContext) {
        let Some(walk) = world.walking else {
            return;
        };
        if self.id != world.player || self.motion.action != MOT_CHASE {
            return;
        }
        let walking = walk.held
            && !self.scripted
            && !world.intelligence.cinematic
            && self.health > 0
            && self.map.height == 0
            && self.pad.held & (UP | L1 | R1 | CIRCLE) == UP
            && world.intelligence.action_halt == crate::ai::Halt::Running;
        if !walking && self.motion.clip != Some(walk.clip) {
            return;
        }
        let index = if walking {
            Some(walk.clip)
        } else {
            crate::motion::Motion::clip_index(
                MOT_CHASE,
                &ctx.mission.characters[self.definition].motions,
                &ctx.mission.common_motions,
                &ctx.mission.clips,
            )
        };
        if let Some(index) = index
            && self.motion.clip != Some(index)
        {
            self.motion.select_clip(index, &ctx.mission.clips);
            self.motion_velocity(ctx);
        }
    }
}
