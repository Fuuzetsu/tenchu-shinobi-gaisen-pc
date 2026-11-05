use super::*;

impl Actor {
    pub(crate) fn guard_input(&self, world: &ActionContext) -> bool {
        self.id == world.player
            && world.intelligence.guard_controls
            && world.intelligence.action_halt == crate::ai::Halt::Running
            && !world.intelligence.cinematic
            && !self.scripted
            && self.health > 0
            && self.map.height == 0
            && self.pad.held & (R1 | L1) == R1
            && self.free_stance()
    }

    pub(super) fn guard_motion(&mut self, ctx: &Context, held: u16) -> Option<(i16, bool)> {
        self.velocity[0] = 0;
        self.velocity[2] = 0;
        let crouching = held & CIRCLE != 0;
        let armed = self.attribute & ATTR_WEAPON_DRAWN != 0;
        if self.pad.trigger & (R1 | UP | DOWN | LEFT | RIGHT) != 0 {
            let actions = if crouching {
                [
                    MOT_SQUAT_ROLL_F,
                    MOT_SQUAT_ROLL_B,
                    MOT_SQUAT_ROLL_L,
                    MOT_SQUAT_ROLL_R,
                ]
            } else if armed {
                [
                    MOT_CHASE_DASH_FWD,
                    MOT_CHASE_DASH_BACK,
                    MOT_CHASE_DASH_LEFT,
                    MOT_CHASE_DASH_RIGHT,
                ]
            } else {
                [
                    MOT_MOVE_DASH_FWD,
                    MOT_MOVE_DASH_BACK,
                    MOT_MOVE_DASH_LEFT,
                    MOT_MOVE_DASH_RIGHT,
                ]
            };
            if let Some((_, action)) = [UP, DOWN, LEFT, RIGHT]
                .into_iter()
                .zip(actions)
                .find(|(button, _)| held & button != 0)
                && self.registered(ctx, action)
            {
                return Some((action, true));
            }
        }
        let stance = if crouching {
            MOT_SQUAT
        } else if armed {
            MOT_ENGAGE_STANCE
        } else if self.registered(ctx, MOT_STATE_DRAW) {
            MOT_STATE_DRAW
        } else {
            MOT_NORMAL
        };
        (self.motion.action != stance && self.registered(ctx, stance)).then_some((stance, true))
    }
}
