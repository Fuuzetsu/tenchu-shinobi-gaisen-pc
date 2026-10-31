use super::Settings;
use crate::debug::Debug;

pub struct Preset {
    pub name: &'static str,
    pub help: &'static str,
    pub active: bool,
    pub sight: u16,
    pub cutscenes: u16,
    pub practice: bool,
}

pub const PRESETS: [Preset; 5] = [
    Preset {
        name: "Original",
        help: "Original activation, 100% sight and 1x cutscenes. Disables practice tools; inventory is kept.",
        active: false,
        sight: 100,
        cutscenes: 1,
        practice: false,
    },
    Preset {
        name: "Living world",
        help: "All enemies keep thinking. Original sight range, 4x cutscenes and no cheats.",
        active: true,
        sight: 100,
        cutscenes: 4,
        practice: false,
    },
    Preset {
        name: "Keen guards",
        help: "All enemies keep thinking with 125% sight range. 4x cutscenes; no cheats.",
        active: true,
        sight: 125,
        cutscenes: 4,
        practice: false,
    },
    Preset {
        name: "Relaxed stealth",
        help: "All enemies keep thinking with 75% sight range. 4x cutscenes; no cheats.",
        active: true,
        sight: 75,
        cutscenes: 4,
        practice: false,
    },
    Preset {
        name: "Practice",
        help: "During a mission: god mode, never noticed and click-to-teleport. All enemies active, 100% sight; inventory is kept.",
        active: true,
        sight: 100,
        cutscenes: 1,
        practice: true,
    },
];

pub fn current(settings: &Settings, debug: &Debug) -> Option<usize> {
    PRESETS.iter().position(|p| {
        settings.keep_enemies_active == p.active
            && (!p.active || settings.reinforcement_limit == 0)
            && debug.sight_percent == p.sight
            && settings.cutscene_speed == p.cutscenes
            && debug.speed == 100
            && debug.god_mode == p.practice
            && debug.stealth == p.practice
            && debug.click_teleport == p.practice
            && !debug.armor
    })
}
