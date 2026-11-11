use super::{
    actor::{Actor, Resources, Velocity},
    control::Request,
    physics::{FALLING, HIT},
    world::{Assets, World},
};
use crate::{Result, game::sheathing::Settings};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub settings: Settings,
    held: bool,
    requested: bool,
}

impl State {
    pub fn input(&mut self, held: bool) {
        self.requested |= held && !self.held;
        self.held = held;
    }

    pub fn clear_input(&mut self) {
        self.held = false;
        self.requested = false;
    }
}

fn free_stance(action: i16) -> bool {
    matches!(action, 0..=4 | 0x100..=0x104 | 0x110 | 0x200..=0x204)
}

impl Actor {
    pub(crate) fn take_sheathing_request(&mut self) -> bool {
        self.sheathing
            .as_mut()
            .is_some_and(|state| std::mem::take(&mut state.requested))
    }

    pub(crate) fn request_sheathing(&self, request: &mut Request, resources: &Resources) {
        if self.health <= 0
            || self.flags & (FALLING | HIT) != 0
            || self.terrain.height > i32::from(self.height)
            || !free_stance(self.motion.action)
            || !free_stance(request.action)
        {
            return;
        }
        let action = if self.weapon_drawn() { 0x701 } else { 0x700 };
        if resources
            .registry
            .clip(self.definition_kind(), action)
            .is_some_and(|id| resources.clips.contains_key(&id))
        {
            *request = Request {
                action,
                velocity: Velocity::Motion,
            };
        }
    }

    pub(crate) fn sheathed_settings(&self) -> Option<Settings> {
        self.sheathing
            .filter(|state| state.settings.enabled && !self.weapon_drawn())
            .map(|state| state.settings)
    }

    pub fn sight_percent(&self, base: u16) -> u16 {
        self.sheathed_settings().map_or(base, |settings| {
            (u32::from(base) * u32::from(settings.visibility) / 100).min(400) as u16
        })
    }

    pub(crate) fn sheathed_velocity(&mut self, resources: &Resources) -> Result<()> {
        if matches!(self.motion.action, 3 | 4 | 0x103 | 0x104)
            && let Some(settings) = self.sheathed_settings().filter(|s| s.speed != 100)
        {
            let clip = resources
                .clips
                .get(&self.motion.clip.ok_or("No T2 locomotion clip")?)
                .ok_or("Missing T2 locomotion clip")?;
            self.set_horizontal_velocity(
                (i32::from(clip.forward_speed) * i32::from(settings.speed) / 100) as i16,
                (i32::from(clip.side_speed) * i32::from(settings.speed) / 100) as i16,
                &resources.math,
            );
        }
        Ok(())
    }

    pub(crate) fn sheathed_damage(&self, power: i16) -> i16 {
        if power > 0
            && let Some(settings) = self.sheathed_settings()
        {
            if settings.one_hit {
                power.max(self.health)
            } else {
                (i32::from(power) * i32::from(settings.damage) / 100).min(i32::from(i16::MAX))
                    as i16
            }
        } else {
            power
        }
    }
}

impl World {
    pub fn set_player_sheathing(&mut self, mut settings: Settings, assets: &Assets) -> Result<()> {
        settings.validate();
        let enabled = self.script == 0
            && self.mission.input_allowed()
            && self.actors[self.player].behavior.programs.idle != super::ai::Program::Disabled;
        let player = &mut self.actors[self.player].actor;
        let before = player.sheathed_settings().map_or(100, |s| s.speed);
        if settings.enabled {
            if let Some(state) = &mut player.sheathing {
                state.settings = settings;
            } else {
                player.sheathing = Some(State {
                    settings,
                    held: false,
                    requested: false,
                });
            }
        } else {
            player.sheathing = None;
        }
        if enabled
            && before != player.sheathed_settings().map_or(100, |s| s.speed)
            && matches!(player.motion.action, 3 | 4 | 0x103 | 0x104)
        {
            player.update_velocity(Velocity::Motion, &assets.resources)?;
            player.sheathed_velocity(&assets.resources)?;
        }
        Ok(())
    }
}
