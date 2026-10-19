mod body;
mod movement;

use crate::{
    Result,
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
    pub mission: &'a MissionAssets,
    pub math: &'a Math,
    pub body: &'a BodyTables,
    pub cursor: &'a mut Cursor,
    pub random: &'a mut Random,
    pub effects: &'a mut crate::effects::Effects,
    pub scene: &'a mut Scene,
    pub collision: &'a mut Collision,
}

#[derive(Clone, Debug)]
pub struct Actor {
    pub id: usize,
    pub root_model: ModelId,
    pub parts: Vec<ModelId>,
    pub rendered: Vec<Matrix>,
    pub kind: i16,
    pub definition: usize,
    pub position: [i32; 3],
    pub previous: [i32; 3],
    pub velocity: [i16; 3],
    pub rotation: [i16; 3],
    pub health: i16,
    pub max_health: i16,
    pub attribute: u16,
    pub status: i16,
    pub map: Probe,
    pub pad: Pad,
    pub motion: Motion,
    pub hit: Option<usize>,
    pub trace_count: i16,
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
            kind,
            definition,
            position,
            previous: [0; 3],
            velocity: [0; 3],
            rotation: [0, yaw, 0],
            health: data.health,
            max_health: data.health,
            attribute: 0,
            status: 0,
            map,
            pad: Pad::default(),
            motion,
            hit: None,
            trace_count: 0,
        };
        actor.position[1] = terrain::level(
            &ctx.mission.terrain,
            ctx.cursor,
            position,
            terrain::STEP_DOWN,
        )?;
        ctx.scene.update(root_model, actor.root(ctx.math));
        if matches!(kind, 0..=1 | 0x81..=0x8a) {
            actor.attribute |= ATTR_WEAPON_DRAWN | 2;
            actor.enter(ctx, MOT_ENGAGE_STANCE, true);
        }
        if matches!(kind, 0xa6 | 0xa7) {
            actor.attribute |= ATTR_FLOAT;
        }
        actor.rendered = actor.motion.pose.matrices(ctx.math, actor.root(ctx.math));
        Ok(actor)
    }

    pub fn root(&self, math: &Math) -> Matrix {
        Matrix {
            translation: self.position,
            rotation: math.rotate_yxz(self.rotation),
        }
    }

    pub fn collision_center(&self, ctx: &Context) -> [i32; 3] {
        ctx.collision.slots[ctx.scene.models[self.parts[0]].conflict.unwrap()].position
    }

    pub fn prepare_draw(&mut self, scene: &mut Scene) -> Result<()> {
        scene.world(self.root_model)?;
        for (i, &model) in self.parts.iter().enumerate() {
            if scene.models[model].attribute & 1 == 0 {
                self.rendered[i] = scene.world(model)?;
            }
        }
        Ok(())
    }

    fn registered(&self, ctx: &Context, action: i16) -> bool {
        ctx.mission.characters[self.definition]
            .motions
            .iter()
            .any(|r| r.action == action && r.clip >= 0)
    }

    fn enter(&mut self, ctx: &Context, action: i16, movement: bool) -> bool {
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

    fn motion_velocity(&mut self, ctx: &Context) {
        if let Some(i) = self.motion.clip {
            let clip = &ctx.mission.clips[i];
            self.move_at(
                ctx.math,
                i16::from(clip.forward_speed),
                i16::from(clip.side_speed),
            );
        }
    }

    fn return_motion(&self) -> i16 {
        if self.attribute & ATTR_WEAPON_DRAWN != 0 {
            MOT_ENGAGE_STANCE
        } else {
            MOT_NORMAL
        }
    }
}
