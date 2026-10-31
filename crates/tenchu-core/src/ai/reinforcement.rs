use super::*;

impl Controller<'_, '_> {
    fn reinforcement_has_cover(&self) -> Result<bool> {
        if !self.roster.keep_all_active {
            return Ok(true);
        }
        // Retail relies on distance; whole-level AI also requires terrain cover.
        let from =
            self.player
                .position
                .wrapping_sub(Position::new(0, i32::from(self.player.height), 0));
        let height = i32::from(self.ctx.mission.characters[self.actor.definition].height);
        let mut cursor = terrain::Cursor::new(&self.ctx.mission.terrain);
        for y in [0, height / 2, height] {
            let to = self.actor.position.wrapping_sub(Position::new(0, y, 0));
            let mut step = to.wrapping_sub(from);
            let mut count = 1i16;
            while step.iter().any(|v| v.abs() > 500) {
                step = step.map(|v| v >> 1);
                count *= 2;
            }
            if terrain::passage(
                &self.ctx.mission.terrain,
                &mut cursor,
                from,
                step.map(|v| v as i16).to_array(),
                count,
            )?
            .is_none()
            {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn breed(&mut self, yaw: i16) -> Result<usize> {
        let choice = self.random() as usize % 2;
        self.roster
            .spawn_reinforcement(choice, self.actor.position, yaw, self.ctx)
    }

    fn equip_guard(&mut self, id: usize) -> Result<()> {
        let actor = self.roster.get_mut(id);
        actor.target = self.actor.target;
        actor.setup_think(0x4444)?;
        actor.equip(self.ctx, true);
        actor.enter(self.ctx, MOT_ENGAGE_STANCE, true);
        Ok(())
    }

    pub(super) fn call_aid(&mut self) -> Result<u16> {
        if self.distance() < 16500 || !self.reinforcement_has_cover()? {
            if self.state.detection != Detection::Gone {
                self.state.detection = Detection::None;
            }
            return self.escape();
        }
        self.state.detection = Detection::Unseen;
        let yaw = self.actor.rotation[1].wrapping_add(self.state.sight.direction);
        let id = self.breed(yaw)?;
        self.roster.get_mut(id).target = self.actor.target;
        self.roster.remove_detached(self.actor, self.ctx);
        self.equip_guard(id)?;
        let mut replacement = self.roster.actors[id].take().unwrap();
        std::mem::swap(self.actor, &mut replacement);
        self.replaced = Some(replacement);
        self.attribute = self.actor.attribute | 2;
        if self.actor.kind & 0xf0 == 0x90 {
            self.state.population_delta[0] = self.state.population_delta[0].wrapping_add(1);
            self.state.population_delta[1] = self.state.population_delta[1].wrapping_sub(1);
        }
        Ok(0)
    }

    pub(super) fn alarm(&mut self) -> Result<u16> {
        let x = self.actor.brain.chase[0].wrapping_sub(self.actor.position.x);
        let z = self.actor.brain.chase[1].wrapping_sub(self.actor.position.z);
        let mut pad = 0;
        match self.actor.brain.timer {
            0 => {
                pad = self.goto(x, z)?;
                if self.length(&[x, z]) < 2000 || self.attribute & ATTR_WALL != 0 {
                    self.alert();
                    self.voice(14);
                    self.actor.brain.count = match self.state.difficulty {
                        Difficulty::Easy => 1,
                        Difficulty::Normal => (self.random() & 1) as u8,
                        Difficulty::Hard => 0,
                    };
                    self.actor.brain.timer = if self.actor.brain.count != 0
                        || !self.roster.can_call_reinforcements()
                        || self.ctx.mission.stage.id == 8
                    {
                        1
                    } else {
                        2
                    };
                }
            }
            1 => {
                self.actor.brain.count = self.actor.brain.count.wrapping_add(1) & 31;
                let count = self.actor.brain.count;
                if count & 8 != 0 {
                    if count != 8 {
                        pad = self.actor.pad.held;
                    } else if self.degree().abs() > 700 {
                        pad = if self.degree() > 0 { RIGHT } else { LEFT };
                    } else if self.random() % 5 != 0 {
                        pad = if self.random() & 1 != 0 { LEFT } else { RIGHT };
                    } else {
                        pad = UP;
                    }
                }
            }
            _ => {
                if self.roster.keep_all_active && !self.roster.can_call_reinforcements() {
                    self.actor.brain.timer = 1;
                    self.actor.brain.count = 0;
                    return Ok(0);
                }
                let degree = self.ctx.math.direction(x, z, self.actor.rotation[1])?;
                let turn = if degree > 0 { LEFT } else { RIGHT };
                pad = turn
                    | if i32::from(degree).abs() >= 1000 {
                        UP
                    } else {
                        DOWN
                    };
                if self.attribute & ATTR_WALL != 0
                    && self.degree().abs() > 1000
                    && self.actor.pad.hold_counter == 0
                {
                    let ticks = 1000i32
                        .checked_div(self.turn())
                        .ok_or("Zero AI turn speed")?;
                    self.hold(if self.degree() > 0 { LEFT } else { RIGHT }, ticks);
                }
                if self.distance() > 16500 && self.reinforcement_has_cover()? {
                    self.alert();
                    self.actor.brain.timer = 0;
                    self.actor.brain.count = 1;
                    let voice = if self.random() & 1 != 0 { 9 } else { 10 };
                    self.voice(voice);
                    // The type RNG precedes the caller's raw rotation write.
                    let choice = self.random() as usize % 2;
                    self.actor.rotation[1] = self.actor.rotation[1].wrapping_add(degree);
                    let id = self.roster.spawn_reinforcement(
                        choice,
                        self.actor.position,
                        self.actor.rotation[1],
                        self.ctx,
                    )?;
                    self.equip_guard(id)?;
                    let actor = self.roster.get_mut(id);
                    actor.brain.timer = 0;
                    actor.brain.count = 1;
                    actor.attribute |= ATTR_SEARCH | 1;
                    let x = self.actor.brain.chase[0].wrapping_add((self.random() % 5 - 2) * 500);
                    let z = self.actor.brain.chase[1].wrapping_add((self.random() % 5 - 2) * 500);
                    self.roster.get_mut(id).brain.chase = [x, z];
                    let voice = if self.random() & 1 != 0 { 9 } else { 10 };
                    self.state.voices.push((id, voice));
                    self.state.population_delta[0] = self.state.population_delta[0].wrapping_add(1);
                }
            }
        }
        Ok(pad)
    }
}
