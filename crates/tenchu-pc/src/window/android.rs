use super::*;

impl Application {
    pub(super) fn poll_android_files(&mut self) {
        let Some(response) = crate::android::poll_file() else {
            return;
        };
        let result = response.and_then(|response| {
            if let Some(error) = response.error {
                return Err(error);
            }
            let Some(path) = response.path else {
                return Ok(());
            };
            match response.kind.as_str() {
                "recording" => self.load_recording(&crate::savestate::read(&path)?),
                "unarmed" => self.install_unarmed(crate::sheathing::imports::from_path(&path)?),
                "trial" => {
                    if path.metadata().map_err(|e| e.to_string())?.len() > 8192 {
                        return Err("Select one .msn or .mcs mission".into());
                    }
                    self.trial.import(
                        &std::fs::read(path).map_err(|e| e.to_string())?,
                        &self.assets,
                    )
                }
                "randomizer" => {
                    if path.metadata().map_err(|e| e.to_string())?.len() > 1024 * 1024 {
                        return Err("Seed document is too large".into());
                    }
                    let run = crate::randomizer::Run::import(
                        &std::fs::read_to_string(path).map_err(|e| e.to_string())?,
                    )?;
                    let library = self
                        .view
                        .as_ref()
                        .and_then(|v| v.model_library.as_ref())
                        .ok_or("Open Randomizer before importing a seed")?;
                    run.document.validate(&crate::randomizer::Catalog::new(
                        library,
                        &self.campaign.stages,
                    ))?;
                    self.randomizer.imported(run);
                    Ok(())
                }
                "export" => {
                    self.menu.notice = "File exported".into();
                    Ok(())
                }
                _ => Err("Unknown Android file request".into()),
            }
        });
        if let Err(error) = result {
            self.menu.notice = error.clone();
            self.randomizer.notice = error.clone();
            self.trial.notice = error;
        }
    }
}
