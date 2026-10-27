use crate::source;
use rfd::{FileDialog, MessageDialog, MessageLevel};
use std::path::{Path, PathBuf};
use tenchu_assets::{Result, game::GameAssets};

type Assets = GameAssets<source::DiscSource>;

const DISC_HELP: &str = "This port needs your original Tenchu: Shinobi Gaisen disc (SLPS-019.01, Japan, with English/Japanese/French/Italian).\n\nSelect its .cue file and keep the referenced .bin files beside it. Game assets are not included. Other Tenchu releases are not supported.";

pub fn open(path: &Path) -> Result<Assets> {
    let load = || {
        let assets = GameAssets::open(source::open_disc(path)?)?;
        assets.verify_contents()?;
        Ok(assets)
    };
    load().map_err(|error: String| format!("{}: {error}\n\n{DISC_HELP}", path.display()))
}

pub fn choose() -> Result<Option<(PathBuf, Assets)>> {
    eprintln!("{DISC_HELP}\n\nCommand line: tenchu-pc play '/path/to/game.cue'");
    if cfg!(target_os = "linux")
        && std::env::var_os("DISPLAY").is_none()
        && std::env::var_os("WAYLAND_DISPLAY").is_none()
    {
        return Err("A graphical desktop is needed for the disc picker and game".into());
    }
    MessageDialog::new()
        .set_title("Tenchu — game disc required")
        .set_description(DISC_HELP)
        .show();
    let mut title = "Tenchu — select the Shinobi Gaisen .cue file (keep its .bin files nearby)";
    loop {
        let Some(path) = FileDialog::new()
            .set_title(title)
            .add_filter("Shinobi Gaisen SLPS-019.01 CUE", &["cue", "CUE"])
            .add_filter(
                "Raw disc image (CUE recommended)",
                &["bin", "BIN", "iso", "ISO"],
            )
            .pick_file()
        else {
            return Ok(None);
        };
        match open(&path) {
            Ok(assets) => return Ok(Some((path, assets))),
            Err(error) => {
                eprintln!("{error}");
                show_error(&error);
                title = "Disc unsupported or damaged — select a Shinobi Gaisen SLPS-019.01 .cue";
            }
        }
    }
}

pub fn show_error(error: &str) {
    MessageDialog::new()
        .set_title("Tenchu — unable to start")
        .set_level(MessageLevel::Error)
        .set_description(error)
        .show();
}
