use crate::source::FileSource;
use std::{ffi::OsString, path::Path};
use tenchu_assets::{Result, executable::Executable, game::GameAssets, stage::MissionAssets};
use tenchu_core::game::Game;

#[derive(Clone, Copy)]
pub struct Options {
    pub mission: usize,
    pub character: i16,
    pub layout: usize,
}

impl Default for Options {
    fn default() -> Self {
        Self {
            mission: 1,
            character: 0,
            layout: 1,
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
            } else if pair[0] == "--layout" {
                options.layout = value
                    .parse::<usize>()
                    .ok()
                    .filter(|n| (1..=3).contains(n))
                    .ok_or("Layout must be 1–3")?;
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
        let id = assets
            .stages()?
            .iter()
            .find(|stage| stage.order == self.mission as i32)
            .ok_or("Unknown campaign mission")?
            .id;
        let mission = MissionAssets::load(assets, id, self.character, self.layout)?;
        Game::new(
            mission,
            &Executable::new(&assets.executable)?,
            self.character,
            1,
        )
    }
}

pub fn replay(mut game: Game, path: &Path) -> Result<()> {
    let input = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    println!("tick,buttons,x,y,z,vx,vy,vz,yaw,motion,count,loops,attributes,rng,rng_calls");
    for (line, text) in input.lines().enumerate() {
        let text = text.split('#').next().unwrap().trim();
        if text.is_empty() {
            continue;
        }
        let words: Vec<_> = text.split_whitespace().collect();
        let invalid = || {
            format!(
                "{}:{}: expected tick count and hexadecimal buttons",
                path.display(),
                line + 1
            )
        };
        if words.len() != 2 {
            return Err(invalid());
        }
        let count = words[0].parse::<u32>().map_err(|_| invalid())?;
        let buttons =
            u16::from_str_radix(words[1].trim_start_matches("0x"), 16).map_err(|_| invalid())?;
        for _ in 0..count {
            game.step(buttons)
                .map_err(|e| format!("Tick {}: {e}", game.tick))?;
            let p = game.player();
            println!(
                "{},{buttons:04x},{},{},{},{},{},{},{},{:04x},{},{},{:04x},{:08x},{}",
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
                game.random.calls
            );
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests;
