use super::*;

impl Editor {
    pub(crate) fn mixed_example(&mut self, assets: &GameAssets<DiscSource>) -> Result<Project> {
        self.scenery_library.load(assets, AssetGame::Tenchu, 0)?;
        self.scenery_library.load(assets, AssetGame::Tenchu2, 2)?;
        let body = self
            .asset_names
            .iter()
            .find(|(id, _)| id.starts_with("tenchu2") && id.ends_with(":rikimaru"))
            .ok_or("Connect a Tenchu 2 disc to load the mixed-game example")?
            .0
            .clone();
        let tables = self.tables.as_ref().ok_or("Open Level Studio first")?;
        let theme = Theme::load(assets, 2)?;
        let mut mission = Mission::new(2, Objective::ReachLord, tables.appearance[2][2])?;
        mission.title = "Two eras, one garden".into();
        mission.author = "Studio".into();
        editing::move_actor(&mut mission, &theme, 0, Point { x: 16, z: 23 })?;
        editing::move_actor(&mut mission, &theme, 1, Point { x: 16, z: 7 })?;
        for z in 0..32 {
            for x in 0..32 {
                let tile = if !(5..=26).contains(&x) || !(4..=28).contains(&z) {
                    5
                } else if (14..=17).contains(&x) || (15..=18).contains(&z) {
                    1
                } else {
                    0
                };
                editing::paint(&mut mission, &theme, Point { x, z }, false, tile)?;
            }
        }
        for z in [10, 17, 24] {
            for x in [13, 18] {
                editing::paint(&mut mission, &theme, Point { x, z }, true, 16)?;
            }
        }
        let actor = editing::add_actor(
            &mut mission,
            &theme,
            Point { x: 20, z: 21 },
            tables.appearance[2][2],
        )?;
        for x in [21, 22, 23] {
            editing::patrol(
                &mut mission,
                &theme,
                actor,
                Waypoint {
                    point: Point { x, z: 21 },
                    wait: if x == 23 { 2 } else { 0 },
                    facing: 1,
                },
            )?;
        }
        let mut project = self.new_project(mission);
        project.actors.insert(actor, body);
        project.graphics.sky.enabled = true;
        project.graphics.sky.time = 900;
        project.graphics.fog = false;
        project.graphics.sky.night_ambient = 35;
        project.graphics.weather.mode = crate::settings::WeatherMode::Clear;
        let source = &self.scenery_library.sources[&(AssetGame::Tenchu, 0)];
        for (name, position) in [
            ("SAKURA", [21_000, 0, -28_000]),
            ("SAKURA", [44_000, 0, -20_000]),
            ("TOROB", [23_000, 0, -40_000]),
            ("TUBO", [45_000, 0, -38_000]),
        ] {
            let part = source
                .names
                .iter()
                .position(|n| n == name)
                .ok_or_else(|| format!("Missing T1 scenery {name}"))?;
            project.scenery.push(project::Scenery {
                name: format!("T1 · {name}"),
                game: AssetGame::Tenchu,
                stage: 0,
                part,
                position,
                yaw: 0,
            });
        }
        for (part, position) in [
            (0, [26_000, 0, -30_000]),
            (4, [39_000, 0, -30_000]),
            (8, [45_000, 0, -32_000]),
        ] {
            project.scenery.push(project::Scenery {
                name: format!("T2 · Training prop {part}"),
                game: AssetGame::Tenchu2,
                stage: 2,
                part,
                position,
                yaw: 0,
            });
        }
        let mut stamp = building::Stamp::copy(
            &project.mission,
            Point { x: 12, z: 9 },
            Point { x: 13, z: 11 },
        )?;
        stamp.name = "Lantern border".into();
        project.stamps.push(stamp);
        for (name, eye, target) in [
            (
                "Garden overview",
                [56_000.0, -20_000.0, -62_000.0],
                [31_000.0, -1500.0, -31_000.0],
            ),
            (
                "Both eras",
                [37_000.0, -6500.0, -60_000.0],
                [33_000.0, -1000.0, -42_000.0],
            ),
            (
                "Cherry trees",
                [49_000.0, -10_000.0, -40_000.0],
                [30_000.0, -3000.0, -27_000.0],
            ),
        ] {
            let path = crate::demo::framing::Path {
                eye,
                target,
                end_eye: None,
                end_target: None,
                follow_player: false,
                fov: 55.0,
            };
            project
                .views
                .push(building::View::new(name.into(), path.sample(0.0)));
        }
        project.events.push(project::Event {
            name: "Storm over the garden".into(),
            cues: vec![
                project::Cue {
                    delay: 0,
                    action: Action::Message {
                        text: "Two eras, one garden. Reach Lord Gohda beyond the trees.".into(),
                        ticks: 120,
                    },
                },
                project::Cue {
                    delay: 90,
                    action: Action::Weather(crate::settings::WeatherMode::Rainstorm),
                },
                project::Cue {
                    delay: 0,
                    action: Action::TimeOfDay(1110),
                },
            ],
            ..Default::default()
        });
        project.events.push(project::Event {
            name: "Courtyard arrival".into(),
            trigger: project::Trigger::Enter {
                min: [28_000, -8000, -38_000],
                max: [36_000, 2000, -30_000],
            },
            cues: vec![project::Cue {
                delay: 0,
                action: Action::Message {
                    text: "The courtyard is watched. Use the garden for cover.".into(),
                    ticks: 120,
                },
            }],
            ..Default::default()
        });
        project.validate()?;
        theme.validate(&project.mission)?;
        Ok(project)
    }

    pub(crate) fn open_mixed_example(&mut self, assets: &GameAssets<DiscSource>) -> Result<()> {
        let project = self.mixed_example(assets)?;
        self.save(false)?;
        self.select(project, assets)?;
        self.panel = Panel::Scenery;
        self.tool = Tool::Select;
        self.notice = "T1 terrain, cherry trees and lanterns; T2 scenery and Rikimaru. F5 starts the weather queue. Camera views offers three saved angles.".into();
        Ok(())
    }
}
