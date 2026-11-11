use super::{Assets, World};
use crate::{
    Result,
    math::Position,
    tenchu2::{
        actor::{State, Velocity},
        terrain::{self, Probe, Query},
    },
};

pub struct Placement {
    pub kind: i16,
    pub definition: i16,
    pub behavior: u16,
    pub position: Position,
    pub yaw: i16,
    pub snap: bool,
}

impl World {
    pub fn place_actor(&mut self, placement: Placement, assets: &Assets) -> Result<usize> {
        let Placement {
            kind,
            definition,
            behavior,
            position,
            yaw,
            snap,
        } = placement;
        let player = &self.actors[self.player].actor;
        if !self.mission.input_allowed()
            || player.health <= 0
            || (0x920..=0x92f).contains(&player.motion.action)
        {
            return Err(
                "Placing actors requires a living player outside a cutscene or stealth kill".into(),
            );
        }
        let mut next = self.clone();
        next.collision.allow_expansion();
        let index = next.create_defined_actor(kind, definition, Position::default(), 0, assets)?;
        let actor = &next.actors[index].actor;
        let (position, probe) =
            next.edit_destination(position, actor.width, actor.height, snap, assets)?;
        next.configure_actor(
            index,
            &tenchu_assets::tenchu2::layout::Actor {
                kind,
                slot: 0,
                behavior,
                position: position.to_array(),
                facing: yaw,
                route: Vec::new(),
            },
            assets,
        )?;
        let actor = &mut next.actors[index];
        actor.slot = -1 - i16::try_from(index).map_err(|_| "Too many placed actors")?;
        actor.actor.previous = position;
        actor.actor.terrain = probe;
        actor.sight.destination = position;
        next.activate_actor(index, assets)?;
        next.publish(index, assets);
        next.collision.compute(|id| next.scene.world(id))?;
        for foot in &mut next.instances[index].feet {
            foot.position = position;
            foot.floor = next.actors[index].actor.terrain.floor;
        }
        // Added actors never replace the campaign's authored target bindings.
        // Their kills still count, so an all-enemies quota must include them.
        if matches!(
            next.mission.objective,
            tenchu_assets::tenchu2::campaign::Objective::EliminateAll
        ) && next.actors[index].actor.health > 0
            && !matches!(kind & 0xf0, 0x10 | 0x60 | 0x70)
        {
            if u16::from(next.mission.population.guards) + u16::from(next.mission.population.bosses)
                >= 255
            {
                return Err("This mission's enemy quota is full".into());
            }
            next.mission.population.guards += 1;
        }
        *self = next;
        Ok(index)
    }

    fn editable_actor(&self, index: usize) -> Result<()> {
        let player = &self.actors[self.player].actor;
        let actor = &self
            .actors
            .get(index)
            .ok_or("Actor no longer exists")?
            .actor;
        if !self.mission.input_allowed()
            || player.health <= 0
            || (0x920..=0x92f).contains(&player.motion.action)
            || index == self.player
            || actor.health < 0
            || self.script_hidden.contains(&index)
            || self.mission.targets.contains(&Some(index))
        {
            return Err("Player, story actors and mission objectives cannot be edited here".into());
        }
        Ok(())
    }

    pub fn reposition_actor(
        &mut self,
        index: usize,
        position: Position,
        yaw: i16,
        snap: bool,
        assets: &Assets,
    ) -> Result<()> {
        self.editable_actor(index)?;
        let mut actor = self.actors[index].actor.clone();
        if actor.health == 0 && actor.max_health != 0 {
            return Err("Only living actors can be moved".into());
        }
        let (position, probe) =
            self.edit_destination(position, actor.width, actor.height, snap, assets)?;
        let action = if actor.weapon_drawn() { 0x100 } else { 0 };
        actor
            .change_motion(action, Velocity::Stop, &assets.resources)?
            .ok_or("No idle motion for this actor")?;
        actor.motion.frame = 0;
        actor
            .motion
            .advance(&assets.resources.clips, &assets.spline, true)?;
        actor.motion.frame = 0;
        actor.position = position;
        actor.previous = position;
        actor.rotation = [0, yaw, 0].into();
        actor.terrain = probe;
        actor.velocity = Default::default();
        actor.last_vertical_velocity = 0;
        actor.flags &= 2 | 0x10;
        actor.hit = None;
        actor.pad = Default::default();
        actor.attack = Default::default();
        actor.target = (!matches!(actor.kind, 0x6a..=0x6e) && actor.kind & 0xf0 != 0x10)
            .then_some(self.player);
        self.collision.allow_expansion();
        let agent = &mut self.actors[index];
        agent.actor = actor;
        agent.behavior.relocate(position);
        agent.sight = Default::default();
        agent.sight.destination = position;
        agent.surroundings = Default::default();
        let actor = &mut agent.actor;
        let rig = &mut self.instances[index].rig;
        actor.set_body_collision(rig.joints[0], index, &mut self.collision, true)?;
        actor.set_secondary_body_collision(rig, index, &mut self.collision, true)?;
        actor.set_weapon(
            rig,
            index,
            &mut self.collision,
            &assets.resources,
            crate::tenchu2::rig::WeaponMode::Refresh,
        )?;
        self.publish(index, assets);
        self.sync_attack_contacts(index);
        for agent in &mut self.actors {
            agent.actor.hit = None;
        }
        self.collision.compute(|id| self.scene.world(id))?;
        for foot in &mut self.instances[index].feet {
            foot.position = position;
            foot.floor = self.actors[index].actor.terrain.floor;
        }
        Ok(())
    }

    pub fn remove_actor(&mut self, index: usize, assets: &Assets) -> Result<()> {
        self.editable_actor(index)?;
        use tenchu_assets::tenchu2::campaign::Objective;
        let actor = &self.actors[index].actor;
        if actor.health > 0
            && !matches!(actor.kind & 0xf0, 0x10 | 0x60 | 0x70)
            && matches!(
                self.mission.objective,
                Objective::EliminateAll | Objective::EliminateUnseen { .. }
            )
        {
            return Err(
                "This actor contributes to the mission's kill quota; use Kill selected actor"
                    .into(),
            );
        }
        self.set_script_visible(index, false, assets)?;
        self.restore_dismemberment(index);
        let actor = &mut self.actors[index].actor;
        actor.health = -1;
        actor.hit = None;
        actor.target = None;
        actor.pad = Default::default();
        actor.attack = Default::default();
        for agent in &mut self.actors {
            agent.actor.hit = None;
            if agent.actor.target == Some(index) {
                agent.actor.target = None;
            }
        }
        if self.controls.focus == Some(index) {
            self.controls.focus = None;
        }
        if self.tracking == Some(index) {
            self.tracking = None;
        }
        self.controls.hud.hide_health(index);
        if self.controls.camera_actor == index {
            self.controls.camera_actor = self.player;
        }
        self.collision.compute(|id| self.scene.world(id))?;
        Ok(())
    }

    pub fn kill_actor(&mut self, index: usize, assets: &Assets) -> Result<()> {
        let player = &self.actors[self.player].actor;
        if !self.mission.input_allowed()
            || player.health <= 0
            || (0x920..=0x92f).contains(&player.motion.action)
        {
            return Err(
                "Killing actors requires a living player outside a cutscene or stealth kill".into(),
            );
        }
        let actor = &self
            .actors
            .get(index)
            .ok_or("Actor no longer exists")?
            .actor;
        if index == self.player
            || actor.health <= 0
            || self.script_hidden.contains(&index)
            || actor.state == State::Dead
        {
            return Err("Choose a living NPC outside a scripted scene".into());
        }
        let mut actor = actor.clone();
        let action = if actor.kind == 0x75 {
            0xb0a
        } else if actor.state == State::Swimming
            && actor.motion.action > 0x50f
            && actor.motion.action != 0x520
        {
            0xb08
        } else {
            0xb00
        };
        let mut random = self.random;
        if actor
            .change_motion(action, Velocity::Motion, &assets.resources)?
            .is_none()
            && (action != 0xb00
                || actor
                    .change_motion(
                        0xb01 + (random.next15() % 5) as i16,
                        Velocity::Motion,
                        &assets.resources,
                    )?
                    .is_none())
        {
            return Err("This actor has no death animation".into());
        }
        actor.health = 0;
        actor.hit = None;
        actor.pad = Default::default();
        actor.attack = Default::default();
        super::controls::credit(
            &mut self.statistics,
            &mut self.script,
            &actor,
            false,
            true,
            assets.hero == 2 && self.map >= 12,
        );
        self.actors[index].actor = actor;
        self.random = random;
        if self.actors[index].actor.flags & 8 != 0 {
            self.activate_actor(index, assets)?;
        }
        self.sync_attack_contacts(index);
        self.controls
            .hud
            .show_health(index, self.actors[index].actor.kind);
        Ok(())
    }

    pub fn teleport_player(
        &mut self,
        position: Position,
        snap: bool,
        assets: &Assets,
    ) -> Result<()> {
        let player = &self.actors[self.player].actor;
        if !self.mission.input_allowed()
            || player.health <= 0
            || matches!(
                player.state,
                State::Attack | State::Item | State::Hurt | State::Dead
            )
        {
            return Err("Teleport requires a living player outside combat animations and scripted transitions.".into());
        }
        let (position, probe) =
            self.edit_destination(position, player.width, player.height, snap, assets)?;
        let mut player = player.clone();
        let action = if player.weapon_drawn() { 0x100 } else { 0 };
        player
            .change_motion(action, Velocity::Keep, &assets.resources)?
            .ok_or("No idle motion for this player.")?;
        player.motion.frame = 0;
        player
            .motion
            .advance(&assets.resources.clips, &assets.spline, true)?;
        player.motion.frame = 0;
        player.position = position;
        player.previous = position;
        player.terrain = probe;
        player.velocity = Default::default();
        player.last_vertical_velocity = 0;
        player.rotation.x = 0;
        player.rotation.z = 0;
        use crate::tenchu2::physics;
        player.flags &= !(physics::FALLING
            | physics::IN_WATER
            | physics::CEILING
            | physics::BLOCKED
            | physics::LANDED
            | physics::UNEVEN
            | physics::TOUCHING
            | physics::HIT
            | physics::TOUCHING_PLAYER);
        player.target = None;
        player.hit = None;
        player.pad = Default::default();
        player.attack = Default::default();
        self.actors[self.player].actor = player;
        self.tracking = None;
        self.controls.focus = None;
        self.controls.camera_actor = self.player;
        self.controls.previous_camera = 0;
        self.controls.ledge_check = 0;
        self.cursor = Default::default();
        self.water = Default::default();
        self.probe = Default::default();
        self.publish(self.player, assets);
        self.sync_attack_contacts(self.player);
        self.collision.compute(|id| self.scene.world(id))?;
        for foot in &mut self.instances[self.player].feet {
            foot.position = position;
            foot.floor = self.actors[self.player].actor.terrain.floor;
        }
        let random = self.random;
        let camera = self.update_camera(assets, true);
        self.random = random;
        camera
    }

    fn edit_destination(
        &self,
        mut position: Position,
        width: i16,
        height: i16,
        snap: bool,
        assets: &Assets,
    ) -> Result<(Position, Probe)> {
        let map = &self.terrain;
        for (axis, coordinate) in [position.x, position.z].into_iter().enumerate() {
            let low = i32::from(map.origin[axis]) * 10;
            let high = low + i32::from(map.cell_size) * map.dimensions[axis] as i32 * 10;
            if !(low..high).contains(&coordinate) {
                return Err("Destination is outside this area's collision map".into());
            }
        }
        let mut cursor = terrain::Cursor::default();
        if snap {
            position.y = terrain::level(
                map,
                &mut cursor,
                position.wrapping_sub(Position::new(0, 1000, 0)),
                Query::Surface,
            )?
            .ok_or("No collision floor below that destination")?;
        }
        let mut probe = Probe::default();
        probe.sample(
            map,
            &mut cursor,
            &assets.terrain_rules,
            position.wrapping_sub(Position::new(0, i32::from(height), 0)),
            width,
            Default::default(),
        )?;
        if probe.floor.is_none() || probe.blocked != 0 {
            return Err("Destination is blocked or too close to a wall".into());
        }
        Ok((position, probe))
    }
}
