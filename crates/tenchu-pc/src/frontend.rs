mod captions;
mod characters;
pub(crate) mod draw;
pub(crate) mod gpu;
mod labels;
#[cfg_attr(target_arch = "wasm32", path = "web/movie.rs")]
mod movie;
mod paint;
mod preparation;
pub(crate) mod raster;
mod results;
pub(crate) mod showcase;
#[cfg(test)]
mod tests;

use crate::{
    campaign::{Action, Campaign},
    source::DiscSource,
};
use characters::Characters;
use raster::Canvas;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};
use tenchu_assets::{
    Result,
    audio::{bank::Bank, stream::Clip},
    frontend::{Image, Movie, Tables, language_suffix, sound_directory},
    game::GameAssets,
    script::Language,
};
use tenchu_core::frontend::Title;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Preferences {
    pub control_scheme: usize,
    pub stereo: bool,
    pub rumble: bool,
    pub auto_save: bool,
    pub music: u8,
    pub effects: u8,
}
impl Default for Preferences {
    fn default() -> Self {
        Self {
            control_scheme: 0,
            stereo: true,
            rumble: true,
            auto_save: true,
            music: 96,
            effects: 96,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Page {
    Title,
    Character,
    Missions,
    Options,
    Choice(usize),
    Volume,
    Controls,
    SoundTest,
    Records,
    Card,
    Editor,
    Movie(Movie),
    Results,
    Briefing,
    Shop,
}

pub enum Event {
    Bank(Arc<Bank>),
    Music(Option<usize>),
    MovieAudio(Arc<Clip>),
    Narration(tenchu_assets::audio::stream::Track),
    Effect(i16),
    Voice(i16),
    Launch,
    StartMission,
    CancelPreparation,
    QuickLauncher,
    Save,
    Load,
    Preferences,
    Results,
    Quit,
}

pub struct Frontend {
    pub page: Page,
    pub preferences: Preferences,
    pub events: Vec<Event>,
    pub notice: String,
    tables: Tables,
    title: Title,
    title_images: Vec<Image>,
    visual_random: tenchu_core::random::Random,
    blood_colors: [u16; 9],
    character_images: Vec<Image>,
    mission_images: Vec<Image>,
    option_images: Vec<Image>,
    result_background: Image,
    result_numbers: Image,
    result_ranks: Vec<Image>,
    mission_art: preparation::Artwork,
    briefing: Option<preparation::Briefing>,
    shop_help: Option<usize>,
    shop_clicked: Option<usize>,
    characters: Characters,
    language: Language,
    pub canvas: Canvas,
    texture: Option<egui::TextureHandle>,
    artwork: Arc<draw::Artwork>,
    frame: Option<Arc<draw::Frame>>,
    high_definition: bool,
    artwork_enabled: bool,
    dirty: bool,
    elapsed: f64,
    ticks: u32,
    training: bool,
    selection: usize,
    option_selection: usize,
    choice: usize,
    mission: usize,
    layout: usize,
    cursor: [f32; 2],
    sound_choices: [usize; 2],
    sound_effects: Vec<i16>,
    volume_saved: [u8; 2],
    movie: Option<movie::Player>,
    disc: PathBuf,
}

impl Frontend {
    pub fn set_rumble(&mut self, enabled: bool) {
        if self.preferences.rumble != enabled {
            self.preferences.rumble = enabled;
            if self.page == Page::Choice(6) {
                self.choice = usize::from(!enabled);
            }
            self.dirty = true;
        }
    }

    pub fn buttons(&self, buttons: u16) -> u16 {
        let mut result = buttons;
        for (source, target) in self.tables.button_assign[0]
            .iter()
            .zip(self.tables.button_assign[self.preferences.control_scheme])
        {
            if buttons & u16::from(*source) != 0 {
                result |= u16::from(target);
            } else {
                result &= !u16::from(target);
            }
        }
        result
    }
    pub fn new(
        assets: &GameAssets<DiscSource>,
        disc: &Path,
        campaign: &Campaign,
        mut preferences: Preferences,
    ) -> Result<Self> {
        preferences.control_scheme = preferences.control_scheme.min(3);
        preferences.music = preferences.music.min(127);
        preferences.effects = preferences.effects.min(127);
        let tables = Tables::load(assets)?;
        let language = campaign.options.language.unwrap_or_default();
        let suffix = language_suffix(language);
        let archive = |path: &str| Image::archive(&assets.asset(path)?);
        let mut result = Self {
            page: Page::Title,
            preferences,
            events: vec![],
            notice: String::new(),
            characters: Characters::load(assets, &tables)?,
            tables,
            title: Title::default(),
            title_images: archive("DEMO/OPEN/TITLE.ARC")?,
            visual_random: tenchu_core::random::Random::new(1),
            blood_colors: [0; 9],
            character_images: archive(&format!("DEMO/PLAYER/PLAY_{suffix}.ARC"))?,
            mission_images: archive(&format!("DEMO/SCORE/SELSTG_{suffix}.ARC"))?,
            option_images: archive("DEMO/MENU/MENU_J.ARC")?,
            result_background: Image::parse(
                &assets.asset(&format!("DEMO/RS_E_{}.TIM", result_suffix(language)))?,
            )?,
            result_numbers: Image::parse(&assets.asset("DEMO/NUMBER.TIM")?)?,
            result_ranks: archive(&format!("DEMO/RANKS_{suffix}.ARC"))?,
            mission_art: preparation::Artwork::load(assets, campaign, language)?,
            briefing: None,
            shop_help: None,
            shop_clicked: None,
            language,
            canvas: Canvas::new(320),
            texture: None,
            artwork: Arc::default(),
            frame: None,
            high_definition: false,
            artwork_enabled: false,
            dirty: true,
            elapsed: 0.0,
            ticks: 0,
            training: false,
            selection: 0,
            option_selection: 0,
            choice: 0,
            mission: 1,
            layout: 0,
            cursor: [35.0, -250.0],
            sound_choices: [0; 2],
            sound_effects: vec![],
            volume_saved: [96; 2],
            movie: None,
            disc: disc.to_owned(),
        };
        result.reset_title();
        result.menu_bank(assets)?;
        if let Err(error) = result.reload_artwork() {
            eprintln!("Frontend asset pack: {error}");
        }
        Ok(result)
    }

    fn menu_bank(&mut self, assets: &GameAssets<DiscSource>) -> Result<()> {
        let bank = Bank::parse(
            &assets.asset(&format!("{}/STAGE9R.VAB", sound_directory(self.language)))?,
        )?;
        self.sound_effects = bank
            .programs
            .iter()
            .enumerate()
            .flat_map(|(program, p)| {
                p.iter().flat_map(move |p| {
                    p.tones
                        .iter()
                        .enumerate()
                        .filter(|(_, tone)| tone.mode == 4)
                        .map(move |(tone, _)| (program * 16 + tone) as i16)
                })
            })
            .take(100)
            .collect();
        if self.sound_effects.is_empty() {
            return Err("Menu sound bank contains no effects".into());
        }
        self.events.push(Event::Bank(Arc::new(bank)));
        Ok(())
    }

    pub fn language(&mut self, assets: &GameAssets<DiscSource>, language: Language) -> Result<()> {
        if self.language == language {
            return Ok(());
        }
        let suffix = language_suffix(language);
        self.character_images =
            Image::archive(&assets.asset(&format!("DEMO/PLAYER/PLAY_{suffix}.ARC"))?)?;
        self.mission_images =
            Image::archive(&assets.asset(&format!("DEMO/SCORE/SELSTG_{suffix}.ARC"))?)?;
        self.result_background =
            Image::parse(&assets.asset(&format!("DEMO/RS_E_{}.TIM", result_suffix(language)))?)?;
        self.result_ranks = Image::archive(&assets.asset(&format!("DEMO/RANKS_{suffix}.ARC"))?)?;
        self.mission_art.localized(assets, language)?;
        self.language = language;
        self.menu_bank(assets)?;
        self.dirty = true;
        if let Err(error) = self.reload_artwork() {
            self.artwork = Arc::default();
            eprintln!("Frontend asset pack: {error}");
        }
        Ok(())
    }

    pub(crate) fn artwork_images(
        &self,
    ) -> std::collections::BTreeMap<String, crate::visuals::pixels::Image> {
        let mut images: std::collections::BTreeMap<_, _> = self
            .title_images
            .iter()
            .chain(&self.character_images)
            .chain(&self.mission_images)
            .chain(&self.option_images)
            .chain(&self.result_ranks)
            .chain([&self.result_background, &self.result_numbers])
            .chain(self.mission_art.images())
            .map(|image| {
                let image = draw::pixels(image);
                (draw::identity(&image), image)
            })
            .collect();
        images.extend(self.characters.texture_images());
        images
    }

    pub fn reload_artwork(&mut self) -> Result<usize> {
        let pack = crate::visuals::pack::Pack::interface_from_env(self.artwork_images())?;
        let count = pack.textures.len();
        self.artwork = Arc::new(pack.textures);
        self.dirty = true;
        Ok(count)
    }

    pub fn enter_title(&mut self, assets: &GameAssets<DiscSource>) -> Result<()> {
        self.movie = None;
        self.briefing = None;
        self.reset_title();
        self.title.activate();
        self.set_page(Page::Title);
        self.menu_bank(assets)?;
        self.events.push(Event::Music(Some(14)));
        Ok(())
    }

    fn reset_title(&mut self) {
        self.title = Title::default();
        self.blood_colors = std::array::from_fn(|_| (self.visual_random.next15() as u16 & 15) | 16);
    }

    fn set_page(&mut self, page: Page) {
        self.notice.clear();
        self.page = page;
        self.ticks = 0;
        self.elapsed = 0.0;
        self.dirty = true;
        let width = if page == Page::Missions { 640 } else { 320 };
        if self.canvas.size[0] != width {
            self.canvas = Canvas::new(width);
        }
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(crate) fn recorded_results(&mut self) {
        self.set_page(Page::Results);
    }

    pub fn ready(&self) -> bool {
        self.ticks
            >= match self.page {
                Page::Title | Page::Movie(_) => 0,
                Page::Briefing => 0,
                Page::Missions => 31,
                Page::Results => 32,
                _ => 16,
            }
    }

    pub fn play_movie(&mut self, movie: Movie, assets: &GameAssets<DiscSource>) -> Result<()> {
        self.events.push(Event::Music(None));
        if let Some(path) = movie.voices(self.language) {
            self.events
                .push(Event::Bank(Arc::new(Bank::parse(&assets.asset(&path)?)?)));
        }
        self.set_page(Page::Movie(movie));
        self.movie = match movie::Player::new(&self.disc, assets, movie, self.language) {
            Ok(player) => Some(player),
            Err(error) => {
                self.notice = format!("{error} — Enter to continue");
                None
            }
        };
        Ok(())
    }

    fn movie_finished(&mut self, assets: &GameAssets<DiscSource>) -> Result<()> {
        let Page::Movie(movie) = self.page else {
            return Ok(());
        };
        self.movie = None;
        self.events.push(Event::Music(None));
        match movie {
            Movie::Opening => {
                self.reset_title();
                self.set_page(Page::Title);
                self.menu_bank(assets)?;
                self.events.push(Event::Music(Some(14)));
            }
            Movie::Training => self.events.push(Event::Launch),
            Movie::Rikimaru | Movie::Ayame => self.play_movie(Movie::Credits, assets)?,
            Movie::Credits => {
                self.set_page(Page::Results);
                self.menu_bank(assets)?;
                self.events.push(Event::Results);
                self.events.push(Event::Music(Some(12)));
            }
        }
        Ok(())
    }

    pub fn advance(
        &mut self,
        elapsed: Duration,
        suspended: bool,
        campaign: &mut Campaign,
        assets: &GameAssets<DiscSource>,
    ) -> Result<()> {
        if suspended {
            return Ok(());
        }
        if let Some(movie) = &mut self.movie {
            let update = match movie.advance(elapsed) {
                Ok(update) => update,
                Err(error) => {
                    self.movie = None;
                    self.events.push(Event::Music(None));
                    self.notice = format!("{error} — Enter to continue");
                    self.dirty = true;
                    return Ok(());
                }
            };
            if let Some(audio) = update.audio {
                self.events.push(Event::MovieAudio(audio));
            }
            for voice in update.voices {
                self.events.push(Event::Voice(voice));
            }
            self.dirty |= update.changed;
            if update.finished {
                self.movie_finished(assets)?;
            }
            return Ok(());
        }
        let rate = if self.page == Page::Missions {
            30.0
        } else {
            60.0
        };
        self.elapsed += elapsed.as_secs_f64().min(0.25) * rate;
        while self.elapsed >= 1.0 {
            self.elapsed -= 1.0;
            self.ticks += 1;
            self.dirty = true;
            match self.page {
                Page::Title => {
                    self.events
                        .extend(self.title.tick().into_iter().map(Event::Effect));
                    if self.title.finished {
                        match self.title.selection {
                            Some(0 | 1) => {
                                self.training = self.title.selection == Some(1);
                                self.characters.reset(&self.tables);
                                self.set_page(Page::Character);
                                self.events.push(Event::Music(Some(15)));
                            }
                            Some(2) => self.set_page(Page::Editor),
                            Some(_) => {
                                self.set_page(Page::Options);
                                self.events.push(Event::Music(Some(15)));
                            }
                            None => self.play_movie(Movie::Opening, assets)?,
                        }
                    }
                }
                Page::Character => {
                    if let Some(sound) = self.characters.tick(&campaign.game.math)? {
                        self.events.push(Event::Effect(sound));
                    }
                    if self.characters.done() {
                        campaign.options.character = self.characters.selected as i16;
                        if self.training {
                            campaign.options.mission = 0;
                            campaign.options.layout = 1;
                            self.play_movie(Movie::Training, assets)?;
                        } else {
                            self.mission = 1;
                            self.layout = 0;
                            self.cursor = self.tables.cursor_starts[0].map(f32::from);
                            self.set_page(Page::Missions);
                        }
                    }
                }
                Page::Missions => {
                    let target = self.tables.cursor_targets[self.mission - 1].map(f32::from);
                    self.cursor[0] = (self.cursor[0] - 10.0).max(target[0]);
                    self.cursor[1] = (self.cursor[1] + 50.0).min(target[1]);
                }
                Page::Briefing => self.advance_briefing(),
                _ => (),
            }
        }
        Ok(())
    }

    pub fn action(
        &mut self,
        action: Action,
        campaign: &mut Campaign,
        assets: &GameAssets<DiscSource>,
    ) -> Result<()> {
        if !self.ready() {
            return Ok(());
        }
        let action = if matches!(action, Action::Start) && self.page != Page::Shop {
            Action::Confirm
        } else {
            action
        };
        if !matches!(self.page, Page::Movie(_)) {
            self.notice.clear();
        }
        self.dirty = true;
        let delta = match action {
            Action::Up | Action::Left => -1,
            _ => 1,
        };
        match self.page {
            Page::Briefing if matches!(action, Action::Confirm | Action::Back) => {
                if let Some(briefing) = &mut self.briefing {
                    briefing.skip();
                }
            }
            Page::Shop => self.shop_action(action, campaign),
            Page::Title => match action {
                Action::Up | Action::Down => {
                    if self.title.navigate(delta) {
                        self.events.push(Event::Effect(3));
                    }
                }
                Action::Confirm => {
                    let confirmed = self.title.activate();
                    self.events
                        .push(Event::Effect(if confirmed { 4 } else { 2 }));
                }
                Action::Back => self.events.push(Event::Quit),
                _ => (),
            },
            Page::Character if !self.characters.confirming => match action {
                Action::Left | Action::Right => {
                    self.characters
                        .select(usize::from(matches!(action, Action::Right)), &self.tables);
                    self.events.push(Event::Effect(6));
                }
                Action::Back => self.enter_title(assets)?,
                Action::Confirm => {
                    self.characters.confirm(&self.tables);
                    self.events
                        .push(Event::Effect(7 + self.characters.selected as i16));
                }
                _ => (),
            },
            Page::Missions => match action {
                Action::Up | Action::Down => {
                    let highest = unlocked_mission(campaign);
                    self.mission = (self.mission as i32 + delta).clamp(1, highest as i32) as usize;
                    while self.layout > 0 && !layout_available(campaign, self.mission, self.layout)
                    {
                        self.layout -= 1;
                    }
                    self.cursor = self.tables.cursor_starts[self.mission - 1].map(f32::from);
                    self.events.push(Event::Effect(3));
                }
                Action::Left | Action::Right => {
                    let layout = (self.layout as i32 + delta).clamp(0, 2) as usize;
                    if layout_available(campaign, self.mission, layout) {
                        self.layout = layout;
                        self.events.push(Event::Effect(5));
                    }
                }
                Action::Confirm => {
                    campaign.options.mission = self.mission;
                    campaign.options.layout = self.layout + 1;
                    self.events.push(Event::Launch);
                }
                Action::Back => {
                    self.characters.reset(&self.tables);
                    self.set_page(Page::Character);
                }
                _ => (),
            },
            Page::Options => match action {
                Action::Up if !self.option_selection.is_multiple_of(5) => {
                    self.option_selection -= 1
                }
                Action::Down if self.option_selection % 5 < 4 => self.option_selection += 1,
                Action::Left if self.option_selection >= 5 => self.option_selection -= 5,
                Action::Right if self.option_selection < 5 => self.option_selection += 5,
                Action::Back => self.enter_title(assets)?,
                Action::Confirm => self.open_option(campaign),
                _ => (),
            },
            Page::Choice(option) => match action {
                Action::Up | Action::Down => {
                    self.choice = (self.choice as i32 + delta).clamp(
                        0,
                        if option == 9 {
                            3
                        } else if option == 1 {
                            2
                        } else {
                            1
                        },
                    ) as usize;
                    self.events.push(Event::Effect(3));
                }
                Action::Back => self.set_page(Page::Options),
                Action::Confirm => {
                    match option {
                        1 => {
                            campaign.options.difficulty = match self.choice {
                                0 => tenchu_core::ai::Difficulty::Easy,
                                1 => tenchu_core::ai::Difficulty::Normal,
                                _ => tenchu_core::ai::Difficulty::Hard,
                            }
                        }
                        2 => self.preferences.stereo = self.choice == 0,
                        6 => self.preferences.rumble = self.choice == 0,
                        8 => self.preferences.auto_save = self.choice == 0,
                        9 => {
                            let language = [
                                Language::Japanese,
                                Language::English,
                                Language::French,
                                Language::Italian,
                            ][self.choice];
                            campaign.options.language = Some(language);
                            self.language(assets, language)?;
                        }
                        _ => (),
                    }
                    self.events.push(Event::Preferences);
                    self.set_page(Page::Options);
                }
                _ => (),
            },
            Page::Volume => match action {
                Action::Up | Action::Down => {
                    self.selection = usize::from(matches!(action, Action::Down))
                }
                Action::Left | Action::Right => {
                    let value = if self.selection == 0 {
                        &mut self.preferences.music
                    } else {
                        &mut self.preferences.effects
                    };
                    *value = (i32::from(*value) + delta * 2).clamp(0, 127) as u8;
                    self.events.push(Event::Preferences);
                    if self.selection == 1 {
                        self.events.push(Event::Effect(3));
                    }
                }
                Action::Back | Action::Confirm => {
                    if matches!(action, Action::Back) {
                        [self.preferences.music, self.preferences.effects] = self.volume_saved;
                    }
                    self.events.push(Event::Preferences);
                    self.set_page(Page::Options);
                }
                _ => (),
            },
            Page::Controls => match action {
                Action::Left | Action::Right => {
                    self.choice = (self.choice as i32 + delta).rem_euclid(4) as usize
                }
                Action::Confirm => {
                    self.preferences.control_scheme = self.choice;
                    self.events.push(Event::Preferences);
                    self.set_page(Page::Options);
                }
                Action::Back => self.set_page(Page::Options),
                _ => (),
            },
            Page::SoundTest => match action {
                Action::Up | Action::Down => {
                    self.selection = usize::from(matches!(action, Action::Down))
                }
                Action::Left | Action::Right => {
                    let count = if self.selection == 0 {
                        20
                    } else {
                        self.sound_effects.len()
                    };
                    self.sound_choices[self.selection] = (self.sound_choices[self.selection] as i32
                        + delta)
                        .rem_euclid(count as i32)
                        as usize;
                }
                Action::Confirm if self.selection == 0 => self.events.push(Event::Music(
                    self.sound_choices[0]
                        .checked_sub(1)
                        .map(|i| self.tables.sound_test_music[i]),
                )),
                Action::Confirm => self
                    .events
                    .push(Event::Effect(self.sound_effects[self.sound_choices[1]])),
                Action::Back => {
                    self.events.push(Event::Music(Some(15)));
                    self.set_page(Page::Options);
                }
                _ => (),
            },
            Page::Records => match action {
                Action::Left | Action::Right => {
                    self.selection = usize::from(matches!(action, Action::Right))
                }
                Action::Back | Action::Confirm => {
                    self.set_page(Page::Options);
                    self.events.push(Event::Music(Some(15)));
                }
                _ => (),
            },
            Page::Card => match action {
                Action::Up | Action::Down => {
                    self.choice = usize::from(matches!(action, Action::Down))
                }
                Action::Confirm => self.events.push(if self.choice == 0 {
                    Event::Load
                } else {
                    Event::Save
                }),
                Action::Back => self.set_page(Page::Options),
                _ => (),
            },
            Page::Editor if matches!(action, Action::Confirm | Action::Back) => {
                self.enter_title(assets)?
            }
            Page::Movie(movie)
                if (movie.skippable() || self.movie.is_none())
                    && matches!(action, Action::Confirm | Action::Back) =>
            {
                self.movie_finished(assets)?
            }
            _ => (),
        }
        Ok(())
    }

    fn open_option(&mut self, campaign: &Campaign) {
        let page = match self.option_selection {
            0 => {
                self.selection = 0;
                self.events.push(Event::Music(Some(12)));
                Page::Records
            }
            3 => {
                self.selection = 0;
                self.volume_saved = [self.preferences.music, self.preferences.effects];
                Page::Volume
            }
            4 => {
                self.selection = 0;
                self.sound_choices = [0; 2];
                Page::SoundTest
            }
            5 => {
                self.choice = self.preferences.control_scheme;
                Page::Controls
            }
            7 => {
                self.choice = 0;
                Page::Card
            }
            option => {
                self.choice = match option {
                    1 => match campaign.options.difficulty {
                        tenchu_core::ai::Difficulty::Easy => 0,
                        tenchu_core::ai::Difficulty::Normal => 1,
                        tenchu_core::ai::Difficulty::Hard => 2,
                    },
                    2 => usize::from(!self.preferences.stereo),
                    6 => usize::from(!self.preferences.rumble),
                    8 => usize::from(!self.preferences.auto_save),
                    9 => match self.language {
                        Language::Japanese => 0,
                        Language::English => 1,
                        Language::French => 2,
                        Language::Italian => 3,
                    },
                    _ => 0,
                };
                Page::Choice(option)
            }
        };
        self.set_page(page);
    }

    pub fn leave(&mut self) {
        self.movie = None;
        self.briefing = None;
        self.events.clear();
        self.set_page(Page::Title);
    }

    pub fn quick_launcher(&mut self) {
        self.leave();
        self.events.push(Event::QuickLauncher);
    }
}

fn result_suffix(language: Language) -> &'static str {
    match language {
        Language::Japanese => "jap",
        Language::English => "eng",
        Language::French => "fre",
        Language::Italian => "ita",
    }
}

fn stage_id(campaign: &Campaign, mission: usize) -> usize {
    campaign
        .stages
        .iter()
        .find(|s| s.order == mission as i32)
        .unwrap()
        .id
}

fn unlocked_mission(campaign: &Campaign) -> usize {
    (campaign.progress.highest[campaign.options.character as usize] as usize + 1).min(10)
}

fn layout_available(campaign: &Campaign, mission: usize, layout: usize) -> bool {
    let records = &campaign.progress.records[campaign.options.character as usize]
        [stage_id(campaign, mission)];
    layout == 0 || records[layout].played() || records[layout - 1].played()
}
