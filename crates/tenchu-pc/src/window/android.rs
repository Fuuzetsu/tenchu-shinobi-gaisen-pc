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
                "tenchu2" => {
                    self.tenchu2.path = path.to_string_lossy().into();
                    self.tenchu2_action(crate::tenchu2::menu::Event::Connect);
                    Ok(())
                }
                "recording" => {
                    let bytes = crate::savestate::read(&path)?;
                    if self.native_tenchu2.is_some() {
                        self.load_native_recording(&bytes)
                    } else {
                        self.load_recording(&bytes)
                    }
                }
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
            if let Some(native) = &mut self.native_tenchu2 {
                native.menu.notice(error.clone());
            }
            self.menu.notice = error.clone();
            self.randomizer.notice = error.clone();
            self.trial.notice = error;
        }
    }
}
