mod bomb;
mod breath;
mod caltrops;
mod flight;
mod grapple;
mod medicine;
mod napalm;
mod pickup;
mod poison;
mod projectile;
mod rolling;
mod sight_shot;
mod sleep;
mod smoke;
mod sprite;
#[cfg(test)]
mod tests;
mod use_item;

use crate::{
    Result,
    actor::{Actor, Context},
    effects,
    math::Matrix,
    roster::{Actors, Roster},
    scene::{ModelId, Scene},
    terrain, visibility,
};
pub use rolling::Rolling;

pub const CAPACITY: usize = 30;
pub const KINDS: usize = 25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Processor {
    Pickup,
    Shuriken,
    Arrow,
    Gun,
    SightShot,
    Medicine,
    Caltrops,
    Grapple,
    Smoke,
    Fire,
    Mine,
    Poison,
    Napalm,
    Breath,
    Sleep,
}

#[derive(Clone, Debug, Default)]
pub struct Contact {
    pub class: i16,
    pub paused: bool,
    pub size: i16,
    pub offset_y: i16,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub owner: Option<usize>,
    pub model: ModelId,
    pub kind: usize,
    pub processor: Option<Processor>,
    pub mode: u8,
    pub collision: Contact,
    pub rolling: Rolling,
    pub count: u8,
    pub flight: flight::Flight,
    pub trail: Option<crate::actor::trail::Trail>,
    pub rotation: [i16; 3],
    pub poison: poison::Poison,
    pub napalm: napalm::Napalm,
    pub breath: breath::Breath,
}

#[derive(Debug)]
pub struct Items {
    pub wall_item: Option<usize>,
    pub slots: Vec<Item>,
    pub drawings: Vec<effects::Drawing>,
    pub models: Vec<(usize, Matrix)>,
    pub weapons: Vec<(i16, Matrix)>,
    pub trails: Vec<crate::actor::trail::Trail>,
    pub sprites: Vec<crate::hud::Sprite>,
    meshes: [ModelId; 4],
    napalm_sprites: [ModelId; 2],
    sleep_sprite: ModelId,
    cursor: usize,
    icons: [ModelId; KINDS + 1],
    icon_scale: [i32; KINDS + 1],
}

fn target_distance(actor: &Actor, position: [i32; 3], radius: i32, ctx: &Context) -> Option<i32> {
    if actor.health <= 0
        || actor.motion.action == crate::ids::MOT_ACTION
        || actor.attribute & crate::ids::ATTR_SUSPEND != 0
    {
        return None;
    }
    let distance = ctx.math.vector_distance(std::array::from_fn(|i| {
        position[i].wrapping_sub(actor.position[i])
    }));
    (distance < radius).then_some(distance)
}

impl Items {
    fn update_coordinate(&self, id: usize, ctx: &mut Context) {
        let item = &self.slots[id];
        ctx.scene.models[item.model].local.rotation = ctx.math.rotate_yxz(item.rotation);
        ctx.scene.models[item.model].flags = 0;
    }

    pub fn new(scene: &mut Scene) -> Self {
        let meshes = std::array::from_fn(|_| scene.add(Some(0), Matrix::default()));
        let slots = (0..CAPACITY)
            .map(|_| Item {
                owner: None,
                model: scene.add(Some(0), Matrix::default()),
                kind: 0,
                processor: None,
                mode: 0,
                collision: Contact::default(),
                rolling: Rolling::default(),
                count: 0,
                flight: flight::Flight::default(),
                trail: None,
                rotation: [0; 3],
                poison: poison::Poison::default(),
                napalm: napalm::Napalm::default(),
                breath: breath::Breath::default(),
            })
            .collect();
        let icons = std::array::from_fn(|_| {
            let id = scene.add(Some(0), Matrix::default());
            scene.models[id].attribute = 0x1c;
            id
        });
        Self {
            wall_item: Some(0),
            slots,
            drawings: Vec::new(),
            models: Vec::new(),
            weapons: Vec::new(),
            trails: Vec::new(),
            sprites: Vec::new(),
            meshes,
            napalm_sprites: std::array::from_fn(|_| scene.add(Some(0), Matrix::default())),
            sleep_sprite: scene.add(Some(0), Matrix::default()),
            cursor: 0,
            icons,
            icon_scale: [12288; KINDS + 1],
        }
    }

    pub(crate) fn begin_tick(&mut self) {
        self.drawings.clear();
        self.models.clear();
        self.weapons.clear();
        self.trails.clear();
        self.sprites.clear();
    }

    fn draw_model(&mut self, id: usize, mesh: usize, ctx: &mut Context) -> Result<()> {
        let model = self.meshes[mesh];
        ctx.scene.copy_coordinate(model, self.slots[id].model);
        let matrix = ctx.scene.world(model)?;
        let view = ctx.camera.view.compose(&matrix);
        if let Some(clip) = visibility::clip(&view, ctx.scene.models[model].attribute, [0; 3]) {
            ctx.drawing.fog = clip.fog;
            let asset = [20, 21, 27, 28][mesh];
            if let Some(geometry) = ctx.mission.common_models[&asset].first() {
                ctx.drawing.bytes += crate::packets::draw_tmd(geometry, &view, ctx.drawing.fog);
            }
            self.models.push((asset, matrix));
        }
        Ok(())
    }

    fn allocate(&mut self, actors: &mut Actors, ctx: &mut Context) -> Result<usize> {
        for _ in 0..CAPACITY - 1 {
            self.cursor = (self.cursor + 1) % CAPACITY;
            if self.slots[self.cursor].processor.is_none() {
                return Ok(self.cursor);
            }
        }
        self.dispose(self.cursor, actors, ctx)?;
        Ok(self.cursor)
    }

    fn dispose(&mut self, id: usize, actors: &mut Actors, ctx: &mut Context) -> Result<()> {
        if self.slots[id].processor == Some(Processor::Poison) {
            self.restore_poison_target(id, actors)?;
        }
        if self.slots[id].processor == Some(Processor::Breath) {
            self.finish_breath(id, actors, ctx)?;
        }
        let item = &mut self.slots[id];
        match item.processor {
            Some(Processor::Grapple) => item.mode = 0,
            Some(Processor::SightShot) => {
                actors
                    .get_mut(item.owner.ok_or("Aimed item has no owner")?)
                    .inventory[25] = 0;
                item.mode = 0;
            }
            Some(Processor::Shuriken) => {
                item.trail = None;
                item.mode = 0;
            }
            Some(_) => item.mode = 0,
            None => return Ok(()),
        }
        ctx.collision.remove(ctx.scene, item.model);
        item.owner = None;
        item.processor = None;
        Ok(())
    }

    fn initialize(
        &mut self,
        id: usize,
        processor: Processor,
        kind: usize,
        owner: Option<usize>,
        position: [i32; 3],
        ctx: &mut Context,
    ) -> Result<()> {
        if kind >= KINDS {
            return Err(format!("Invalid item kind {kind}"));
        }
        let item = &mut self.slots[id];
        item.owner = owner;
        item.kind = kind;
        item.processor = Some(processor);
        item.mode = 0;
        let model = &mut ctx.scene.models[item.model];
        model.parent = None;
        model.local.translation = position;
        model.flags = 0;
        Ok(())
    }

    fn contact(&mut self, id: usize, size: i16, class: i16, ctx: &mut Context) -> Result<()> {
        let item = &mut self.slots[id];
        ctx.collision.remove(ctx.scene, item.model);
        let slot = ctx.collision.insert(ctx.scene, item.model)?;
        ctx.collision.slots[slot].size = [size; 3];
        ctx.collision.slots[slot].class = class;
        item.collision = Contact {
            class,
            size,
            ..Contact::default()
        };
        Ok(())
    }

    fn draw_icon(&mut self, id: usize, ctx: &mut Context) -> Result<()> {
        let item = &self.slots[id];
        self.draw_sprite(
            item.model,
            self.icons[item.kind],
            sprite::Sprite {
                graphic: effects::Graphic::Sprite {
                    image: if item.kind < 20 {
                        20 + item.kind as u8
                    } else {
                        15
                    },
                    blend: 0,
                },
                scale: self.icon_scale[item.kind],
                color: 128,
                rotation: 0,
            },
            ctx,
        )
    }

    fn update_contacts(&mut self, ctx: &mut Context) -> Result<()> {
        for item in &mut self.slots {
            if item.processor.is_none() {
                continue;
            }
            let position = ctx.scene.models[item.model].local.translation;
            let nearby = (0..3).all(|i| {
                ctx.camera.position[i]
                    .wrapping_sub(position[i])
                    .wrapping_abs()
                    < 15000
            });
            let contact = &mut item.collision;
            if nearby {
                if contact.paused && contact.size != 0 {
                    ctx.collision.remove(ctx.scene, item.model);
                    let slot = ctx.collision.insert(ctx.scene, item.model)?;
                    let slot = &mut ctx.collision.slots[slot];
                    slot.size = [contact.size; 3];
                    slot.offset[1] = contact.offset_y;
                    slot.class = contact.class;
                    contact.paused = false;
                }
            } else if !contact.paused && ctx.scene.models[item.model].parent.is_none() {
                contact.paused = true;
                ctx.collision.remove(ctx.scene, item.model);
            }
        }
        Ok(())
    }

    pub(crate) fn step(
        &mut self,
        ctx: &mut Context,
        roster: &mut Roster,
        intelligence: &mut crate::ai::Intelligence,
        player: usize,
        clock: i32,
    ) -> Result<()> {
        let actors = &mut Actors::new(roster);
        if clock % 10 == 0 {
            self.update_contacts(ctx)?;
        }
        for id in 0..CAPACITY {
            match self.slots[id].processor {
                Some(Processor::Pickup) => {
                    self.pickup(id, ctx, actors, intelligence.action_halt)?
                }
                Some(Processor::Shuriken) => self.shuriken(id, ctx, actors, intelligence, clock)?,
                Some(Processor::Arrow) => self.arrow(id, ctx, actors, clock)?,
                Some(Processor::Gun) => self.gun(id, ctx, actors)?,
                Some(Processor::SightShot) => self.sight_shot(id, actors, ctx)?,
                Some(Processor::Medicine) => {
                    self.medicine(id, actors, intelligence.action_halt, ctx)?
                }
                Some(Processor::Caltrops) => self.caltrops(id, actors, ctx)?,
                Some(Processor::Grapple) => self.grapple(id, actors, ctx)?,
                Some(Processor::Smoke) => self.smoke(id, actors, intelligence, clock, ctx)?,
                Some(Processor::Fire) => self.fire(id, actors, intelligence, ctx)?,
                Some(Processor::Mine) => self.mine(id, actors, intelligence, ctx)?,
                Some(Processor::Poison) => {
                    self.poison(id, actors, intelligence, player, clock, ctx)?
                }
                Some(Processor::Napalm) => self.napalm(id, actors, ctx)?,
                Some(Processor::Breath) => {
                    self.breath(id, actors, intelligence.action_halt, clock, ctx)?
                }
                Some(Processor::Sleep) => self.sleep(id, actors, intelligence, ctx)?,
                None => {}
            }
        }
        Ok(())
    }

    pub(crate) fn drop_inventory(
        &mut self,
        actor: &mut Actor,
        roster: &mut Roster,
        ctx: &mut Context,
    ) -> Result<()> {
        let owner = actor.id;
        let actors = &mut Actors {
            roster,
            active: Some(actor),
        };
        for kind in 0..KINDS {
            for _ in 0..actors.get(owner).inventory[kind] {
                let position = ctx.scene.world(actors.get(owner).parts[0])?.translation;
                let velocity = [
                    ctx.random.next15() % 200 - 100,
                    ctx.random.next15() % 100 - 200,
                    ctx.random.next15() % 200 - 100,
                ];
                self.drop(kind, Some(owner), position, velocity, actors, ctx)?;
            }
            actors.get_mut(owner).inventory[kind] = 0;
        }
        Ok(())
    }

    pub fn kind_for_contact(&self, scene: &Scene, slot: usize) -> usize {
        self.slots
            .iter()
            .find(|item| scene.models[item.model].conflict == Some(slot))
            .map_or(0, |item| item.kind)
    }

    pub(crate) fn restore_layout(&mut self, roster: &mut Roster, ctx: &mut Context) -> Result<()> {
        let actors = &mut Actors::new(roster);
        for id in 0..CAPACITY {
            self.dispose(id, actors, ctx)?;
        }
        for spawn in ctx.mission.pickups.clone() {
            let mut position = spawn.position;
            let floor = terrain::level(
                &ctx.mission.terrain,
                ctx.cursor,
                position,
                terrain::STEP_DOWN,
            )?;
            let valid = |floor: i32| {
                floor != terrain::NONE && floor.wrapping_sub(position[1]).wrapping_abs() < 1000
            };
            if valid(floor) {
                position[1] = floor;
            } else {
                let mut found = None;
                for [x, z] in [[-1, -1], [1, -1], [-1, 1], [1, 1]] {
                    let offset = [
                        position[0].wrapping_add(x * 1000),
                        position[1],
                        position[2].wrapping_add(z * 1000),
                    ];
                    let floor = terrain::level(
                        &ctx.mission.terrain,
                        ctx.cursor,
                        offset,
                        terrain::STEP_DOWN,
                    )?;
                    if valid(floor) {
                        found = Some([offset[0], floor, offset[2]]);
                        break;
                    }
                }
                let Some(found) = found else { continue };
                position = found;
            }
            self.stay(spawn.kind, position, actors, ctx)?;
        }
        Ok(())
    }
}
