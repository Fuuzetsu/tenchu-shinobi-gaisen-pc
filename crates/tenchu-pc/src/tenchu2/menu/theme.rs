use super::{Event, Menu};
use crate::{audio::theme::Theme, campaign::Campaign, storage::UserFile};
use tenchu_assets::Result;

impl Menu {
    pub(super) fn load_theme(&mut self) {
        match UserFile::SandboxMusic.path() {
            Ok(path) => {
                let (theme, fallback, notice) = Theme::load(&path);
                self.theme = theme;
                self.music = fallback;
                self.theme_notice = notice.unwrap_or_default();
            }
            Err(error) => self.theme_notice = error,
        }
    }

    pub(crate) fn set_theme(&mut self, theme: Theme, fallback: Option<usize>) -> Result<()> {
        theme.save(&UserFile::SandboxMusic.path()?, fallback)?;
        self.theme = theme;
        self.music = fallback;
        self.theme_notice.clear();
        self.notice = if self.theme.enabled {
            "Custom theme ready for the next sandbox start; the complete file repeats.".into()
        } else {
            "The next sandbox start uses the selected disc music or stage ambience.".into()
        };
        Ok(())
    }

    pub(super) fn show_theme(&self, ui: &mut egui::Ui, campaign: &Campaign) -> Option<Event> {
        let mut event = None;
        let label = |row, text: &str| {
            if campaign.cursor == row {
                format!("> {text}")
            } else {
                text.to_owned()
            }
        };
        ui.horizontal_wrapped(|ui| {
            let mut enabled = self.theme.enabled;
            if ui
                .add_enabled(
                    self.theme.asset.is_some(),
                    egui::Checkbox::new(&mut enabled, label(15, "Use custom theme")),
                )
                .changed()
            {
                event = Some(Event::ThemeEnabled(enabled));
            }
            if ui.button(label(16, "Import / replace…")).clicked() {
                event = Some(Event::ImportTheme);
            }
            if ui
                .add_enabled(
                    self.theme.asset.is_some(),
                    egui::Button::new(label(17, "Remove")),
                )
                .clicked()
            {
                event = Some(Event::RemoveTheme);
            }
        });
        if let Some(asset) = &self.theme.asset {
            ui.small(format!("{} · {}", asset.name, asset.description()));
        }
        ui.small("WAV / Ogg Vorbis / MP3 / FLAC · up to 32 MiB. Stored with your preferences; loops the whole file. Applies on Explore area.");
        if !self.theme_notice.is_empty() {
            ui.small(&self.theme_notice);
        }
        if let Some(music) = campaign.sandbox.as_ref().map(|s| &s.music)
            && (self.theme.selected().as_ref().map(|a| a.id().to_owned())
                != music.custom.as_ref().map(|a| a.id().to_owned())
                || self.music != music.selected)
        {
            let disc = music
                .selected
                .and_then(|i| tenchu_assets::tenchu2::music::THEMES.get(i))
                .map_or("stage ambience", |&(_, name)| name);
            let current = music.custom.as_ref().map_or_else(
                || disc.to_owned(),
                |a| format!("{} (disc fallback: {disc})", a.name),
            );
            ui.small(format!(
                "The current stage retains: {current}. Choices above apply on Explore area."
            ));
        }
        event
    }
}
