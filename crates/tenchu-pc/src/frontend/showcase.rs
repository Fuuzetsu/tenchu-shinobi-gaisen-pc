use super::*;

#[derive(Clone, Copy, Debug, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Menu {
    Title,
    Character,
    Missions,
    Options,
    Controls,
    Volume,
    Language,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub page: Menu,
    #[serde(default = "english")]
    pub language: String,
}

fn english() -> String {
    "en".into()
}

impl Scene {
    fn language(&self) -> Result<Language> {
        match self.language.as_str() {
            "en" => Ok(Language::English),
            "ja" => Ok(Language::Japanese),
            "fr" => Ok(Language::French),
            "it" => Ok(Language::Italian),
            _ => Err("Frontend demo language must be en, ja, fr or it".into()),
        }
    }

    pub fn validate(&self) -> Result<()> {
        self.language().map(|_| ())
    }
}

impl Frontend {
    pub fn stage_showcase(
        &mut self,
        scene: &Scene,
        assets: &GameAssets<DiscSource>,
        campaign: &mut Campaign,
    ) -> Result<()> {
        let language = scene.language()?;
        self.language(assets, language)?;
        campaign.options.language = Some(language);
        self.movie = None;
        self.visual_random = tenchu_core::random::Random::new(1);
        self.reset_title();
        self.title.activate();
        self.characters.reset(&self.tables);
        self.characters
            .select(campaign.options.character as usize, &self.tables);
        self.training = false;
        self.selection = 0;
        self.option_selection = 0;
        self.choice = 0;
        self.mission = campaign.options.mission;
        self.layout = 0;
        self.cursor = self.tables.cursor_targets[self.mission - 1].map(f32::from);
        self.set_page(match scene.page {
            Menu::Title => Page::Title,
            Menu::Character => Page::Character,
            Menu::Missions => Page::Missions,
            Menu::Options => Page::Options,
            Menu::Controls => Page::Controls,
            Menu::Volume => Page::Volume,
            Menu::Language => Page::Choice(9),
        });
        self.ticks = 120;
        self.events.clear();
        campaign.screen = crate::campaign::Screen::Frontend;
        Ok(())
    }
}
