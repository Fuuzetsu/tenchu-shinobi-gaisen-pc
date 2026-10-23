use crate::{
    campaign::{Campaign, Screen},
    render_effects::{self, Geometry},
};
use tenchu_core::progress::{INFINITE, LOCKED, Loadout};

const ITEMS: [&str; 20] = [
    "Grappling hook",
    "Shuriken",
    "Caltrops",
    "Healing potion",
    "Grenade",
    "Smoke bomb",
    "Mine",
    "Poisoned rice",
    "Colored rice",
    "Sleeping gas",
    "Resurrection",
    "Disguise",
    "Protection",
    "Speed scroll",
    "Decoy",
    "Eight-way shuriken",
    "Ninja dog",
    "Fire-eater scroll",
    "Ninja flute",
    "Ninja armour",
];
const RANKS: [&str; 5] = ["Thug", "Novice", "Ninja", "Master Ninja", "Grand Master"];
const WHITE: [f32; 3] = [1.0; 3];
const GOLD: [f32; 3] = [1.0, 0.8, 0.4];
const DIM: [f32; 3] = [0.55, 0.6, 0.65];

fn time(clock: i32) -> String {
    let seconds = clock / 30;
    format!("{}:{:02}", seconds / 60, seconds % 60)
}

fn count(value: u8) -> String {
    match value {
        LOCKED => "Locked".into(),
        INFINITE => "Infinite".into(),
        _ => value.to_string(),
    }
}

pub fn build(
    campaign: &Campaign,
    paused: bool,
    error: Option<&str>,
    text: Option<&mut Vec<crate::fonts::Text>>,
) -> Geometry {
    if campaign.playing() && !paused && error.is_none() {
        return render_effects::build(&campaign.game);
    }
    let mut lines = Vec::new();
    let mut line = |x, y, text: String, color| lines.push((x, y, text, color));
    if let Some(error) = error {
        line(-140, -90, "Gameplay stopped".into(), GOLD);
        for (i, chunk) in error.as_bytes().chunks(42).take(8).enumerate() {
            line(
                -140,
                -60 + i as i16 * 16,
                String::from_utf8_lossy(chunk).into_owned(),
                WHITE,
            );
        }
        line(-140, 92, "F5/West: retry  Esc/East: inventory".into(), DIM);
    } else if paused {
        line(-40, -20, "Paused".into(), GOLD);
        line(-140, 12, "P/Start/South: resume".into(), WHITE);
        line(-140, 32, "Esc/East: abandon attempt".into(), DIM);
        line(-140, 52, "Returns to equipment selection".into(), DIM);
    } else {
        match &campaign.screen {
            Screen::Frontend | Screen::Ending(_) => (),
            Screen::Launcher => {
                line(-140, -102, "TENCHU: SHINOBI GAISEN".into(), GOLD);
                line(-140, -84, "PC port - development build".into(), DIM);
                let options = campaign.options;
                let labels = [
                    if campaign.saved.is_some() {
                        "Continue saved game".into()
                    } else {
                        "Continue (no save yet)".into()
                    },
                    "Start selected mission".into(),
                    format!(
                        "Character: {}",
                        if options.character == 0 {
                            "Rikimaru"
                        } else {
                            "Ayame"
                        }
                    ),
                    format!("Difficulty: {:?}", options.difficulty),
                    if options.mission == 0 {
                        "Mission: Training".into()
                    } else {
                        format!("Mission: {}", options.mission)
                    },
                    if options.layout == 0 {
                        "Layout: Random".into()
                    } else {
                        format!("Layout: {}", options.layout)
                    },
                    format!("Language: {:?}", options.language.unwrap_or_default()),
                ];
                for (i, label) in labels.into_iter().enumerate() {
                    line(
                        -140,
                        -55 + i as i16 * 18,
                        format!("{} {label}", if campaign.cursor == i { ">" } else { " " }),
                        if campaign.cursor == i { GOLD } else { WHITE },
                    );
                }
                line(-140, 88, "Arrows/D-pad/stick: choose/change".into(), DIM);
                line(
                    -140,
                    104,
                    if campaign.frontend {
                        "Enter/South: select  Esc/East: title"
                    } else {
                        "Enter/South: select  Esc/East: quit"
                    }
                    .into(),
                    DIM,
                );
            }
            Screen::Inventory(loadout) => {
                line(-140, -102, "MISSION EQUIPMENT".into(), GOLD);
                let character = if campaign.options.character == 0 {
                    "Rikimaru"
                } else {
                    "Ayame"
                };
                line(
                    -140,
                    -84,
                    if campaign.options.mission == 0 {
                        format!("{character}   Training equipment")
                    } else {
                        format!(
                            "{character}  Items: {}/{}  Types: {}/{}",
                            loadout.taken,
                            loadout.capacity,
                            loadout.kinds,
                            Loadout::KIND_LIMIT
                        )
                    },
                    WHITE,
                );
                line(33, -64, "Take / Stock".into(), DIM);
                let first = campaign.cursor / 8 * 8;
                for (row, &item) in campaign.shop.order.iter().enumerate().skip(first).take(8) {
                    let y = -44 + (row - first) as i16 * 16;
                    let color = if row == campaign.cursor {
                        GOLD
                    } else if loadout.stock[item] == LOCKED {
                        DIM
                    } else {
                        WHITE
                    };
                    line(
                        -140,
                        y,
                        format!(
                            "{} {}",
                            if row == campaign.cursor { ">" } else { " " },
                            ITEMS[item]
                        ),
                        color,
                    );
                    line(
                        62,
                        y,
                        format!(
                            "{} / {}",
                            loadout.selected[item],
                            count(loadout.stock[item])
                        ),
                        color,
                    );
                }
                line(-140, 88, "Arrows/D-pad/stick: item/count".into(), DIM);
                line(-140, 104, "Enter/South: start  Esc/East: back".into(), DIM);
            }
            Screen::Results(results) => {
                line(-140, -102, "MISSION COMPLETE".into(), GOLD);
                line(-140, -80, RANKS[results.score.grade as usize].into(), GOLD);
                for (i, (label, value)) in [
                    ("Stealth kills", results.score.criticals),
                    ("Other kills", results.score.murders),
                    ("Civilian penalty", results.score.friends),
                    ("Undetected bonus", results.score.stealth),
                    ("Total score", results.score.total),
                ]
                .into_iter()
                .enumerate()
                {
                    line(-140, -55 + i as i16 * 18, label.into(), WHITE);
                    line(92, -55 + i as i16 * 18, value.to_string(), WHITE);
                }
                line(
                    -140,
                    40,
                    format!(
                        "Time {}   Best {}{}",
                        time(results.record.clock),
                        results.best.score(campaign.game.mission.stage.id).total,
                        if results.new_best { " (new)" } else { "" }
                    ),
                    WHITE,
                );
                if let Some(place) = results.training_place {
                    line(-140, 59, format!("Training record: #{}", place + 1), GOLD);
                }
                line(
                    -140,
                    88,
                    if campaign.options.mission == 0 {
                        "Enter/South: train  Esc/East: menu".into()
                    } else {
                        "Enter/South: next  R/West: retry".into()
                    },
                    DIM,
                );
                line(-140, 104, "Esc/East: menu  Progress saved".into(), DIM);
            }
            Screen::GameOver => {
                line(-80, -40, "MISSION FAILED".into(), GOLD);
                line(-140, 4, "Enter/South: retry  Esc/East: menu".into(), WHITE);
                line(
                    -130,
                    32,
                    if campaign.options.mission == 0 {
                        "Training equipment resets.".into()
                    } else {
                        "Carried items were lost.".into()
                    },
                    DIM,
                );
            }
            Screen::Complete => {
                line(-112, -50, "CAMPAIGN COMPLETE".into(), GOLD);
                line(-130, -10, "All ten missions completed.".into(), WHITE);
                line(-130, 14, "Ending movie unavailable.".into(), DIM);
                line(-130, 60, "Enter/South: return to launcher".into(), WHITE);
            }
            Screen::Playing => {}
        }
    }
    if let Some(text) = text {
        text.extend(
            lines
                .into_iter()
                .map(|(x, y, text, color)| crate::fonts::Text {
                    position: egui::pos2(f32::from(x), f32::from(y) - 3.0),
                    text,
                    color: crate::fonts::color(color),
                    size: 12.0,
                    width: 300.0 - f32::from(x + 140),
                    centered: false,
                }),
        );
        render_effects::panel(&campaign.game.mission.font, &[])
    } else {
        render_effects::panel(&campaign.game.mission.font, &lines)
    }
}
