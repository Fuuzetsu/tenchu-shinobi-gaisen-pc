use crate::{random::Random, scene::ModelId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum HeadRest {
    Tumble,
    FaceUp,
    Left,
    Right,
    #[default]
    Varied,
}

impl HeadRest {
    pub const ALL: [Self; 5] = [
        Self::Tumble,
        Self::FaceUp,
        Self::Left,
        Self::Right,
        Self::Varied,
    ];
    pub fn name(self) -> &'static str {
        match self {
            Self::Tumble => "Tumbled pose",
            Self::FaceUp => "Face up",
            Self::Left => "Left side",
            Self::Right => "Right side",
            Self::Varied => "Varied natural poses",
        }
    }
    pub(crate) fn rotation(self, rotation: [i16; 3], root: usize) -> [i16; 3] {
        let choice = if self == Self::Varied {
            Self::ALL[1 + root % 3]
        } else {
            self
        };
        match choice {
            Self::FaceUp => [-1024, rotation[1], 0],
            Self::Left => [0, rotation[1], -1024],
            Self::Right => [0, rotation[1], 1024],
            _ => rotation,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub stealth_head: u16,
    pub dive_head: u16,
    pub arms: u16,
    pub right_arm: u16,
    pub player: bool,
    pub player_head: u16,
    pub player_arms: u16,
    pub player_right_arm: u16,
    pub execution_scene: bool,
    pub head_rest: HeadRest,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            stealth_head: 4,
            dive_head: 4,
            arms: 50,
            right_arm: 50,
            player: false,
            player_head: 4,
            player_arms: 50,
            player_right_arm: 50,
            execution_scene: false,
            head_rest: HeadRest::default(),
        }
    }
}

impl Settings {
    pub fn validate(&mut self) {
        self.stealth_head = self.stealth_head.min(100);
        self.dive_head = self.dive_head.min(100);
        self.arms = self.arms.min(100);
        self.right_arm = self.right_arm.min(100);
        self.player_head = self.player_head.min(100);
        self.player_arms = self.player_arms.min(100);
        self.player_right_arm = self.player_right_arm.min(100);
    }

    pub fn active(self) -> bool {
        self.enabled || self.player || self.execution_scene
    }

    pub(crate) fn head(chance: u16, random: &mut Random) -> bool {
        let roll = random.next15();
        if chance == 4 {
            roll % 25 == 0
        } else {
            roll % 100 < i32::from(chance)
        }
    }

    pub(crate) fn arm(chance: u16, random: &mut Random) -> bool {
        let roll = random.next15();
        if chance == 50 {
            roll & 1 != 0
        } else {
            roll % 100 < i32::from(chance)
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Part {
    pub index: usize,
    pub parent: Option<ModelId>,
    pub velocity: [i16; 3],
    pub rotation: [i16; 3],
    pub moving: bool,
}
