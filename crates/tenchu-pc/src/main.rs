mod audio;
mod campaign;
mod capture;
mod controls;
mod recording;
mod render_effects;
mod render_mesh;
mod renderer;
mod session;
mod source;
mod storage;
mod ui;
mod window;

use tenchu_assets::{Result, executable::Executable, game::GameAssets, stage::MissionAssets};

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || args[0] == "--help" {
        println!(
            "tenchu-pc play DISC [OPTIONS] [--record DIRECTORY]\ntenchu-pc replay DISC INPUT.txt [OPTIONS] [--capture OUTPUT.png]\ntenchu-pc inspect DISC [ASSET]\ntenchu-pc audit DISC\ntenchu-pc render DISC OUTPUT.png\n\nDISC: SLPS-019.01 .cue, raw .bin or .iso.\nOPTIONS: --character rikimaru|ayame --mission 1..10|training --layout 1..3|random --difficulty easy|normal|hard --language english|french|italian|japanese\nplay opens the launcher and equipment selection for the campaign or training. replay reads a recorded session or lines of tick count and hex buttons, and prints a state trace. --record writes a separate input file for each mission attempt.\ninspect lists mission resources; audit validates mission assets and animation; render captures Mission 1."
        );
        return Ok(());
    }
    if args[0] == "play" && args.len() >= 2 {
        let mut launch = Vec::new();
        let mut recordings = None;
        for pair in args[2..].chunks(2) {
            if pair[0] == "--record" {
                if pair.len() != 2 || recordings.is_some() {
                    return Err("Play accepts one --record DIRECTORY".into());
                }
                recordings = Some(std::path::PathBuf::from(&pair[1]));
            } else {
                launch.extend_from_slice(pair);
            }
        }
        let options = session::Options::parse(&launch)?;
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        return window::run(assets, options, args[1].as_ref(), recordings);
    }
    if args[0] == "replay" && args.len() >= 3 {
        let mut launch = Vec::new();
        let mut capture = None;
        for pair in args[3..].chunks(2) {
            if pair[0] == "--capture" {
                if pair.len() != 2 || capture.is_some() {
                    return Err("Replay accepts one --capture OUTPUT.png".into());
                }
                capture = Some(std::path::Path::new(&pair[1]));
            } else {
                launch.extend_from_slice(pair);
            }
        }
        let options = session::Options::parse(&launch)?;
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        let game = session::replay(&assets, options, args[2].as_ref(), !launch.is_empty())?;
        if let Some(path) = capture {
            capture::frame(&game, path)?;
        }
        return Ok(());
    }
    if args[0] == "render" && args.len() == 3 {
        let game = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        return capture::render(&game, args[2].as_ref());
    }
    if args[0] == "audit" && args.len() == 2 {
        let game = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        let exe = Executable::new(&game.executable)?;
        let math = tenchu_core::math::Math {
            tables: exe.math_tables()?,
        };
        println!(
            "{} characters, {} command patterns",
            exe.characters()?.len(),
            exe.commands()?.len()
        );
        for id in 0..11 {
            for character in 0..2 {
                let mission = MissionAssets::load(&game, id, character, 1)?;
                for parts in mission.models.values() {
                    tenchu_core::motion::Pose::new(parts)?;
                }
                for clip in &mission.clips {
                    for track in std::iter::once(&clip.root)
                        .chain(&clip.bones)
                        .filter(|t| !t.is_empty())
                    {
                        for frame in 0..=clip.duration {
                            tenchu_core::motion::sample(
                                track,
                                frame,
                                clip.duration,
                                &math.tables.hermite,
                            )?;
                        }
                    }
                }
                println!(
                    "{} {}: {} triangles, {} enemies, {} clips",
                    mission.stage.directory,
                    if character == 0 { "Rikimaru" } else { "Ayame" },
                    mission.triangle_count(),
                    mission.enemies.len(),
                    mission.clips.len()
                );
            }
        }
        return Ok(());
    }
    if args[0] != "inspect" || !(2..=3).contains(&args.len()) {
        return Err("Usage: tenchu-pc inspect DISC [ASSET] | audit DISC".into());
    }
    let game = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
    println!(
        "Tenchu: Shinobi Gaisen (SLPS-019.01), {:?}",
        game.disc.format()
    );
    println!(
        "MAIN.EXE verified; {} archived assets",
        game.archive.files.len()
    );
    for stage in game.stages()? {
        println!(
            "{:2}  {:12} {:24} start {:?}",
            stage.order, stage.directory, stage.name, stage.start
        );
    }
    if let Some(name) = args.get(2) {
        let name = name.to_str().ok_or("Invalid asset path")?;
        let bytes = game.asset(name)?;
        println!("{name}: {} bytes", bytes.len());
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
