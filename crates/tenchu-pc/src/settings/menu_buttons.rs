use super::Settings;
use tenchu_core::input::{CIRCLE, CROSS, TRIANGLE};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum MenuButtons {
    #[default]
    CrossCircle,
    CircleCross,
    CrossTriangle,
}

impl From<bool> for MenuButtons {
    fn from(circle_confirm: bool) -> Self {
        if circle_confirm {
            Self::CircleCross
        } else {
            Self::CrossCircle
        }
    }
}

impl MenuButtons {
    pub const ALL: [Self; 3] = [Self::CrossCircle, Self::CircleCross, Self::CrossTriangle];

    pub fn buttons(self) -> (u16, u16) {
        match self {
            Self::CrossCircle => (CROSS, CIRCLE),
            Self::CircleCross => (CIRCLE, CROSS),
            Self::CrossTriangle => (CROSS, TRIANGLE),
        }
    }

    pub fn names(self) -> (&'static str, &'static str) {
        match self {
            Self::CrossCircle => ("Cross", "Circle"),
            Self::CircleCross => ("Circle", "Cross"),
            Self::CrossTriangle => ("Cross", "Triangle"),
        }
    }

    pub fn positions(self) -> (&'static str, &'static str) {
        match self {
            Self::CrossCircle => ("South", "East"),
            Self::CircleCross => ("East", "South"),
            Self::CrossTriangle => ("South", "North"),
        }
    }

    pub fn symbols(self) -> (char, char) {
        match self {
            Self::CrossCircle => ('×', '○'),
            Self::CircleCross => ('○', '×'),
            Self::CrossTriangle => ('×', '△'),
        }
    }

    pub fn source_rows(self) -> (usize, usize) {
        match self {
            Self::CrossCircle => (2, 1),
            Self::CircleCross => (1, 2),
            Self::CrossTriangle => (2, 4),
        }
    }

    pub fn japanese(self, buttons: u16) -> u16 {
        let (confirm, back) = self.buttons();
        let mask = CIRCLE
            | CROSS
            | if self == Self::CrossTriangle {
                TRIANGLE
            } else {
                0
            };
        (buttons & !mask)
            | if buttons & confirm != 0 { CIRCLE } else { 0 }
            | if buttons & back != 0 { CROSS } else { 0 }
    }
}

impl Settings {
    pub fn menu_buttons(&self) -> MenuButtons {
        if self.triangle_cancel {
            MenuButtons::CrossTriangle
        } else {
            self.circle_confirm.into()
        }
    }

    pub fn set_menu_buttons(&mut self, buttons: MenuButtons) {
        self.circle_confirm = buttons == MenuButtons::CircleCross;
        self.triangle_cancel = buttons == MenuButtons::CrossTriangle;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::menu::Item;

    #[test]
    fn legacy_preferences_and_menu_cycle_round_trip_without_gameplay_edits() {
        for (json, expected) in [
            (r#"{}"#, MenuButtons::CrossCircle),
            (r#"{"circle_confirm":true}"#, MenuButtons::CircleCross),
            (
                r#"{"circle_confirm":true,"triangle_cancel":true}"#,
                MenuButtons::CrossTriangle,
            ),
        ] {
            let settings: Settings = serde_json::from_str(json).unwrap();
            assert_eq!(settings.menu_buttons(), expected);
        }
        let mut settings = Settings::default();
        settings.sheathing.enabled = true;
        settings.back_special = true;
        let original = settings.clone();
        for expected in [
            MenuButtons::CircleCross,
            MenuButtons::CrossTriangle,
            MenuButtons::CrossCircle,
        ] {
            assert!(Item::MenuButtons.change_setting(&mut settings, 1));
            assert_eq!(settings.menu_buttons(), expected);
            let json = serde_json::to_vec(&settings).unwrap();
            assert_eq!(serde_json::from_slice::<Settings>(&json).unwrap(), settings);
            let chosen = settings.clone();
            settings.set_visual_preset(crate::settings::presets::MODERN_ULTRA);
            assert_eq!(settings.menu_buttons(), expected);
            settings.retro = true;
            assert_eq!(settings.rendering().menu_buttons(), expected);
            settings = chosen;
        }
        assert_eq!(settings, original);
        Item::MenuButtons.change_setting(&mut settings, -1);
        assert_eq!(settings.menu_buttons(), MenuButtons::CrossTriangle);
        assert!(Item::MenuButtons.reset_setting(&mut settings));
        assert_eq!(settings, original);
    }
}
