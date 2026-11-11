use std::sync::Arc;
use tenchu_assets::{
    ReadAt, Result, audio::stream::Clip, disc::Disc, tenchu2::selection::Resources,
};
use tenchu_core::{
    math::Math,
    tenchu2::{audio::Sound, motion::Motion, progress::Progress, selection::Selection as State},
};

#[derive(Clone, Copy, serde::Serialize, serde::Deserialize)]
pub enum Edit {
    Hero(bool),
    Mission(usize),
    Scroll(bool),
}

pub struct Selection {
    pub resources: Resources,
    pub state: State,
    pub motions: [Motion; 3],
    pub math: Math,
    pub bank: Arc<tenchu_assets::audio::bank::Bank>,
    pub music: Arc<Clip>,
}

impl Selection {
    pub fn load(disc: &Disc<impl ReadAt>, progress: Progress) -> Result<Self> {
        let resources = Resources::load(disc)?;
        let mut motions = [
            Motion::new(&resources.models[0].parts)?,
            Motion::new(&resources.models[1].parts)?,
            Motion::new(&resources.models[2].parts)?,
        ];
        motions[0].select_clip(0, resources.clips[0], &resources.motions)?;
        let music = Arc::new(Clip::load(disc, &resources.music)?);
        Ok(Self {
            resources,
            state: State::new(progress, true),
            motions,
            math: Math {
                tables: tenchu_assets::tenchu2::math_tables(disc)?,
            },
            bank: Arc::new(tenchu_assets::tenchu2::audio::menu(disc)?),
            music,
        })
    }

    pub fn step(
        &mut self,
        buttons: u16,
        action: Option<super::campaign::Action>,
    ) -> Result<Vec<Sound>> {
        use super::campaign::Action;
        let mut sounds = Vec::new();
        let input = match action.filter(|_| self.state.fade == 0 && !self.state.finished) {
            Some(Action::MissionSelection(Edit::Hero(right))) => {
                [0, if right { 0x20 } else { 0x80 }]
            }
            Some(Action::MissionSelection(Edit::Scroll(down))) => {
                [0, if down { 0x40 } else { 0x10 }]
            }
            Some(Action::MissionSelection(Edit::Mission(mission)))
                if self.state.available(mission, &self.resources.rules) =>
            {
                if self.state.mission != mission {
                    self.state.mission = mission;
                    self.state.page = self.state.page.min(mission).max(mission.saturating_sub(3));
                    sounds.push(4);
                }
                [0; 2]
            }
            Some(Action::Continue) => [0, 0x2000],
            Some(Action::Quit) => [0, 0x4000],
            _ => [buttons; 2],
        };
        for buttons in input {
            let old_hero = self.state.hero;
            let old_displayed = self.state.displayed;
            sounds.extend(self.state.step(buttons, &self.resources.rules));
            if self.state.finished {
                break;
            }
            if old_hero != self.state.hero {
                for hero in [self.state.hero, old_displayed] {
                    self.motions[hero].select_clip(
                        (hero + 3) as i16,
                        self.resources.clips[hero + 3],
                        &self.resources.motions,
                    )?;
                }
            }
            if old_displayed != self.state.displayed {
                let hero = self.state.hero;
                self.motions[hero].select_clip(
                    hero as i16,
                    self.resources.clips[hero],
                    &self.resources.motions,
                )?;
            }
            if self.state.hero != self.state.displayed {
                self.motions[self.state.hero].advance(
                    &self.resources.motions,
                    &self.math.tables.hermite,
                    true,
                )?;
            }
            self.motions[self.state.displayed].advance(
                &self.resources.motions,
                &self.math.tables.hermite,
                true,
            )?;
        }
        Ok(sounds
            .into_iter()
            .map(|id| Sound {
                id,
                slot: 0,
                note: 36,
                volume: [127; 2],
            })
            .collect())
    }
}
