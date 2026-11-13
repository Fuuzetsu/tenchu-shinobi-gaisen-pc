use super::*;
use crate::demo::studio::Feature;

impl Editor {
    pub(crate) fn for_showcase() -> Self {
        let mut editor = Self {
            ephemeral: true,
            ..Self::new(PathBuf::new())
        };
        editor
            .default_graphics
            .set_preset(crate::settings::presets::MODERN_HIGH);
        editor.project.graphics = editor.default_graphics.clone();
        editor
    }

    pub(crate) fn load_showcase(&mut self, assets: &GameAssets<DiscSource>) -> Result<()> {
        let project = self.mixed_example(assets)?;
        self.select(project, assets)?;
        self.library = store::Library::default();
        self.tool = Tool::Select;
        self.search.clear();
        Ok(())
    }

    pub(crate) fn prepare_showcase(&mut self, feature: Feature) {
        self.notice.clear();
        self.grid = matches!(
            feature,
            Feature::Terrain | Feature::Stamps | Feature::Patrols | Feature::Regions
        );
        self.guides = !matches!(
            feature,
            Feature::Lighting
                | Feature::Weather
                | Feature::Views
                | Feature::Mixed
                | Feature::Characters
        );
        self.panel = match feature {
            Feature::Workspace | Feature::Terrain | Feature::Stamps => Panel::Terrain,
            Feature::Scenery | Feature::Mixed => Panel::Scenery,
            Feature::Characters | Feature::Patrols => Panel::Actors,
            Feature::Lighting | Feature::Weather | Feature::Views => Panel::Environment,
            Feature::Regions | Feature::Sounds => Panel::Events,
            Feature::Playtest => Panel::Mission,
            Feature::Projects => Panel::Library,
        };
        self.actor = 2;
        if matches!(feature, Feature::Characters | Feature::Playtest) {
            self.camera = self.project.views[1].camera();
        }
        if feature == Feature::Scenery {
            self.scenery_selection = Some(6);
            self.scenery_game = AssetGame::Tenchu2;
            self.scenery_stage = 2;
        }
        if feature == Feature::Stamps {
            self.clipboard = Some(self.project.stamps[0].clone());
            self.selection = Some((Point { x: 12, z: 9 }, Point { x: 13, z: 11 }));
            self.tool = Tool::Stamp;
        }
        if feature == Feature::Terrain {
            self.selection = Some((Point { x: 7, z: 7 }, Point { x: 11, z: 11 }));
            self.tool = Tool::Area;
            self.brush = 2;
            self.ground = true;
        }
        if feature == Feature::Regions {
            self.project.events.clear();
            self.tool = Tool::Area;
        }
        if feature == Feature::Sounds {
            self.event = self.project.events.len() - 1;
        }
    }

    pub(crate) fn showcase_frame(
        &mut self,
        feature: Feature,
        frame: u64,
        total: u64,
    ) -> Result<()> {
        let p = frame as f32 / total as f32;
        let first = frame == total / 4;
        let middle = frame == total / 2;
        let last = frame == total * 3 / 4;
        match feature {
            Feature::Workspace => {
                self.grid = p >= 0.65;
                if !self.grid {
                    self.camera = self.project.views[0].camera();
                    self.camera.yaw += (p * std::f32::consts::TAU).sin() * 0.12;
                }
            }
            Feature::Terrain => {
                if first {
                    self.fill_area()?;
                }
                if middle {
                    self.undo(false);
                }
                if last {
                    self.undo(true);
                }
            }
            Feature::Stamps => {
                if first {
                    self.clipboard.as_mut().unwrap().rotate();
                }
                if middle {
                    self.use_cell(Point { x: 20, z: 10 });
                    self.commit();
                }
                if last {
                    self.use_cell(Point { x: 21, z: 13 });
                    self.commit();
                }
            }
            Feature::Scenery => {
                if first {
                    self.move_scenery([43_550.0, -1800.0, -39_200.0]);
                    self.commit();
                }
                if middle {
                    self.nudge_scenery(0, 1);
                }
                if last {
                    self.ground_scenery();
                }
            }
            Feature::Characters => {
                self.camera = self.project.views[1].camera();
                self.camera.position += self.camera.forward() * (p * 1200.0);
            }
            Feature::Patrols => {
                if first || middle || last {
                    self.tool = Tool::Patrol;
                    self.use_cell(Point {
                        x: 23,
                        z: 21
                            + if first {
                                1
                            } else if middle {
                                2
                            } else {
                                3
                            },
                    });
                    self.commit();
                }
            }
            Feature::Lighting => {
                self.project.graphics.sky.time = if p < 0.34 {
                    780
                } else if p < 0.67 {
                    1080
                } else {
                    1320
                };
            }
            Feature::Weather => {
                self.project.graphics.weather.mode = if p < 0.25 {
                    crate::settings::WeatherMode::Clear
                } else if p < 0.65 {
                    crate::settings::WeatherMode::Rainstorm
                } else {
                    crate::settings::WeatherMode::Snow
                };
                self.project.graphics.weather.amount = 100;
                self.project.graphics.weather.snow = 90;
                self.project.graphics.weather.wind = 70;
                self.project.graphics.sky.time = if p < 0.25 { 780 } else { 1050 };
            }
            Feature::Regions => {
                if first {
                    self.selection = Some((Point { x: 14, z: 16 }, Point { x: 17, z: 19 }));
                }
                if middle {
                    self.region_from_area()?;
                    self.grid = false;
                }
            }
            Feature::Projects => {
                self.panel = if p < 0.5 {
                    Panel::Mission
                } else {
                    Panel::Library
                };
                if first {
                    self.save(false)?;
                }
                if middle {
                    let bytes = self.project.encode()?;
                    let restored = Project::decode(&bytes)?;
                    if restored != self.project {
                        return Err("Project round-trip changed the showcase".into());
                    }
                    self.notice = "Exported project round-trip verified: terrain, both games' assets, views, stamps and events.".into();
                }
            }
            Feature::Views => {
                self.camera = self.project.views[crate::demo::section(p, 3)].camera();
                self.notice = format!(
                    "Camera view: {}",
                    self.project.views[crate::demo::section(p, 3)].name
                );
            }
            Feature::Mixed => {
                self.camera = self.project.views[if p < 0.5 { 0 } else { 1 }].camera();
                self.camera.position += self.camera.forward() * ((p % 0.5) * 1000.0);
            }
            Feature::Sounds | Feature::Playtest => (),
        }
        Ok(())
    }
}
