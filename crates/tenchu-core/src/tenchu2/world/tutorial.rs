use super::{Assets, World};
use crate::tenchu2::{
    actor::Velocity,
    items::{self, pickup::Pickup},
    rig::WeaponMode,
};
use crate::{Result, math::Position};

impl World {
    pub fn update_training(&mut self, assets: &Assets) -> Result<crate::tenchu2::tutorial::Update> {
        let Some(training) = &mut self.tutorial else {
            return Ok(Default::default());
        };
        let player = &self.actors[self.player].actor;
        let duration = player
            .motion
            .clip
            .and_then(|clip| assets.resources.clips.get(&clip))
            .map_or(0, |c| c.duration);
        let audio_finished = training.playing.is_none_or(|(cue, start)| {
            let track = &training.data.tracks[&cue];
            self.tick.wrapping_sub(start) >= (track.end - track.start) / 5
        });
        let update = training.state.step(
            &training.data,
            crate::tenchu2::tutorial::Input {
                player,
                duration,
                camera: self.camera.mode,
                defeated_practice_guards: self
                    .actors
                    .iter()
                    .filter(|a| {
                        a.actor.kind == 0x5f && a.actor.state == crate::tenchu2::actor::State::Dead
                    })
                    .count(),
                audio_finished,
            },
        );
        if let Some(cue) = update.cue {
            training.playing = Some((cue, self.tick));
        }
        if let Some(course) = update.entered {
            self.enter_training_course(course, assets)?;
        }
        if update.clear_alert {
            self.ai.global.perception.alert.remaining = 0;
        }
        Ok(update)
    }

    // JP DATAMNG.BIN 800d2354..800d24d0: supplies and practice-guard reset.
    pub fn enter_training_course(&mut self, course: usize, assets: &Assets) -> Result<()> {
        match course {
            2 => {
                let held = self.actors[self.player].actor.inventory.counts[4];
                for _ in held..3 {
                    let position = Position::new(
                        self.random.next15() % 2000 - 41000,
                        -4000,
                        self.random.next15() % 2000 - 9000,
                    );
                    self.items.insert(
                        items::State::Pickup(Pickup::new(4, position)),
                        &mut self.scene,
                        &mut self.collision,
                    )?;
                }
            }
            3 => {
                for index in 0..self.actors.len() {
                    let agent = &mut self.actors[index];
                    let actor = &mut agent.actor;
                    if actor.kind != 0x5f {
                        continue;
                    }
                    actor.health = actor.max_health;
                    actor.position = agent.behavior.home.wrapping_sub(Position::new(0, 1000, 0));
                    actor.rotation.x = 0;
                    actor.motion.mask = u32::MAX;
                    actor.target = Some(self.player);
                    actor.change_motion(0, Velocity::Motion, &assets.resources)?;
                    let rig = &mut self.instances[index].rig;
                    actor.set_weapon(
                        rig,
                        index,
                        &mut self.collision,
                        &assets.resources,
                        WeaponMode::Draw,
                    )?;
                    actor.width = 400;
                    actor.height = 1700;
                    actor.set_body_collision(rig.joints[0], index, &mut self.collision, true)?;
                    actor.inventory.resurrections = 0;
                    self.publish(index, assets);
                }
            }
            _ => {}
        }
        Ok(())
    }
}
