use super::*;
use crate::{
    session::Options,
    source::DiscSource,
    visuals::appearance::library::{Body, Library},
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use tenchu_assets::{
    Result,
    executable::{Executable, MotionRegistration},
    game::GameAssets,
    script::Language,
    stage::MissionAssets,
};
use tenchu_core::{ai::Difficulty, game::Game, ids::MOT_NORMAL};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Run {
    pub document: Document,
    pub round: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub mission: usize,
    pub character: i16,
    pub layout: usize,
    pub difficulty: u8,
    pub player_model: Option<String>,
    pub health: Option<u16>,
    pub inventory: [u8; 20],
    pub sight: u16,
    pub seed: u32,
}

impl Run {
    pub fn new(document: Document) -> Self {
        Self { document, round: 0 }
    }

    pub fn export(&self) -> String {
        serde_json::to_string_pretty(self).unwrap()
    }

    pub fn import(text: &str) -> Result<Self> {
        if text.len() > 65536 {
            return Err("Seed document exceeds 64 KiB".into());
        }
        let run = match serde_json::from_str::<Self>(text) {
            Ok(run) => run,
            Err(_) => Self::new(Document::import(text)?),
        };
        run.document.validate_format()?;
        Ok(run)
    }

    pub fn next(&self) -> Result<Self> {
        Ok(Self {
            document: self.document.clone(),
            round: self
                .round
                .checked_add(1)
                .ok_or("Randomizer round limit reached")?,
        })
    }

    pub fn plan(&self, catalog: &Catalog) -> Result<Plan> {
        let document = &self.document;
        document.validate(catalog)?;
        let rules = &document.rules;
        let random = |domain| document.random(self.round, domain, 0);
        let mut stages: Vec<_> = catalog
            .stages
            .iter()
            .filter(|s| !rules.blocked_stages.contains(&s.0))
            .map(|s| s.0)
            .collect();
        let mission = match rules.stages {
            Stages::Chosen => rules.mission,
            Stages::Random => stages[random("stage").index(stages.len())],
            Stages::Tour => {
                document
                    .random(self.round / stages.len() as u32, "tour", 0)
                    .shuffle(&mut stages);
                stages[self.round as usize % stages.len()]
            }
        };
        let protagonists: Vec<_> = [0, 1]
            .into_iter()
            .filter(|c| !rules.player_pool.blocked.contains(&format!("disc:{c}")))
            .collect();
        let character = if rules.player == Player::Chosen || protagonists.is_empty() {
            rules.character
        } else {
            protagonists[random("protagonist").index(protagonists.len())]
        };
        let player_model = if rules.player == Player::Models {
            let choices = rules.player_pool.choices(catalog);
            Some(
                choices[random("player-model").index(choices.len())]
                    .id
                    .clone(),
            )
        } else {
            None
        };
        let mut inventory = [0; 20];
        if rules.equipment {
            let mut choices: Vec<_> = (0..20)
                .filter(|i| !rules.blocked_items.contains(i) && (*i != 0 || !rules.keep_grapple))
                .collect();
            random("item-choice").shuffle(&mut choices);
            let count =
                usize::from(rules.item_types.sample(&mut random("item-types"))).min(choices.len());
            for &item in choices.iter().take(count) {
                inventory[item] = if item == 19 {
                    1
                } else if item == 0 || rules.infinite_items {
                    255
                } else {
                    rules.item_count.sample(&mut document.random(
                        self.round,
                        "item-count",
                        item as u32,
                    )) as u8
                };
            }
        } else {
            inventory[..4].copy_from_slice(&[0, 10, 5, 2]);
            for &item in &rules.blocked_items {
                inventory[item] = 0;
            }
        }
        if rules.keep_grapple {
            inventory[0] = 255;
        }
        Ok(Plan {
            mission,
            character,
            player_model,
            layout: if rules.layout == 0 {
                random("layout").index(3) + 1
            } else {
                rules.layout
            },
            difficulty: rules
                .difficulty
                .unwrap_or_else(|| random("difficulty").index(3) as u8),
            health: rules.health.map(|r| r.sample(&mut random("player-health"))),
            inventory,
            sight: rules.sight.sample(&mut random("sight")),
            seed: random("game").next() as u32,
        })
    }

    pub fn load(&self, assets: &GameAssets<DiscSource>, library: &Library) -> Result<(Game, Plan)> {
        let language = self.document.rules.language.game();
        let catalog = Catalog::new(library, &assets.stages()?);
        let plan = self.plan(&catalog)?;
        let stage = assets
            .stages()?
            .into_iter()
            .find(|s| s.order == plan.mission as i32)
            .ok_or("Unknown mission")?;
        let mut mission = MissionAssets::load_equipped(
            assets,
            stage.id,
            plan.character,
            plan.layout,
            language,
            plan.inventory[19],
        )?;
        let rules = &self.document.rules;
        if rules.pickups {
            let pool: Vec<_> = (1..19)
                .filter(|i| !rules.blocked_items.contains(i))
                .collect();
            for (i, item) in mission.pickups.iter_mut().enumerate() {
                if item.kind < 19 {
                    item.kind = pool[self
                        .document
                        .random(self.round, "pickup", i as u32)
                        .index(pool.len())];
                }
            }
        }
        let mut game = Game::new(
            mission,
            &Executable::new(&assets.executable)?,
            plan.character,
            plan.seed,
        )?;
        game.intelligence.difficulty = plan.options(language).difficulty;
        game.intelligence.sight_percent = plan.sight;
        game.activation.keep_all_active = rules.keep_active;
        game.combat.armor = plan.inventory[19] != 0;
        let player = game.roster.get_mut(game.player_id);
        player.inventory[..19].copy_from_slice(&plan.inventory[..19]);
        if let Some(health) = plan.health {
            player.health = health as i16;
            player.max_health = health as i16;
        }
        let random = game.random;
        self.enemies(&mut game, library, &catalog)?;
        if rules.model_moveset
            && let Some(body) = plan.player_model.as_ref().and_then(|id| library.get(id))
        {
            game.set_player_moveset(
                body.character.as_ref(),
                &body.clips,
                tenchu_core::game::moveset::Options {
                    player_movement: rules.player_movement,
                    no_idle_taunts: rules.no_idle_taunts,
                },
            )?;
        }
        game.random = random;
        Ok((game, plan))
    }

    fn enemies(&self, game: &mut Game, library: &Library, catalog: &Catalog) -> Result<()> {
        let rules = &self.document.rules;
        let pool = rules.enemy_pool.choices(catalog);
        let protected: BTreeSet<_> = game
            .mission
            .story
            .iter()
            .map(|a| a.kind)
            .chain(
                game.mission
                    .scripts
                    .events
                    .iter()
                    .filter(|e| e.target != 255)
                    .map(|e| i16::from(e.target)),
            )
            .collect();
        let mut imported = BTreeMap::new();
        for id in game.roster.order.clone() {
            let original = game.roster.get(id);
            if id == game.player_id
                || original.health < 0
                || original.kind == 0xa9
                || rules.protect_story && protected.contains(&original.kind)
                || !rules.replace_civilians && original.kind & 0xf0 == 0x90
            {
                continue;
            }
            let mut facing = original.rotation[1];
            if rules.random_facing {
                facing = self
                    .document
                    .random(self.round, "facing", id as u32)
                    .index(4096) as i16;
            }
            if rules.enemies != Enemies::Original {
                let entry = if rules.enemies == Enemies::Uniform {
                    0
                } else {
                    id as u32
                };
                let body = library
                    .get(
                        &pool[self
                            .document
                            .random(self.round, "enemy", entry)
                            .index(pool.len())]
                        .id,
                    )
                    .unwrap();
                let source = import_body(game, library, body, &mut imported)?;
                game.replace_actor(id, source.actor, source.think)?;
                game.roster.get_mut(id).voice_program = source.voice;
            }
            let actor = game.roster.get_mut(id);
            actor.rotation[1] = facing;
            let percentage = rules.enemy_health.sample(&mut self.document.random(
                self.round,
                "enemy-health",
                id as u32,
            ));
            let health =
                (i32::from(actor.max_health) * i32::from(percentage) / 100).clamp(1, 32767) as i16;
            actor.health = health;
            actor.max_health = health;
            actor.initialize_rendered(&game.scene, &game.math);
        }
        for slot in 0..2 {
            let source = if rules.enemies == Enemies::Original {
                None
            } else {
                let (domain, entry) = if rules.enemies == Enemies::Uniform {
                    ("enemy", 0)
                } else {
                    ("reinforcement", slot as u32)
                };
                let choice = self
                    .document
                    .random(self.round, domain, entry)
                    .index(pool.len());
                Some(import_body(
                    game,
                    library,
                    library.get(&pool[choice].id).unwrap(),
                    &mut imported,
                )?)
            };
            game.mission.reinforcement_overrides[slot] =
                Some(tenchu_assets::stage::imports::Reinforcement {
                    actor: source.map(|s| s.actor),
                    voice: source.and_then(|s| s.voice),
                    health_percent: rules.enemy_health.sample(&mut self.document.random(
                        self.round,
                        "reinforcement-health",
                        slot as u32,
                    )),
                });
        }
        Ok(())
    }
}

#[derive(Clone, Copy)]
struct Imported {
    actor: tenchu_assets::stage::imports::Actor,
    voice: Option<i16>,
    think: i16,
}

fn import_body(
    game: &mut Game,
    library: &Library,
    body: &Body,
    imported: &mut BTreeMap<String, Imported>,
) -> Result<Imported> {
    if let Some(&source) = imported.get(&body.id) {
        return Ok(source);
    }
    let behavior = if body.character.is_some() {
        body
    } else {
        library
            .bodies
            .iter()
            .filter(|b| {
                b.character
                    .as_ref()
                    .is_some_and(|c| c.kind & 0xf0 == 0xa0 && c.health > 0)
            })
            .min_by_key(|b| {
                (
                    b.parts.len().abs_diff(body.parts.len()),
                    b.character.as_ref().unwrap().health,
                )
            })
            .ok_or("No animal behavior is available for this resource-only model")?
    };
    let mut character = behavior.character.clone().unwrap();
    if !character.motions.iter().any(|r| r.action == MOT_NORMAL)
        && let Some(idle) = behavior.idle
    {
        character.motions.push(MotionRegistration {
            action: MOT_NORMAL,
            clip: behavior.clips[idle].id,
        });
    }
    character.health = character.health.max(1);
    character.width = character.width.max(1);
    character.turn = character.turn.max(1);
    let actor = game.mission.import_actor(
        character,
        body.parts.clone(),
        &behavior.clips,
        body.vram.clone(),
    )?;
    let voice = behavior.voice.map(|voice| {
        let program = 128 + game.mission.imported_voices.len() as i16;
        game.mission.imported_voices.push(voice);
        program
    });
    let source = Imported {
        actor,
        voice,
        think: behavior.think,
    };
    imported.insert(body.id.clone(), source);
    Ok(source)
}

impl Plan {
    pub fn options(&self, language: Language) -> Options {
        Options {
            mission: self.mission,
            character: self.character,
            layout: self.layout,
            difficulty: [Difficulty::Easy, Difficulty::Normal, Difficulty::Hard]
                [self.difficulty as usize],
            language: Some(language),
        }
    }
}
