use super::*;

impl Application {
    pub(super) fn install_unarmed(&mut self, moves: tenchu_assets::tenchu2::Moves) -> Result<()> {
        let bytes = crate::sheathing::imports::encode(&moves)?;
        crate::storage::atomic_write(&crate::storage::UserFile::Tatsumaru.path()?, &bytes)?;
        self.set_sheathing(tenchu_core::game::sheathing::Settings {
            enabled: true,
            unarmed: true,
            unarmed_style: tenchu_core::game::sheathing::UnarmedStyle::Tatsumaru,
            ..self.settings.sheathing
        })?;
        if self.studio_previous.is_none() {
            self.settings_store.write(&self.settings)?;
        }
        self.menu.notice = "Imported Tatsumaru punches and kicks. Sheathe your weapons to use them (H by default).".into();
        self.clock.discard_host_backlog();
        self.last_frame = Instant::now();
        Ok(())
    }

    pub(super) fn import_unarmed(&mut self) -> Result<()> {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            let Some(path) = rfd::FileDialog::new()
                .set_title("Import Tatsumaru moves from your Tenchu 2 disc")
                .add_filter("Tenchu 2 disc or archive", crate::source::DISC_EXTENSIONS)
                .add_filter("Exported moves", &["t2moves"])
                .pick_file()
            else {
                return Ok(());
            };
            self.install_unarmed(crate::sheathing::imports::from_path(&path)?)
        }
        #[cfg(target_os = "android")]
        return crate::android::import("unarmed");
        #[cfg(target_arch = "wasm32")]
        crate::web::recording::start(crate::web::recording::Kind::Unarmed)
    }

    pub(super) fn export_unarmed(&mut self) -> Result<()> {
        if crate::sheathing::imports::export(&crate::sheathing::imports::load()?)? {
            self.menu.notice =
                "Exported imported game data for personal use on your other devices.".into();
        }
        Ok(())
    }
}
