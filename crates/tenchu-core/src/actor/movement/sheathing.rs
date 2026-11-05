use super::*;

impl Actor {
    pub(crate) fn free_stance(&self) -> bool {
        matches!(
            self.motion.action,
            MOT_NORMAL
                | MOT_NORMAL_TURN_L
                | MOT_NORMAL_TURN_R
                | MOT_MOVE
                | MOT_MOVE_BACK
                | MOT_ENGAGE_STANCE
                | MOT_ENGAGE_TURN_L
                | MOT_ENGAGE_TURN_R
                | MOT_CHASE
                | MOT_CHASE_BACK
                | MOT_SQUAT
                | MOT_SQUAT_WALK_F
                | MOT_SQUAT_WALK_B
                | MOT_SQUAT_WALK_L
                | MOT_SQUAT_WALK_R
        )
    }

    pub(super) fn sheath_input(&mut self, world: &ActionContext) -> Option<(i16, bool)> {
        let requested = self
            .sheathing
            .as_mut()
            .is_some_and(|s| std::mem::take(&mut s.requested));
        (requested
            && self.id == world.player
            && self.free_stance()
            && self.health > 0
            && self.map.height == 0
            && !self.scripted
            && !world.intelligence.cinematic
            && world.intelligence.action_halt == crate::ai::Halt::Running)
            .then_some((
                if self.attribute & ATTR_WEAPON_DRAWN != 0 {
                    MOT_STATE_SHEATHE
                } else {
                    MOT_STATE_DRAW
                },
                true,
            ))
    }
}
