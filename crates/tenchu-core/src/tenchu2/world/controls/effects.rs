use super::*;
use crate::math::Vector3;
use crate::tenchu2::effects::blood::Spurt;

impl<E: Events> Context<'_, E> {
    pub(super) fn particle(
        &mut self,
        actor: &Actor,
        event: Event,
        random: &mut Random,
    ) -> Result<bool> {
        match event {
            Event::Splash(kind) => self.splash(actor, kind, random)?,
            Event::Dust => {
                self.effects
                    .smoke(actor.position, 0, self.hidden, &self.math, random)?
            }
            Event::DustTrail => {
                self.effects
                    .trail(actor.position, actor.velocity, 0, self.hidden, random)?
            }
            Event::Blood { position, angle } => {
                self.effects.blood(
                    Spurt {
                        position,
                        yaw: angle,
                        speed: 100,
                        gravity: 20,
                    },
                    &self.math,
                    self.effect_rules,
                    random,
                )?;
            }
            Event::WaterRipple { position, size } => {
                self.effects
                    .splash(position, i32::from(size), &self.math, random)?
            }
            Event::Bubbles => {
                self.effects
                    .trail(actor.position, actor.velocity, 2, self.hidden, random)?
            }
            Event::Sparks { position, parried } => self.effects.flash(position, !parried)?,
            Event::Burn => {
                self.effects
                    .burn(&self.instances[self.index].rig, 10, 120, self.effect_rules)?
            }
            Event::Heal => self.effects.heal(
                &self.instances[self.index].rig,
                0,
                self.effect_rules,
                random,
            )?,
            Event::Medicine(kind) => self.effects.heal(
                &self.instances[self.index].rig,
                kind,
                self.effect_rules,
                random,
            )?,
            _ => return Ok(false),
        }
        Ok(true)
    }

    fn splash(&mut self, actor: &Actor, kind: u8, random: &mut Random) -> Result<()> {
        let position = Position::new(
            actor.position.x,
            if self.water.depth.is_some() {
                self.water.surface
            } else {
                actor.terrain.floor.unwrap_or(i32::MIN)
            },
            actor.position.z,
        );
        match kind {
            0 | 1 => {
                self.emit(actor, Event::Voice(28), random)?;
                self.effects.splash(
                    position,
                    if kind == 0 { 1024 } else { 2048 },
                    &self.math,
                    random,
                )?;
                if self.index == self.player && actor.motion.action != 0x513 {
                    self.alert.raise(
                        crate::tenchu2::ai::status::AlertCause::Suspicion,
                        self.difficulty,
                        self.rules,
                    )?;
                }
            }
            2 | 3 => {
                if kind == 2 {
                    self.emit(actor, Event::Voice(26), random)?;
                }
                self.effects.ripple(position, random.next15() % 20 + 40)?;
            }
            4 => {
                self.effects.splash(position, 0, &self.math, random)?;
                if random.next15() % 10 == 0 {
                    self.splash(actor, 3, random)?;
                }
                self.emit(actor, Event::Voice(25), random)?;
            }
            5 => {
                let bone = *self.instances[self.index]
                    .rig
                    .joints
                    .get(2)
                    .ok_or("Missing Tenchu 2 breathing joint")?;
                let position = self.scene.world(bone)?.translation;
                self.effects
                    .trail(position, Vector3::default(), 2, self.hidden, random)?;
                self.emit(actor, Event::Voice(27), random)?;
            }
            _ => return Err("Unknown Tenchu 2 splash".into()),
        }
        Ok(())
    }
}
