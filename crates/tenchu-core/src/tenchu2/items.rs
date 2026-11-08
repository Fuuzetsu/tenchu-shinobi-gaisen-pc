pub mod aim;
pub mod flight;
pub mod grapple;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Items {
    pub aim: aim::Aim,
    pub grapple: Option<grapple::Grapple>,
    pub held: Option<u8>,
}
