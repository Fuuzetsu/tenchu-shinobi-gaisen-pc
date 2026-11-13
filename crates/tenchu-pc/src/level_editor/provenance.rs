use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Build {
    pub version: String,
    pub revision: String,
}

impl Build {
    pub fn current() -> Self {
        Self {
            version: env!("CARGO_PKG_VERSION").into(),
            revision: env!("TENCHU_BUILD_REVISION").into(),
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Provenance {
    pub created_with: Option<Build>,
    pub last_saved_with: Option<Build>,
}

impl Provenance {
    pub fn new() -> Self {
        let current = Build::current();
        Self {
            created_with: Some(current.clone()),
            last_saved_with: Some(current),
        }
    }

    pub fn validate(&self) -> tenchu_assets::Result<()> {
        for build in self.created_with.iter().chain(&self.last_saved_with) {
            if build.version.is_empty()
                || build.version.len() > 64
                || build.revision.is_empty()
                || build.revision.len() > 128
            {
                return Err("Invalid map build provenance".into());
            }
        }
        Ok(())
    }

    pub fn show(&self, ui: &mut egui::Ui, schema: u32) {
        ui.collapsing("Map format & build", |ui| {
            ui.label(format!("Map schema {schema}"));
            for (label, build) in [
                ("Created with", &self.created_with),
                ("Last saved with", &self.last_saved_with),
            ] {
                ui.label(label);
                if let Some(build) = build {
                    ui.label(format!("Tenchu {}", build.version));
                    ui.add(
                        egui::Label::new(egui::RichText::new(&build.revision).monospace()).wrap(),
                    );
                } else {
                    ui.weak("Unknown — this map predates build markers");
                }
            }
        });
    }
}
