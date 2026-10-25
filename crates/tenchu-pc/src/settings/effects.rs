use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Effects {
    pub sparks: bool,
    pub spark_density: u16,
    pub splashes: bool,
    pub scatter: bool,
    pub leaves: bool,
    pub leaf_density: u16,
}
impl Default for Effects {
    fn default() -> Self {
        Self {
            sparks: false,
            spark_density: 100,
            splashes: false,
            scatter: false,
            leaves: false,
            leaf_density: 70,
        }
    }
}
impl Effects {
    pub fn active(&self) -> bool {
        self.sparks || self.splashes || self.scatter || self.leaves
    }
    pub fn validate(&mut self) {
        self.spark_density = self.spark_density.min(300);
        self.leaf_density = self.leaf_density.min(200);
    }
}
