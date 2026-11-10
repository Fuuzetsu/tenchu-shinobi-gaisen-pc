use crate::settings::{Blood, BloodStyle, Settings};
use tenchu_core::effects::Graphic;

#[derive(Clone, Copy)]
pub struct Style {
    pub sharp_text: bool,
    pub blood: Blood,
    pub lightning: bool,
    pub gunfire: bool,
    pub sparks: Option<u16>,
    pub distant_doors: bool,
    pub alpha: f32,
}

impl Default for Style {
    fn default() -> Self {
        Self {
            sharp_text: false,
            blood: Blood::default(),
            lightning: false,
            gunfire: false,
            sparks: None,
            distant_doors: false,
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
            distant_doors: settings.graphics.distant_doors
                && (!settings.graphics.fog || settings.graphics.fog_distance > 100),
            sparks: settings
                .graphics
                .effects
                .sparks
                .then_some(settings.graphics.effects.spark_density),
            ..Self::default()
        }
    }
}

impl Style {
    fn blood_style(self, graphic: &Graphic, color: [u8; 3]) -> Option<BloodStyle> {
        match graphic {
            Graphic::Blood { airborne, .. } => Some(if *airborne {
                self.blood.hit
            } else {
                self.blood.ground
            }),
            Graphic::Bleed | Graphic::GunBleed if blood_color(color) => Some(self.blood.hit),
            Graphic::Impact(2) => Some(self.blood.hit),
            _ => None,
        }
    }

    pub(super) fn present(
        self,
        graphic: &Graphic,
        color: [u8; 3],
        vertices: &mut [crate::render_mesh::Vertex],
        blend: u8,
    ) -> u8 {
        if self.blood_style(graphic, color) == Some(BloodStyle::OriginalHd) {
            super::combat::blood::original_hd(vertices, blend)
        } else {
            blend
        }
    }

    pub(super) fn replaces(self, graphic: &Graphic, color: [u8; 3]) -> bool {
        (self.lightning && matches!(graphic, Graphic::Bleed) && lightning_spark(color))
            || self.gunfire && matches!(graphic, Graphic::GunImpact(_) | Graphic::GunBleed)
            || self.sparks.is_some()
                && (matches!(graphic, Graphic::Bleed) && metal_spark(color)
                    || matches!(graphic, Graphic::Impact(1)))
            || self.blood_style(graphic, color).is_some_and(|style| {
                !style.uses_original()
                    || matches!(
                        graphic,
                        Graphic::Blood {
                            airborne: false,
                            fading,
                            ..
                        }
                    if self.blood.retained_ground() && *fading)
            })
    }
}

pub(crate) fn metal_spark(color: [u8; 3]) -> bool {
    matches!(color, [255, 255, 0] | [0, 127, 255])
}

pub(super) fn lightning_spark(color: [u8; 3]) -> bool {
    // SetLightning's scatter color is exclusive to its impact sparks.
    color == [255, 255, 120]
}

pub(super) fn blood_color(color: [u8; 3]) -> bool {
    u16::from(color[0]) > u16::from(color[1].max(color[2])) * 2
}
