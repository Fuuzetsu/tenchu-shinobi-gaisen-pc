use crate::{capture, demo, session, source, visuals, window};
use tenchu_assets::{Result, executable::Executable, game::GameAssets, stage::MissionAssets};

fn run() -> Result<()> {
    let mut args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || disc_argument(&args[0]) {
        args.insert(0, "play".into());
    }
    if args[0] == "--version" {
        println!(
            "tenchu-pc {} ({})",
            env!("CARGO_PKG_VERSION"),
            option_env!("TENCHU_BUILD_REVISION").unwrap_or("local build")
        );
        return Ok(());
    }
    if args[0] == "--help" {
        println!(
            "tenchu-pc play [DISC] [OPTIONS] [--quick] [--record DIRECTORY]\ntenchu-pc benchmark DISC [--preset NAME] [--output REPORT.json]\ntenchu-pc demo DISC [--clips IDS] [--loop] [--clean]\ntenchu-pc demo --list\ntenchu-pc replay DISC INPUT.txt [OPTIONS] [--capture OUTPUT.png]\ntenchu-pc check-retail DISC UPDATES.ram.gz [UPDATE_LIMIT]\ntenchu-pc prepare-retail DISC UPDATES.ram.gz ARCHIVE.json REPLAY.json\ntenchu-pc record-retail DISC REPLAY.json VIDEO.mp4 [START_FRAME [END_FRAME]]\ntenchu-pc verify DISC\ntenchu-pc inspect DISC [ASSET]\ntenchu-pc audit DISC\ntenchu-pc render DISC OUTPUT.png [OPTIONS]\ntenchu-pc export-assets DISC DIRECTORY [OPTIONS]\ntenchu-pc export-interface DISC DIRECTORY\ntenchu-pc preview-materials DISC DIRECTORY TEXTURE_IDS [OPTIONS]\ntenchu-pc preview-models DISC DIRECTORY MODEL_IDS [OPTIONS] [--outfit normal|armour|alternate] [--background black|grey]\ntenchu-pc preview-sprites DISC DIRECTORY SPRITE_INDICES [OPTIONS]\ntenchu-pc prepare-web DISC DIRECTORY [--artwork]\n\nStart without arguments to reuse the last verified disc, or choose one if it is unavailable. You can also drag its CUE onto the executable.\nDISC: original Shinobi Gaisen SLPS-019.01 .cue with its referenced .bin files. Raw .bin/.iso are accepted, but CUE is recommended for complete audio/movie data. Native play verifies the executable and base artwork hashes.\nOPTIONS: --character rikimaru|ayame --mission 1..10|training --layout 1..3|random --difficulty easy|normal|hard --language english|french|italian|japanese\nplay opens the original menu; F6 opens the quick launcher, or use --quick at launch. F1 opens PC settings. replay reads a recorded session or lines of tick count and hex buttons, and prints a state trace. --record writes a separate input file for each mission attempt.\ninspect lists mission resources; audit validates mission assets and animation; render captures the selected mission after 30 ticks. Explicit TENCHU_SETTINGS applies visual settings to render/replay captures."
        );
        return Ok(());
    }
    if args[0] == "verify" && args.len() == 2 {
        crate::startup::open(args[1].as_ref())?;
        println!(
            "Verified Shinobi Gaisen SLPS-019.01: MAIN.EXE, MENU.EXE, ENDING.EXE and DATA.VOL (SHA-256)."
        );
        return Ok(());
    }
    if args[0] == "record-retail" && (4..=6).contains(&args.len()) {
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        let tape = crate::retail::tape::Tape::open(args[2].as_ref())?;
        let number = |i: usize| {
            args.get(i)
                .map(|s| s.to_string_lossy().parse::<u64>())
                .transpose()
                .map_err(|e| e.to_string())
        };
        return window::retail::record(
            assets,
            args[1].as_ref(),
            tape,
            args[3].as_ref(),
            number(4)?.unwrap_or(0),
            number(5)?,
        );
    }
    if args[0] == "benchmark" {
        if args.len() < 2 || args[1] == "--help" {
            println!(
                "tenchu-pc benchmark DISC [--preset NAME | --auto 30|60|120] [--size WIDTHxHEIGHT] [--seconds 2..120] [--output REPORT.json] [--offscreen]\nRuns fixed scenes through the live renderer, excludes loading and 60 warm-up frames per scene, reports CPU/GPU distributions, and exits. The default uses your current graphics. Keep the window focused; Esc cancels. No preferences or saves are written. --offscreen excludes window presentation and waits for each GPU frame; use it for profiling on a virtual display, not as a prediction of interactive FPS."
            );
            return Ok(());
        }
        let mut benchmark = window::benchmark::Options {
            exit: true,
            ..Default::default()
        };
        let mut arguments = args[2..].iter();
        while let Some(option) = arguments.next() {
            if option == "--offscreen" {
                benchmark.offscreen = true;
                continue;
            }
            let value = arguments
                .next()
                .and_then(|s| s.to_str())
                .ok_or("Benchmark options need a value")?;
            match option.to_str().ok_or("Invalid benchmark option")? {
                "--preset" => {
                    benchmark.preset = Some(
                        crate::settings::presets::NAMES[..crate::settings::presets::CUSTOM as usize]
                            .iter()
                            .position(|name| name.eq_ignore_ascii_case(value))
                            .ok_or("Unknown visual preset name")? as u16,
                    );
                }
                "--seconds" => {
                    benchmark.seconds = value
                        .parse::<u16>()
                        .ok()
                        .filter(|n| (2..=120).contains(n))
                        .ok_or("--seconds needs 2–120 seconds per scene")?
                }
                "--auto" => {
                    benchmark.auto_target = Some(
                        value
                            .parse::<u16>()
                            .ok()
                            .filter(|fps| [30, 60, 120].contains(fps))
                            .ok_or("--auto needs a target of 30, 60 or 120 FPS")?,
                    )
                }
                "--size" => {
                    let (w, h) = value.split_once('x').ok_or("--size needs WIDTHxHEIGHT")?;
                    let dimension = |s: &str| {
                        s.parse::<u32>()
                            .ok()
                            .filter(|n| (320..=7680).contains(n))
                            .ok_or("Benchmark dimensions must be 320–7680 pixels")
                    };
                    benchmark.size = Some([dimension(w)?, dimension(h)?]);
                }
                "--output" => benchmark.output = Some(value.into()),
                other => return Err(format!("Unknown benchmark option {other}")),
            }
        }
        if benchmark.auto_target.is_some() && benchmark.preset.is_some() {
            return Err("Choose --auto or --preset, not both".into());
        }
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        return window::run(
            assets,
            session::Options::default(),
            args[1].as_ref(),
            None,
            None,
            window::Startup {
                benchmark: Some(benchmark),
                ..Default::default()
            },
        );
    }
    if args[0] == "prepare-retail" && args.len() == 5 {
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        return crate::retail::check(
            &assets,
            args[2].as_ref(),
            None,
            Some((args[3].as_ref(), args[4].as_ref())),
        );
    }
    if args[0] == "check-retail" && (args.len() == 3 || args.len() == 4) {
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        let limit = args
            .get(3)
            .map(|v| v.to_string_lossy().parse::<u64>())
            .transpose()
            .map_err(|e| e.to_string())?;
        return crate::retail::check(&assets, args[2].as_ref(), limit, None);
    }
    if args[0] == "prepare-web" && (args.len() == 3 || args.len() == 4 && args[3] == "--artwork") {
        return crate::web_export::export(args[1].as_ref(), args[2].as_ref(), args.len() == 4);
    }
    if args[0] == "export-interface" && args.len() == 3 {
        let assets = GameAssets::open(source::open_disc(args[1].as_ref())?)?;
        return visuals::export_interface(&assets, args[2].as_ref());
    }
    if args[0] == "demo" {
        if args.get(1).is_some_and(|a| a == "--list") {
            return demo::Player::list();
        }
        if args.len() < 2 || args[1] == "--help" {
            println!(
                "tenchu-pc demo DISC [--clips moon,shadows,motion] [--loop | --exit] [--clean] [--mute] [--duration SECONDS] [--catalog FILE] [--video OUTPUT.mp4] [--sound]\n--video exports 3840x2160 at exactly 60 fps with chapters, then exits. Add --sound for synchronized stereo game audio; otherwise the export is silent.\nUse demo --list to list clips. Space pauses; arrows change clip; Home restarts; Tab opens the clip picker; H toggles captions; V toggles transport; F1 inspects temporary settings; F9 captures a PNG; Esc exits."
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
    if args[0] == "play" && args.get(1).is_some_and(|s| s == "--help") {
        println!(
            "tenchu-pc play [DISC.cue] [--quick] [--record DIRECTORY]\nWithout DISC, reuses the last verified disc if available; otherwise opens a file picker. Keep the referenced BIN files with the CUE. Requires original Shinobi Gaisen SLPS-019.01; the executable and base artwork are checked with SHA-256. F1: settings and benchmarks. F6: quick launcher."
        );
        return Ok(());
    }
    if args[0] == "play" {
        let disc = args
            .get(1)
            .filter(|p| !p.to_string_lossy().starts_with("--"));
        let mut launch = Vec::new();
        let mut recordings = None;
        let mut startup = window::Startup::default();
        let mut arguments = args[if disc.is_some() { 2 } else { 1 }..].iter();
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
        let selected = crate::startup::select(disc.map(std::path::Path::new))?;
        let Some((path, assets)) = selected else {
            return Ok(());
        };
        return window::run(assets, options, &path, recordings, None, startup);
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
            capture::frame(&game, path, &assets)?;
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
    let first = std::env::args_os().nth(1);
    let interactive = first.as_ref().is_none_or(|arg| disc_argument(arg))
        || (first.as_ref().is_some_and(|arg| arg == "play")
            && std::env::args_os().nth(2).is_none());
    if let Err(error) = run() {
        eprintln!("{error}");
        if interactive {
            crate::startup::show_error(&error);
        }
        std::process::exit(1);
    }
}

fn disc_argument(argument: &std::ffi::OsStr) -> bool {
    std::path::Path::new(argument).extension().is_some_and(|e| {
        ["cue", "bin", "iso"]
            .iter()
            .any(|s| e.eq_ignore_ascii_case(s))
    })
}
