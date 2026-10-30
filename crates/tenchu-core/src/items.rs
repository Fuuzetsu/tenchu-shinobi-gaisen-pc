use crate::math::Position;
mod bomb;
mod breath;
mod caltrops;
mod charm;
mod darts;
mod decoy;
mod disguise;
mod dog;
mod flight;
mod flute;
mod grapple;
mod kind;
mod lightning;
pub use kind::Kind;
mod medicine;
mod napalm;
mod pickup;
mod poison;
mod projectile;
mod rice;
mod rolling;
mod sight_shot;
mod sleep;
mod smoke;
mod speed;
mod sprite;
pub mod state;
mod teleport;
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
pub use flight::Flight;
pub use rolling::{Rolling, Status as RollingStatus};
pub use state::{Behavior, Pickup as PickupPhase, Used};

pub const CAPACITY: usize = 30;
pub const KINDS: usize = Kind::ALL.len();

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
    Flute,
    Resurrection,
    Rice,
    Darts,
    Speed,
    Charm,
    Decoy,
    Disguise,
    Dog,
    Lightning,
    Teleport,
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
    pub model: ModelId,
    pub kind: Kind,
    pub behavior: Behavior,
    pub collision: Contact,
    pub rolling: Rolling,
    pub count: u8,
    pub decoy: decoy::Decoy,
    pub lightning: lightning::Lightning,
    pub flight: Flight,
    pub trail: Option<crate::actor::trail::Trail>,
    pub rotation: [i16; 3],
    pub poison: poison::Poison,
    pub napalm: napalm::Napalm,
    pub breath: breath::Breath,
}

impl Item {
    pub fn owner(&self) -> Option<usize> {
        self.behavior.owner()
    }
    pub fn summoned_dog(&self) -> Option<usize> {
        match self.behavior {
            Behavior::Dog(Used {
                phase: state::Dog::Active { slave, .. },
                ..
            }) => Some(slave),
            _ => None,
        }
    }
    pub fn charm_remaining(&self) -> u16 {
        match self.behavior {
            Behavior::Charm(Used {
                phase: state::Charm::Protected { remaining },
                ..
            }) => remaining,
            _ => 0,
        }
    }

    pub fn processor(&self) -> Option<Processor> {
        self.behavior.processor()
    }
    pub fn source_mode(&self) -> u8 {
        self.behavior.source_mode()
    }
}

#[derive(Debug)]
pub(crate) struct Request {
    pub kind: Kind,
    pub start: Position,
    pub end: Position,
}

#[derive(Debug)]
pub struct Items {
    pub wall_item: Option<Kind>,
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
    decoy_count: u8,
    disguise: disguise::Disguise,
    pub(crate) dog_actor: Option<usize>,
}

fn target_distance(actor: &Actor, position: Position, radius: i32, ctx: &Context) -> Option<i32> {
    if actor.health <= 0
        || actor.motion.action == crate::ids::MOT_ACTION
        || actor.attribute & crate::ids::ATTR_SUSPEND != 0
    {
        return None;
    }
    let distance = ctx.math.vector_distance(Position::from_fn(|i| {
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

    fn draw_sight(&mut self, ctx: &mut Context) {
        let mut sprite = crate::hud::Sprite::new(ctx.mission.images[52], [0; 2]);
        sprite.blend = 2;
        self.sprites.push(sprite);
        ctx.drawing.bytes += 24;
    }

    pub fn new(scene: &mut Scene) -> Self {
        let meshes = std::array::from_fn(|_| scene.add(Some(0), Matrix::default()));
        let slots = (0..CAPACITY)
            .map(|_| Item {
                model: scene.add(Some(0), Matrix::default()),
                kind: Kind::Grapple,
                behavior: Behavior::Inactive,
                collision: Contact::default(),
                rolling: Rolling::default(),
                count: 0,
                decoy: decoy::Decoy::default(),
                lightning: lightning::Lightning::default(),
                flight: Flight::default(),
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
            wall_item: Some(Kind::Grapple),
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
            decoy_count: 0,
            disguise: disguise::Disguise::default(),
            dog_actor: None,
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
            if matches!(self.slots[self.cursor].behavior, Behavior::Inactive) {
                return Ok(self.cursor);
            }
        }
        self.dispose(self.cursor, actors, ctx)?;
        Ok(self.cursor)
    }

    fn dispose(&mut self, id: usize, actors: &mut Actors, ctx: &mut Context) -> Result<()> {
        match self.slots[id].behavior {
            Behavior::Inactive => return Ok(()),
            Behavior::Poison(used) => self.restore_poison_target(id, used.owner, actors),
            Behavior::Breath(used) => self.finish_breath(id, used.owner, actors, ctx)?,
            Behavior::Decoy(..) => self.finish_decoy(id, actors),
            Behavior::Disguise(used) => self.finish_disguise(id, used.owner, actors, ctx)?,
            Behavior::Dog(used) => self.finish_dog(used, actors, ctx)?,
            Behavior::SightShot { owner } => actors.get_mut(owner).aiming = false,
            Behavior::Shuriken { .. } | Behavior::Darts { .. } => self.slots[id].trail = None,
            Behavior::Charm(used) => actors.get_mut(used.owner).active_item = 0,
            _ => {}
        }
        let item = &mut self.slots[id];
        ctx.collision.remove(ctx.scene, item.model);
        item.behavior = Behavior::Inactive;
        Ok(())
    }

    fn initialize(
        &mut self,
        id: usize,
        behavior: Behavior,
        kind: Kind,
        position: Position,
        ctx: &mut Context,
    ) {
        let item = &mut self.slots[id];
        item.behavior = behavior;
        item.kind = kind;
        let model = &mut ctx.scene.models[item.model];
        model.parent = None;
        model.local.translation = position;
        self.update_coordinate(id, ctx);
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
            self.icons[item.kind.index()],
            sprite::Sprite {
                graphic: effects::Graphic::Sprite {
                    image: if item.kind.index() < 20 {
                        20 + item.kind.index() as u8
                    } else {
                        15
                    },
                    blend: 0,
                },
                scale: self.icon_scale[item.kind.index()],
                color: 128,
                rotation: 0,
            },
            ctx,
        )
    }

    fn update_contacts(&mut self, ctx: &mut Context) -> Result<()> {
        for item in &mut self.slots {
            if matches!(item.behavior, Behavior::Inactive) {
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
            self.process(id, actors, intelligence, player, clock, ctx)?;
        }
        Ok(())
    }

    fn process(
        &mut self,
        id: usize,
        actors: &mut Actors,
        intelligence: &mut crate::ai::Intelligence,
        player: usize,
        clock: i32,
        ctx: &mut Context,
    ) -> Result<()> {
        match self.slots[id].behavior {
            Behavior::Pickup { owner, phase } => {
                self.pickup(id, owner, phase, ctx, actors, intelligence.action_halt)?
            }
            Behavior::Shuriken { owner } => {
                self.shuriken(id, owner, ctx, actors, intelligence, clock)?
            }
            Behavior::Arrow(used) => self.arrow(id, used, ctx, actors, clock)?,
            Behavior::Gun(used) => self.gun(id, used, ctx, actors)?,
            Behavior::SightShot { owner } => self.sight_shot(id, owner, actors, ctx)?,
            Behavior::Medicine(used) => {
                self.medicine(id, used, actors, intelligence.action_halt, ctx)?
            }
            Behavior::Caltrops(used) => self.caltrops(id, used, actors, ctx)?,
            Behavior::Grapple { owner } => self.grapple(id, owner, actors, ctx)?,
            Behavior::Smoke(used) => self.smoke(id, used, actors, intelligence, clock, ctx)?,
            Behavior::Fire(used) => self.fire(id, used, actors, intelligence, ctx)?,
            Behavior::Mine(used) => self.mine(id, used, actors, intelligence, ctx)?,
            Behavior::Poison(used) => {
                self.poison(id, used, actors, intelligence, (player, clock), ctx)?
            }
            Behavior::Napalm(used) => self.napalm(id, used, actors, ctx)?,
            Behavior::Breath(used) => {
                self.breath(id, used, actors, intelligence.action_halt, clock, ctx)?
            }
            Behavior::Sleep(used) => self.sleep(id, used, actors, intelligence, ctx)?,
            Behavior::Flute(used) => self.flute(id, used, actors, intelligence, ctx)?,
            Behavior::Resurrection(used) => self.resurrection(id, used, actors, ctx)?,
            Behavior::Rice(used) => {
                self.rice(id, used, actors, intelligence.action_halt, player, ctx)?
            }
            Behavior::Darts { .. } => self.darts(id, actors, ctx)?,
            Behavior::Speed(used) => self.speed(id, used, actors, player, ctx)?,
            Behavior::Charm(used) => self.charm(id, used, actors, player, ctx)?,
            Behavior::Decoy(used) => self.decoy(id, used, actors, ctx)?,
            Behavior::Disguise(used) => self.disguise(id, used, actors, intelligence, ctx)?,
            Behavior::Dog(used) => self.dog(id, used, actors, clock, ctx)?,
            Behavior::Lightning(used) => self.lightning(id, used, actors, player, clock, ctx)?,
            Behavior::Teleport { owner } => self.teleport(id, owner, actors, player, ctx)?,
            Behavior::Inactive => {}
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
        self.spill_inventory(owner, actors, ctx)
    }

    fn spill_inventory(
        &mut self,
        owner: usize,
        actors: &mut Actors,
        ctx: &mut Context,
    ) -> Result<()> {
        for kind in Kind::ALL {
            for _ in 0..actors.get(owner).inventory[kind.index()] {
                let position = ctx.scene.world(actors.get(owner).parts[0])?.translation;
                let velocity = [
                    ctx.random.next15() % 200 - 100,
                    ctx.random.next15() % 100 - 200,
                    ctx.random.next15() % 200 - 100,
                ];
                self.drop(kind, Some(owner), position, velocity.into(), actors, ctx)?;
            }
            actors.get_mut(owner).inventory[kind.index()] = 0;
        }
        Ok(())
    }

    pub fn kind_for_contact(&self, scene: &Scene, slot: usize) -> Kind {
        self.slots
            .iter()
            .find(|item| scene.models[item.model].conflict == Some(slot))
            .map_or(Kind::Grapple, |item| item.kind)
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
                position.into(),
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
                        offset.into(),
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
            self.stay(Kind::try_from(spawn.kind)?, position.into(), actors, ctx)?;
        }
        Ok(())
    }
}
