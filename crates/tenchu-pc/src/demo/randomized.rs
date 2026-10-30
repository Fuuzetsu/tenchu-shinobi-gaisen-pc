use super::Player;
use crate::{
    randomizer::{Plan, Run},
    session::Options,
    source::DiscSource,
    visuals::appearance::library::Library,
};
use tenchu_assets::{Result, game::GameAssets};
use tenchu_core::game::Game;

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Scene {
    pub run: Run,
    #[serde(default)]
    pub inputs: Vec<(u16, u16)>,
    pub spawn: Option<[i32; 3]>,
    pub facing: Option<i16>,
    pub item: Option<usize>,
    #[serde(default)]
    pub engage: bool,
    #[serde(default)]
    pub menu_tabs: Vec<usize>,
    pub distance: Option<f32>,
}

impl Scene {
    pub fn validate(&self) -> Result<()> {
        self.run.document.validate_format()?;
        if self.inputs.len() > 3600
            || self.inputs.iter().any(|(ticks, _)| *ticks == 0)
            || self
                .inputs
                .iter()
                .map(|(ticks, _)| u32::from(*ticks))
                .sum::<u32>()
                > 3600
            || self.menu_tabs.len() > 7
            || self.menu_tabs.iter().any(|&tab| tab > 6)
            || self.item.is_some_and(|item| item >= 20)
            || self
                .spawn
                .is_some_and(|p| p.iter().any(|n| !(-320_000..=320_000).contains(n)))
            || self
                .distance
                .is_some_and(|d| !d.is_finite() || !(800.0..=30_000.0).contains(&d))
        {
            return Err("Invalid randomizer demo staging".into());
        }
        Ok(())
    }
}

impl Player {
    pub(super) fn load_randomized(
        &mut self,
        assets: &GameAssets<DiscSource>,
        library: &Library,
    ) -> Result<(Game, Options)> {
        let scene = self.clip().randomizer.as_ref().unwrap().clone();
        let (mut game, plan) = scene.run.load(assets, library)?;
        if plan.mission != self.clip().mission || plan.character != self.clip().character {
            return Err(format!(
                "Demo '{}' mission/character differ from the randomizer roll",
                self.clip().id
            ));
        }
        let options = plan.options(scene.run.document.rules.language.game());
        game.events = Default::default();
        if let Some(spawn) = scene.spawn {
            game.teleport_player(spawn.into(), true)?;
        }
        if let Some(facing) = scene.facing {
            game.roster.get_mut(game.player_id).rotation[1] = facing;
        }
        for _ in 0..30 {
            game.step(0)?;
        }
        if let Some(index) = scene.item {
            let item = tenchu_core::items::Kind::try_from(index)?;
            if game.player().inventory[item.index()] == 0 {
                return Err(format!("Randomizer roll does not include item {item:?}"));
            }
            game.hud.selected = Some(item);
            game.hud.cursor = index;
        }
        self.randomized = Some((scene.run.clone(), plan));
        self.inputs = Some(
            scene
                .inputs
                .iter()
                .flat_map(|&(ticks, buttons)| std::iter::repeat_n(buttons, usize::from(ticks)))
                .collect(),
        );
        Ok((game, options))
    }

    pub fn randomizer_tab(&self) -> Option<usize> {
        let tabs = &self.clip().randomizer.as_ref()?.menu_tabs;
        (!tabs.is_empty()).then(|| tabs[super::section(self.progress(), tabs.len())])
    }

    pub fn randomizer_plan(&self) -> Option<&Plan> {
        self.randomized.as_ref().map(|(_, plan)| plan)
    }
}
