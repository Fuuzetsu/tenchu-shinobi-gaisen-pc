use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Models {
    pub characters: u16,
    pub foliage: u16,
    pub variation: bool,
    pub grapple: bool,
}

impl Models {
    pub fn validate(&mut self) {
        self.characters = self.characters.min(2);
        self.foliage = self.foliage.min(4);
    }
}
