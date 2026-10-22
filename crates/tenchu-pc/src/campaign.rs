use crate::{session::Options, source::DiscSource};
use tenchu_assets::{
    Result,
    executable::Executable,
    game::{GameAssets, Stage},
    script::Language,
    shop::Shop,
};
use tenchu_core::{
    game::{Game, events::Outcome},
    progress::{
        Completion, Loadout, Progress, Results,
        save::{Save, Selection},
    },
};

#[cfg(test)]
mod tests;

pub enum Screen {
    Frontend,
    Ending(Results),
    Launcher,
    Inventory(Loadout),
    Playing,
    Results(Results),
    GameOver,
    Complete,
}

#[derive(Clone, Copy)]
pub enum Action {
    Up,
    Down,
    Left,
    Right,
    Confirm,
    Back,
    Replay,
}

pub struct Campaign {
    pub game: Game,
    pub options: Options,
    pub progress: Progress,
    pub screen: Screen,
    pub cursor: usize,
    pub stages: Vec<Stage>,
    pub shop: Shop,
    pub saved: Option<Save>,
    pub save_request: Option<Save>,
    pub seed: u32,
    pub frontend: bool,
    pub quick_launcher: bool,
}

impl Campaign {
    pub fn new(
        assets: &GameAssets<DiscSource>,
        mut options: Options,
        saved: Option<Save>,
    ) -> Result<Self> {
        options.language = options
            .language
            .or_else(|| saved.as_ref().map(|save| save.selection.language));
        Ok(Self {
            game: options.load(assets)?,
            options,
            progress: Progress::default(),
            screen: Screen::Launcher,
            cursor: usize::from(saved.is_none()),
            stages: assets.stages()?,
            shop: Shop::load(&Executable::new(&assets.executable)?)?,
            saved,
            save_request: None,
            seed: 1,
            frontend: false,
            quick_launcher: true,
        })
    }

    pub fn playing(&self) -> bool {
        matches!(self.screen, Screen::Playing)
    }

    fn selection(&self, options: Options) -> Selection {
        Selection {
            character: options.character as u8,
            stage: self
                .stages
                .iter()
                .find(|s| s.order == options.mission as i32)
                .unwrap()
                .id as u8,
            layout: if options.layout == 0 {
                255
            } else {
                options.layout as u8 - 1
            },
            difficulty: options.difficulty,
            language: options.language.unwrap_or_default(),
        }
    }

    pub fn checkpoint(&mut self, options: Options) {
        self.save_request = Some(Save {
            progress: self.progress.clone(),
            selection: self.selection(options),
            seed: self.seed,
        });
    }

    pub fn inventory(&mut self) {
        self.checkpoint(self.options);
        self.screen = Screen::Inventory(Loadout::new(
            self.progress.stock[self.options.character as usize],
            &self.shop,
            self.options.mission == 0,
        ));
        self.cursor = 0;
    }

    fn next_options(&self) -> Options {
        let mut options = self.options;
        if (1..10).contains(&options.mission) {
            options.mission += 1;
            let stage = self.selection(options).stage as usize;
            options.layout = self
                .progress
                .next_layout(options.character as usize, stage)
                .map_or(0, |index| index + 1);
        }
        options
    }

    pub fn tick(&mut self, buttons: u16) -> Result<()> {
        if !self.playing() {
            return Ok(());
        }
        self.game.step(buttons)?;
        match self.game.events.outcome {
            Outcome::Running => {}
            Outcome::GameOver => {
                self.seed = self.game.random.seed;
                self.checkpoint(self.options);
                self.screen = Screen::GameOver;
            }
            Outcome::Complete => {
                let completion = Completion {
                    stage: &self.game.mission.stage,
                    character: self.options.character as usize,
                    layout: self.game.mission.layout_index,
                    language: self.options.language.unwrap_or_default(),
                    record: self.game.record(),
                    remaining: self.game.player().inventory[..20].try_into().unwrap(),
                };
                let results = self.progress.complete(completion, &mut self.game.random);
                self.seed = self.game.random.seed;
                self.checkpoint(self.next_options());
                self.screen = if self.frontend && self.options.mission == 10 {
                    Screen::Ending(results)
                } else {
                    Screen::Results(results)
                };
            }
        }
        Ok(())
    }

    pub fn action(&mut self, action: Action, assets: &GameAssets<DiscSource>) -> Result<bool> {
        match &mut self.screen {
            Screen::Frontend | Screen::Ending(_) => (),
            Screen::Launcher => match action {
                Action::Up => self.cursor = (self.cursor + 6) % 7,
                Action::Down => self.cursor = (self.cursor + 1) % 7,
                Action::Left | Action::Right => {
                    let delta = if matches!(action, Action::Right) {
                        1
                    } else {
                        -1
                    };
                    match self.cursor {
                        2 => self.options.character ^= 1,
                        3 => self.options.difficulty = self.options.difficulty.cycle(delta),
                        4 => {
                            self.options.mission =
                                (self.options.mission as i32 + delta).rem_euclid(11) as usize
                        }
                        5 => {
                            self.options.layout =
                                (self.options.layout as i32 + delta).rem_euclid(4) as usize
                        }
                        6 => {
                            let languages = [
                                Language::English,
                                Language::French,
                                Language::Italian,
                                Language::Japanese,
                            ];
                            let index = languages
                                .iter()
                                .position(|&language| {
                                    language == self.options.language.unwrap_or_default()
                                })
                                .unwrap();
                            self.options.language =
                                Some(languages[(index as i32 + delta).rem_euclid(4) as usize]);
                        }
                        _ => {}
                    }
                }
                Action::Confirm if self.cursor == 0 => {
                    if let Some(save) = self.saved.clone() {
                        self.options = Options {
                            character: i16::from(save.selection.character),
                            mission: self.stages[save.selection.stage as usize].order as usize,
                            layout: if save.selection.layout == 255 {
                                0
                            } else {
                                usize::from(save.selection.layout) + 1
                            },
                            difficulty: save.selection.difficulty,
                            language: self.options.language,
                        };
                        self.progress = save.progress;
                        self.seed = save.seed;
                        self.inventory();
                    }
                }
                Action::Confirm if self.cursor == 1 => self.inventory(),
                _ => {}
            },
            Screen::Inventory(loadout) => match action {
                Action::Up => self.cursor = (self.cursor + 18) % 19,
                Action::Down => self.cursor = (self.cursor + 1) % 19,
                Action::Right => {
                    loadout.take(self.shop.order[self.cursor]);
                }
                Action::Left => {
                    loadout.put_back(self.shop.order[self.cursor]);
                }
                Action::Back => {
                    self.screen = Screen::Launcher;
                    self.cursor = 1;
                }
                Action::Confirm => {
                    let Screen::Inventory(loadout) =
                        std::mem::replace(&mut self.screen, Screen::Launcher)
                    else {
                        unreachable!()
                    };
                    let (stock, selected) = loadout.finish();
                    let game = self.options.load_equipped(assets, self.seed, selected)?;
                    self.options.layout = game.mission.layout_index + 1;
                    self.progress.stock[self.options.character as usize] = stock;
                    self.game = game;
                    self.screen = Screen::Playing;
                    return Ok(true);
                }
                _ => {}
            },
            Screen::Playing => {
                if matches!(action, Action::Back | Action::Replay) {
                    self.seed = self.game.random.seed;
                    self.inventory();
                }
            }
            Screen::Results(_) => match action {
                Action::Confirm if self.options.mission == 10 => self.screen = Screen::Complete,
                Action::Confirm => {
                    self.options = self.next_options();
                    self.inventory();
                }
                Action::Replay => self.inventory(),
                Action::Back => {
                    self.options = self.next_options();
                    self.screen = Screen::Launcher;
                    self.cursor = 0;
                }
                _ => {}
            },
            Screen::GameOver => match action {
                Action::Confirm | Action::Replay => self.inventory(),
                Action::Back => {
                    self.screen = Screen::Launcher;
                    self.cursor = 0;
                }
                _ => {}
            },
            Screen::Complete => {
                if matches!(action, Action::Confirm | Action::Back) {
                    self.screen = Screen::Launcher;
                    self.cursor = 0;
                }
            }
        }
        Ok(false)
    }
}
