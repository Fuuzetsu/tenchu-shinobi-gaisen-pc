mod source;

use tenchu_assets::{Result, game::GameAssets};

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.is_empty() || args[0] == "--help" {
        println!(
            "tenchu-pc inspect DISC [ASSET]\n\nDISC: SLPS-019.01 .cue, raw .bin or .iso.\nLists mission resources; ASSET reads a named file from DATA.VOL."
        );
        return Ok(());
    }
    if args[0] != "inspect" || !(2..=3).contains(&args.len()) {
        return Err("Usage: tenchu-pc inspect DISC [ASSET]".into());
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
