use super::{Assets, Placement, World};
use crate::{
    Result,
    ai::Recruitment,
    math::Position,
    roster::ReinforcementCall,
    tenchu2::{
        actor::{State, Velocity},
        ai::{direction, sight, status::AlertCause},
        rig::WeaponMode,
        terrain::{self, Cursor, Probe, Query},
    },
};

impl World {
    pub fn configure_reinforcements(&mut self, mut settings: ReinforcementCall, limit: u16) {
        settings.count = settings.count.min(ReinforcementCall::MAX_COUNT);
        let calls = &mut self.ai.reinforcements;
        if settings.witnessed_kills && calls.alive.is_empty() {
            calls.alive = self.actors.iter().map(|a| a.actor.health > 0).collect();
        } else if !settings.witnessed_kills {
            calls.alive.clear();
        }
        calls.settings = settings;
        calls.limit = (limit != 0).then_some(usize::from(limit));
    }

    fn call_has_room(&self) -> bool {
        let limit = if self.keep_all_active {
            self.ai.reinforcements.limit
        } else {
            Some(30)
        };
        self.ai.reinforcements.settings.count != 0
            && self.actors.len() <= i16::MAX as usize
            && limit.is_none_or(|limit| self.actors.len() < limit)
            && (!matches!(
                self.mission.objective,
                tenchu_assets::tenchu2::campaign::Objective::EliminateAll
            ) || u16::from(self.mission.population.guards)
                + u16::from(self.mission.population.bosses)
                < 255)
    }

    pub(super) fn update_reinforcements(&mut self, assets: &Assets) -> Result<()> {
        let tracking = self.ai.reinforcements.settings.witnessed_kills;
        let witnessed = self.keep_all_active && tracking;
        let deaths: Vec<_> = if tracking {
            self.actors
                .iter()
                .enumerate()
                .filter_map(|(index, agent)| {
                    let alive = agent.actor.health > 0;
                    let previous = self.ai.reinforcements.alive.get_mut(index);
                    let died = previous.as_ref().is_some_and(|before| **before && !alive);
                    if let Some(previous) = previous {
                        *previous = alive;
                    }
                    died.then_some(index)
                })
                .collect()
        } else {
            Vec::new()
        };
        if tracking {
            self.ai.reinforcements.alive.truncate(self.actors.len());
            self.ai.reinforcements.alive.extend(
                self.actors[self.ai.reinforcements.alive.len()..]
                    .iter()
                    .map(|a| a.actor.health > 0),
            );
        }
        if self.script != 0
            || !self.mission.input_allowed()
            || self.ai.ignore_player
            || self.actors[self.player].actor.health <= 0
            || (0x920..=0x92f).contains(&self.actors[self.player].actor.motion.action)
        {
            return Ok(());
        }
        if witnessed && self.call_has_room() {
            for victim in deaths {
                self.request_witness(victim, assets)?;
            }
        }
        for index in 0..self.actors.len() {
            if !self.profile_eligible(index) {
                continue;
            }
            let actor = &self.actors[index].actor;
            let calm = actor.flags & 3 == 0 && self.ai.global.perception.alert.remaining == 0;
            let caller = &mut self.actors[index].behavior.profile.recruitment;
            if calm && matches!(*caller, Recruitment::Called | Recruitment::Requested) {
                *caller = Recruitment::Available;
            }
            if *caller != Recruitment::Requested {
                continue;
            }
            if !self.call_has_room() || (!witnessed && !self.coward(index)) {
                self.actors[index].behavior.profile.recruitment = Recruitment::Available;
                continue;
            }
            if self.actors[index].actor.state != State::Ready {
                continue;
            }
            let player = &self.actors[self.player].actor;
            let actor = &self.actors[index].actor;
            if assets
                .resources
                .math
                .length_wide(actor.position.wrapping_sub(player.position))
                <= 16500
            {
                continue;
            }
            let Some(position) = self.hidden_call_position(index, assets)? else {
                continue;
            };
            let settings = self.ai.reinforcements.settings;
            let count = if settings.random && settings.count > 1 {
                1 + self.random.next15() as u16 % settings.count
            } else {
                settings.count
            };
            let kind = self.actors[index].actor.kind;
            let definition = self.actors[index].actor.definition_kind();
            let target = self.actors[self.player].actor.position;
            let delta = target.wrapping_sub(position);
            let yaw = direction(&assets.resources.math, delta.x, delta.z, 0)?;
            for _ in 0..count {
                if !self.call_has_room() {
                    break;
                }
                let id = self.place_actor(
                    Placement {
                        kind,
                        definition,
                        behavior: 0x2123,
                        position,
                        yaw,
                        snap: true,
                    },
                    assets,
                )?;
                self.actors[id].behavior.profile.recruitment = Recruitment::Reinforcement;
                self.actors[id].behavior.profile.recruited_by = Some(index);
                self.actors[index].behavior.profile.recruitment = Recruitment::Called;
                let actor = &mut self.actors[id].actor;
                actor.target = Some(self.player);
                actor.flags |= 3;
                actor.change_motion(0x100, Velocity::Stop, &assets.resources)?;
                actor.set_weapon(
                    &mut self.instances[id].rig,
                    id,
                    &mut self.collision,
                    &assets.resources,
                    WeaponMode::Draw,
                )?;
                self.actors[id].sight.destination = target;
                self.actors[id].behavior.remember(target);
                self.publish(id, assets);
            }
            self.ai.global.perception.alert.raise(
                AlertCause::Combat,
                self.difficulty,
                &assets.combat_rules,
            )?;
            self.ai.global.perception.alert.position = Some(position);
        }
        Ok(())
    }

    fn hidden_call_position(&self, index: usize, assets: &Assets) -> Result<Option<Position>> {
        let actor = &self.actors[index].actor;
        let mut cursor = Cursor::default();
        let mut position = actor.position;
        position.y = position.y.wrapping_sub(1000);
        let Some(floor) = terrain::level(&self.terrain, &mut cursor, position, Query::Surface)?
        else {
            return Ok(None);
        };
        if cursor.material & 0xf00 != 0 || floor.abs_diff(actor.position.y) > 500 {
            return Ok(None);
        }
        position.y = floor;
        let mut probe = Probe::default();
        probe.sample(
            &self.terrain,
            &mut Cursor::default(),
            &assets.terrain_rules,
            position.wrapping_sub(Position::new(0, i32::from(actor.height), 0)),
            actor.width,
            Default::default(),
        )?;
        if probe.floor.is_none() || probe.blocked != 0 {
            return Ok(None);
        }
        let player = &self.actors[self.player].actor;
        let mut eye = player.position;
        eye.y = eye.y.wrapping_sub(i32::from(player.height));
        for height in [0, i32::from(actor.height) / 2, i32::from(actor.height)] {
            let mut target = position;
            target.y = target.y.wrapping_sub(height);
            if !self.reinforcement_cover(eye, target, assets)? {
                return Ok(None);
            }
        }
        Ok(Some(position))
    }

    fn request_witness(&mut self, victim: usize, assets: &Assets) -> Result<()> {
        let dead = &self.actors[victim].actor;
        if victim == self.player
            || dead.health != 0
            || !(0x20..0x60).contains(&dead.kind)
            || !dead.weapon_drawn()
            || self.actors[victim].behavior.profile.protected
            || self.mission.targets.contains(&Some(victim))
        {
            return Ok(());
        }
        let mut nearest = None;
        for index in 0..self.actors.len() {
            let agent = &self.actors[index];
            if index == victim
                || !self.profile_eligible(index)
                || agent.actor.flags & 8 != 0
                || !matches!(agent.actor.state, State::Normal | State::Ready)
                || agent.behavior.profile.recruitment != Recruitment::Available
            {
                continue;
            }
            let delta = dead.position.wrapping_sub(agent.actor.position);
            let distance = assets.resources.math.length_wide(delta);
            let range = sight::scale_range(self.ai.ranges.standing.notice, self.ai.sight_percent);
            if distance >= range
                || nearest.is_some_and(|(best, _)| distance >= best)
                || i32::from(direction(
                    &assets.resources.math,
                    delta.x,
                    delta.z,
                    agent.actor.rotation.y,
                )?)
                .abs()
                    >= 700
            {
                continue;
            }
            let mut eye = agent.actor.position;
            eye.y = eye.y.wrapping_add(300 - i32::from(agent.actor.height));
            let mut target = dead.position;
            target.y = target.y.wrapping_sub(i32::from(dead.height) / 2);
            if !self.reinforcement_cover(eye, target, assets)? {
                nearest = Some((distance, index));
            }
        }
        if let Some((_, index)) = nearest {
            let position = dead.position;
            let agent = &mut self.actors[index];
            agent.actor.target = Some(self.player);
            agent.actor.flags |= 3;
            if !agent.actor.weapon_drawn() {
                agent
                    .actor
                    .change_motion(0x700, Velocity::Stop, &assets.resources)?;
            }
            agent.behavior.profile.recruitment = Recruitment::Requested;
            agent.behavior.remember(position);
            agent.behavior.reset(false);
            self.ai.global.perception.alert.raise(
                AlertCause::Combat,
                self.difficulty,
                &assets.combat_rules,
            )?;
            self.ai.global.perception.alert.position = Some(position);
        }
        Ok(())
    }

    fn reinforcement_cover(&self, from: Position, to: Position, assets: &Assets) -> Result<bool> {
        let start = from.to_array().map(i64::from);
        let end = to.to_array().map(i64::from);
        let delta = std::array::from_fn::<_, 3, _>(|i| end[i] - start[i]);
        // The native segment takes a signed 16-bit displacement. Calls can be
        // farther away than source sight, so keep every segment within range.
        let chunks = (delta.into_iter().map(i64::abs).max().unwrap() + 29999) / 30000;
        let mut previous = from;
        for chunk in 1..=chunks {
            let next = Position::from(std::array::from_fn(|i| {
                (start[i] + delta[i] * chunk / chunks) as i32
            }));
            if terrain::segment(
                &self.terrain,
                &mut Cursor::default(),
                previous,
                next.wrapping_sub(previous),
                &assets.resources.math,
            )?
            .is_some()
            {
                return Ok(true);
            }
            previous = next;
        }
        Ok(false)
    }
}
