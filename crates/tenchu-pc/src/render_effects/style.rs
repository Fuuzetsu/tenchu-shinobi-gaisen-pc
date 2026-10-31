use crate::settings::{Blood, BloodStyle, Settings};
use tenchu_core::effects::Graphic;

#[derive(Clone, Copy)]
pub struct Style {
    pub sharp_text: bool,
    pub blood: Blood,
    pub lightning: bool,
    pub gunfire: bool,
    pub alpha: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            sharp_text: false,
            blood: Blood::default(),
            lightning: false,
            gunfire: false,
            alpha: 1.0,
        }
    }
}

impl From<&Settings> for Style {
    fn from(settings: &Settings) -> Self {
        Self {
            sharp_text: settings.sharp_text,
            blood: settings.graphics.blood,
            lightning: settings.graphics.effects.lightning,
            gunfire: settings.graphics.effects.gunfire,
            ..Self::default()
        }
    }
}

impl Style {
    pub(super) fn replaces(self, graphic: &Graphic, color: [u8; 3]) -> bool {
        (self.lightning && matches!(graphic, Graphic::Bleed) && lightning_spark(color))
            || self.gunfire && matches!(graphic, Graphic::GunImpact(_) | Graphic::GunBleed)
            || match graphic {
                Graphic::Blood { airborne, .. } => {
                    if *airborne {
                        self.blood.hit != BloodStyle::Original
                    } else {
                        self.blood.ground != BloodStyle::Original || self.blood.retained_ground()
                    }
                }
                Graphic::Bleed | Graphic::GunBleed => {
                    self.blood.hit != BloodStyle::Original && blood_color(color)
                }
                Graphic::Impact(2) => self.blood.hit != BloodStyle::Original,
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
