use crate::settings::Settings;
use tenchu_core::effects::Graphic;

#[derive(Clone, Copy)]
pub struct Style {
    pub sharp_text: bool,
    pub blood_spray: bool,
    pub blood_stains: bool,
    pub lightning: bool,
    pub alpha: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            sharp_text: false,
            blood_spray: false,
            blood_stains: false,
            lightning: false,
            alpha: 1.0,
        }
    }
}

impl From<&Settings> for Style {
    fn from(settings: &Settings) -> Self {
        Self {
            sharp_text: settings.sharp_text,
            blood_spray: settings.graphics.blood.spray,
            blood_stains: settings.graphics.blood.enabled && settings.graphics.blood.amount > 0,
            lightning: settings.graphics.effects.lightning,
            ..Self::default()
        }
    }
}

impl Style {
    pub(super) fn replaces(self, graphic: &Graphic, color: [u8; 3]) -> bool {
        (self.lightning && matches!(graphic, Graphic::Bleed) && lightning_spark(color))
            || self.blood_spray
                && match graphic {
                    Graphic::Blood { airborne, .. } => *airborne || self.blood_stains,
                    Graphic::Bleed => blood_color(color),
                    _ => false,
                }
    }
}

pub(super) fn lightning_spark(color: [u8; 3]) -> bool {
    // SetLightning's scatter color is exclusive to its impact sparks.
    color == [255, 255, 120]
}

pub(super) fn blood_color(color: [u8; 3]) -> bool {
    u16::from(color[0]) > u16::from(color[1].max(color[2])) * 2
}
