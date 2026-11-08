use super::{mission::Population, world::Statistics};
use crate::{Result, random::Random};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::campaign::Reward;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Record {
    pub rank: u8,
    pub stealth_kills: u8,
    pub score: i16,
    pub time_tenths: i32,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Summary {
    pub population: Population,
    pub kills: Statistics,
    pub detections: u8,
    pub effort: i16,
    pub time_tenths: i32,
}

impl Summary {
    pub fn rank(&mut self, sequence: usize) -> Record {
        if self.population.bosses < self.kills.combat_kills
            && self.detections == 0
            && sequence != 207
        {
            self.detections = (self.kills.combat_kills >> 1) + 1;
        }
        let score = (i32::from(self.kills.stealth_kills) * 20
            + i32::from(self.kills.combat_kills) * 5
            + 400
            - i32::from(self.detections) * 30
            - i32::from(self.kills.innocents) * 150) as i16;
        let mut rank = (score / 100).clamp(0, 6) as u8;
        if sequence == 207 {
            self.population.guards = self.population.guards.wrapping_sub(2);
        }
        if (self.detections == 0
            && self.kills.innocents == 0
            && self.population.guards <= self.kills.stealth_kills)
            || (sequence == 211 && self.kills.innocents == 0 && self.kills.combat_kills == 0)
        {
            rank = 6;
        }
        Record {
            rank,
            stealth_kills: self.kills.stealth_kills,
            score,
            time_tenths: self.time_tenths,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Stock {
    pub items: [Option<u8>; 21],
}

impl Default for Stock {
    fn default() -> Self {
        let mut items = [None; 21];
        for (item, count) in [(1, 3), (4, 10), (5, 10), (12, 5), (15, 0)] {
            items[item] = Some(count);
        }
        Self { items }
    }
}

impl Stock {
    fn award(&mut self, rank: u8, rewards: &[Reward; 7], random: &mut Random) {
        if rank < 4 {
            let mut remaining = rank + 3;
            while remaining > 0 {
                let item = (random.next15() % 21 + 1) as usize;
                let Some(Some(count)) = self.items.get_mut(item) else {
                    continue;
                };
                if rewards.iter().any(|reward| reward.item == item) && random.next15() % 3 != 0 {
                    continue;
                }
                if *count < 99 {
                    *count += 1;
                }
                remaining -= 1;
            }
        } else {
            for (item, stock) in self.items.iter_mut().enumerate() {
                let Some(count) = stock else { continue };
                let amount = if rewards.iter().any(|reward| reward.item == item) {
                    1
                } else {
                    rank - 3
                };
                if *count <= 99 - amount {
                    *count += amount;
                }
            }
        }
    }

    fn return_items(&mut self, remaining: &[u8; 21]) {
        for (stock, &amount) in self.items.iter_mut().zip(remaining) {
            if let Some(count) = stock {
                *count = (u16::from(*count) + u16::from(amount)).min(99) as u8;
            }
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Progress {
    pub stock: [Stock; 3],
    pub records: [[Record; 11]; 3],
    pub tatsumaru_unlocked: bool,
    pub all_grand_master: bool,
}

impl Default for Progress {
    fn default() -> Self {
        let mut records = [[Record::default(); 11]; 3];
        for hero in &mut records {
            hero[0].time_tenths = 59999;
        }
        Self {
            stock: std::array::from_fn(|_| Stock::default()),
            records,
            tatsumaru_unlocked: false,
            all_grand_master: false,
        }
    }
}

pub struct Completion {
    pub hero: usize,
    pub mission: usize,
    pub sequence: usize,
    pub summary: Summary,
    pub remaining: [u8; 21],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Results {
    pub summary: Summary,
    pub record: Record,
    pub best: Record,
    pub new_best: bool,
    pub unlocked_item: Option<usize>,
}

impl Progress {
    pub fn complete(
        &mut self,
        completion: Completion,
        rewards: &[Reward; 7],
        random: &mut Random,
    ) -> Result<Results> {
        let Completion {
            hero,
            mission,
            sequence,
            mut summary,
            remaining,
        } = completion;
        if hero >= 3 || mission >= 11 {
            return Err("Invalid Tenchu 2 result slot".into());
        }
        let record = summary.rank(sequence);
        if record.rank < 4 && self.stock[hero].items[1..].iter().all(Option::is_none) {
            return Err("Tenchu 2 rewards require at least one unlocked supply".into());
        }
        let best = &mut self.records[hero][mission];
        let new_best = best.time_tenths == 0
            || record.rank > best.rank
            || (record.rank == best.rank && record.time_tenths < best.time_tenths);
        if new_best {
            *best = record;
        }
        let best = *best;
        if let Some(next) = self
            .records
            .as_flattened_mut()
            .get_mut(hero * 11 + mission + 1)
            && next.time_tenths == 0
        {
            next.time_tenths = 59999;
        }
        self.stock[hero].award(record.rank, rewards, random);
        self.stock[hero].return_items(&remaining);
        let masters = self
            .records
            .iter()
            .flatten()
            .filter(|r| r.rank == 6)
            .count();
        let reward = rewards
            .iter()
            .find(|r| usize::from(r.grand_masters) == masters);
        let mut unlocked_item = None;
        if let Some(reward) = reward {
            for stock in &mut self.stock {
                if stock.items[reward.item].is_none() {
                    stock.items[reward.item] = Some(1);
                    unlocked_item = Some(reward.item);
                }
            }
        } else if masters == 29 {
            self.all_grand_master = true;
        }
        if hero != 2
            && self.records[..2]
                .iter()
                .all(|r| !matches!(r[10].time_tenths, 0 | 59999))
        {
            self.tatsumaru_unlocked = true;
        }
        Ok(Results {
            summary,
            record,
            best,
            new_best,
            unlocked_item,
        })
    }
}
