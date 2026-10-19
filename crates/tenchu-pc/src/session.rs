use crate::source::FileSource;
use std::{ffi::OsString, path::Path};
use tenchu_assets::{Result, executable::Executable, game::GameAssets, stage::MissionAssets};
use tenchu_core::game::Game;

#[derive(Clone, Copy, Default)]
pub struct Options {
    pub mission: usize,
    pub character: i16,
    pub layout: usize,
}

impl Options {
    pub fn parse(args: &[OsString]) -> Result<Self> {
        let mut options = Self {
            layout: 1,
            ..Self::default()
        };
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
                options.mission = value
                    .parse::<usize>()
                    .ok()
                    .filter(|n| (1..=11).contains(n))
                    .ok_or("Mission must be 1–11 (11 is training)")?
                    - 1;
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
        let mission = MissionAssets::load(assets, self.mission, self.character, self.layout)?;
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
            let p = &game.player;
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
mod tests {
    use super::*;
    use std::time::Duration;
    use tenchu_core::{clock::GameClock, input::*};

    #[test]
    #[ignore = "requires TENCHU_DISC pointing to the private SLPS-019.01 disc"]
    fn retail_movement_is_independent_of_host_frame_cadence() {
        let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
        let tape: Vec<_> = [
            (30, 0),
            (30, UP),
            (1, UP | CROSS),
            (30, UP),
            (30, 0),
            (20, RIGHT),
            (20, UP),
            (30, R1),
            (30, UP | R1),
            (60, 0),
        ]
        .into_iter()
        .flat_map(|(count, input)| std::iter::repeat_n(input, count))
        .collect();
        for character in 0..=1 {
            let options = Options {
                character,
                layout: 1,
                ..Options::default()
            };
            let mut reference = options.load(&assets).unwrap();
            let mut states = Vec::new();
            let mut motions = std::collections::BTreeSet::new();
            for &input in &tape {
                reference.step(input).unwrap();
                motions.insert(reference.player.motion.action);
                states.push(format!(
                    "{:?}",
                    (&reference.player, &reference.random, &reference.effects)
                ));
            }
            assert!(
                motions.iter().any(|motion| motion >> 8 == 9),
                "jump was not reached"
            );
            assert!(
                motions.iter().any(|motion| motion >> 8 == 11),
                "crouch was not reached"
            );
            for fps in [24, 144] {
                let mut game = options.load(&assets).unwrap();
                let mut clock = GameClock::default();
                for frame in 0..fps * 10 {
                    let elapsed = (frame + 1) * 1_000_000_000 / fps - frame * 1_000_000_000 / fps;
                    clock.elapse(Duration::from_nanos(elapsed));
                    while let Some(tick) = clock.step() {
                        if tick as usize == tape.len() {
                            break;
                        }
                        let tick = tick as usize;
                        game.step(tape[tick]).unwrap();
                        assert_eq!(
                            format!("{:?}", (&game.player, &game.random, &game.effects)),
                            states[tick],
                            "character {character}, {fps} Hz, tick {tick}"
                        );
                    }
                    if game.tick as usize == tape.len() {
                        break;
                    }
                }
                assert_eq!(game.tick as usize, tape.len());
            }
        }
    }
}
