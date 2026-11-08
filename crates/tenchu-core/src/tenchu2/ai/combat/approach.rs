use super::{Context, ItemChoices, Melee};
use crate::{
    Result,
    tenchu2::{
        actor::{Actor, State},
        ai::Decision,
        input,
    },
};

impl Melee {
    pub fn approach_combat(
        &mut self,
        c: &mut Context<'_>,
        mut apply: impl FnMut(&mut Actor, i16) -> Result<()>,
    ) -> Result<Decision> {
        let category = c.category()?;
        if c.actor.state == State::Item {
            return Ok(Decision::default());
        }
        if c.actor.state == State::Attack {
            let (reach, cone) = if category == 4 {
                (20000, 500)
            } else {
                (3000, 1200)
            };
            return Ok(Decision {
                held: c.combo(reach, cone)?,
                ..Default::default()
            });
        }
        c.keep_visible(if category == 4 { 14000 } else { 10000 });
        let reach = i32::from(c.rules.reach[category]);
        let close = if category == 4 {
            4000
        } else {
            i32::from(c.rules.close[category])
        };
        let mut decision = Decision::default();
        if c.target.flags & 0x200 == 0 {
            self.engaged = false;
        } else if !self.engaged {
            decision = c.approach(&mut self.offset, reach - 200)?;
            if decision.held != 0 && c.observed_flags & 0x80000 == 0 {
                return Ok(decision);
            }
            self.engaged = true;
        }
        if c.turn() != 0 {
            decision.held = c.turn();
        }
        if c.sight.distance >= close && c.sight.distance >= reach && c.sight.distance >= 2000 {
            if c.random.next15() & 1 != 0 {
                if c.facing < 400 && c.actor.motion.frame & 7 == 0 {
                    let choices = if c.sight.distance < reach + 1000 && c.actor.motion.loops > 2 {
                        Some(ItemChoices::DEFENSIVE)
                    } else if (c.sight.distance > reach + 800
                        || i32::from(c.sight.player_angle).abs() >= 1401)
                        && c.player_held & input::ATTACK == 0
                        && c.actor.motion.loops >= 3
                    {
                        Some(ItemChoices::OFFENSIVE)
                    } else {
                        None
                    };
                    if let Some(choices) = choices {
                        decision.held |= c.choose_item(choices)?;
                    }
                } else if c.facing > 1200
                    && c.player_held & 0xf000 == 0
                    && c.actor.motion.frame & 7 == 0
                {
                    decision.held = c.select_item(5)?;
                }
            }
        } else if c.facing < 200 {
            decision.held = if c.likely()? && c.random.next15() & 3 == 0 {
                c.attack()
            } else {
                c.defend(decision.held)?
            };
        } else if c.facing < 512 {
            decision.held = c.defend(decision.held)?;
        } else {
            let held = decision.held;
            decision.held = self.close(c, &mut decision, held)?;
            c.apply(&mut decision, &mut apply)?;
        }
        if category == 4 && decision.held & input::ATTACK != 0 {
            c.actor.rotation.y = c.actor.rotation.y.wrapping_add(c.sight.angle);
            c.sight.angle = 0;
        }
        if (c.actor.motion.frame == 0 && c.random.next15() & 31 == 0 && decision.held == 0)
            || c.actor.motion.loops > 5
        {
            apply(c.actor, 0x70d)?;
        }
        decision.held &= !(c.actor.pad.held & input::ATTACK);
        Ok(decision)
    }
}
