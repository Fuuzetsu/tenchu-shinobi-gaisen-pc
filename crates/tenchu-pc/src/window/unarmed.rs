use super::*;
use tenchu_core::game::sheathing::UnarmedStyle;

impl Application {
    pub(super) fn install_unarmed(&mut self, moves: tenchu_assets::tenchu2::Moves) -> Result<()> {
        let bytes = crate::sheathing::imports::encode(&moves)?;
        if self.campaign.game.player().status == tenchu_core::actor::State::Attack
            && self.campaign.game.player().attribute & tenchu_core::ids::ATTR_WEAPON_DRAWN == 0
        {
            return Err("Finish the unarmed attack before changing its moveset".into());
        }
        let style = UnarmedStyle::from_moves(&moves);
        crate::storage::atomic_write(&crate::sheathing::imports::file(style).path()?, &bytes)?;
        let hero = self.campaign.game.player().kind as usize;
        let previous = self.settings.hero_unarmed_styles[hero];
        if previous.is_some() {
            self.settings.hero_unarmed_styles[hero] = Some(style);
        }
        if let Err(error) = self.set_sheathing(tenchu_core::game::sheathing::Settings {
            enabled: true,
            unarmed: true,
            unarmed_style: if previous.is_some() {
                self.settings.sheathing.unarmed_style
            } else {
                style
            },
            ..self.settings.sheathing
        }) {
            self.settings.hero_unarmed_styles[hero] = previous;
            return Err(error);
        }
        self.campaign.game.set_player_t2_unarmed(&moves)?;
        if self.studio_previous.is_none() {
            self.settings_store.write(&self.settings)?;
        }
        self.menu.notice = format!(
            "Imported {} punches and kicks. Sheathe your weapons to use them (H by default).",
            style.name()
        );
        self.clock.discard_host_backlog();
        self.last_frame = Instant::now();
        Ok(())
    }

    pub(super) fn import_unarmed(&mut self, style: UnarmedStyle) -> Result<()> {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        {
            let Some(path) = rfd::FileDialog::new()
                .set_title(format!(
                    "Import {} moves from your Tenchu 2 disc",
                    style.name()
                ))
                .add_filter("Tenchu 2 disc or archive", crate::source::DISC_EXTENSIONS)
                .add_filter("Exported moves", &["t2moves"])
                .pick_file()
            else {
                return Ok(());
            };
            self.install_unarmed(crate::sheathing::imports::from_path(&path, style)?)
        }
        #[cfg(any(target_arch = "wasm32", target_os = "android"))]
        let _ = style;
        #[cfg(target_os = "android")]
        return crate::android::import("unarmed");
        #[cfg(target_arch = "wasm32")]
        crate::web::recording::start(crate::web::recording::Kind::Unarmed)
    }

    pub(super) fn export_unarmed(&mut self) -> Result<()> {
        if crate::sheathing::imports::export(&crate::sheathing::imports::load(
            self.settings
                .sheathing_for(self.campaign.game.player().kind as usize)
                .unarmed_style,
        )?)? {
            self.menu.notice =
                "Exported imported game data for personal use on your other devices.".into();
        }
        Ok(())
    }
}
