use super::*;
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Serialize, Deserialize)]
pub(super) struct Library {
    version: u32,
    pub draft: Option<Project>,
    pub projects: Vec<Project>,
    pub record_name: String,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            draft: None,
            projects: vec![],
            record_name: "Player".into(),
        }
    }
}

impl Library {
    pub fn load(path: &Path) -> Result<Self> {
        let bytes = match crate::storage::read(path) {
            Ok(bytes) => bytes,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Self::default()),
            Err(e) => return Err(format!("Read Level Studio library: {e}")),
        };
        if bytes.len() > 512 * 1024 * 1024 {
            return Err("Level Studio library is too large".into());
        }
        let mut library: Self = serde_json::from_slice(&bytes)
            .map_err(|e| format!("Read Level Studio library: {e}"))?;
        if library.version != 1 || library.projects.len() > 128 || library.record_name.len() > 512 {
            return Err("Unsupported Level Studio library".into());
        }
        let mut shared = custom::Assets::new();
        for project in library.projects.iter_mut().chain(&mut library.draft) {
            custom::share_payloads(&mut project.assets, &mut shared);
            project.validate()?;
        }
        Ok(library)
    }

    pub fn write(&self, path: &Path) -> Result<()> {
        let bytes = serde_json::to_vec(self).map_err(|e| e.to_string())?;
        if bytes.len() > 512 * 1024 * 1024 {
            return Err("Level Studio library is too large".into());
        }
        crate::storage::atomic_write(path, &bytes)
    }
}

impl Editor {
    pub fn import(&mut self, bytes: &[u8], assets: &GameAssets<DiscSource>) -> Result<()> {
        let project = if bytes.iter().find(|b| !b.is_ascii_whitespace()) == Some(&b'{') {
            Project::decode(bytes)?
        } else {
            self.new_project(Mission::decode(bytes)?)
        };
        self.save(true)?;
        self.select(project, assets)?;
        self.notice = "Imported project; previous work is saved in the library".into();
        Ok(())
    }

    pub(super) fn import_file(&mut self, assets: &GameAssets<DiscSource>) -> Result<()> {
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter(
                "Level Studio or retail mission",
                &["tenchu", "json", "msn", "mcs"],
            )
            .pick_file()
        {
            if std::fs::metadata(&path).map_err(|e| e.to_string())?.len()
                > project::MAX_BYTES as u64
            {
                return Err("Project file is too large".into());
            }
            self.import(&std::fs::read(path).map_err(|e| e.to_string())?, assets)?;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = assets;
            crate::web::recording::start(crate::web::recording::Kind::Studio)?;
        }
        #[cfg(target_os = "android")]
        {
            let _ = assets;
            self.notice = "Paste a project or retail share code below to import".into();
        }
        Ok(())
    }

    pub(super) fn export_file(&mut self, retail: bool) -> Result<()> {
        let bytes = if retail {
            self.project.mission.encode()?
        } else {
            self.project.encode()?
        };
        let (name, extension) = if retail {
            ("mission.msn", "msn")
        } else {
            ("mission.tenchu", "tenchu")
        };
        #[cfg(all(not(target_arch = "wasm32"), not(target_os = "android")))]
        if let Some(path) = rfd::FileDialog::new()
            .add_filter("Tenchu mission", &[extension])
            .set_file_name(name)
            .save_file()
        {
            crate::storage::atomic_write(&path, &bytes)?;
        }
        #[cfg(target_os = "android")]
        {
            let _ = extension;
            crate::android::export(name, &bytes)?;
        }
        #[cfg(target_arch = "wasm32")]
        {
            let _ = extension;
            crate::web::recording::download(name, &bytes)?;
        }
        Ok(())
    }
}
