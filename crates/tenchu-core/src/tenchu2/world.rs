use super::{
    actor::{Actor, State},
    ai::runtime::{Agent, System},
    collision::{BODY, Collision},
    inventory::Inventory,
    physics::Water,
    rig::{Rig, WeaponMode},
    terrain::{Cursor, Probe},
};
use crate::{
    Result,
    math::{Matrix, Position, Vector3},
    motion::Pose,
    random::Random,
    scene::Scene,
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::layout::Layout;

mod activation;
mod assets;
mod camera;
pub mod controls;
mod effects;
mod frame;
mod items;
mod mission;
mod ribbons;
mod update;
pub use assets::Assets;
pub use frame::Frame;
pub use update::Movement;

pub struct Options {
    pub hero: usize,
    pub difficulty: u8,
    pub seed: u32,
    pub inventory: Inventory,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Instance {
    pub rig: Rig,
    pub voice: u8,
    pub model_height: i16,
    pub feet: [Foot; 2],
    pub motion_changes: u16,
    pub translucent: bool,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct Foot {
    pub position: Position,
    pub floor: Option<i32>,
}

impl Default for Foot {
    fn default() -> Self {
        Self {
            position: Position::default(),
            floor: Some(0),
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Statistics {
    pub killed: u8,
    pub stealth_kills: u8,
    pub combat_kills: u8,
    pub innocents: u8,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct World {
    pub audio: super::audio::Audio,
    pub items: super::items::Items,
    pub effects: super::effects::Effects,
    pub camera: super::camera::Camera,
    pub terrain: tenchu_assets::tenchu2::collision::Collision,
    pub environment: super::environment::Environment,
    pub hidden_scene: bool,
    pub actors: Vec<Agent>,
    pub instances: Vec<Instance>,
    pub scene: Scene,
    pub collision: Collision,
    pub ai: System,
    pub random: Random,
    pub map: usize,
    pub view_distance: i32,
    pub sequence: u16,
    pub player: usize,
    pub difficulty: u8,
    pub tick: u32,
    pub script: u16,
    pub cursor: Cursor,
    pub probe: Probe,
    pub water: Water,
    pub terrain_options: super::terrain::Options,
    pub tracking: Option<usize>,
    pub travel: i32,
    pub effort: u16,
    pub controls: controls::Controls,
    pub statistics: Statistics,
    pub mission: super::mission::Mission,
}

impl World {
    pub fn new(layout: &Layout, options: Options, assets: &Assets) -> Result<Self> {
        if options.hero > 2 || !(1..=3).contains(&options.difficulty) {
            return Err("Invalid Tenchu 2 hero or difficulty".into());
        }
        if layout.map != assets.level.id
            || layout.sequence != assets.sequence
            || options.hero != assets.hero
        {
            return Err("Tenchu 2 layout or hero does not match its resources".into());
        }
        let cast = assets
            .resources
            .registry
            .cast(options.hero, layout.map)
            .ok_or("Missing Tenchu 2 cast")?;
        let sequence = *assets
            .campaign
            .sequences
            .get(layout.sequence)
            .ok_or("Missing Tenchu 2 mission descriptor")?;
        let mut world = Self {
            audio: super::audio::Audio::default(),
            items: super::items::Items::default(),
            effects: super::effects::Effects::default(),
            camera: super::camera::Camera::new(&assets.camera),
            terrain: assets.level.collision.clone(),
            environment: super::environment::Environment::default(),
            hidden_scene: false,
            actors: Vec::new(),
            instances: Vec::new(),
            scene: Scene::default(),
            collision: Collision::default(),
            ai: System::new((assets.draw_distance >> 4) * 16),
            random: Random::new(options.seed),
            map: layout.map,
            view_distance: (assets.draw_distance >> 4) * 16,
            sequence: layout.sequence as u16,
            player: 0,
            difficulty: options.difficulty,
            tick: 0,
            script: 0,
            cursor: Cursor::default(),
            probe: Probe::default(),
            water: Water::default(),
            terrain_options: super::terrain::Options::default(),
            tracking: None,
            travel: 0,
            effort: 0,
            controls: controls::Controls::default(),
            statistics: Statistics::default(),
            mission: super::mission::Mission::new(
                sequence,
                sequence.next.map(|id| assets.campaign.sequences[id].kind),
                0,
            )?,
        };
        world.create_actor(
            options.hero as i16,
            Position::from(layout.start),
            layout.facing,
            &options,
            assets,
        )?;
        world.actors[0].actor.inventory = options.inventory.clone();
        let stock = super::progress::Stock::default();
        world.controls.unlocked_items = std::array::from_fn(|i| stock.items[i].is_some());
        for entry in layout
            .actors
            .iter()
            .filter(|actor| cast.contains(&actor.kind))
        {
            let index = world.create_actor(entry.kind, Position::default(), 0, &options, assets)?;
            let agent = &mut world.actors[index];
            let actor = &mut agent.actor;
            if actor
                .motion
                .select(
                    0x700,
                    actor.kind,
                    &assets.resources.registry,
                    &assets.resources.clips,
                )?
                .is_some()
            {
                actor.motion.select(
                    0,
                    actor.kind,
                    &assets.resources.registry,
                    &assets.resources.clips,
                )?;
            } else {
                actor.motion.select(
                    0x100,
                    actor.kind,
                    &assets.resources.registry,
                    &assets.resources.clips,
                )?;
                actor.state = State::Ready;
                actor.flags |= 2;
                actor.set_weapon(
                    &mut world.instances[index].rig,
                    index,
                    &mut world.collision,
                    &assets.resources,
                    WeaponMode::Draw,
                )?;
            }
            actor.position = entry.position.into();
            actor.target =
                (!matches!(actor.kind, 0x6a..=0x6e) && actor.kind & 0xf0 != 0x10).then_some(0);
            agent.behavior.home = actor.position;
            if agent.behavior.assign(
                entry.behavior,
                layout.map,
                options.difficulty,
                &mut world.random,
            )? {
                actor.flags |= 0x10;
            }
            actor.rotation.y = entry.facing;
            agent.behavior.patrol = super::ai::Patrol::new(&entry.route, actor.position);
            agent.slot = entry.slot as i16;
            actor.inventory = Inventory::for_actor(actor.kind, &assets.loot, &mut world.random);
            world.publish(index, assets);
        }
        world.environment = super::environment::Environment::new(
            &assets.level,
            &mut world.scene,
            &assets.resources.math,
            &mut world.random,
        )?;
        world.update_camera(assets, true)?;
        world.initialize_activation(assets)?;
        world.mission.initialize(&world.actors, options.hero);
        Ok(world)
    }

    fn create_actor(
        &mut self,
        kind: i16,
        mut position: Position,
        facing: i16,
        options: &Options,
        assets: &Assets,
    ) -> Result<usize> {
        let resources = &assets.resources;
        let definition = resources
            .registry
            .actor(kind)
            .ok_or("Missing Tenchu 2 actor")?;
        let model = assets
            .models
            .get(&definition.model)
            .ok_or("Missing Tenchu 2 actor model")?;
        let mut actor = Actor::new(kind, &model.parts, resources)?;
        position.y = position.y.wrapping_sub(10);
        actor.position = position;
        actor.previous = position;
        actor.rotation.y = facing;
        let owner = self.actors.len();
        let mut rig = Rig::bind(&actor, &mut self.scene, &resources.math);
        if matches!(kind, 0x71 | 0x75 | 0xe5) {
            let slot = self.collision.insert(rig.joints[1])?;
            slot.owner = Some(owner);
            slot.flags = BODY;
            slot.size = Vector3::new(actor.width / 2, actor.height / 2, actor.width / 2);
            slot.offset = if kind == 0x71 {
                Vector3::new(0, -500, 0)
            } else {
                Vector3::new(0, 0, -1000)
            };
        }
        for weapon in resources.weapons.parts(definition.weapon) {
            let Some(name) = &weapon.model else { continue };
            let parts = assets
                .weapons
                .get(name)
                .ok_or("Missing Tenchu 2 weapon model")?;
            let parent = *rig
                .joints
                .get(weapon.bone)
                .ok_or("Missing Tenchu 2 weapon bone")?;
            let root = self.scene.add(Some(parent), Matrix::default());
            Pose::new(parts)?.bind(&mut self.scene, root, &resources.math);
            rig.weapons[weapon.slot] = Some(root);
        }
        actor.set_body_collision(rig.joints[0], owner, &mut self.collision, true)?;
        if kind == 2 || (kind == 0 && self.sequence > 93) {
            actor
                .motion
                .select(0x100, kind, &resources.registry, &resources.clips)?;
            actor.state = State::Ready;
            actor.flags |= 2;
            actor.set_weapon(
                &mut rig,
                owner,
                &mut self.collision,
                resources,
                WeaponMode::Draw,
            )?;
        }
        let sight_range = ((assets.draw_distance >> 4) as i16)
            .wrapping_sub(definition.sight_penalty.wrapping_mul(125))
            .wrapping_mul(16);
        let mut agent = Agent::new(actor, 0, sight_range, &[]);
        agent
            .behavior
            .assign(1, self.map, self.difficulty, &mut self.random)?;
        self.actors.push(agent);
        let voice = resources
            .registry
            .cast(options.hero, self.map)
            .and_then(|cast| cast.iter().position(|&id| id == kind))
            .map_or(4, |slot| slot as u8 + 5);
        self.instances.push(Instance {
            rig,
            voice,
            model_height: super::actor::model_bounds(&model.parts)?[1],
            feet: [Foot::default(); 2],
            motion_changes: 0,
            translucent: false,
        });
        Ok(owner)
    }

    pub fn publish(&mut self, index: usize, assets: &Assets) {
        let actor = &self.actors[index].actor;
        let rig = &self.instances[index].rig;
        self.scene.update(
            rig.root,
            Matrix {
                rotation: assets.resources.math.rotate_yxz(actor.rotation.to_array()),
                translation: actor.position,
            },
        );
        actor.motion.pose.publish(
            &mut self.scene,
            &rig.joints,
            &assets.resources.math,
            u32::MAX,
            rig.joints.len(),
        );
    }
}
