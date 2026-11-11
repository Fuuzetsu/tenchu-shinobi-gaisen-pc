use super::{Catalog, Entry, Group, Source, random::Random};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use tenchu_assets::Result;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Document {
    pub format: String,
    pub version: u8,
    pub seed: String,
    pub rules: Rules,
}

impl Default for Document {
    fn default() -> Self {
        Self {
            format: "tenchu-randomizer".into(),
            version: 1,
            seed: "morning-ninja".into(),
            rules: Rules::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Stages {
    Chosen,
    Random,
    Tour,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Player {
    Chosen,
    Protagonists,
    Models,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Enemies {
    Original,
    Mixed,
    Uniform,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Language {
    English,
    French,
    Italian,
    Japanese,
}

impl Language {
    pub fn game(self) -> tenchu_assets::script::Language {
        match self {
            Self::English => tenchu_assets::script::Language::English,
            Self::French => tenchu_assets::script::Language::French,
            Self::Italian => tenchu_assets::script::Language::Italian,
            Self::Japanese => tenchu_assets::script::Language::Japanese,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Range {
    pub min: u16,
    pub max: u16,
}

impl Range {
    pub const fn new(min: u16, max: u16) -> Self {
        Self { min, max }
    }

    pub fn sample(self, random: &mut Random) -> u16 {
        self.min + random.index(usize::from(self.max - self.min) + 1) as u16
    }

    fn validate(self, name: &str, min: u16, max: u16) -> Result<()> {
        if self.min < min || self.max > max || self.min > self.max {
            return Err(format!("{name}: use an ordered range within {min}–{max}"));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Pool {
    pub groups: BTreeSet<Group>,
    pub blocked: BTreeSet<String>,
}

impl Pool {
    pub fn all() -> Self {
        Self {
            groups: Group::ALL.into(),
            blocked: BTreeSet::new(),
        }
    }

    pub fn allows(&self, entry: &Entry) -> bool {
        self.groups.contains(&entry.group) && !self.blocked.contains(&entry.id)
    }

    pub fn choices<'a>(&self, catalog: &'a Catalog) -> Vec<&'a Entry> {
        self.choices_for(catalog, false)
    }

    pub fn choices_for<'a>(&self, catalog: &'a Catalog, enemy: bool) -> Vec<&'a Entry> {
        catalog
            .actors
            .iter()
            .filter(|entry| self.allows(entry) && if enemy { entry.enemy } else { entry.player })
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    pub stages: Stages,
    pub mission: usize,
    pub blocked_stages: BTreeSet<usize>,
    pub layout: usize,
    pub difficulty: Option<u8>,
    pub language: Language,
    pub player: Player,
    pub character: i16,
    pub player_pool: Pool,
    pub model_moveset: bool,
    pub player_movement: bool,
    pub no_idle_taunts: bool,
    pub health: Option<Range>,
    pub equipment: bool,
    pub item_types: Range,
    pub item_count: Range,
    pub infinite_items: bool,
    pub keep_grapple: bool,
    pub blocked_items: BTreeSet<usize>,
    pub pickups: bool,
    pub enemies: Enemies,
    pub enemy_pool: Pool,
    pub replace_civilians: bool,
    pub protect_story: bool,
    pub enemy_health: Range,
    pub random_facing: bool,
    pub sight: Range,
    pub keep_active: bool,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            stages: Stages::Chosen,
            mission: 1,
            blocked_stages: [0].into(),
            layout: 0,
            difficulty: Some(1),
            language: Language::English,
            player: Player::Protagonists,
            character: 1,
            player_pool: Pool::all(),
            model_moveset: true,
            player_movement: true,
            no_idle_taunts: true,
            health: Some(Range::new(75, 150)),
            equipment: true,
            item_types: Range::new(3, 6),
            item_count: Range::new(2, 8),
            infinite_items: false,
            keep_grapple: true,
            blocked_items: BTreeSet::new(),
            pickups: true,
            enemies: Enemies::Mixed,
            enemy_pool: Pool {
                groups: [Group::Guards].into(),
                blocked: BTreeSet::new(),
            },
            replace_civilians: false,
            protect_story: true,
            enemy_health: Range::new(75, 125),
            random_facing: false,
            sight: Range::new(100, 100),
            keep_active: false,
        }
    }
}

impl Document {
    pub fn for_source(source: Source) -> Self {
        let mut document = Self {
            format: source.format().into(),
            ..Default::default()
        };
        if source == Source::Tenchu2 {
            document.rules.language = Language::Japanese;
        }
        document
    }

    pub fn source(&self) -> Result<Source> {
        match self.format.as_str() {
            "tenchu-randomizer" => Ok(Source::Gaisen),
            "tenchu2-jp-randomizer" => Ok(Source::Tenchu2),
            _ => Err("Unknown randomizer game format".into()),
        }
    }

    #[cfg(test)]
    pub fn export(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    pub fn import(text: &str) -> Result<Self> {
        if text.len() > 65536 {
            return Err("Seed document exceeds 64 KiB".into());
        }
        let document: Self =
            serde_json::from_str(text).map_err(|e| format!("Invalid seed document: {e}"))?;
        document.validate_format()?;
        Ok(document)
    }

    pub(crate) fn validate_format(&self) -> Result<()> {
        if self.source().is_err() || self.version != 1 {
            return Err(
                "Unsupported randomizer format/version; use a version 1 seed document".into(),
            );
        }
        if self.seed.trim().is_empty()
            || self.seed.len() > 128
            || self.seed.chars().any(char::is_control)
        {
            return Err("Seed must contain 1–128 bytes of printable text".into());
        }
        Ok(())
    }

    pub fn fingerprint(&self) -> String {
        let hash = Sha256::digest(serde_json::to_vec(self).unwrap());
        hash[..6].iter().map(|byte| format!("{byte:02x}")).collect()
    }

    pub fn random(&self, round: u32, domain: &str, entry: u32) -> Random {
        if self.format == Source::Tenchu2.format() {
            Random::new(&self.seed, round, &format!("t2-{domain}"), entry)
        } else {
            Random::new(&self.seed, round, domain, entry)
        }
    }

    pub fn validate(&self, catalog: &Catalog) -> Result<()> {
        self.validate_format()?;
        let source = self.source()?;
        if source != catalog.source {
            return Err("This seed is for the other game. Open its randomizer to use it.".into());
        }
        let r = &self.rules;
        if !catalog.stages.iter().any(|stage| stage.0 == r.mission)
            || r.layout > 3
            || r.character < 0
            || r.character as usize >= source.heroes().len()
            || r.difficulty.is_some_and(|d| d > 2)
        {
            return Err("Invalid mission, layout, protagonist or difficulty".into());
        }
        if r.blocked_stages
            .iter()
            .any(|&s| !catalog.stages.iter().any(|stage| stage.0 == s))
            || r.blocked_items
                .iter()
                .any(|&i| !source.items().any(|(id, _)| id == i))
        {
            return Err("The blacklist contains an unknown stage or item".into());
        }
        if r.blocked_stages.len() == catalog.stages.len()
            || r.stages == Stages::Chosen && r.blocked_stages.contains(&r.mission)
        {
            return Err("Allow at least one stage, including the chosen stage".into());
        }
        for (name, pool, needed, enemy) in [
            ("player", &r.player_pool, r.player == Player::Models, false),
            ("enemy", &r.enemy_pool, r.enemies != Enemies::Original, true),
        ] {
            if pool
                .blocked
                .iter()
                .any(|id| !catalog.actors.iter().any(|a| a.id == *id))
            {
                return Err(format!("The {name} blacklist contains an unknown model ID"));
            }
            if needed && pool.choices_for(catalog, enemy).is_empty() {
                return Err(format!("Allow at least one {name} model"));
            }
        }
        if r.player == Player::Protagonists
            && (0..source.heroes().len()).all(|hero| {
                r.player_pool
                    .blocked
                    .contains(&source.hero_model(hero as i16))
            })
        {
            return Err("Allow at least one protagonist in the player pool".into());
        }
        if r.player == Player::Chosen
            && r.player_pool
                .blocked
                .contains(&source.hero_model(r.character))
        {
            return Err("Allow the chosen protagonist in the player pool".into());
        }
        if let Some(health) = r.health {
            health.validate("Player HP", 1, 999)?;
        }
        r.enemy_health.validate("Enemy HP percentage", 1, 400)?;
        r.sight.validate("Detection percentage", 0, 400)?;
        r.item_types.validate("Item types", 0, 20)?;
        r.item_count.validate("Item quantity", 1, 99)?;
        if r.equipment
            && r.item_types.min as usize
                > source
                    .items()
                    .map(|(id, _)| id)
                    .filter(|i| !r.blocked_items.contains(i) && (*i != 0 || !r.keep_grapple))
                    .count()
        {
            return Err("Not enough allowed items for the minimum loadout size".into());
        }
        if r.keep_grapple && r.blocked_items.contains(&0) {
            return Err("Allow the grappling hook or turn off Guaranteed grapple".into());
        }
        if r.pickups
            && source
                .items()
                .map(|(id, _)| id)
                .filter(|&i| source.consumable(i))
                .all(|i| r.blocked_items.contains(&i))
        {
            return Err("Allow at least one consumable for randomized pickups".into());
        }
        Ok(())
    }
}
