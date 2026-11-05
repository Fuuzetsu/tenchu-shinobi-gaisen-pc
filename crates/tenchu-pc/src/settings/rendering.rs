use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct Rendering {
    pub half_resolution_ao: bool,
    pub adaptive_shadows: bool,
    pub cached_blood: bool,
}
