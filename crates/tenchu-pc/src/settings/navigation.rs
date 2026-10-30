use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Navigation {
    pub camera_relative: bool,
    pub mouse_sensitivity: u16,
    pub stick_sensitivity: u16,
    pub deadzone: u16,
    pub invert_y: bool,
    pub distance: u16,
}

impl Default for Navigation {
    fn default() -> Self {
        Self {
            camera_relative: false,
            mouse_sensitivity: 100,
            stick_sensitivity: 100,
            deadzone: 18,
            invert_y: false,
            distance: 48,
        }
    }
}

impl Navigation {
    pub(super) fn validate(&mut self) {
        self.mouse_sensitivity = self.mouse_sensitivity.clamp(10, 300);
        self.stick_sensitivity = self.stick_sensitivity.clamp(10, 300);
        self.deadzone = self.deadzone.clamp(5, 40);
        self.distance = self.distance.clamp(20, 100);
    }
}
