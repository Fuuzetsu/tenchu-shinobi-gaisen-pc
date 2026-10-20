use crate::source::FileSource;
use std::{ffi::OsString, path::Path};
use tenchu_assets::{Result, executable::Executable, game::GameAssets, stage::MissionAssets};
use tenchu_core::{ai::Difficulty, game::Game};

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub mission: usize,
    pub character: i16,
    pub layout: usize,
    pub difficulty: Difficulty,
    pub language: Option<tenchu_assets::script::Language>,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            mission: 1,
            character: 0,
            layout: 1,
            difficulty: Difficulty::Normal,
            language: None,
        }
    }
}

impl Options {
    pub fn parse(args: &[OsString]) -> Result<Self> {
        let mut options = Self::default();
        for pair in args.chunks(2) {
            if pair.len() != 2 {
                return Err("Launch options require a value".into());
            }
            let value = pair[1].to_str().ok_or("Invalid launch option")?;
            if pair[0] == "--character" {
                options.character = match value {
                    "rikimaru" => 0,
                    "ayame" => 1,
                    _ => return Err("Character must be rikimaru or ayame".into()),
                };
            } else if pair[0] == "--mission" {
                options.mission = if value == "training" {
                    0
                } else {
                    value
                        .parse::<usize>()
                        .ok()
                        .filter(|n| (1..=10).contains(n))
                        .ok_or("Mission must be 1–10 or training")?
                };
            } else if pair[0] == "--difficulty" {
                options.difficulty = match value {
                    "easy" => Difficulty::Easy,
                    "normal" => Difficulty::Normal,
                    "hard" => Difficulty::Hard,
                    _ => return Err("Difficulty must be easy, normal or hard".into()),
                };
            } else if pair[0] == "--layout" {
                options.layout = if value == "random" {
                    0
                } else {
                    value
                        .parse::<usize>()
                        .ok()
                        .filter(|n| (1..=3).contains(n))
                        .ok_or("Layout must be 1–3 or random")?
                };
            } else if pair[0] == "--language" {
                options.language = Some(match value {
                    "english" => tenchu_assets::script::Language::English,
                    "french" => tenchu_assets::script::Language::French,
                    "italian" => tenchu_assets::script::Language::Italian,
                    "japanese" => tenchu_assets::script::Language::Japanese,
                    _ => return Err("Language must be english, french, italian or japanese".into()),
                });
            } else {
                return Err(format!(
                    "Unknown launch option {}",
                    pair[0].to_string_lossy()
                ));
            }
        }
        Ok(options)
    }

    pub fn load(self, assets: &GameAssets<FileSource>) -> Result<Game> {
        let mut inventory = [0; 20];
        inventory[..4].copy_from_slice(&[255, 10, 5, 2]);
        self.load_equipped(assets, 1, inventory)
    }

    pub fn load_equipped(
        self,
        assets: &GameAssets<FileSource>,
        seed: u32,
        mut inventory: [u8; 20],
    ) -> Result<Game> {
        let id = assets
            .stages()?
            .iter()
            .find(|stage| stage.order == self.mission as i32)
            .ok_or("Unknown campaign mission")?
            .id;
        let mission = MissionAssets::load_equipped(
            assets,
            id,
            self.character,
            self.layout,
            self.language.unwrap_or_default(),
            inventory[19],
        )?;
        let mut game = Game::new(
            mission,
            &Executable::new(&assets.executable)?,
            self.character,
            seed,
        )?;
        game.intelligence.difficulty = self.difficulty;
        game.combat.armor = inventory[19] != 0;
        inventory[19] = 0;
        game.roster.get_mut(game.player_id).inventory[..20].copy_from_slice(&inventory);
        Ok(game)
    }
}

pub fn replay(
    assets: &GameAssets<FileSource>,
    options: Options,
    path: &Path,
    explicit_options: bool,
) -> Result<Game> {
    let input = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let mut game = if let Some(start) = crate::recording::Start::parse(&input)? {
        if explicit_options {
            return Err("Recorded sessions supply their own launch options".into());
        }
        start.load(assets)?
    } else {
        options.load(assets)?
    };
    let runs = parse_inputs(&input).map_err(|e| format!("{}:{e}", path.display()))?;
    println!(
        "tick,buttons,x,y,z,vx,vy,vz,yaw,motion,count,loops,attributes,rng,rng_calls,health,clock,cinematic,outcome"
    );
    for (count, buttons) in runs {
        for _ in 0..count {
            game.step(buttons)
                .map_err(|e| format!("Tick {}: {e}", game.tick))?;
            let p = game.player();
            println!(
                "{},{buttons:04x},{},{},{},{},{},{},{},{:04x},{},{},{:04x},{:08x},{},{},{},{},{:?}",
                game.tick,
                p.position[0],
                p.position[1],
                p.position[2],
                p.velocity[0],
                p.velocity[1],
                p.velocity[2],
                p.rotation[1],
                p.motion.action,
                p.motion.count,
                p.motion.loops,
                p.attribute,
                game.random.seed,
                game.random.calls,
                p.health,
                game.clock,
                u8::from(game.cinematic.active),
                game.events.outcome
            );
        }
    }
    Ok(game)
}

pub fn parse_inputs(input: &str) -> Result<Vec<(u32, u16)>> {
    let mut runs = Vec::new();
    for (line, text) in input.lines().enumerate() {
        let text = text.split('#').next().unwrap().trim();
        if text.is_empty() {
            continue;
        }
        let words: Vec<_> = text.split_whitespace().collect();
        let invalid = || format!("{}: expected tick count and hexadecimal buttons", line + 1);
        if words.len() != 2 {
            return Err(invalid());
        }
        let count = words[0].parse::<u32>().map_err(|_| invalid())?;
        let buttons =
            u16::from_str_radix(words[1].trim_start_matches("0x"), 16).map_err(|_| invalid())?;
        runs.push((count, buttons));
    }
    Ok(runs)
}

#[cfg(test)]
pub(crate) mod tests;
