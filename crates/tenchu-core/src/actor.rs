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
    pub mesh_kind: i16,
    pub rendered_mesh_kind: i16,
    pub definition: usize,
    pub position: [i32; 3],
    pub previous: [i32; 3],
    pub velocity: [i16; 3],
    pub rotation: [i16; 3],
    pub health: i16,
    pub max_health: i16,
    pub attribute: u16,
    pub status: i16,
    pub scripted: bool,
    pub map: Probe,
    pub pad: Pad,
    pub motion: Motion,
    pub hit: Option<usize>,
    pub home: [i32; 2],
    pub target: Option<ModelId>,
    pub think: [u8; 4],
    pub brain: crate::ai::Brain,
    pub active_item: i16,
    pub inventory: [u8; 26],
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
        position: [i32; 3],
        yaw: i16,
        ctx: &mut Context,
    ) -> Result<Self> {
        let definition = ctx
            .mission
            .characters
            .iter()
            .position(|c| c.kind == kind)
            .ok_or("Missing actor definition")?;
        let data = &ctx.mission.characters[definition];
        let parts = ctx.mission.models.get(&kind).ok_or("Missing actor model")?;
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
            [0; 3],
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
        body.owner = Some(id);
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
            mesh_kind: kind,
            rendered_mesh_kind: kind,
            definition,
            position,
            previous: [0; 3],
            velocity: [0; 3],
            rotation: [0, yaw, 0],
            health: data.health,
            max_health: data.health,
            attribute: 0,
            status: 0,
            scripted: false,
            map,
            pad: Pad::default(),
            motion,
            hit: None,
            home: [position[0], position[2]],
            target: None,
            think: [0; 4],
            brain: crate::ai::Brain::default(),
            active_item: 0,
            inventory: [0; 26],
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
        actor.position[1] = terrain::level(
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

    pub fn setup_think(&mut self, mix: i16) {
        self.think = std::array::from_fn(|i| ((mix as u16 >> (i * 4)) & 15) as u8);
        if matches!(mix, 0 | 0x1111 | 0x2222) {
            self.attribute &= !ATTR_CUSTOMAI;
        } else {
            self.attribute |= ATTR_CUSTOMAI;
        }
    }

    pub fn collision_center(&self, ctx: &Context) -> [i32; 3] {
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
        if self.health <= 0 || self.status == 12 {
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
            target[0].wrapping_sub(self.position[0]),
            target[2].wrapping_sub(self.position[2]),
            yaw,
        )?;
        if i32::from(degree).abs() >= 1800 {
            return Ok(());
        }
        self.motion.pose.rotation[2][1] = degree.clamp(-900, 900);
        let pitch = target[1].wrapping_sub(self.position[1]) / 2;
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
        ctx.mission.characters[self.definition]
            .motions
            .iter()
            .find(|r| r.action == action)
            .map_or(-1, |r| r.clip)
    }

    fn enter_unless_scripted(&mut self, ctx: &Context, action: i16, movement: bool) -> bool {
        !self.scripted && self.enter(ctx, action, movement)
    }

    pub(crate) fn enter(&mut self, ctx: &Context, action: i16, movement: bool) -> bool {
        if self.status == 17 && self.motion.loops == -1 {
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
        self.status = (action >> 8) as i8 as i16;
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
