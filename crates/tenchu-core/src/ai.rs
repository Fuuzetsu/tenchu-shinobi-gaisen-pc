mod combat;
mod idle;
mod melee;
mod reinforcement;
mod search;
mod transition;

#[cfg(test)]
mod tests;

use crate::{
    Result,
    actor::{
        Actor, Context,
        sight::{Detection, Sight},
    },
    ids::*,
    input::*,
    roster::Roster,
    scene::ModelId,
    terrain,
};
use tenchu_assets::executable::Command;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum Difficulty {
    Easy,
    #[default]
    Normal,
    Hard,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Brain {
    pub chase: [i32; 2],
    pub mode: u8,
    pub flag: u8,
    pub count: u8,
    pub timer: u8,
}

#[derive(Debug)]
pub struct Intelligence {
    pub difficulty: Difficulty,
    pub attack_clock: i32,
    pub alert: i32,
    pub strain: i32,
    pub player_strain: i32,
    pub found: i16,
    pub death_index: u16,
    pub sight: Sight,
    pub detection: Detection,
    pub probe: [i32; 2],
    pub probe_attribute: [u16; 2],
    pub action_halt: i16,
    pub voices: Vec<(usize, i16)>,
    pub population_delta: [i16; 2],
}

impl Default for Intelligence {
    fn default() -> Self {
        Self {
            difficulty: Difficulty::Normal,
            attack_clock: 0,
            alert: 0,
            strain: 0,
            player_strain: 0,
            found: 0,
            death_index: 0,
            sight: Sight::default(),
            detection: Detection::None,
            probe: [0; 2],
            probe_attribute: [0; 2],
            action_halt: 0,
            voices: Vec::new(),
            population_delta: [0; 2],
        }
    }
}

#[derive(Clone, Copy)]
pub struct Player {
    pub root: ModelId,
    pub position: [i32; 3],
    pub status: i16,
    pub motion: i16,
    pub height: i16,
    pub active_item: i16,
}

impl Player {
    pub fn new(actor: &Actor, ctx: &Context) -> Self {
        Self {
            root: actor.root_model,
            position: actor.position,
            status: actor.status,
            motion: actor.motion.action,
            height: ctx.mission.characters[actor.definition].height,
            active_item: actor.active_item,
        }
    }
}

pub struct Controller<'a, 'b> {
    pub actor: &'a mut Actor,
    pub player: Player,
    pub roster: &'a mut Roster,
    pub state: &'a mut Intelligence,
    pub clock: i32,
    pub input: u16,
    pub commands: &'a [Command],
    pub ctx: &'a mut Context<'b>,
    attribute: u16,
    replaced: Option<Actor>,
}

impl<'a, 'b> Controller<'a, 'b> {
    pub fn new(
        actor: &'a mut Actor,
        player: Player,
        roster: &'a mut Roster,
        state: &'a mut Intelligence,
        clock: i32,
        input: u16,
        commands: &'a [Command],
        ctx: &'a mut Context<'b>,
    ) -> Self {
        let attribute = actor.attribute;
        Self {
            actor,
            player,
            roster,
            state,
            clock,
            input,
            commands,
            ctx,
            attribute,
            replaced: None,
        }
    }

    fn player_target(&self) -> bool {
        self.actor.target == Some(self.player.root)
    }

    fn target(&self) -> Option<[i32; 3]> {
        self.actor.target.map(|id| {
            if id == self.actor.root_model {
                self.actor.position
            } else {
                self.ctx.scene.models[id].local.translation
            }
        })
    }

    fn degree(&self) -> i32 {
        i32::from(self.state.sight.direction)
    }
    fn distance(&self) -> i32 {
        self.state.sight.distance
    }
    fn turn(&self) -> i32 {
        i32::from(self.ctx.mission.characters[self.actor.definition].turn)
    }
    fn width(&self) -> i16 {
        self.ctx.mission.characters[self.actor.definition].width
    }
    fn class(&self) -> usize {
        (self.ctx.mission.characters[self.actor.definition].weapon >> 4) as usize
    }
    fn engage(&self) -> i32 {
        3 - self.state.difficulty as i32
    }
    fn random(&mut self) -> i32 {
        i32::from(self.ctx.random.next15())
    }

    fn voice(&mut self, id: i16) {
        self.state.voices.push((self.actor.id, id));
    }

    fn alert(&mut self) {
        self.state.alert = if self.state.difficulty == Difficulty::Hard {
            600
        } else {
            300
        };
    }

    fn found(&mut self) {
        if self.actor.kind < 0x80 && self.player_target() {
            self.state.found = self.state.found.wrapping_add(1);
        }
    }

    fn motion(&mut self, id: i16) -> bool {
        self.actor.enter(self.ctx, id, true)
    }

    fn command(&mut self, id: u16) -> u16 {
        self.actor.pad.issue(self.commands, id)
    }

    fn hold(&mut self, pad: u16, frames: i32) {
        self.actor.pad.hold_counter = (u32::from(pad) << 16) | frames as u32;
    }

    fn length(&self, delta: &[i32]) -> i32 {
        self.ctx.math.square_root(
            delta
                .iter()
                .fold(0i32, |s, &v| s.wrapping_add(v.wrapping_mul(v))),
        )
    }

    fn level(&mut self, position: [i32; 3], mode: u16) -> Result<i32> {
        terrain::level(&self.ctx.mission.terrain, self.ctx.cursor, position, mode)
    }

    fn goto(&mut self, x: i32, z: i32) -> Result<u16> {
        let degree = if x != 0 || z != 0 {
            i32::from(self.ctx.math.direction(x, z, self.actor.rotation[1])?)
        } else {
            self.degree()
        };
        let mut pad = if degree > self.turn() {
            RIGHT
        } else if degree < -self.turn() {
            LEFT
        } else {
            0
        };
        if degree.abs() < 500 {
            pad |= UP;
        }
        if self.attribute & ATTR_PHASE == 0 && self.attribute & ATTR_WALL != 0 {
            let cached = self.state.probe[0];
            if cached != terrain::NONE {
                return Ok(pad);
            }
            if self.actor.pad.hold_counter == 0 {
                let v = self
                    .ctx
                    .math
                    .velocity(self.actor.rotation[1], 0, self.width());
                let [x, y, z] = self.actor.position;
                let a = self.level(
                    [
                        x.wrapping_add(i32::from(v[0])),
                        y.wrapping_sub(500),
                        z.wrapping_add(i32::from(v[2])),
                    ],
                    26,
                )?;
                let b = self.level(
                    [
                        x.wrapping_sub(i32::from(v[0])),
                        y.wrapping_sub(500),
                        z.wrapping_sub(i32::from(v[2])),
                    ],
                    26,
                )?;
                if pad & RIGHT != 0 && a != cached {
                    self.hold(RIGHT, 30);
                } else if pad & LEFT != 0 && b != terrain::NONE {
                    self.hold(LEFT, 30);
                } else {
                    pad = 0;
                }
            }
        }
        Ok(pad)
    }

    fn chase(&mut self, length: i32) -> Result<u16> {
        let Some(target) = self.target() else {
            return Ok(0);
        };
        let x = target[0]
            .wrapping_add(self.actor.brain.chase[0])
            .wrapping_sub(self.actor.position[0]);
        let z = target[2]
            .wrapping_add(self.actor.brain.chase[1])
            .wrapping_sub(self.actor.position[2]);
        if (x.wrapping_abs() < 500 && z.wrapping_abs() < 500)
            || self.attribute & ATTR_WALL != 0
            || self.distance() < 1000
        {
            return Ok(0);
        }
        if self.attribute & (ATTR_HIT | ATTR_PUSH) != 0 || self.actor.brain.chase == [0; 2] {
            let direction = self.random() as i16;
            let [sin, cos] = self.ctx.math.pair(direction);
            self.actor.brain.chase = [
                cos.wrapping_mul(length) >> 12,
                sin.wrapping_mul(length) >> 12,
            ];
        }
        self.goto(x, z)
    }

    fn player_input(&self) -> u16 {
        let mut pad = self.input;
        if self.actor.map.attribute & 0x200 != 0 && matches!(self.actor.status, 7 | 9) {
            pad &= !0xf000;
        }
        if pad & R1 != 0 {
            pad = (pad & !R1) | CIRCLE;
        }
        pad
    }

    fn program(&mut self, phase: usize) -> Result<u16> {
        let index = self.actor.think[phase];
        match (phase, index) {
            (_, 0 | 2) => Ok(0),
            (_, 1) => Ok(self.player_input()),
            (0, 3) => self.trace(),
            (0, 4) => Ok(self.watch()),
            (0, 5) => self.wander(),
            (0, 6) => self.ninja(),
            (0, 7) => self.sleep(),
            (0, 8) => self.hunt(),
            (0, 9) => self.follow(),
            (1, 3) => Ok(self.goto(0, 0)? & (LEFT | RIGHT)),
            (1, 4) => self.contact(),
            (1, 5) => self.alarm(),
            (2, 3) => self.call_aid(),
            (2, 4) => self.attack_chase(),
            (2, 5) => self.attack_point(),
            (2, 6) => self.escape(),
            (2, 7) => self.attack_area(),
            (2, 8) => self.hit_away(),
            (2, 9) => self.first_attack(),
            (3, 3) => self.abandon(),
            (3, 4 | 5) => self.investigate(index == 5),
            _ => Err(format!("AI program {phase}/{index} is not implemented yet")),
        }
    }
}
