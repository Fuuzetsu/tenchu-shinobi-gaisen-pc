mod inventory;
pub mod save;
#[cfg(test)]
mod tests;

use crate::{
    game::score::{CLOCK_MAX, Record, Score},
    random::Random,
};
pub use inventory::Loadout;
use tenchu_assets::{game::Stage, script::Language, shop::ITEMS};

pub const LOCKED: u8 = 254;
pub const INFINITE: u8 = 255;
pub const STAGES: usize = 11;
pub const LAYOUTS: usize = 3;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct TrainingRecord {
    pub character: u8,
    pub grade: u8,
    pub clock: i32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Progress {
    pub stock: [[u8; ITEMS]; 2],
    pub records: [[[Record; LAYOUTS]; STAGES]; 2],
    pub highest: [i16; 2],
    pub flags: u32,
    pub training: [TrainingRecord; 5],
}

impl Default for Progress {
    fn default() -> Self {
        let mut stock = [LOCKED; ITEMS];
        for (item, count) in [
            (0, INFINITE),
            (1, 6),
            (2, 6),
            (3, 2),
            (4, 1),
            (5, 1),
            (7, 3),
            (8, 5),
        ] {
            stock[item] = count;
        }
        Self {
            stock: [stock; 2],
            records: [[[Record::default(); LAYOUTS]; STAGES]; 2],
            highest: [0; 2],
            flags: 0,
            training: [TrainingRecord::default(); 5],
        }
    }
}

pub struct Completion<'a> {
    pub stage: &'a Stage,
    pub character: usize,
    pub layout: usize,
    pub language: Language,
    pub record: Record,
    pub remaining: [u8; ITEMS],
}

#[derive(Clone, Debug)]
pub struct Results {
    pub record: Record,
    pub score: Score,
    pub best: Record,
    pub new_best: bool,
    pub training_place: Option<usize>,
}

impl Progress {
    pub fn complete(&mut self, completion: Completion, random: &mut Random) -> Results {
        let Completion {
            stage,
            character,
            layout,
            language,
            record,
            remaining,
        } = completion;
        self.flags |= 1u32.wrapping_shl(stage.order.wrapping_sub(1) as u32);
        if stage.id == 7 && language == Language::English {
            self.flags |= 1 << 10;
        }
        let score = record.score(stage.id);
        let best = &mut self.records[character][stage.id][layout];
        let previous = best.score(stage.id);
        let new_best = score.total > previous.total
            || (score.total == previous.total && record.clock < best.clock);
        if new_best {
            *best = record;
        }
        let mut results = Results {
            record,
            score,
            best: *best,
            new_best,
            training_place: None,
        };
        if stage.order == 0 {
            results.training_place = self.rank_training(character, record.clock, score.grade);
            if score.grade == 4 && self.stock[character][stage.reward] == LOCKED {
                self.stock[character][stage.reward] = 1;
            }
            return results;
        }
        let stock = &mut self.stock[character];
        award(stock, score.grade, stage.reward, random);
        self.highest[character] = self.highest[character].max(stage.order as i16);
        return_items(stock, &remaining);
        results
    }

    pub fn next_layout(&self, character: usize, stage: usize) -> Option<usize> {
        self.records[character][stage]
            .iter()
            .position(|record| !record.played())
    }

    fn rank_training(&mut self, character: usize, clock: i32, grade: i16) -> Option<usize> {
        for record in &mut self.training {
            if record.clock == 0 {
                *record = TrainingRecord {
                    clock: CLOCK_MAX,
                    ..TrainingRecord::default()
                };
            }
        }
        let place = self.training.iter().position(|record| {
            i16::from(record.grade) < grade
                || (i16::from(record.grade) == grade && clock < record.clock)
        })?;
        self.training.copy_within(place..4, place + 1);
        self.training[place] = TrainingRecord {
            character: character as u8,
            grade: grade as u8,
            clock,
        };
        Some(place)
    }
}

fn award(stock: &mut [u8; ITEMS], grade: i16, reward: usize, random: &mut Random) {
    let tier = 4i16.wrapping_sub(grade);
    if tier < 3 {
        let amount = if tier == 1 || tier == 2 { 1 } else { 2 };
        for (item, stock) in stock.iter_mut().enumerate().skip(1) {
            if item < 9 && *stock == LOCKED {
                *stock = stock.wrapping_add(2);
            }
            if *stock != LOCKED {
                *stock = stock.wrapping_add(amount);
            }
        }
        if tier != 1 && tier != 2 && stock[reward] == LOCKED {
            stock[reward] = stock[reward].wrapping_add(3);
        }
    }
    let mut remaining = if tier == 4 {
        3
    } else if tier >= 3 || tier == 1 {
        5
    } else {
        0
    };
    while remaining != 0 {
        let item = random.next15() as usize % 18 + 1;
        if item < 9 && stock[item] == LOCKED {
            stock[item] = stock[item].wrapping_add(2);
        }
        if stock[item] != LOCKED {
            stock[item] = stock[item].wrapping_add(1);
            remaining -= 1;
        }
    }
    if stock[19] != LOCKED {
        stock[19] = 1;
    }
}

fn return_items(stock: &mut [u8; ITEMS], remaining: &[u8; ITEMS]) {
    for item in 1..ITEMS {
        if remaining[item] == INFINITE {
            stock[item] = INFINITE;
        } else if remaining[item] != 0 {
            if stock[item] == LOCKED {
                stock[item] = stock[item].wrapping_add(2);
            }
            stock[item] = stock[item].wrapping_add(remaining[item]);
            if stock[item] >= 100 {
                stock[item] = 99;
            }
        }
    }
}
