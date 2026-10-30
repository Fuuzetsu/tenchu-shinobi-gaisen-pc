use super::Game;
use crate::items::Kind;
use crate::math::Position;
use crate::{
    Result,
    actor::Context,
    input::Pad,
    terrain::{self, Cursor},
};

impl Game {
    pub fn replace_actor(
        &mut self,
        id: usize,
        source: tenchu_assets::stage::imports::Actor,
        think: i16,
    ) -> Result<()> {
        if self.tick != 0 || id == self.player_id {
            return Err("Actor replacement is only available before a mission starts".into());
        }
        let original = self.roster.get(id).clone();
        let mut ctx = Context {
            mission: &mut self.mission,
            math: &self.math,
            body: &self.body,
            cursor: &mut self.cursor,
            random: &mut self.random,
            effects: &mut self.effects,
            scene: &mut self.scene,
            collision: &mut self.collision,
            camera: &mut self.camera,
            drawing: &mut self.drawing,
        };
        let mut actor = crate::actor::Actor::from_definition(
            id,
            source.definition,
            source.model,
            original.position,
            original.rotation[1],
            &mut ctx,
        )?;
        actor.script_kind = Some(original.script_kind.unwrap_or(original.kind));
        if original.health < 0 {
            actor.health = original.health;
        }
        actor.target = original.target;
        actor.home = original.home;
        actor.setup_think(if original.patrol.is_some() {
            (think & !15) | 3
        } else {
            think
        })?;
        actor.patrol = original.patrol.clone();
        actor.attribute |= original.attribute & (crate::ids::ATTR_SUSPEND | crate::ids::ATTR_TRACE);
        if actor.attribute & crate::ids::ATTR_SUSPEND != 0 {
            ctx.scene.models[actor.parts[0]].attribute &= !crate::scene::COLLIDE;
        }
        ctx.collision.remove(ctx.scene, original.parts[0]);
        for weapon in original.weapons.iter().flatten() {
            ctx.collision.remove(ctx.scene, weapon.model);
        }
        self.roster.actors[id] = Some(actor);
        Ok(())
    }

    pub fn set_inventory(&mut self, kind: usize, count: u8) -> Result<()> {
        if kind >= 19 {
            return Err("Only equipment slots can be edited".into());
        }
        if self.cinematic.active || self.player().health <= 0 {
            return Err("Inventory editing requires a living player outside a cutscene".into());
        }
        self.roster.get_mut(self.player_id).inventory[kind] = count;
        if self
            .hud
            .selected
            .is_none_or(|slot| self.player().inventory[slot.index()] == 0)
        {
            self.hud.selected = self.player().inventory[..19]
                .iter()
                .position(|&n| n != 0)
                .map(|i| Kind::ALL[i]);
            self.hud.cursor = self.hud.selected.map_or(0, Kind::index);
        }
        Ok(())
    }

    pub fn teleport_player(&mut self, mut position: Position, snap_to_floor: bool) -> Result<()> {
        if self.cinematic.active || self.player().scripted || self.player().health <= 0 {
            return Err("Teleport requires a living player outside a cutscene".into());
        }
        if position.iter().any(|v| !(-320_000..=320_000).contains(v)) {
            return Err("Teleport coordinates must be within +/-320000".into());
        }
        let mut cursor = Cursor::new(&self.mission.terrain);
        if snap_to_floor {
            let floor = terrain::level(
                &self.mission.terrain,
                &mut cursor,
                Position::new(position[0], position[1] - 1000, position[2]),
                0,
            )?;
            if floor == terrain::NONE {
                return Err("No collision floor below the destination".into());
            }
            position[1] = floor;
        }
        let width = self.mission.characters[self.player().definition].width;
        let probe = terrain::vector(
            &self.mission.terrain,
            &mut cursor,
            position,
            i32::from(width),
            terrain::STEP_DOWN,
        )?;
        let delta = std::array::from_fn::<_, 3, _>(|i| position[i] - self.player().position[i]);
        let actor = self.roster.get_mut(self.player_id);
        let mut ctx = Context {
            mission: &mut self.mission,
            math: &self.math,
            body: &self.body,
            cursor: &mut self.cursor,
            random: &mut self.random,
            effects: &mut self.effects,
            scene: &mut self.scene,
            collision: &mut self.collision,
            camera: &mut self.camera,
            drawing: &mut self.drawing,
        };
        actor.clear_attack(&mut ctx, 3)?;
        actor.position = position;
        actor.previous = position;
        actor.velocity = [0; 3];
        actor.pad = Pad::default();
        actor.hit = None;
        actor.aiming = false;
        actor.attribute &= 0xff;
        actor.map = probe;
        actor.motion.action = -1;
        let action = actor.return_normal(&mut ctx, self.player_id)?;
        actor.enter(&ctx, action, false);
        actor.hide_body(ctx.scene, false);
        actor.play_pose(&mut ctx)?;
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
        for &part in &actor.parts {
            ctx.scene.models[part].flags = 0;
        }
        actor.initialize_rendered(ctx.scene, ctx.math);
        ctx.collision.compute(ctx.scene)?;
        let camera_position =
            std::array::from_fn(|i| ctx.camera.position[i].saturating_add(delta[i]));
        let reference = std::array::from_fn(|i| ctx.camera.reference[i].saturating_add(delta[i]));
        ctx.camera
            .set_view(camera_position.into(), reference.into(), ctx.math)?;
        ctx.camera.target = position;
        self.rendered_view = ctx.camera.view;
        self.effects.wires.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use crate::math::Position;
    #[test]
    #[ignore = "requires TENCHU_DISC; checks teleport pose/collision refresh, validation, RNG isolation and continued play"]
    fn teleport_refreshes_player_without_advancing_simulation() {
        for character in 0..2 {
            let mut game = crate::oracle::game(0, character);
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            let random = game.random;
            let tick = game.tick;
            let position = game.player().position;
            assert!(
                game.teleport_player(Position::new(320001, 0, 0), false)
                    .is_err()
            );
            assert_eq!(game.player().position, position);
            game.cinematic.active = true;
            assert!(game.teleport_player(position, false).is_err());
            game.cinematic.active = false;
            let target = [position[0] + 250, position[1] - 100, position[2] + 250];
            game.teleport_player(target.into(), true).unwrap();
            let player = game.player();
            assert_eq!(player.position.x, target[0]);
            assert_eq!(player.position.z, target[2]);
            assert_eq!(player.position.y, player.map.level);
            assert_eq!(player.previous, player.position);
            assert_eq!(player.velocity, [0; 3]);
            assert_eq!(
                game.scene.models[player.root_model].local.translation,
                player.position
            );
            let slot = game.collision.slots[..game.collision.len]
                .iter()
                .find(|s| s.actor() == Some(game.player_id) && s.class & crate::collision::HIT == 0)
                .unwrap();
            assert!((slot.position.x - target[0]).abs() < 500);
            assert!((slot.position.z - target[2]).abs() < 500);
            assert_eq!(game.random, random);
            assert_eq!(game.tick, tick);
            for _ in 0..30 {
                game.step(crate::input::UP).unwrap();
            }
            assert!(game.player().health > 0);
        }
    }
}
