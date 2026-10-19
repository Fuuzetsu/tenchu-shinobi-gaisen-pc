mod pickup;
mod rolling;
#[cfg(test)]
mod tests;

use crate::{
    Result,
    actor::{Actor, Context},
    effects,
    math::Matrix,
    roster::Roster,
    scene::{ModelId, Scene},
    terrain, visibility,
};
pub use rolling::Rolling;

pub const CAPACITY: usize = 30;
pub const KINDS: usize = 25;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Processor {
    Pickup,
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
}

#[derive(Debug)]
pub struct Items {
    pub slots: Vec<Item>,
    pub drawings: Vec<effects::Drawing>,
    cursor: usize,
    icons: [ModelId; KINDS + 1],
}

impl Items {
    pub fn new(scene: &mut Scene) -> Self {
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
            })
            .collect();
        let icons = std::array::from_fn(|_| {
            let id = scene.add(Some(0), Matrix::default());
            scene.models[id].attribute = 0x1c;
            id
        });
        Self {
            slots,
            drawings: Vec::new(),
            cursor: 0,
            icons,
        }
    }

    fn allocate(&mut self, ctx: &mut Context) -> Result<usize> {
        for _ in 0..CAPACITY - 1 {
            self.cursor = (self.cursor + 1) % CAPACITY;
            if self.slots[self.cursor].processor.is_none() {
                return Ok(self.cursor);
            }
        }
        self.dispose(self.cursor, ctx)?;
        Ok(self.cursor)
    }

    fn dispose(&mut self, id: usize, ctx: &mut Context) -> Result<()> {
        let item = &mut self.slots[id];
        match item.processor {
            Some(Processor::Pickup) => item.mode = 0,
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
        let icon = self.icons[item.kind];
        let source = ctx.scene.models[item.model].clone();
        let target = &mut ctx.scene.models[icon];
        target.local = source.local;
        target.cached = source.cached;
        target.parent = source.parent;
        target.flags = source.flags;
        let matrix = ctx.camera.view.compose(&ctx.scene.world(icon)?);
        let Some(clip) = visibility::clip(&matrix, ctx.scene.models[icon].attribute, [0; 3]) else {
            return Ok(());
        };
        let depth = clip.depth as i16 - 5;
        if depth < 1 {
            return Ok(());
        }
        let scale = ((0x3000 >> 2) * 300 / i32::from(depth)) as i16;
        self.drawings.push(effects::Drawing {
            graphic: effects::Graphic::Sprite {
                image: if item.kind < 20 {
                    20 + item.kind as u8
                } else {
                    15
                },
                blend: 0,
            },
            xy: visibility::project(&matrix, [0; 3]).0,
            scale: [i32::from(scale); 2],
            color: [128; 3],
            rotation: 0,
            depth,
        });
        ctx.drawing.bytes += if scale == 4096 { 24 } else { 40 };
        Ok(())
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
        halt: i16,
        clock: i32,
    ) -> Result<()> {
        self.drawings.clear();
        if clock % 10 == 0 {
            self.update_contacts(ctx)?;
        }
        for id in 0..CAPACITY {
            match self.slots[id].processor {
                Some(Processor::Pickup) => self.pickup(id, ctx, roster, halt)?,
                None => {}
            }
        }
        Ok(())
    }

    pub(crate) fn drop_inventory(&mut self, actor: &mut Actor, ctx: &mut Context) -> Result<()> {
        for kind in 0..KINDS {
            for _ in 0..actor.inventory[kind] {
                let position = ctx.scene.world(actor.parts[0])?.translation;
                let velocity = [
                    ctx.random.next15() % 200 - 100,
                    ctx.random.next15() % 100 - 200,
                    ctx.random.next15() % 200 - 100,
                ];
                self.drop(kind, Some(actor.id), position, velocity, ctx)?;
            }
            actor.inventory[kind] = 0;
        }
        Ok(())
    }

    pub fn kind_for_contact(&self, scene: &Scene, slot: usize) -> usize {
        self.slots
            .iter()
            .find(|item| scene.models[item.model].conflict == Some(slot))
            .map_or(0, |item| item.kind)
    }

    pub(crate) fn restore_layout(&mut self, ctx: &mut Context) -> Result<()> {
        for id in 0..CAPACITY {
            self.dispose(id, ctx)?;
        }
        for spawn in &ctx.mission.pickups {
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
            self.stay(spawn.kind, position, ctx)?;
        }
        Ok(())
    }
}
