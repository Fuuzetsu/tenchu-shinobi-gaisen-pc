use super::{Assets, World};
use crate::{
    Result,
    math::Position,
    tenchu2::{
        actor::{State, Velocity},
        terrain::{self, Probe, Query},
    },
};

impl World {
    pub fn teleport_player(
        &mut self,
        mut position: Position,
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
        let map = &self.terrain;
        for (axis, coordinate) in [position.x, position.z].into_iter().enumerate() {
            let low = i32::from(map.origin[axis]) * 10;
            let high = low + i32::from(map.cell_size) * map.dimensions[axis] as i32 * 10;
            if !(low..high).contains(&coordinate) {
                return Err("Teleport destination is outside this area's collision map.".into());
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
            .ok_or("No collision floor below that destination.")?;
        }
        let mut probe = Probe::default();
        probe.sample(
            map,
            &mut cursor,
            &assets.terrain_rules,
            position.wrapping_sub(Position::new(0, i32::from(player.height), 0)),
            player.width,
            Default::default(),
        )?;
        if probe.floor.is_none() || probe.blocked != 0 {
            return Err("Teleport destination is blocked or too close to a wall.".into());
        }
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
}
