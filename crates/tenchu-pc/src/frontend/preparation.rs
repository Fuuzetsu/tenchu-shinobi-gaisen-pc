use super::paint::digits;
use super::raster::{Blend, Sprite};
use super::*;
use tenchu_assets::frontend::mission::{Briefing as BriefingAssets, Screens};
use tenchu_core::progress::{INFINITE, LOCKED, Loadout};

pub(super) struct Artwork {
    pub shop: Image,
    pub help: Vec<Image>,
    pub results: Image,
    pub training: Image,
    pub ranks: Vec<Image>,
    pub icons: Vec<Image>,
    pub cursor: Image,
    pub numbers: Image,
}

impl Artwork {
    pub fn load(
        assets: &GameAssets<DiscSource>,
        campaign: &Campaign,
        language: Language,
    ) -> Result<Self> {
        let paths = Screens::load(&assets.executable, language)?;
        let mission = &campaign.game.mission;
        let image = |index| Image::from_vram(&mission.vram, mission.images[index]);
        let help = Image::archive(&assets.asset(&paths.help)?)?;
        let ranks = Image::archive(&assets.asset(&paths.training_ranks)?)?;
        if help.len() < 21 || ranks.len() < 7 {
            return Err("Incomplete mission menu artwork".into());
        }
        Ok(Self {
            shop: Image::parse(&assets.asset(&paths.shop)?)?,
            help,
            results: Image::parse(&assets.asset(&paths.results)?)?,
            training: Image::parse(&assets.asset(&paths.training)?)?,
            ranks,
            icons: (20..40).map(image).collect(),
            cursor: image(50),
            numbers: image(51),
        })
    }

    pub fn localized(&mut self, assets: &GameAssets<DiscSource>, language: Language) -> Result<()> {
        let paths = Screens::load(&assets.executable, language)?;
        self.shop = Image::parse(&assets.asset(&paths.shop)?)?;
        self.help = Image::archive(&assets.asset(&paths.help)?)?;
        self.results = Image::parse(&assets.asset(&paths.results)?)?;
        self.training = Image::parse(&assets.asset(&paths.training)?)?;
        self.ranks = Image::archive(&assets.asset(&paths.training_ranks)?)?;
        Ok(())
    }

    pub fn images(&self) -> impl Iterator<Item = &Image> {
        [
            &self.shop,
            &self.results,
            &self.training,
            &self.cursor,
            &self.numbers,
        ]
        .into_iter()
        .chain(&self.help)
        .chain(&self.ranks)
        .chain(&self.icons)
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    FadeIn,
    Waiting,
    Hold,
    Scroll,
    FadeOut { scrolling: bool },
}

pub(super) struct Briefing {
    pub assets: BriefingAssets,
    pub background: Image,
    pub text: Image,
    phase: Phase,
    pub fade: i32,
    pub scroll: i32,
    counter: u32,
    played: u32,
    pub audio_ready: bool,
}

impl Briefing {
    pub fn load(
        assets: &GameAssets<DiscSource>,
        language: Language,
        stage: usize,
        character: i16,
    ) -> Result<Self> {
        let data = BriefingAssets::load(&assets.executable, language, stage, character)?;
        Ok(Self {
            background: Image::parse(&assets.asset(&data.background)?)?,
            text: Image::parse(&assets.asset(&data.text)?)?,
            assets: data,
            phase: Phase::FadeIn,
            fade: 254,
            scroll: -0xa000,
            counter: 0,
            played: 0,
            audio_ready: false,
        })
    }

    pub fn skip(&mut self) {
        self.phase = Phase::FadeOut {
            scrolling: self.scrolling(),
        };
    }

    pub fn scrolling(&self) -> bool {
        matches!(
            self.phase,
            Phase::Scroll | Phase::FadeOut { scrolling: true }
        )
    }

    pub fn tick(&mut self, events: &mut Vec<Event>) -> bool {
        match self.phase {
            Phase::FadeIn if self.fade == 0 => {
                events.push(Event::Narration(self.assets.narration.clone()));
                self.phase = Phase::Waiting;
            }
            Phase::FadeIn => self.fade = (self.fade - 8).max(0),
            Phase::Waiting if self.audio_ready => self.phase = Phase::Hold,
            Phase::Hold => {
                self.played += 1;
                if self.counter as i32 > i32::from(self.assets.hold) {
                    self.phase = Phase::Scroll;
                }
                self.counter += 1;
            }
            Phase::Scroll => {
                self.played += 1;
                self.scroll += i32::from(self.assets.scroll_step);
                let duration = self.assets.narration.end - self.assets.narration.start;
                if u64::from(self.played) * 150 >= u64::from(duration) * 60 {
                    self.phase = Phase::FadeOut { scrolling: true };
                }
            }
            Phase::FadeOut { scrolling } => {
                if scrolling {
                    self.scroll += i32::from(self.assets.scroll_step);
                }
                self.fade = (self.fade + 8).min(255);
            }
            _ => {}
        }
        matches!(self.phase, Phase::FadeOut { .. }) && self.fade == 255
    }
}

pub(super) fn navigate(positions: &[[i16; 2]], cursor: usize, direction: [i32; 2]) -> usize {
    let [dx, dy] = direction.map(|v| v * 16);
    let [x, y] = positions[cursor].map(i32::from);
    positions
        .iter()
        .enumerate()
        .filter_map(|(index, position)| {
            let ex = i32::from(position[0]) - x - dx;
            let ey = i32::from(position[1]) - y - dy;
            (index != cursor && ex * dx >= 0 && ey * dy >= 0).then_some((index, ex * ex + ey * ey))
        })
        .min_by_key(|&(_, distance)| distance)
        .map_or(cursor, |(index, _)| index)
}

impl Frontend {
    pub fn prepare(
        &mut self,
        campaign: &Campaign,
        assets: &GameAssets<DiscSource>,
        briefing: bool,
    ) -> Result<()> {
        self.language(assets, campaign.options.language.unwrap_or_default())?;
        self.briefing = None;
        self.shop_help = None;
        self.shop_clicked = None;
        self.menu_bank(assets)?;
        if campaign.options.mission == 0 {
            self.set_page(Page::Shop);
            self.events.push(Event::StartMission);
        } else if briefing {
            self.briefing = Some(Briefing::load(
                assets,
                self.language,
                stage_id(campaign, campaign.options.mission),
                campaign.options.character,
            )?);
            if let Err(error) = self.reload_artwork() {
                eprintln!("Briefing asset pack: {error}");
            }
            self.events.push(Event::Music(None));
            self.set_page(Page::Briefing);
        } else {
            self.open_shop();
        }
        Ok(())
    }

    fn open_shop(&mut self) {
        self.briefing = None;
        self.set_page(Page::Shop);
        self.events.push(Event::Music(None));
    }

    pub(super) fn shop_action(&mut self, action: Action, campaign: &mut Campaign) {
        let crate::campaign::Screen::Inventory(loadout) = &mut campaign.screen else {
            return;
        };
        if let Some(index) = self.shop_clicked.take() {
            campaign.cursor = index;
        }
        let direction = match action {
            Action::Up => Some([0, -1]),
            Action::Down => Some([0, 1]),
            Action::Left => Some([-1, 0]),
            Action::Right => Some([1, 0]),
            _ => None,
        };
        if let Some(direction) = direction {
            campaign.cursor = navigate(&campaign.shop.positions, campaign.cursor, direction);
            self.shop_help = None;
            self.events.push(Event::Effect(0x0b));
            return;
        }
        let item = campaign.shop.order[campaign.cursor];
        match action {
            Action::Confirm => {
                self.shop_help = if loadout.stock[item] == 0 || loadout.stock[item] == LOCKED {
                    None
                } else if loadout.taken >= loadout.capacity {
                    Some(19)
                } else if loadout.selected[item] == 0 && loadout.kinds >= Loadout::KIND_LIMIT {
                    Some(20)
                } else {
                    None
                };
                if loadout.take(item) {
                    self.events.push(Event::Effect(0x0d));
                } else if self.shop_help.is_some() {
                    self.events.push(Event::Effect(0x0c));
                }
            }
            Action::Replay => {
                if loadout.put_back(item) {
                    self.events.push(Event::Effect(0x1f));
                }
                self.shop_help = None;
            }
            Action::Start => self.events.push(Event::StartMission),
            Action::Back => self.events.push(Event::CancelPreparation),
            _ => {}
        }
    }

    pub(super) fn advance_briefing(&mut self) {
        if let Some(briefing) = &mut self.briefing
            && briefing.tick(&mut self.events)
        {
            self.open_shop();
        }
    }

    pub fn narration_ready(&mut self, audio: Option<&crate::audio::Audio>) {
        if let Some(briefing) = &mut self.briefing {
            briefing.audio_ready = audio.is_none_or(|a| a.track_ready(&briefing.assets.narration));
        }
    }

    pub fn enter_results(
        &mut self,
        assets: &GameAssets<DiscSource>,
        campaign: &Campaign,
    ) -> Result<()> {
        self.language(assets, campaign.options.language.unwrap_or_default())?;
        self.set_page(Page::Results);
        self.menu_bank(assets)?;
        self.events.push(Event::Music(Some(12)));
        Ok(())
    }

    pub(super) fn paint_briefing(&mut self) {
        let Some(briefing) = &self.briefing else {
            return;
        };
        self.canvas
            .sprite(Sprite::new(&briefing.background, 0.0, 0.0));
        if briefing.scrolling() {
            let text = &briefing.text;
            for column in (0..text.size[0] / 16).rev() {
                let x = briefing.scroll / 256 - (text.size[0] - column * 16) as i32 * 2;
                let brightness = if !(-160..=160).contains(&x) {
                    0
                } else if x < -120 {
                    (x + 160) * 3
                } else if x > 120 {
                    (160 - x) * 3
                } else {
                    128
                };
                self.canvas.sprite(
                    Sprite::new(text, x as f32, -104.0 + text.size[1] as f32 / 2.0)
                        .region([column * 16, 0, 16, text.size[1]])
                        .light(brightness)
                        .blend(Blend::Add),
                );
            }
        }
        self.canvas.fade((briefing.fade + 1) / 2);
    }

    pub(super) fn paint_shop(&mut self, campaign: &Campaign) {
        let crate::campaign::Screen::Inventory(loadout) = &campaign.screen else {
            return;
        };
        self.canvas
            .sprite(Sprite::new(&self.mission_art.shop, 0.0, 0.0));
        if let Some(lighting) = self.shop_lighting
            && let Some(frame) = &mut self.canvas.frame
        {
            frame.light_room(lighting);
        }
        for (&item, &[x, y]) in campaign.shop.order.iter().zip(&campaign.shop.positions) {
            if loadout.stock[item] == LOCKED {
                continue;
            }
            self.canvas.sprite(Sprite::new(
                &self.mission_art.icons[item],
                f32::from(x),
                f32::from(y),
            ));
            if loadout.stock[item] != INFINITE {
                self.shop_number(loadout.stock[item], f32::from(x) + 26.0, f32::from(y) + 8.0);
            }
        }
        let [x, y] = campaign.shop.positions[campaign.cursor];
        self.canvas.sprite(
            Sprite::new(&self.mission_art.cursor, f32::from(x), f32::from(y))
                .angle(-(self.ticks as f32) * 3.0f32.to_radians())
                .blend(Blend::Add),
        );
        let item = campaign.shop.order[campaign.cursor];
        let help = self
            .shop_help
            .or_else(|| (loadout.stock[item] != LOCKED).then_some(item - 1));
        if let Some(help) = help {
            let image = &self.mission_art.help[help];
            self.canvas.sprite(
                Sprite::new(
                    image,
                    -146.0 + image.size[0] as f32 / 2.0,
                    35.0 + image.size[1] as f32 / 2.0,
                )
                .blend(Blend::Add),
            );
        }
        let mut shown = 0;
        for (item, &count) in loadout.selected.iter().enumerate() {
            if count == 0 {
                continue;
            }
            self.canvas.sprite(Sprite::new(
                &self.mission_art.icons[item],
                140.0 - shown as f32 * 25.0,
                90.0,
            ));
            if count != INFINITE {
                self.shop_number(count, 166.0 - shown as f32 * 25.0, 98.0);
            }
            shown += 1;
        }
        digits(
            &mut self.canvas,
            &self.result_numbers,
            i32::from(loadout.capacity - loadout.taken),
            34.0,
            -50.0,
            12.0,
            false,
        );
    }

    fn shop_number(&mut self, value: u8, x: f32, y: f32) {
        let text = value.to_string();
        let image = &self.mission_art.numbers;
        let width = image.size[0] / 10;
        // PutNumber retains the pivot of the full digit strip after changing its width.
        let pivot = (image.size[0] - width) as f32 / 2.0;
        for (index, digit) in text.bytes().rev().enumerate() {
            self.canvas.sprite(
                Sprite::new(image, x - index as f32 * 6.0 - pivot, y)
                    .region([usize::from(digit - b'0') * width, 0, width, image.size[1]])
                    .blend(Blend::Add),
            );
        }
    }
}
