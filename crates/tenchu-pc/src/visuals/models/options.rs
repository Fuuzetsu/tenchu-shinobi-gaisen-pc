use crate::settings::Graphics;

#[derive(Clone, Copy, PartialEq)]
pub struct Options {
    artwork: bool,
    smooth: bool,
    detail: u16,
    water: bool,
    water_detail: u16,
    characters: u16,
    heroes: bool,
    foliage: u16,
    variation: bool,
    grapple: bool,
    leaves: bool,
    secondary: bool,
    wind: bool,
    baked_snow: bool,
}

impl From<&Graphics> for Options {
    fn from(graphics: &Graphics) -> Self {
        Self {
            artwork: graphics.asset_pack,
            smooth: graphics.smooth_models,
            detail: graphics.model_detail,
            water: graphics.water.enabled,
            water_detail: graphics.water.detail,
            characters: graphics.models.characters,
            heroes: graphics.models.heroes,
            foliage: graphics.models.foliage,
            variation: graphics.models.variation,
            grapple: graphics.models.grapple,
            leaves: graphics.effects.leaves || graphics.effects.scatter,
            secondary: graphics.motion.actors(),
            wind: graphics.motion.wind != 0,
            baked_snow: graphics.weather.baked_snow,
        }
    }
}
