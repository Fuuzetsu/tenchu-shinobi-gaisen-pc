use super::{Assets, World};
use crate::{
    Result,
    ai::profiles::{Combat, Overrides, Settings},
    tenchu2::ai::{Program, profiles::Selection},
};

impl World {
    pub fn profile_eligible(&self, index: usize) -> bool {
        self.actors.get(index).is_some_and(|agent| {
            index != self.player
                && (0x20..0x60).contains(&agent.actor.kind)
                && agent.actor.health > 0
                && agent.actor.flags & 0x10 != 0
                && !agent.behavior.profile.protected
                && !matches!(
                    agent.behavior.programs.idle,
                    Program::PlayerOne | Program::PlayerTwo
                )
                && !matches!(agent.behavior.programs.combat, Program::Disabled)
                && !self.mission.targets.contains(&Some(index))
                && !self.script_hidden.contains(&index)
        })
    }

    pub fn profile_selection(&self, index: usize) -> Option<Selection> {
        if !self.ai.profiles_allowed
            || self.script != 0
            || !self.mission.input_allowed()
            || !self.profile_eligible(index)
        {
            return None;
        }
        let agent = &self.actors[index];
        let settings = self.ai.profiles;
        let overrides = agent.behavior.profile.overrides;
        Some(Selection {
            actor: index,
            combat: settings.assignment(index, agent.actor.kind, overrides.combat),
            idle: overrides.idle.unwrap_or(settings.idle),
            seed: settings.seed,
        })
    }

    pub fn configure_profiles(&mut self, settings: Settings, assets: &Assets) {
        self.ai.profiles = settings;
        let order = || {
            assets.level.gaisen.as_ref().map_or_else(
                || {
                    assets
                        .campaign
                        .mission(assets.hero, assets.sequence)
                        .unwrap_or(0)
                },
                |stage| stage.order.max(0) as usize,
            )
        };
        self.ai.profiles_allowed = settings.enabled
            && (!settings.progression
                || order()
                    >= match self.difficulty {
                        3 => 6,
                        2 => 4,
                        _ => 2,
                    });
        for index in 0..self.actors.len() {
            self.refresh_profile(index);
        }
    }

    pub(super) fn refresh_profile(&mut self, index: usize) {
        let selection = self.profile_selection(index);
        let caller = selection.is_some_and(|s| s.combat == Combat::Coward)
            || self.keep_all_active && self.ai.reinforcements.settings.witnessed_kills;
        if self.actors[index].behavior.profile.recruitment == crate::ai::Recruitment::Requested
            && (!caller
                || self.ai.reinforcements.settings.count == 0
                || self.script != 0
                || !self.mission.input_allowed()
                || !self.profile_eligible(index))
        {
            self.actors[index].behavior.profile.recruitment = crate::ai::Recruitment::Available;
        }
        self.actors[index].behavior.select_profile(selection);
    }

    pub fn set_actor_behavior(&mut self, index: usize, overrides: Overrides) -> Result<()> {
        if self.script != 0
            || !self.mission.input_allowed()
            || self.actors[self.player].actor.health <= 0
            || !self.profile_eligible(index)
        {
            return Err(
                "Behavior overrides require an ordinary living guard outside a scripted scene"
                    .into(),
            );
        }
        self.actors[index].behavior.profile.overrides = overrides;
        self.refresh_profile(index);
        Ok(())
    }

    pub(super) fn coward(&self, index: usize) -> bool {
        self.profile_selection(index)
            .is_some_and(|s| s.combat == Combat::Coward)
    }
}
