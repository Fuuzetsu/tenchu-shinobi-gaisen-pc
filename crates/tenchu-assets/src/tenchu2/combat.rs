use super::{Edition, executable};
use crate::{ReadAt, Result, disc::Disc, executable::Executable, i16_le, u32_le};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rules {
    pub probability: [i32; 4],
    pub reach: [i16; 6],
    pub close: [i16; 6],
    pub item_odds: [i16; 3],
    pub alert_duration: [i16; 3],
    pub alert_reset_duration: [i16; 3],
}

impl Rules {
    pub fn load(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let (edition, bytes) = executable(disc)?;
        if edition != Edition::Japan {
            return Err("Native Tenchu 2 AI requires the Japanese disc".into());
        }
        let executable = Executable::new(&bytes)?;
        let addresses = edition.addresses();
        let table = executable.bytes(addresses.combat_rules, 40)?;
        let odds = executable.bytes(addresses.item_odds, 6)?;
        let alerts = executable.bytes(addresses.alert_rules, 22)?;
        let mut rules = Self {
            probability: [0; 4],
            reach: [0; 6],
            close: [0; 6],
            item_odds: [0; 3],
            alert_duration: [0; 3],
            alert_reset_duration: [0; 3],
        };
        for (index, value) in rules.probability.iter_mut().enumerate() {
            *value = u32_le(table, index * 4)? as i32;
        }
        for index in 0..6 {
            rules.reach[index] = i16_le(table, 16 + index * 2)?;
            rules.close[index] = i16_le(table, 26 + index * 2)?;
        }
        for (index, value) in rules.item_odds.iter_mut().enumerate() {
            *value = i16_le(odds, index * 2)?;
            if *value <= 0 {
                return Err("Invalid Tenchu 2 item probability".into());
            }
        }
        for index in 0..3 {
            rules.alert_duration[index] = i16_le(alerts, index * 2)?;
            rules.alert_reset_duration[index] = i16_le(alerts, 16 + index * 2)?;
        }
        Ok(rules)
    }
}
