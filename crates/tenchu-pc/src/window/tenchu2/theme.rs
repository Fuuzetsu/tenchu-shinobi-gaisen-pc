use super::*;
#[cfg(any(test, not(target_arch = "wasm32")))]
use crate::audio::theme::Theme;
#[cfg(not(target_arch = "wasm32"))]
use std::path::Path;

#[cfg(test)]
mod tests;

impl Application {
    #[cfg(any(test, not(target_arch = "wasm32")))]
    pub(super) fn install_sandbox_theme(&mut self, name: String, bytes: Vec<u8>) -> Result<()> {
        let theme = Theme::imported(name, bytes)?;
        self.tenchu2.set_theme(theme, self.tenchu2.music)
    }

    #[cfg(not(target_arch = "wasm32"))]
    pub(in crate::window) fn import_sandbox_theme_path(&mut self, path: &Path) -> Result<()> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(path)
            .map_err(|e| e.to_string())?
            .take(crate::level_editor::custom::FILE_LIMIT as u64 + 1)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        let name = path
            .file_name()
            .ok_or("Missing filename")?
            .to_string_lossy()
            .into_owned();
        self.install_sandbox_theme(name, bytes)
    }

    pub(super) fn import_sandbox_theme_file(&mut self) -> Result<()> {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(path) = rfd::FileDialog::new()
            .set_title("Import a sandbox music theme")
            .add_filter("Audio", &["wav", "ogg", "mp3", "flac"])
            .pick_file()
        {
            self.import_sandbox_theme_path(&path)?;
        }
        #[cfg(target_os = "android")]
        crate::android::import("sandbox-theme")?;
        #[cfg(target_arch = "wasm32")]
        return Err(
            "Tenchu 2 scenery sandbox currently requires the desktop or Android game".into(),
        );
        #[allow(unreachable_code)]
        Ok(())
    }
}
