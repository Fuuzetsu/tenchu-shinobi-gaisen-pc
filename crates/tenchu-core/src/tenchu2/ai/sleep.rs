use super::{Sentry, SentryInput};
use crate::{
    Result,
    math::Position,
    random::Random,
    tenchu2::{
        actor::{Actor, Resources},
        input,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Sleep {
    pub wait: i16,
    pub sentry: Sentry,
}

pub struct SleepInput {
    pub home: Position,
    pub alert: i16,
}

pub struct SleepOutput {
    pub held: u16,
    pub unnoticed: bool,
}

impl Sleep {
    pub fn update(
        &mut self,
        actor: &mut Actor,
        input: SleepInput,
        destination: &mut Position,
        resources: &Resources,
        random: &mut Random,
        mut apply: impl FnMut(&mut Actor, i16) -> Result<()>,
    ) -> Result<SleepOutput> {
        let poisoned = actor.flags & 0x100 != 0;
        let resting = matches!(actor.motion.action, 0 | 0x100);
        let near_home = actor.position.x.wrapping_sub(input.home.x).wrapping_abs() < 5000
            && actor.position.z.wrapping_sub(input.home.z).wrapping_abs() < 5000;
        if near_home && !poisoned {
            if resting && actor.motion.loops > 0 && input.alert == 0 {
                apply(actor, 0x70c)?;
            }
        } else if resting {
            if !poisoned && (self.wait < 451 || input.alert != 0) {
                self.wait = self.wait.wrapping_add(1);
            } else {
                if !poisoned {
                    apply(actor, 0x70c)?;
                }
                self.wait = 0;
            }
        }
        if actor.motion.action == 0x70c {
            let unnoticed = input.alert == 0 && actor.flags & 0x40000 == 0;
            if !unnoticed {
                *destination = actor.position;
            }
            return Ok(SleepOutput {
                held: if unnoticed { 0 } else { input::GUARD },
                unnoticed,
            });
        }
        let (held, action) = self.sentry.update(
            resources.registry.edition,
            SentryInput {
                action: actor.motion.action,
                health: actor.health,
                turn: actor.turn,
                held: actor.pad.held,
            },
            random,
        )?;
        if let Some(action) = action {
            apply(actor, action)?;
        }
        Ok(SleepOutput {
            held,
            unnoticed: false,
        })
    }
}
