use super::{Graphics, Settings, presets};
use crate::storage;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tenchu_assets::Result;

pub const AUTO: &str = "Auto-optimised";
const LIMIT: usize = 128;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct Visual {
    pub graphics: Graphics,
    pub sharp_text: bool,
}

impl Visual {
    pub fn capture(settings: &Settings) -> Self {
        Self {
            graphics: settings.graphics.clone(),
            sharp_text: settings.sharp_text,
        }
    }

    pub fn apply(&self, settings: &mut Settings) {
        settings.graphics.clone_from(&self.graphics);
        settings.sharp_text = self.sharp_text;
        settings.retro = false;
    }

    fn matches(&self, settings: &Settings) -> bool {
        self.graphics == settings.graphics && self.sharp_text == settings.sharp_text
    }
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Auto {
    pub visual: Visual,
    pub quality: String,
    pub adapter: String,
    pub size: [u32; 2],
    pub target_fps: u16,
    pub met_target: bool,
    pub note: String,
}

#[derive(Clone, Serialize, Deserialize)]
pub struct Named {
    pub name: String,
    pub visual: Visual,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Library {
    pub auto: Option<Auto>,
    pub custom: Vec<Named>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Builtin(u16),
    Auto,
    User(usize),
}

impl Library {
    fn validate(&mut self) -> Result<()> {
        if self.custom.len() > LIMIT {
            return Err(format!("At most {LIMIT} user presets are supported"));
        }
        for i in 0..self.custom.len() {
            self.custom[i].name = self.name(&self.custom[i].name, Some(i))?;
            self.custom[i].visual.graphics.validate();
        }
        if let Some(auto) = &mut self.auto {
            auto.visual.graphics.validate();
        }
        Ok(())
    }

    fn name(&self, name: &str, replacing: Option<usize>) -> Result<String> {
        let name = name.trim();
        if name.is_empty() || name.chars().count() > 64 || name.chars().any(char::is_control) {
            return Err("Use a name of 1–64 visible characters".into());
        }
        if presets::NAMES
            .iter()
            .chain([&AUTO])
            .any(|n| n.eq_ignore_ascii_case(name))
        {
            return Err("That name belongs to a built-in preset".into());
        }
        if self
            .custom
            .iter()
            .enumerate()
            .any(|(i, p)| Some(i) != replacing && p.name.eq_ignore_ascii_case(name))
        {
            return Err("A user preset already has that name".into());
        }
        Ok(name.into())
    }

    pub fn choices(&self) -> Vec<(Choice, String)> {
        let mut entries: Vec<_> = (0..presets::CUSTOM)
            .map(|i| (Choice::Builtin(i), presets::NAMES[i as usize].into()))
            .collect();
        if self.auto.is_some() {
            entries.push((Choice::Auto, AUTO.into()));
        }
        entries.extend(
            self.custom
                .iter()
                .enumerate()
                .map(|(i, p)| (Choice::User(i), p.name.clone())),
        );
        entries
    }

    pub fn current(&self, settings: &Settings) -> Option<Choice> {
        if self
            .auto
            .as_ref()
            .is_some_and(|p| p.visual.matches(settings))
        {
            return Some(Choice::Auto);
        }
        if let Some(i) = self.custom.iter().position(|p| p.visual.matches(settings)) {
            return Some(Choice::User(i));
        }
        let index = settings.graphics.preset();
        (index < presets::CUSTOM).then_some(Choice::Builtin(index))
    }

    pub fn apply(&self, choice: Choice, settings: &mut Settings) -> Result<()> {
        match choice {
            Choice::Builtin(index) if index < presets::CUSTOM => settings.set_visual_preset(index),
            Choice::Auto => self
                .auto
                .as_ref()
                .ok_or("Run automatic optimisation first")?
                .visual
                .apply(settings),
            Choice::User(index) => self
                .custom
                .get(index)
                .ok_or("User preset no longer exists")?
                .visual
                .apply(settings),
            _ => return Err("Unknown visual preset".into()),
        }
        Ok(())
    }
}

pub struct Manager {
    pub library: Library,
    pub open: bool,
    path: PathBuf,
    target: Option<usize>,
    name: String,
    notice: String,
}

impl Manager {
    pub fn new(path: PathBuf) -> Self {
        let loaded = match storage::read(&path) {
            Ok(bytes) => serde_json::from_slice::<Library>(&bytes)
                .map_err(|e| e.to_string())
                .and_then(|mut library| {
                    library.validate()?;
                    Ok(library)
                }),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Library::default()),
            Err(e) => Err(e.to_string()),
        };
        let (library, notice) = match loaded {
            Ok(library) => (library, String::new()),
            Err(e) => (
                Library::default(),
                format!("Could not read {}: {e}", path.display()),
            ),
        };
        Self {
            library,
            path,
            open: false,
            target: None,
            name: String::new(),
            notice,
        }
    }

    fn replace(&mut self, mut library: Library) -> Result<()> {
        library.validate()?;
        storage::atomic_write(
            &self.path,
            &serde_json::to_vec_pretty(&library).map_err(|e| e.to_string())?,
        )?;
        self.library = library;
        Ok(())
    }

    pub fn save_auto(&mut self, auto: Auto) -> Result<()> {
        let mut library = self.library.clone();
        library.auto = Some(auto);
        self.replace(library)
    }

    fn save(&mut self, settings: &Settings) -> Result<()> {
        let mut library = self.library.clone();
        let visual = Visual::capture(settings);
        if let Some(i) = self.target {
            library
                .custom
                .get_mut(i)
                .ok_or("User preset no longer exists")?
                .visual = visual;
        } else {
            let name = library.name(&self.name, None)?;
            library.custom.push(Named { name, visual });
        }
        self.replace(library)?;
        if self.target.is_none() {
            self.target = Some(self.library.custom.len() - 1);
        }
        Ok(())
    }

    fn rename(&mut self) -> Result<()> {
        let index = self.target.ok_or("Choose a user preset to rename")?;
        let mut library = self.library.clone();
        library.custom[index].name = library.name(&self.name, Some(index))?;
        self.replace(library)
    }

    fn delete(&mut self) -> Result<()> {
        let index = self.target.ok_or("Choose a user preset to remove")?;
        let mut library = self.library.clone();
        library.custom.remove(index);
        self.replace(library)?;
        self.target = None;
        self.name.clear();
        Ok(())
    }

    pub fn show(&mut self, ctx: &egui::Context, settings: &Settings) -> Option<Choice> {
        let mut selected = None;
        let mut open = self.open;
        if !open {
            return None;
        }
        egui::Window::new("Visual presets").open(&mut open).default_width(460.0).show(ctx, |ui| {
            ui.label("Presets change graphics and HUD text. Gameplay, controls, audio and window preferences are kept.");
            ui.horizontal(|ui| {
                for (index, label) in presets::QUALITY.into_iter().zip(["Low", "Medium", "High", "Ultra"]) {
                    if ui.button(label).clicked() { selected = Some(Choice::Builtin(index)); }
                }
            });
            ui.weak("Quality tiers preserve the selected weather. Ultra enables the visual enhancements; it does not force rain, snow or a day cycle.");
            if let Some(auto) = &self.library.auto {
                ui.separator();
                if ui.button(AUTO).clicked() { selected = Some(Choice::Auto); }
                ui.label(format!("{} · {}×{} · target {} FPS · {}", auto.adapter, auto.size[0], auto.size[1], auto.target_fps, auto.quality));
                ui.weak(&auto.note);
            }
            ui.separator();
            ui.label("Save current graphics as");
            let before = self.target;
            egui::ComboBox::from_id_salt("save-preset-target").selected_text(self.target.map_or("New preset…", |i| self.library.custom[i].name.as_str())).show_ui(ui, |ui| {
                ui.selectable_value(&mut self.target, None, "New preset…");
                for (i, preset) in self.library.custom.iter().enumerate() { ui.selectable_value(&mut self.target, Some(i), &preset.name); }
            });
            if before != self.target { self.name = self.target.map_or_else(String::new, |i| self.library.custom[i].name.clone()); }
            ui.add(egui::TextEdit::singleline(&mut self.name).hint_text("Preset name").char_limit(64));
            ui.horizontal(|ui| {
                if ui.button(if self.target.is_some() { "Overwrite graphics" } else { "Save new preset" }).clicked() {
                    self.notice = self.save(settings).map_or_else(|e| e, |_| "Saved".into());
                }
                ui.add_enabled_ui(self.target.is_some(), |ui| {
                    if ui.button("Apply").clicked() { selected = self.target.map(Choice::User); }
                    if ui.button("Rename").clicked() { self.notice = self.rename().map_or_else(|e| e, |_| "Renamed".into()); }
                    if ui.button("Delete").clicked() { self.notice = self.delete().map_or_else(|e| e, |_| "Deleted".into()); }
                });
            });
            ui.weak("Built-in and Auto-optimised presets cannot be overwritten, renamed or deleted here.");
            if !self.notice.is_empty() { ui.label(&self.notice); }
        });
        self.open = open;
        selected
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_presets_round_trip_without_changing_gameplay_and_protect_builtins() {
        let directory = std::env::temp_dir().join(format!("tenchu-presets-{}", std::process::id()));
        let mut manager = Manager {
            library: Library::default(),
            path: directory.join("presets.json"),
            open: false,
            target: None,
            name: "My evening".into(),
            notice: String::new(),
        };
        let mut source = Settings::default();
        source.set_visual_preset(presets::MODERN_HIGH);
        source.graphics.weather.mode = super::super::WeatherMode::Snow;
        manager.save(&source).unwrap();
        let mut target = Settings {
            sight_percent: 140,
            volume: 37,
            keep_enemies_active: true,
            pause_on_defocus: false,
            ..Default::default()
        };
        let previous = target.clone();
        manager.library.apply(Choice::User(0), &mut target).unwrap();
        assert_eq!(Visual::capture(&target), Visual::capture(&source));
        target.graphics = previous.graphics.clone();
        target.sharp_text = previous.sharp_text;
        assert_eq!(target, previous);

        manager.name = "Original".into();
        assert!(manager.rename().is_err());
        manager.name = "Moon garden".into();
        manager.rename().unwrap();
        source.graphics.lighting.count = 4;
        manager.name = "Typing here must not rename when saving".into();
        manager.save(&source).unwrap();
        let saved: Library =
            serde_json::from_slice(&storage::read(&manager.path).unwrap()).unwrap();
        assert_eq!(saved.custom[0].name, "Moon garden");
        assert_eq!(saved.custom[0].visual, Visual::capture(&source));
        manager.target = None;
        manager.name = "  moon garden  ".into();
        assert!(manager.save(&source).is_err());
        manager.name = AUTO.into();
        assert!(manager.save(&source).is_err());
        manager.target = Some(0);
        manager.delete().unwrap();
        assert!(manager.library.custom.is_empty());
        assert_eq!(manager.library.choices().len(), presets::CUSTOM as usize);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
