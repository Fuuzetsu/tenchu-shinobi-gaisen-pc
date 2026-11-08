use crate::{ReadAt, Result, disc::Disc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Edition {
    Europe,
    Japan,
}

pub(super) struct Addresses {
    pub volume_tag: u32,
    pub game_hash: &'static str,
    pub module_hash: &'static str,
    pub module_base: u32,
    pub characters: usize,
    pub common_motions: usize,
    pub cast: usize,
    pub spline_basis: u32,
    pub sine: u32,
    pub atan: u32,
    pub sqrt: u32,
    pub normal: u32,
    pub weapons: u32,
    pub commands: [u32; 3],
    pub collision_rules: u32,
    pub combat_rules: u32,
    pub item_odds: u32,
    pub sequences: u32,
    pub music_path: u32,
    pub music_cues: u32,
    pub ambience_intervals: u32,
}

impl Edition {
    pub fn detect(disc: &Disc<impl ReadAt>) -> Result<Self> {
        let system = disc.find("SYSTEM.CNF")?;
        let bytes = disc.file(&system).read(0, system.size.min(4096) as usize)?;
        let text = String::from_utf8_lossy(&bytes);
        if text.contains("SLES_024.52") {
            Ok(Self::Europe)
        } else if text.contains("SLPS_030.40") {
            Ok(Self::Japan)
        } else {
            Err("Tenchu 2 requires Europe (SLES-02452) or Japan (SLPS-03040)".into())
        }
    }

    pub fn name(self) -> &'static str {
        match self {
            Self::Europe => "Tenchu 2 (Europe)",
            Self::Japan => "Tenchu 2 (Japan)",
        }
    }

    pub fn ticks_per_second(self) -> u32 {
        match self {
            Self::Europe => 25,
            Self::Japan => 30,
        }
    }

    pub fn model_prefix(self) -> &'static str {
        match self {
            Self::Europe => "tenchu2:",
            Self::Japan => "tenchu2-jp:",
        }
    }

    pub(super) fn addresses(self) -> Addresses {
        match self {
            Self::Europe => Addresses {
                volume_tag: 0xa0704707,
                game_hash: "55151e486984c1794ce4f48ab7e91b2d3a5b1abeb7a30ab8d1b326b2606d5422",
                module_hash: "089bdad062d085d8692399c9379bfb62633d81ac17c75c49543aae3ec5ff5773",
                module_base: 0x800ce598,
                characters: 0x6958,
                common_motions: 0x23fc,
                cast: 0x7720,
                spline_basis: 0x8007fe24,
                sine: 0x80086124,
                atan: 0x8008af64,
                sqrt: 0x80086934,
                normal: 0x80086ac8,
                weapons: 0x8007ff2c,
                commands: [0x800814e0, 0x80081530, 0x80081580],
                collision_rules: 0x8008164c,
                combat_rules: 0x8008176c,
                item_odds: 0x8008c7dc,
                sequences: 0x80082bec,
                music_path: 0x8001323c,
                music_cues: 0x8007f354,
                ambience_intervals: 0x80083d24,
            },
            Self::Japan => Addresses {
                volume_tag: 0xa0a28433,
                game_hash: "3bbc636c1d18dba2827b61dab84ae247218d7073eab7482ce382d15402dd829b",
                module_hash: "e4c5075b7757c1649077c159bcae06f8c22935a23720c967a273511a021e281b",
                module_base: 0x800d0610,
                characters: 0x6ed4,
                common_motions: 0x2958,
                cast: 0x7c9c,
                spline_basis: 0x80081d54,
                sine: 0x80088198,
                atan: 0x8008cfd8,
                sqrt: 0x800889a8,
                normal: 0x80088b3c,
                weapons: 0x80081e5c,
                commands: [0x80083410, 0x80083460, 0x800834b0],
                collision_rules: 0x8008357c,
                combat_rules: 0x8008369c,
                item_odds: 0x8008e85c,
                sequences: 0x80084b2c,
                music_path: 0x80013260,
                music_cues: 0x80081244,
                ambience_intervals: 0x80085c78,
            },
        }
    }
}
