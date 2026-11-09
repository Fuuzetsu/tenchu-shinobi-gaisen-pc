use std::sync::Arc;
use tenchu_assets::{
    ReadAt, Result,
    disc::Disc,
    tenchu2::{
        audio::Audio,
        equipment::{Images, Rules},
    },
};
use tenchu_core::tenchu2::{audio::Sound, equipment::Equipment, progress::Stock};

#[derive(Clone, Copy)]
pub enum Edit {
    Select(usize),
    Add,
    Remove,
}

pub struct Selection {
    pub state: Equipment,
    pub rules: Rules,
    pub images: Images,
    pub bank: Arc<tenchu_assets::audio::bank::Bank>,
}

impl Selection {
    pub fn load(disc: &Disc<impl ReadAt>, hero: usize, map: usize, stock: Stock) -> Result<Self> {
        Ok(Self {
            state: Equipment::new(stock, false),
            rules: Rules::load(disc)?,
            images: Images::load(disc)?,
            bank: Arc::new(Audio::load(disc, hero, map)?.bank),
        })
    }

    pub fn step(&mut self, buttons: u16, action: Option<super::campaign::Action>) -> Vec<Sound> {
        use super::campaign::Action;
        let mut sounds = Vec::new();
        let input = match action.filter(|_| self.state.fade == 0 && !self.state.finished) {
            Some(Action::Equipment(Edit::Select(index))) if index < self.rules.cells.len() => {
                if self.state.selected != index {
                    self.state.selected = index;
                    self.state.description = index + 3;
                    sounds.push(5);
                }
                [0, 0]
            }
            Some(Action::Equipment(Edit::Add)) => [0, 0x2000],
            Some(Action::Equipment(Edit::Remove)) => [0, 0x4000],
            Some(Action::Continue) => [0, 8],
            Some(Action::Quit) => [0, 9],
            _ => [buttons; 2],
        };
        for buttons in input {
            sounds.extend(self.state.step(buttons, &self.rules));
        }
        sounds
            .into_iter()
            .map(|id| Sound {
                id,
                slot: 0,
                note: 36,
                volume: [127; 2],
            })
            .collect()
    }
}
