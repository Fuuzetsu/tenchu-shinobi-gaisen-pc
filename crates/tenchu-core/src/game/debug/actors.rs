use super::*;
use crate::{
    actor::Actor,
    ai::programs::{Idle, Programs},
    ids::*,
};

impl Game {
    fn can_edit_actors(&self) -> Result<()> {
        if self.cinematic.active
            || self.player().scripted
            || self.player().health <= 0
            || self.camera.mode == crate::camera::Mode::Critical
        {
            return Err(
                "Actor editing requires a living player outside a cutscene or stealth kill".into(),
            );
        }
        Ok(())
    }

    fn editable_actor(&self, id: usize) -> Result<()> {
        self.can_edit_actors()?;
        let actor = self
            .roster
            .actors
            .get(id)
            .and_then(Option::as_ref)
            .ok_or("Actor no longer exists")?;
        let kind = actor.script_kind.unwrap_or(actor.kind);
        if id == self.player_id
            || Some(id) == self.items.dog_actor
            || actor.scripted
            || self.mission.story.iter().any(|a| a.kind == kind)
            || self
                .mission
                .scripts
                .events
                .iter()
                .any(|e| e.target != 255 && i16::from(e.target) == kind)
        {
            return Err("Player, story actors and mission objectives cannot be edited here".into());
        }
        Ok(())
    }

    pub fn place_actor(
        &mut self,
        source: tenchu_assets::stage::imports::Actor,
        mut think: Programs,
        position: Position,
        yaw: i16,
        snap: bool,
    ) -> Result<usize> {
        self.can_edit_actors()?;
        let definition = self
            .mission
            .characters
            .get(source.definition)
            .ok_or("Missing actor definition")?;
        let (position, probe) = self.edit_destination(position, definition.width, snap)?;
        if matches!(think.idle, Idle::Trace | Idle::Player) {
            think.idle = Idle::Watch;
        }
        let player = self.player().root_model;
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
        let id = self.roster.spawn_with(&mut ctx, |id, ctx| {
            let mut actor =
                Actor::from_definition(id, source.definition, source.model, position, yaw, ctx)?;
            // An extra boss must not replace the authored actor in mission script lookups.
            actor.script_kind = Some(-1);
            actor.position = position;
            actor.previous = position;
            actor.map = probe;
            actor.target = Some(player);
            actor.think = think;
            actor.attribute |= ATTR_CUSTOMAI;
            ctx.scene.update(actor.root_model, actor.root(ctx.math));
            actor.initialize_rendered(ctx.scene, ctx.math);
            Ok(actor)
        })?;
        ctx.collision.compute(ctx.scene)?;
        Ok(id)
    }

    pub fn move_actor(
        &mut self,
        id: usize,
        position: Position,
        yaw: i16,
        snap: bool,
    ) -> Result<()> {
        self.editable_actor(id)?;
        let actor = self.roster.get(id);
        if actor.health <= 0 {
            return Err("Only living actors can be moved".into());
        }
        let width = self.mission.characters[actor.definition].width;
        let (position, probe) = self.edit_destination(position, width, snap)?;
        let actor = self.roster.get_mut(id);
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
        actor.rotation[1] = yaw;
        actor.velocity = [0; 3];
        actor.home = [position.x, position.z];
        actor.patrol = None;
        if actor.think.idle == Idle::Trace {
            actor.think.idle = Idle::Watch;
        }
        actor.attribute &=
            ATTR_PHASE | ATTR_CUSTOMAI | ATTR_SEARCH | ATTR_FLOAT | ATTR_WEAPON_DRAWN;
        actor.pad = Pad::default();
        actor.hit = None;
        actor.aiming = false;
        actor.map = probe;
        actor.motion.action = -1;
        let action = actor.return_normal(&mut ctx, self.player_id)?;
        actor.enter(&ctx, action, false);
        actor.play_pose(&mut ctx)?;
        ctx.scene.update(actor.root_model, actor.root(ctx.math));
        ctx.scene.models[actor.parts[0]].attribute |= crate::scene::COLLIDE;
        for &part in &actor.parts {
            ctx.scene.models[part].flags = 0;
        }
        actor.initialize_rendered(ctx.scene, ctx.math);
        ctx.effects
            .shadows
            .retain(|shadow| shadow.owner != actor.root_model);
        ctx.collision.compute(ctx.scene)?;
        Ok(())
    }

    pub fn remove_actor(&mut self, id: usize) -> Result<()> {
        self.editable_actor(id)?;
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
        let actor = self.roster.get_mut(id);
        actor.clear_attack(&mut ctx, 3)?;
        ctx.effects
            .shadows
            .retain(|shadow| shadow.owner != actor.root_model);
        self.roster.remove(id, &mut ctx);
        ctx.collision.compute(ctx.scene)?;
        self.visible.retain(|&actor| actor != id);
        Ok(())
    }
}
