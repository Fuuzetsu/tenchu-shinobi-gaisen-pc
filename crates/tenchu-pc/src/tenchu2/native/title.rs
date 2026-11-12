use std::sync::Arc;
use tenchu_assets::{ReadAt, Result, disc::Disc, tenchu2::title::Resources};
use tenchu_core::tenchu2::{audio::Sound, title::Title as State};

pub struct Title {
    pub resources: Resources,
    pub state: State,
    pub bank: Arc<tenchu_assets::audio::bank::Bank>,
    pub editor_notice: bool,
    notice_released: bool,
}

impl Title {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        Ok(Self {
            resources: Resources::load(disc)?,
            state: State::default(),
            bank: Arc::new(tenchu_assets::tenchu2::audio::menu(disc)?),
            editor_notice: false,
            notice_released: false,
        })
    }

    pub fn step(&mut self, buttons: u16, action: Option<super::campaign::Action>) -> Vec<Sound> {
        use super::campaign::Action;
        if self.editor_notice {
            self.notice_released |= buttons == 0;
            if matches!(action, Some(Action::Continue))
                || self.notice_released && matches!(buttons, 8 | 0x2000 | 0x4000)
            {
                self.editor_notice = false;
            }
            return Vec::new();
        }
        let input = match action.filter(|_| self.state.fade == 0 && !self.state.finished) {
            Some(Action::Title(choice)) => {
                self.state.choice = choice;
                self.state.active = true;
                [0, 0x2000]
            }
            Some(Action::Continue) => [0, 0x2000],
            _ => [buttons; 2],
        };
        input
            .into_iter()
            .filter_map(|buttons| self.state.step(buttons))
            .map(|id| Sound {
                id,
                slot: 0,
                note: 36,
                volume: [127; 2],
            })
            .collect()
    }
}
