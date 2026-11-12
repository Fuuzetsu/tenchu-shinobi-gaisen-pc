use super::{
    actor::State,
    inventory::Inventory,
    mission::{Phase, Population},
    progress::{Completion, Progress, Results, Summary},
    rig::WeaponMode,
    world::{Assets, World},
};
use crate::tenchu2::actor::flags::POISONED;
use crate::{Result, random::Random};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::campaign::{Campaign, Kind, Objective, Sequence};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Carry {
    pub health: u8,
    pub breath: u8,
    pub drawn: bool,
    pub poisoned: bool,
    pub alternate: bool,
    pub inventory: [u8; 21],
    #[serde(default)]
    pub bleeding: Option<crate::game::bleeding::State>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Destination {
    Sequence(usize),
    Failed,
    Complete,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Run {
    pub hero: usize,
    pub difficulty: u8,
    pub sequence: usize,
    pub mission: usize,
    pub random: Random,
    pub progress: Progress,
    pub summary: Summary,
    pub carry: Carry,
    pub played_seconds: i32,
    pub results: Option<Results>,
    previous_area: Option<u16>,
}

impl Run {
    pub fn new(
        campaign: &Campaign,
        hero: usize,
        sequence: usize,
        difficulty: u8,
        seed: u32,
    ) -> Result<Self> {
        if !(1..=3).contains(&difficulty) {
            return Err("Invalid Tenchu 2 difficulty".into());
        }
        let mission = campaign
            .mission(hero, sequence)
            .ok_or("Sequence is not in this Tenchu 2 hero's campaign")?;
        Ok(Self {
            hero,
            difficulty,
            sequence,
            mission,
            random: Random::new(seed),
            progress: Progress::default(),
            summary: Summary::default(),
            carry: Carry::default(),
            played_seconds: 0,
            results: None,
            previous_area: None,
        })
    }

    pub fn descriptor<'a>(&self, campaign: &'a Campaign) -> Result<&'a Sequence> {
        campaign
            .sequences
            .get(self.sequence)
            .ok_or_else(|| "Missing Tenchu 2 campaign sequence".into())
    }

    pub fn prepare_area(&mut self, campaign: &Campaign) -> Result<bool> {
        let sequence = *self.descriptor(campaign)?;
        let Kind::Gameplay(objective) = sequence.kind else {
            return Err("Not a Tenchu 2 gameplay sequence".into());
        };
        if !matches!(
            objective,
            Objective::Weaken { .. } | Objective::Defeat { .. }
        ) && self.previous_area != Some(sequence.resource)
            && (campaign.first_gameplay(self.hero, self.mission) == Some(self.sequence)
                || sequence.resource == 1)
        {
            self.summary = Summary::default();
            self.carry = Carry {
                inventory: self.carry.inventory,
                ..Default::default()
            };
            self.carry.inventory[0] = 0;
        }
        self.previous_area = Some(sequence.resource);
        Ok(self.carry.inventory[0] == 0)
    }

    pub fn restore_area(
        &self,
        world: &mut World,
        assets: &Assets,
        allow_sheathed: bool,
    ) -> Result<()> {
        if self.sequence != assets.sequence || self.hero != assets.hero {
            return Err("Tenchu 2 campaign does not match the loaded area".into());
        }
        world.random.calls += self.random.calls;
        world.statistics.clone_from(&self.summary.kills);
        world.ai.detections = self.summary.detections;
        world.effort = self.summary.effort as u16;
        let before = self.summary.population;
        let population = &mut world.mission.population;
        *population = Population {
            bosses: population.bosses.wrapping_add(before.bosses),
            guards: population.guards.wrapping_add(before.guards),
            civilians: population.civilians.wrapping_add(before.civilians),
        };
        world.controls.unlocked_items =
            std::array::from_fn(|i| self.progress.stock[self.hero].items[i].is_some());
        world.controls.armor = self.carry.alternate;
        let player = world.player;
        let actor = &mut world.actors[player].actor;
        actor.inventory = Inventory::from_amounts(self.carry.inventory);
        actor.breath = self.carry.breath;
        actor.bleeding.clone_from(&self.carry.bleeding);
        if self.carry.health != 0 {
            actor.health = i16::from(self.carry.health);
            if self.carry.poisoned {
                actor.flags |= POISONED;
            }
            if self.carry.drawn || !allow_sheathed {
                actor.state = State::Ready;
                actor.motion.select(
                    0x100,
                    actor.kind,
                    &assets.resources.registry,
                    &assets.resources.clips,
                )?;
                actor.set_weapon(
                    &mut world.instances[player].rig,
                    player,
                    &mut world.collision,
                    &assets.resources,
                    WeaponMode::Draw,
                )?;
            }
        }
        world.publish(player, assets);
        Ok(())
    }

    pub fn finish_area(&mut self, world: &World, campaign: &Campaign) -> Result<Destination> {
        if usize::from(world.sequence) != self.sequence {
            return Err("Finishing a different Tenchu 2 sequence".into());
        }
        let Phase::Finished {
            outcome,
            finished_at,
        } = world.mission.phase
        else {
            return Err("Tenchu 2 area has not finished".into());
        };
        let actor = &world.actors[world.player].actor;
        self.carry.health = actor.health as u8;
        self.carry.breath = actor.breath;
        self.carry.drawn = actor.weapon_drawn();
        self.carry.poisoned = actor.flags & POISONED != 0;
        self.carry.inventory = actor.inventory.amounts();
        self.carry.bleeding.clone_from(&actor.bleeding);
        let ticks = finished_at as i32;
        self.played_seconds = self.played_seconds.wrapping_add(ticks / 30);
        self.summary = Summary {
            population: world.mission.population,
            kills: world.statistics.clone(),
            detections: world.ai.detections,
            effort: world.effort as i16,
            time_tenths: self
                .summary
                .time_tenths
                .wrapping_add((ticks / 30).wrapping_mul(10))
                .wrapping_add(ticks % 30 / 3),
        };
        self.random = world.random;
        let sequence = self.descriptor(campaign)?;
        if outcome.success() {
            self.go(campaign, sequence.next, Destination::Complete)
        } else {
            self.go(campaign, sequence.failure, Destination::Failed)
        }
    }

    pub fn enter_results(&mut self, campaign: &Campaign) -> Result<&Results> {
        if self.descriptor(campaign)?.kind != Kind::Results {
            return Err("Not a Tenchu 2 results sequence".into());
        }
        if self.results.is_none() {
            self.carry.drawn = false;
            self.carry.poisoned = false;
            self.carry.bleeding = None;
            self.carry.alternate = false;
            let results = self.progress.complete(
                Completion {
                    hero: self.hero,
                    mission: self.mission,
                    sequence: self.sequence,
                    summary: self.summary.clone(),
                    remaining: self.carry.inventory,
                },
                &campaign.rewards,
                &mut self.random,
            )?;
            self.summary.clone_from(&results.summary);
            self.results = Some(results);
        }
        Ok(self.results.as_ref().unwrap())
    }

    pub fn advance(&mut self, campaign: &Campaign, skip_story: bool) -> Result<Destination> {
        let sequence = self.descriptor(campaign)?;
        let terminal = if sequence.kind == Kind::Event {
            Destination::Failed
        } else {
            Destination::Complete
        };
        let mut next = sequence.next;
        if skip_story {
            let mut visited = vec![false; campaign.sequences.len()];
            while let Some(index) = next {
                let row = &campaign.sequences[index];
                if !matches!(row.kind, Kind::Event | Kind::Movie) {
                    break;
                }
                if visited[index] {
                    return Err("Cycle in Tenchu 2 story sequence links".into());
                }
                visited[index] = true;
                next = row.next;
            }
        }
        self.go(campaign, next, terminal)
    }

    pub fn retry(&mut self, campaign: &Campaign) -> Result<Destination> {
        let first = campaign
            .first_gameplay(self.hero, self.mission)
            .ok_or("Missing Tenchu 2 retry area")?;
        self.previous_area = None;
        self.carry.bleeding = None;
        self.go(campaign, Some(first), Destination::Failed)
    }

    fn go(
        &mut self,
        campaign: &Campaign,
        next: Option<usize>,
        terminal: Destination,
    ) -> Result<Destination> {
        let Some(next) = next else {
            return Ok(terminal);
        };
        if next >= campaign.sequences.len() {
            return Err("Invalid Tenchu 2 campaign destination".into());
        }
        self.sequence = next;
        self.results = None;
        if let Some(mission) = campaign.starts[self.hero]
            .iter()
            .position(|&s| s == Some(next))
        {
            if self.mission != mission {
                self.carry.bleeding = None;
            }
            self.mission = mission;
        }
        Ok(Destination::Sequence(next))
    }
}
