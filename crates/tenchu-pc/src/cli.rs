use crate::{capture, demo, session, source, visuals, window};
use tenchu_assets::{Result, executable::Executable, game::GameAssets, stage::MissionAssets};

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || args[0] == "--help" {
        println!(
            "tenchu-pc play DISC [OPTIONS] [--quick] [--record DIRECTORY]\ntenchu-pc demo DISC [--clips IDS] [--loop] [--clean]\ntenchu-pc demo --list\ntenchu-pc replay DISC INPUT.txt [OPTIONS] [--capture OUTPUT.png]\ntenchu-pc inspect DISC [ASSET]\ntenchu-pc audit DISC\ntenchu-pc render DISC OUTPUT.png [OPTIONS]\ntenchu-pc export-assets DISC DIRECTORY [OPTIONS]\ntenchu-pc preview-materials DISC DIRECTORY TEXTURE_IDS [OPTIONS]\ntenchu-pc preview-models DISC DIRECTORY MODEL_IDS [OPTIONS] [--outfit normal|armour|alternate] [--background black|grey]\ntenchu-pc preview-sprites DISC DIRECTORY SPRITE_INDICES [OPTIONS]\ntenchu-pc prepare-web DISC DIRECTORY\n\nDISC: SLPS-019.01 .cue, raw .bin or .iso.\nOPTIONS: --character rikimaru|ayame --mission 1..10|training --layout 1..3|random --difficulty easy|normal|hard --language english|french|italian|japanese\nplay opens the original menu; F6 opens the quick launcher, or use --quick at launch. F1 opens PC settings. replay reads a recorded session or lines of tick count and hex buttons, and prints a state trace. --record writes a separate input file for each mission attempt.\ninspect lists mission resources; audit validates mission assets and animation; render captures the selected mission after 30 ticks. Explicit TENCHU_SETTINGS applies visual settings to render/replay captures."
        );
        return Ok(());
    }
    if args[0] == "prepare-web" && args.len() == 3 {
        return crate::web_export::export(args[1].as_ref(), args[2].as_ref());
    }
    if args[0] == "demo" {
        if args.get(1).is_some_and(|a| a == "--list") {
            return demo::Player::list();
        }
        if args.len() < 2 || args[1] == "--help" {
            println!(
                "tenchu-pc demo DISC [--clips moon,shadows,motion] [--loop | --exit] [--clean] [--mute] [--duration SECONDS] [--catalog FILE] [--video OUTPUT.mp4]\n--video exports silent 3840x2160 at exactly 60 fps with chapters, then exits.\nUse demo --list to list clips. Space pauses; arrows change clip; Home restarts; Tab opens the clip picker; H toggles captions; V toggles transport; F1 inspects temporary settings; F9 captures a PNG; Esc exits."
            );
            return Ok(());
        }
        let player = demo::Player::parse(&args[2..])?;
        let options = session::Options {
            mission: player.clip().mission,
            character: player.clip().character,
            ..Default::default()
        };
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        return window::run(
            assets,
            options,
            args[1].as_ref(),
            None,
            Some(player),
            window::Startup::default(),
        );
    }
    if args[0] == "preview-materials" && args.len() >= 4 {
        let mut launch = Vec::new();
        let mut ticks = 0;
        for pair in args[4..].chunks(2) {
            if pair[0] == "--ticks" {
                ticks = pair
                    .get(1)
                    .and_then(|s| s.to_str())
                    .and_then(|s| s.parse::<u32>().ok())
                    .filter(|&n| n <= 108_000)
                    .ok_or("--ticks requires 0–108000")?;
            } else {
                launch.extend_from_slice(pair);
            }
        }
        let options = session::Options::parse(&launch)?;
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        let ids = args[3].to_str().ok_or("Texture identities must be UTF-8")?;
        return capture::materials::render(
            &assets,
            args[2].as_ref(),
            &ids.split(',').collect::<Vec<_>>(),
            options,
            ticks,
        );
    }
    if args[0] == "preview-sprites" && args.len() >= 4 {
        let options = session::Options::parse(&args[4..])?;
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        let game = options.load(&assets)?;
        let indices = args[3].to_str().ok_or("Sprite indices must be UTF-8")?;
        return capture::sprites::render(
            &game.mission,
            args[2].as_ref(),
            &indices.split(',').collect::<Vec<_>>(),
        );
    }
    if (args[0] == "export-assets" && args.len() >= 3)
        || (args[0] == "preview-models" && args.len() >= 4)
    {
        let preview = args[0] == "preview-models";
        let start = if preview { 4 } else { 3 };
        let mut launch = Vec::new();
        let mut armour = 0;
        let mut grey_background = false;
        for pair in args[start..].chunks(2) {
            if pair[0] == "--outfit" {
                armour = match pair.get(1).and_then(|s| s.to_str()) {
                    Some("normal") => 0,
                    Some("armour") => 1,
                    Some("alternate") => 255,
                    _ => return Err("--outfit requires normal, armour or alternate".into()),
                };
            } else if preview && pair[0] == "--background" {
                grey_background = match pair.get(1).and_then(|s| s.to_str()) {
                    Some("black") => false,
                    Some("grey") => true,
                    _ => return Err("--background requires black or grey".into()),
                };
            } else {
                launch.extend_from_slice(pair);
            }
        }
        let options = session::Options::parse(&launch)?;
        if armour == 255 && options.character != 1 {
            return Err("The alternate outfit is only available for Ayame".into());
        }
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        let mut inventory = [0; 20];
        inventory[19] = armour;
        let game = options.load_equipped(&assets, 1, inventory)?;
        return if preview {
            let ids = args[3].to_str().ok_or("Model identities must be UTF-8")?;
            capture::models::render(
                &game,
                args[2].as_ref(),
                &ids.split(',').collect::<Vec<_>>(),
                grey_background,
            )
        } else {
            visuals::export(&game.mission, args[2].as_ref())
        };
    }
    if args[0] == "play" && args.len() >= 2 {
        let mut launch = Vec::new();
        let mut recordings = None;
        let mut startup = window::Startup::default();
        let mut arguments = args[2..].iter();
        while let Some(key) = arguments.next() {
            if key == "--quick" {
                startup.quick_launcher = true;
                continue;
            }
            let value = arguments.next().ok_or("Launch options require a value")?;
            if key == "--record" {
                if recordings.is_some() {
                    return Err("Play accepts one --record DIRECTORY".into());
                }
                recordings = Some(std::path::PathBuf::from(value));
            } else {
                launch.extend([key.clone(), value.clone()]);
            }
        }
        let options = session::Options::parse(&launch)?;
        startup.difficulty = launch
            .chunks(2)
            .any(|p| p[0] == "--difficulty")
            .then_some(options.difficulty);
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        return window::run(assets, options, args[1].as_ref(), recordings, None, startup);
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
    if args[0] == "render" && args.len() >= 3 {
        let game = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        return capture::render(
            &game,
            args[2].as_ref(),
            session::Options::parse(&args[3..])?,
        );
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

pub fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
