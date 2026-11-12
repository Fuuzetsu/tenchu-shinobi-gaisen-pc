use super::{
    artwork::Artwork,
    updates::{Status, UPDATES},
};
use crate::settings::overlay::Edit;

pub fn update_notice(ui: &mut egui::Ui) {
    if let Status::Available(release) = UPDATES.status() {
        ui.horizontal_wrapped(|ui| {
            ui.strong(format!("Update available: {}", release.tag));
            ui.hyperlink_to("Release notes / download", release.url());
        });
    }
}

pub fn update_settings(ui: &mut egui::Ui, enabled: bool, edits: &mut Vec<Edit>) {
    let mut value = enabled;
    if ui.checkbox(&mut value, "Check for game updates at startup").on_hover_text("One background request to GitHub per game launch. Checks stable releases only; never installs game updates.").changed() {
        edits.push(Edit::CheckForUpdates(value));
        UPDATES.start(value);
    }
    match UPDATES.status() {
        Status::NotChecked => {
            ui.weak("Update checks are off.");
        }
        Status::Checking => {
            ui.weak("Checking for a newer stable release…");
            ui.ctx()
                .request_repaint_after(std::time::Duration::from_millis(250));
        }
        Status::Current => {
            ui.weak(format!(
                "No newer stable release found (installed {}).",
                env!("CARGO_PKG_VERSION")
            ));
        }
        Status::Available(_) => {}
        Status::Failed(error) => {
            ui.weak("Could not check for updates. Will try next launch.")
                .on_hover_text(error);
        }
    }
}

pub fn artwork(ui: &mut egui::Ui, tenchu2: bool) {
    ui.weak("Install packs for the games you play. Each pack also supplies that game's imported stages. Replacement artwork controls whether they are shown.");
    let order = if tenchu2 {
        [Artwork::Tenchu2, Artwork::Gaisen]
    } else {
        Artwork::ALL
    };
    for artwork in order {
        let release = artwork.release();
        ui.label(format!(
            "{} · {} · {:.1} MiB",
            artwork.name(),
            release.tag,
            release.bytes as f64 / 1_048_576.0
        ));
        #[cfg(not(target_arch = "wasm32"))]
        download(ui, artwork);
        #[cfg(target_arch = "wasm32")]
        ui.hyperlink_to("Download archive for desktop / Android", release.url());
    }
    #[cfg(target_arch = "wasm32")]
    ui.weak("The browser game uses artwork supplied by its host. Local archive installation is available in desktop and Android builds.");
    ui.hyperlink_to(
        "Installation / manual import",
        format!(
            "{}/blob/master/docs/install.md#add-hd-artwork",
            super::REPOSITORY
        ),
    );
}

#[cfg(not(target_arch = "wasm32"))]
fn download(ui: &mut egui::Ui, artwork: Artwork) {
    use super::download::{DOWNLOADS, Status};
    let active = DOWNLOADS.snapshot();
    let busy = active.as_ref().is_some_and(|(_, status)| status.busy());
    let current = artwork
        .directory()
        .ok()
        .map(|root| super::download::destination(&root, artwork.release()));
    let selected = artwork.installed();
    let installed = current.is_some() && selected == current;
    ui.horizontal(|ui| {
        if ui
            .add_enabled(
                !busy && !installed,
                egui::Button::new(if installed {
                    "Installed"
                } else {
                    "Download and install"
                }),
            )
            .clicked()
        {
            DOWNLOADS.start(artwork);
        }
        if selected.is_some() && ui.add_enabled(!busy, egui::Button::new("Use local pack")).on_hover_text("Switch back to a manually imported or bundled pack. Does not delete downloaded files.").clicked() {
            DOWNLOADS.use_local(artwork);
        }
        if let Some((which, status)) = &active
            && *which == artwork
        {
            match status {
                Status::Connecting => {
                    ui.spinner();
                    ui.label("Connecting…");
                }
                Status::Receiving(bytes) => {
                    ui.add(
                        egui::ProgressBar::new(*bytes as f32 / artwork.release().bytes as f32)
                            .desired_width(150.0)
                            .show_percentage(),
                    );
                }
                Status::Checking => {
                    ui.spinner();
                    ui.label("Checking and unpacking…");
                }
                Status::Installed(path) => {
                    ui.weak("Ready").on_hover_text(path.display().to_string());
                }
                Status::Cancelled => {
                    ui.weak("Cancelled; previous pack kept.");
                }
                Status::Failed(error) => {
                    ui.colored_label(egui::Color32::LIGHT_RED, error);
                }
            }
            if status.busy() && ui.button("Cancel").clicked() {
                DOWNLOADS.cancel();
            }
        }
    });
    if busy {
        ui.ctx()
            .request_repaint_after(std::time::Duration::from_millis(100));
    }
    let variable = match artwork {
        Artwork::Gaisen => "TENCHU_ASSET_PACK",
        Artwork::Tenchu2 => "TENCHU2_ASSET_PACK",
    };
    if std::env::var_os(variable).is_some() {
        ui.weak(format!(
            "{variable} is set and takes priority over downloaded packs."
        ));
    }
}

pub fn artwork_changed(seen: &mut u64) -> bool {
    #[cfg(not(target_arch = "wasm32"))]
    {
        let generation = super::download::DOWNLOADS.generation();
        if generation != *seen {
            *seen = generation;
            return true;
        }
    }
    #[cfg(target_arch = "wasm32")]
    let _ = seen;
    false
}
