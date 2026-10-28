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

pub fn select(explicit: Option<&Path>) -> Result<Option<(PathBuf, Assets)>> {
    let remembered = crate::storage::UserFile::LastDisc
        .path()
        .inspect_err(|error| eprintln!("Disc path will not be remembered: {error}"))
        .ok();
    select_with(explicit, remembered.as_deref(), open, choose)
}

fn select_with<T>(
    explicit: Option<&Path>,
    saved: Option<&Path>,
    open: impl Fn(&Path) -> Result<T>,
    choose: impl FnOnce() -> Result<Option<(PathBuf, T)>>,
) -> Result<Option<(PathBuf, T)>> {
    let selected = if let Some(path) = explicit {
        Some((path.to_owned(), open(path)?))
    } else {
        let previous = saved
            .and_then(remembered_path)
            .and_then(|path| match open(&path) {
                Ok(assets) => {
                    println!("Using previous disc: {}", path.display());
                    Some((path, assets))
                }
                Err(error) => {
                    eprintln!("Previous disc could not be opened; choose it again: {error}");
                    None
                }
            });
        match previous {
            Some(previous) => Some(previous),
            None => choose()?,
        }
    };
    if let Some((path, _)) = &selected
        && let Some(saved) = saved
    {
        let remember = || {
            let absolute = std::path::absolute(path).map_err(|e| e.to_string())?;
            let bytes = serde_json::to_vec(&absolute).map_err(|e| e.to_string())?;
            crate::storage::atomic_write(saved, &bytes)
        };
        if let Err(error) = remember() {
            eprintln!("Game disc opened, but its path could not be saved: {error}");
        }
    }
    Ok(selected)
}

fn remembered_path(saved: &Path) -> Option<PathBuf> {
    let load = || {
        let bytes = match crate::storage::read(saved) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(error.to_string()),
        };
        let path: PathBuf = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        Ok((path.is_absolute() && path.is_file()).then_some(path))
    };
    load().unwrap_or_else(|error: String| {
        eprintln!(
            "Could not read previous disc path from {}: {error}",
            saved.display()
        );
        None
    })
}

fn choose() -> Result<Option<(PathBuf, Assets)>> {
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

#[cfg(test)]
mod tests;
