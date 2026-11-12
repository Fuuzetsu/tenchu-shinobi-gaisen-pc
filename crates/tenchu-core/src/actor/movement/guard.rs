use super::*;

impl Actor {
    pub fn circle_guards(&self, intelligence: &crate::ai::Intelligence) -> bool {
        intelligence
            .circle_action
            .guards(self.imported_circle_guard)
    }

    pub(crate) fn guard_input(&self, world: &ActionContext) -> bool {
        self.id == world.player
            && (world.intelligence.guard_controls || self.circle_guards(world.intelligence))
            && world.intelligence.action_halt == crate::ai::Halt::Running
            && !world.intelligence.cinematic
            && !self.scripted
            && self.health > 0
            && self.map.height == 0
            && self.pad.held & (R1 | L1) == R1
            && self.free_stance()
            && (!self.circle_guards(world.intelligence)
                || self.motion.action == MOT_ENGAGE_GUARD
                || self.pad.trigger & SQUARE == 0)
    }

    pub(super) fn circle_guard_motion(&mut self, ctx: &Context, held: u16) -> Option<(i16, bool)> {
        self.velocity[0] = 0;
        self.velocity[2] = 0;
        let action = if self.status == State::Crouch && held & CIRCLE != 0 {
            [
                (UP, MOT_SQUAT_ROLL_F),
                (DOWN, MOT_SQUAT_ROLL_B),
                (RIGHT, MOT_SQUAT_ROLL_R),
                (LEFT, MOT_SQUAT_ROLL_L),
            ]
            .into_iter()
            .find_map(|(button, action)| (held & button != 0).then_some(action))
            .unwrap_or(MOT_SQUAT)
        } else if self.motion.action == MOT_ENGAGE_GUARD {
            [
                (CIRCLE, MOT_SQUAT),
                (UP, MOT_CHASE_DASH_FWD),
                (DOWN, MOT_CHASE_DASH_BACK),
                (RIGHT, MOT_CHASE_DASH_RIGHT),
                (LEFT, MOT_CHASE_DASH_LEFT),
            ]
            .into_iter()
            .find_map(|(button, action)| (held & button != 0).then_some(action))
            .unwrap_or(MOT_ENGAGE_GUARD)
        } else {
            MOT_ENGAGE_GUARD
        };
        (self.motion.action != action && self.registered(ctx, action)).then_some((action, true))
    }

    pub(super) fn circle_guard_step(
        &mut self,
        ctx: &mut Context,
        world: &mut ActionContext,
        held: u16,
        center: Position,
    ) -> Result<Option<(i16, bool)>> {
        let mut pending = None;
        if matches!(self.motion.action, MOT_CHASE_DASH_FWD | MOT_CHASE_DASH_BACK)
            && self.pad.trigger & CROSS != 0
        {
            pending = self.start_jump(ctx, world, held, center);
        }
        if self.motion.loops > 0 && pending.is_none() {
            pending = Some((
                if held & R1 != 0 {
                    MOT_ENGAGE_GUARD
                } else {
                    self.return_normal(ctx, world.player)?
                },
                true,
            ));
        }
        if self.motion.count == 1 {
            world.voice(self, ctx, self.id, 0x13);
            ctx.effects
                .smoke_burst(self.position, 150, 12, 1, ctx.random);
        }
        if self.pad.trigger & SQUARE != 0 && self.attribute & ATTR_WEAPON_DRAWN != 0 {
            pending = self.attack_input(ctx, world, held, pending)?;
        }
        Ok(pending)
    }

    pub(super) fn guard_motion(
        &mut self,
        ctx: &Context,
        held: u16,
        draw: bool,
    ) -> Option<(i16, bool)> {
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
        } else if draw && self.registered(ctx, MOT_STATE_DRAW) {
            MOT_STATE_DRAW
        } else {
            MOT_NORMAL
        };
        (self.motion.action != stance && self.registered(ctx, stance)).then_some((stance, true))
    }
}
