mod config;
mod generate;
mod menu;
mod random;
#[cfg(test)]
mod tests;

pub use config::*;
pub use generate::{Plan, Run};
pub use menu::{Editor, Event};

pub struct MenuState<'a> {
    pub editor: &'a mut Editor,
    pub catalog: Option<Catalog>,
    pub active: Option<(&'a Run, &'a Plan)>,
    pub launcher: bool,
}

use crate::visuals::appearance::library::Library;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Source {
    Gaisen,
    Tenchu2,
}

impl Source {
    pub fn format(self) -> &'static str {
        match self {
            Self::Gaisen => "tenchu-randomizer",
            Self::Tenchu2 => "tenchu2-jp-randomizer",
        }
    }

    pub fn heroes(self) -> &'static [&'static str] {
        match self {
            Self::Gaisen => &["Rikimaru", "Ayame"],
            Self::Tenchu2 => &["Rikimaru", "Ayame", "Tatsumaru"],
        }
    }

    pub fn hero_model(self, hero: i16) -> String {
        match self {
            Self::Gaisen => format!("disc:{hero}"),
            Self::Tenchu2 => format!(
                "tenchu2-jp:{}",
                ["rikimaru", "ayame", "tatumaru"][hero as usize]
            ),
        }
    }

    pub fn item_names(self) -> &'static [&'static str] {
        match self {
            Self::Gaisen => &crate::ui::ITEMS,
            Self::Tenchu2 => &[
                "Grappling hook",
                "Healing potion",
                "Antidote",
                "Unused",
                "Shuriken",
                "Caltrops",
                "Grenade",
                "Mine",
                "Poison dart",
                "Fire arrow",
                "Smoke bomb",
                "Poisoned rice",
                "Coloured rice",
                "Invisibility",
                "Confusion powder",
                "Rice ball",
                "Teleport",
                "Fire breath",
                "Sleeping bomb",
                "Ninja armour",
                "Resurrection leaf",
            ],
        }
    }

    pub fn items(self) -> impl Iterator<Item = (usize, &'static str)> {
        self.item_names()
            .iter()
            .copied()
            .enumerate()
            .filter(move |&(id, _)| self != Self::Tenchu2 || id != 3)
    }

    pub fn consumable(self, item: usize) -> bool {
        item != 0 && item != 19 && self.items().any(|(id, _)| id == item)
    }
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, serde::Serialize, serde::Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Group {
    Players,
    Guards,
    Bosses,
    Civilians,
    Animals,
}

impl Group {
    pub const ALL: [Self; 5] = [
        Self::Players,
        Self::Guards,
        Self::Bosses,
        Self::Civilians,
        Self::Animals,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Self::Players => "Players",
            Self::Guards => "Guards",
            Self::Bosses => "Bosses",
            Self::Civilians => "Civilians",
            Self::Animals => "Animals",
        }
    }

    fn of(kind: i16) -> Self {
        if matches!(kind, 0 | 1) {
            return Self::Players;
        }
        match kind & 0xf0 {
            0 => Self::Civilians,
            0x80 => Self::Bosses,
            0x90 => Self::Civilians,
            0xa0 => Self::Animals,
            _ => Self::Guards,
        }
    }
}

pub struct Entry {
    pub id: String,
    pub name: String,
    pub group: Group,
    pub enemy: bool,
    pub player: bool,
}

pub struct Catalog {
    pub source: Source,
    pub actors: Vec<Entry>,
    pub stages: Vec<(usize, String)>,
}

impl Catalog {
    pub fn new(library: &Library, stages: &[tenchu_assets::game::Stage]) -> Self {
        let mut actors: Vec<_> = library
            .bodies
            .iter()
            .map(|body| Entry {
                id: body.id.clone(),
                name: body.name.clone(),
                group: body
                    .character
                    .as_ref()
                    .map_or(Group::Animals, |c| Group::of(c.kind)),
                enemy: true,
                player: true,
            })
            .collect();
        actors.sort_by(|a, b| a.id.cmp(&b.id));
        let mut stages: Vec<_> = stages
            .iter()
            .map(|s| (s.order as usize, s.name.clone()))
            .collect();
        stages.sort_by_key(|s| s.0);
        Self {
            source: Source::Gaisen,
            actors,
            stages,
        }
    }

    pub fn name(&self, id: &str) -> &str {
        self.actors
            .iter()
            .find(|a| a.id == id)
            .map_or("Unknown model", |a| a.name.as_str())
    }
}
