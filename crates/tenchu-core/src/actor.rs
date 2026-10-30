use crate::actor::State as ActorState;
mod state;
use crate::math::Position;
pub use state::State;
mod body;
mod combat;
mod damage;
mod death;
mod grapple;
mod item;
pub use combat::Combat;
mod movement;
pub mod patrol;
mod ranged;
mod shadow;
pub mod sight;
#[cfg(test)]
mod tests;
pub mod trail;
mod weapon;

use crate::{
    Result,
    camera::Camera,
    collision::Collision,
    effects::RumbleAction,
    ids::*,
    input::Pad,
    math::{Math, Matrix},
    motion::{Change, Motion},
    random::Random,
    scene::{ModelId, Scene},
    terrain::{self, Cursor, Probe},
};
use tenchu_assets::{executable::BodyTables, stage::MissionAssets};

pub struct Context<'a> {
    pub mission: &'a mut MissionAssets,
    pub math: &'a Math,
    pub body: &'a BodyTables,
    pub cursor: &'a mut Cursor,
    pub random: &'a mut Random,
    pub effects: &'a mut crate::effects::Effects,
    pub scene: &'a mut Scene,
    pub collision: &'a mut Collision,
    pub camera: &'a mut Camera,
    pub drawing: &'a mut crate::drawing::Drawing,
}

impl Context<'_> {
    pub(crate) fn swap_balma_terrain(&mut self) -> Result<()> {
        let alternate = self
            .mission
            .balma_terrain
            .as_mut()
            .ok_or("Missing Balma terrain")?;
        std::mem::swap(&mut self.mission.terrain, alternate);
        self.cursor.row = 0;
        let first = self
            .mission
            .terrain
            .rows
            .first()
            .ok_or("Empty Balma terrain")?;
        self.cursor.area = first.lookup.unwrap_or(first.list);
        Ok(())
    }
}

pub struct ActionContext<'a> {
    pub items: &'a mut crate::items::Items,
    pub roster: &'a mut crate::roster::Roster,
    pub intelligence: &'a mut crate::ai::Intelligence,
    pub statistics: &'a mut crate::game::Statistics,
    pub combat: &'a mut Combat,
    pub player: usize,
    pub selected_item: Option<usize>,
    pub clock: i32,
}

#[derive(Clone, Debug)]
pub struct Actor {
    pub id: usize,
    pub root_model: ModelId,
    pub parts: Vec<ModelId>,
    pub rendered: Vec<Matrix>,
    pub render_mask: u32,
    pub draw_fog: bool,
    pub kind: i16,
    pub script_kind: Option<i16>,
    pub voice_program: Option<i16>,
    pub mesh_kind: i16,
    pub rendered_mesh_kind: i16,
    pub definition: usize,
    pub position: Position,
    pub previous: Position,
    pub velocity: [i16; 3],
    pub rotation: [i16; 3],
    pub health: i16,
    pub invulnerable: bool,
    pub(crate) suppress_idle_taunts: bool,
    pub max_health: i16,
    pub attribute: u16,
    pub status: State,
    pub scripted: bool,
    pub map: Probe,
    pub pad: Pad,
    pub motion: Motion,
    pub hit: Option<usize>,
    pub home: [i32; 2],
    pub target: Option<ModelId>,
    pub think: crate::ai::programs::Programs,
    pub brain: crate::ai::Brain,
    pub active_item: i16,
    pub inventory: [u8; 25],
    pub aiming: bool,
    pub attack: usize,
    pub patrol: Option<patrol::Patrol>,
    pub weapons: [Option<weapon::Weapon>; 4],
    pub weapon_hitbox: [Option<usize>; 2],
    pub trails: [Option<trail::Trail>; 2],
}

impl Actor {
    pub fn new(
        id: usize,
        kind: i16,
        position: Position,
        yaw: i16,
        ctx: &mut Context,
    ) -> Result<Self> {
        let definition = ctx
            .mission
            .characters
            .iter()
            .position(|c| c.kind == kind)
            .ok_or("Missing actor definition")?;
        Self::from_definition(id, definition, kind, position, yaw, ctx)
    }

    pub(crate) fn from_definition(
        id: usize,
        definition: usize,
        mesh_kind: i16,
        position: Position,
        yaw: i16,
        ctx: &mut Context,
    ) -> Result<Self> {
        let data = &ctx.mission.characters[definition];
        let kind = data.kind;
        let parts = ctx
            .mission
            .models
            .get(&mesh_kind)
            .ok_or("Missing actor model")?;
        let mut motion = Motion::new(parts)?;
        if motion.change(
            MOT_NORMAL,
            &data.motions,
            &ctx.mission.common_motions,
            &ctx.mission.clips,
        ) == Change::Missing
        {
            return Err("Actor has no initial motion".into());
        }
        let map = terrain::vector(
            &ctx.mission.terrain,
            ctx.cursor,
            Position::ZERO,
            i32::from(data.width),
            terrain::STEP_DOWN,
        )?;
        let root_model = ctx.scene.add(Some(0), Matrix::default());
        ctx.scene.models[root_model].attribute = 0x1c;
        ctx.scene.models[root_model].clip = [0, -data.height / 2, 0];
        let nodes = motion.pose.bind(ctx.scene, root_model, ctx.math);
        let waist = *nodes.first().ok_or("Actor model has no waist")?;
        let slot = ctx.collision.insert(ctx.scene, waist)?;
        let body = &mut ctx.collision.slots[slot];
        body.owner = Some(crate::collision::Owner::Actor(id));
        body.size = [data.width / 2, data.height / 2, data.width / 2];
        body.offset[1] = (-data.height / 2).wrapping_sub(parts[0].position[1]);
        if matches!(kind, 0x86 | 0x89) {
            body.offset = [0, -0x1c5, 0xc0];
        }
        let mut actor = Self {
            id,
            root_model,
            parts: nodes,
            rendered: Vec::new(),
            render_mask: u32::MAX,
            draw_fog: false,
            kind,
            script_kind: None,
            voice_program: None,
            mesh_kind,
            rendered_mesh_kind: mesh_kind,
            definition,
            position,
            previous: Position::ZERO,
            velocity: [0; 3],
            rotation: [0, yaw, 0],
            health: data.health,
            max_health: data.health,
            attribute: 0,
            status: State::Normal,
            scripted: false,
            invulnerable: false,
            suppress_idle_taunts: false,
            map,
            pad: Pad::default(),
            motion,
            hit: None,
            home: [position[0], position[2]],
            target: None,
            think: crate::ai::programs::Programs::default(),
            brain: crate::ai::Brain::default(),
            active_item: 0,
            inventory: [0; 25],
            aiming: false,
            attack: 0,
            patrol: None,
            weapons: std::array::from_fn(|_| None),
            weapon_hitbox: [None; 2],
            trails: std::array::from_fn(|_| None),
        };
        actor.setup_weapons(ctx)?;
        if kind == 0x87 {
            actor.inventory[3] = 1;
        }
        actor.position.y = terrain::level(
            &ctx.mission.terrain,
            ctx.cursor,
            position,
            terrain::STEP_DOWN,
        )?;
        ctx.scene.update(root_model, actor.root(ctx.math));
        if matches!(kind, 0..=1 | 0x81..=0x8a) {
            actor.attribute |= 2;
            actor.equip(ctx, true);
            actor.enter(ctx, MOT_ENGAGE_STANCE, true);
        }
        if matches!(kind, 0xa6 | 0xa7) {
            actor.attribute |= ATTR_FLOAT;
        }
        actor.initialize_rendered(ctx.scene, ctx.math);
        Ok(actor)
    }

    pub fn initialize_rendered(&mut self, scene: &Scene, math: &Math) {
        self.rendered_mesh_kind = self.mesh_kind;
        self.rendered = self.motion.pose.matrices(math, self.root(math));
        for weapon in self.weapons.iter_mut().flatten() {
            let model = &scene.models[weapon.model];
            let part = self
                .parts
                .iter()
                .position(|&id| Some(id) == model.parent)
                .unwrap();
            weapon.rendered = self.rendered[part].compose(&model.local);
        }
    }

    pub fn root(&self, math: &Math) -> Matrix {
        Matrix {
            translation: self.position,
            rotation: math.rotate_yxz(self.rotation),
        }
    }

    pub fn setup_think(&mut self, mix: i16) -> Result<()> {
        self.think = crate::ai::programs::Programs::decode(mix)?;
        if matches!(mix, 0 | 0x1111 | 0x2222) {
            self.attribute &= !ATTR_CUSTOMAI;
        } else {
            self.attribute |= ATTR_CUSTOMAI;
        }
        Ok(())
    }

    pub fn collision_center(&self, ctx: &Context) -> Position {
        ctx.collision.slots[ctx.scene.models[self.parts[0]].conflict.unwrap()].position
    }

    pub fn prepare_draw(
        &mut self,
        scene: &mut Scene,
        view: Matrix,
        first: bool,
        drawing: &mut crate::drawing::Drawing,
        mission: &MissionAssets,
    ) -> Result<()> {
        drawing.fog = self.draw_fog;
        let draw_body = if first {
            if let Some(clip) = crate::visibility::model(scene, view, self.root_model)? {
                drawing.fog = clip.fog;
                true
            } else {
                false
            }
        } else {
            true
        };
        self.render_mask = 0;
        self.rendered_mesh_kind = self.mesh_kind;
        if draw_body {
            for (i, &model) in self.parts.iter().enumerate() {
                if scene.models[model].attribute & 1 == 0 {
                    self.rendered[i] = scene.world(model)?;
                    self.render_mask |= 1 << i;
                    if let Some(mesh) = mission.models[&self.mesh_kind]
                        .get(i)
                        .and_then(|part| part.geometry.first())
                    {
                        drawing.bytes += crate::packets::draw_tmd(
                            mesh,
                            &view.compose(&self.rendered[i]),
                            drawing.fog,
                        );
                    }
                }
            }
        }
        for weapon in self.weapons[..2].iter_mut().flatten() {
            weapon.rendered = scene.world(weapon.model)?;
            if let Some(mesh) = mission.weapons.models[&weapon.kind].first() {
                drawing.bytes +=
                    crate::packets::draw_tmd(mesh, &view.compose(&weapon.rendered), drawing.fog);
            }
        }
        for trail in self.trails.iter_mut().flatten() {
            drawing.bytes += trail.draw(scene, view, true)?;
        }
        Ok(())
    }

    pub fn look_at_camera(&mut self, ctx: &mut Context) -> Result<()> {
        if self.health <= 0 || self.status == ActorState::Wall {
            return Ok(());
        }
        if self.parts.len() < 3 {
            return Err("Player model has no head".into());
        }
        let angles = if matches!(
            ctx.camera.mode,
            crate::camera::Mode::Direction | crate::camera::Mode::Sight
        ) {
            let pair = i32::from(self.motion.pose.rotation[0][1])
                + i32::from(self.motion.pose.rotation[1][1]);
            [
                ctx.camera.direction[0].clamp(-500, 500),
                (i32::from(ctx.camera.direction[1]) - pair).clamp(-900, 900) as i16,
            ]
        } else {
            if self.motion.loops != -1 {
                return Ok(());
            }
            let clip = &ctx.mission.clips[self.motion.clip.ok_or("Missing player motion")?];
            let track = clip
                .bones
                .get(2)
                .and_then(|t| t.first())
                .ok_or("Missing head motion")?;
            let angles = [track.value[0], track.value[1]];
            if self.motion.pose.rotation[2][..2] == angles {
                return Ok(());
            }
            angles
        };
        self.motion.pose.rotation[2][0] = angles[0];
        self.motion.pose.rotation[2][1] = angles[1];
        self.motion
            .pose
            .publish(ctx.scene, &self.parts, ctx.math, 1 << 2, 3);
        Ok(())
    }

    pub fn look_at_target(&mut self, player: ModelId, ctx: &mut Context) -> Result<()> {
        if self.health <= 0
            || (self.attribute & ATTR_PHASE != 2
                && (self.target == Some(player) || self.motion.action == MOT_ACTION))
        {
            return Ok(());
        }
        let Some(target) = self.target else {
            return Ok(());
        };
        let target = if target == self.root_model {
            self.position
        } else {
            ctx.scene.models[target].local.translation
        };
        if self.parts.len() < 3 {
            return Err("Tracking actor has no head".into());
        }
        let yaw = self.motion.pose.rotation[0][1]
            .wrapping_add(self.motion.pose.rotation[1][1])
            .wrapping_add(self.rotation[1]);
        let degree = ctx.math.direction(
            target[0].wrapping_sub(self.position.x),
            target[2].wrapping_sub(self.position.z),
            yaw,
        )?;
        if i32::from(degree).abs() >= 1800 {
            return Ok(());
        }
        self.motion.pose.rotation[2][1] = degree.clamp(-900, 900);
        let pitch = target[1].wrapping_sub(self.position.y) / 2;
        if pitch != 0 {
            self.motion.pose.rotation[2][0] = pitch.clamp(-500, 100) as i16;
        }
        self.motion
            .pose
            .publish(ctx.scene, &self.parts, ctx.math, 1 << 2, 3);
        Ok(())
    }

    fn registered(&self, ctx: &Context, action: i16) -> bool {
        self.motion_id(ctx, action) >= 0
    }

    pub(crate) fn hide_body(&self, scene: &mut Scene, hidden: bool) {
        for &part in self
            .parts
            .iter()
            .take(1)
            .chain(self.parts.iter().take(13).skip(7))
        {
            if hidden {
                scene.models[part].attribute |= 1;
            } else {
                scene.models[part].attribute &= !1;
            }
        }
    }

    fn motion_id(&self, ctx: &Context, action: i16) -> i16 {
        let clip = ctx.mission.characters[self.definition]
            .motions
            .iter()
            .find(|r| r.action == action)
            .map_or(-1, |r| r.clip);
        ctx.mission
            .imported_models
            .get(&self.mesh_kind)
            .and_then(|model| model.clip_ids.get(&clip))
            .copied()
            .unwrap_or(clip)
    }

    fn enter_unless_scripted(&mut self, ctx: &Context, action: i16, movement: bool) -> bool {
        !self.scripted && self.enter(ctx, action, movement)
    }

    pub(crate) fn enter(&mut self, ctx: &Context, action: i16, movement: bool) -> bool {
        if self.status == ActorState::Dead && self.motion.loops == -1 {
            return false;
        }
        let data = &ctx.mission.characters[self.definition];
        if self.motion.change(
            action,
            &data.motions,
            &ctx.mission.common_motions,
            &ctx.mission.clips,
        ) == Change::Missing
        {
            return false;
        }
        self.status = ActorState::from_motion(action);
        if movement {
            self.motion_velocity(ctx);
        }
        true
    }

    fn move_at(&mut self, math: &Math, forward: i16, side: i16) {
        let v = math.velocity(self.rotation[1], forward, side);
        self.velocity[0] = v[0];
        self.velocity[2] = v[2];
    }

    pub(crate) fn motion_velocity(&mut self, ctx: &Context) {
        if let Some(i) = self.motion.clip {
            let clip = &ctx.mission.clips[i];
            self.move_at(
                ctx.math,
                i16::from(clip.forward_speed),
                i16::from(clip.side_speed),
            );
        }
    }

    pub(crate) fn return_normal(&self, ctx: &mut Context, player: usize) -> Result<i16> {
        if self.id == player {
            ctx.camera.normal(self, ctx.math)?;
        }
        Ok(if self.attribute & ATTR_WEAPON_DRAWN != 0 {
            MOT_ENGAGE_STANCE
        } else {
            MOT_NORMAL
        })
    }
}
