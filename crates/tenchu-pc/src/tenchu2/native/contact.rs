use std::collections::VecDeque;
use tenchu_core::{
    math::Position,
    tenchu2::{
        actor::State,
        effects::{Contact, Frame},
        world::World,
    },
};
pub(super) mod blood;

#[derive(Clone, Copy)]
pub struct Corpse {
    pub tick: u64,
    pub position: Position,
    pub source_pool: bool,
}

// Keep source contacts between rendered frames, including fast-forward updates.
#[derive(Default)]
pub struct History {
    frames: VecDeque<(u64, Vec<Contact>)>,
    pub blood: blood::History,
    pub player_corpse: Option<Corpse>,
    pub wounds: Vec<(usize, Position)>,
}

impl History {
    pub fn record(&mut self, tick: u64, frame: &Frame) {
        if self
            .frames
            .back()
            .is_some_and(|(previous, _)| previous + 1 != tick)
        {
            self.frames.clear();
            self.blood.reset();
            self.player_corpse = None;
        }
        if self.frames.len() == 30 {
            self.frames.pop_front();
        }
        self.frames.push_back((tick, frame.contacts.clone()));
        self.blood.record(&frame.quads);
    }

    pub fn record_player(&mut self, world: &World) {
        let actor = &world.actors[world.player].actor;
        if actor.health > 0
            || actor.state != State::Dead
            || actor.motion.loops != i16::MIN
            || actor.velocity.y != 0
            || matches!(actor.motion.action, 0xb08 | 0xb0a)
            || matches!(actor.terrain.material & 0xf00, 0x200 | 0x300)
        {
            self.player_corpse = None;
            return;
        }
        if self.player_corpse.is_some() {
            return;
        }
        let Some(&joint) = world.instances[world.player].rig.joints.get(1) else {
            return;
        };
        let Ok(matrix) = world.scene.clone().world(joint) else {
            return;
        };
        self.player_corpse = Some(Corpse {
            tick: u64::from(world.tick),
            position: Position::new(
                matrix.translation.x,
                actor.terrain.floor.unwrap_or(actor.position.y),
                matrix.translation.z,
            ),
            source_pool: (0xb01..0xb06).contains(&actor.motion.action)
                && !matches!(world.map, 0 | 12),
        });
    }

    pub fn events(&self, tick: u64) -> &[Contact] {
        self.frames
            .iter()
            .find(|(t, _)| *t == tick)
            .map_or(&[], |(_, bursts)| bursts)
    }
}
