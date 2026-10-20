use super::Game;
use crate::{
    Result,
    actor::Context,
    input::Pad,
    terrain::{self, Cursor},
};

impl Game {
    pub fn teleport_player(&mut self, mut position: [i32; 3], snap_to_floor: bool) -> Result<()> {
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
                [position[0], position[1] - 1000, position[2]],
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
        actor.inventory[25] = 0;
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
        ctx.camera.set_view(camera_position, reference, ctx.math)?;
        ctx.camera.target = position;
        self.rendered_view = ctx.camera.view;
        self.effects.wires.clear();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
            assert!(game.teleport_player([320001, 0, 0], false).is_err());
            assert_eq!(game.player().position, position);
            game.cinematic.active = true;
            assert!(game.teleport_player(position, false).is_err());
            game.cinematic.active = false;
            let target = [position[0] + 250, position[1] - 100, position[2] + 250];
            game.teleport_player(target, true).unwrap();
            let player = game.player();
            assert_eq!(player.position[0], target[0]);
            assert_eq!(player.position[2], target[2]);
            assert_eq!(player.position[1], player.map.level);
            assert_eq!(player.previous, player.position);
            assert_eq!(player.velocity, [0; 3]);
            assert_eq!(
                game.scene.models[player.root_model].local.translation,
                player.position
            );
            let slot = game.collision.slots[..game.collision.len]
                .iter()
                .find(|s| s.owner == Some(game.player_id) && s.class & crate::collision::HIT == 0)
                .unwrap();
            assert!((slot.position[0] - target[0]).abs() < 500);
            assert!((slot.position[2] - target[2]).abs() < 500);
            assert_eq!(game.random, random);
            assert_eq!(game.tick, tick);
            for _ in 0..30 {
                game.step(crate::input::UP).unwrap();
            }
            assert!(game.player().health > 0);
        }
    }
}
