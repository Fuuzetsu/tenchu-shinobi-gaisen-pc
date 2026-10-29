use super::{
    overlay::Extras,
    performance::Performance,
    view::{Frame, View},
};
use crate::{
    campaign::{Campaign, Screen},
    debug::Debug,
    frontend::{Frontend, showcase},
    retail::tape::{Tape, source_field, video_frame},
    settings::{Settings, menu::Menu},
    source::DiscSource,
    video::{Chapters, Export},
};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tenchu_assets::{Result, game::GameAssets};
use tenchu_core::{game::events::Outcome, progress::Completion};
use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, ControlFlow, EventLoop},
    window::WindowId,
};

struct Recorder {
    assets: GameAssets<DiscSource>,
    tape: Tape,
    campaign: Campaign,
    frontend: Frontend,
    settings: Settings,
    debug: Debug,
    menu: Menu,
    performance: Performance,
    view: Option<View>,
    video: Option<Export>,
    output: PathBuf,
    start: u64,
    end: u64,
    frame: u64,
    mission: usize,
    update: usize,
    loaded: bool,
    completed: bool,
    previous_field: u32,
    current_field: u32,
    page: Option<usize>,
    error: Option<String>,
}

impl Recorder {
    fn menu(&mut self, page: usize) -> Result<()> {
        if self.page == Some(page) {
            return Ok(());
        }
        self.page = Some(page);
        self.frontend.stage_showcase(
            &showcase::Scene {
                page: if page == 0 {
                    showcase::Menu::Title
                } else {
                    showcase::Menu::Missions
                },
                language: match self.campaign.options.language.unwrap_or_default() {
                    tenchu_assets::script::Language::English => "en",
                    tenchu_assets::script::Language::French => "fr",
                    tenchu_assets::script::Language::Italian => "it",
                    tenchu_assets::script::Language::Japanese => "ja",
                }
                .into(),
            },
            &self.assets,
            &mut self.campaign,
        )
    }

    fn advance(&mut self, field: f64) -> Result<()> {
        let view = self.view.as_mut().unwrap();
        if !self.loaded && field >= f64::from(self.tape.missions[self.mission].entry_field) {
            let entry = &self.tape.missions[self.mission];
            self.campaign.game = entry.initial.load(&self.assets)?;
            self.campaign.options = entry
                .initial
                .options(self.campaign.game.mission.stage.order as usize)?;
            self.debug = Debug::new(&self.campaign.game)?;
            view.reload(&self.campaign.game, &self.settings)?;
            self.previous_field = entry.entry_field;
            self.current_field = entry.entry_field;
            self.loaded = true;
            self.completed = false;
            self.update = 0;
            self.page = None;
            eprintln!(
                "Rendering mission {} at source field {}",
                self.campaign.options.mission, entry.entry_field
            );
        }
        if self.loaded && !self.completed {
            let entry = &self.tape.missions[self.mission];
            while let Some(update) = entry
                .updates
                .get(self.update)
                .filter(|u| f64::from(u.field) <= field)
            {
                self.campaign.screen = Screen::Playing;
                update.apply(&mut self.campaign.game)?;
                self.debug.update(&self.campaign.game);
                view.interpolation.tick(&self.campaign.game);
                view.cosmetic_tick(&self.campaign.game, &self.settings, &self.debug);
                self.previous_field = self.current_field;
                self.current_field = update.field;
                self.update += 1;
            }
            if field >= f64::from(entry.result_field) {
                if self.update != entry.updates.len() {
                    return Err("Results precede final update".into());
                }
                while self.campaign.game.cinematic.tail > 0 {
                    self.campaign.game.step(0)?;
                }
                self.campaign.game.step(0)?;
                if self.campaign.game.events.outcome != Outcome::Complete {
                    return Err("Mission did not complete during export".into());
                }
                let game = &mut self.campaign.game;
                let results = self.campaign.progress.complete(
                    Completion {
                        stage: &game.mission.stage,
                        character: self.campaign.options.character as usize,
                        layout: game.mission.layout_index,
                        language: self.campaign.options.language.unwrap_or_default(),
                        record: game.record(),
                        remaining: game.player().inventory[..20].try_into().unwrap(),
                    },
                    &mut game.random,
                );
                self.campaign.screen = Screen::Results(results);
                self.frontend.recorded_results();
                self.completed = true;
                self.page = None;
            }
        }
        if self.completed
            && field >= f64::from(self.tape.missions[self.mission].result_end)
            && self.mission + 1 < self.tape.missions.len()
        {
            self.mission += 1;
            self.loaded = false;
            self.completed = false;
            self.campaign.options = self.tape.missions[self.mission]
                .initial
                .options(self.mission + 1)?;
            self.menu(self.mission + 1)?;
        }
        Ok(())
    }

    fn render_frame(&mut self) -> Result<bool> {
        let field = source_field(self.frame);
        self.advance(field)?;
        if self.frame < self.start {
            self.frame += 1;
            return Ok(false);
        }
        // The display is smoothly interpolated with one source update of visual latency.
        // Core updates and their recorded draw/swap decisions are never interpolated.
        let span = self
            .current_field
            .saturating_sub(self.previous_field)
            .clamp(1, 4);
        let alpha =
            ((field - f64::from(self.current_field)) / f64::from(span)).clamp(0.0, 1.0) as f32;
        let frontend_visible =
            matches!(self.campaign.screen, Screen::Frontend | Screen::Results(_));
        if frontend_visible && self.page != Some(0) {
            self.frontend.advance(
                Duration::from_secs_f64(1.0 / 60.0),
                false,
                &mut self.campaign,
                &self.assets,
            )?;
            self.frontend.events.clear();
        }
        let view = self.view.as_mut().unwrap();
        if !frontend_visible {
            view.update(Frame {
                campaign: &self.campaign,
                paused: false,
                error: None,
                settings: &self.settings,
                menu: &self.menu,
                fps: 60.0,
                offline: true,
                debug: &self.debug,
                alpha,
            });
        }
        view.overlay.update(
            &view.window,
            &view.renderer,
            &mut self.menu,
            crate::settings::overlay::State {
                settings: &self.settings,
                debug: &self.debug,
                paused: false,
                lights: view.lights.len(),
                samples: &view.renderer.sample_counts,
            },
            Extras {
                randomizer: None,
                benchmarks: None,
                presets: None,
                controls: None,
                models: None,
                labels: &view.labels,
                text: &view.text,
                performance: &mut self.performance,
                demo: None,
                video_time: Some(self.frame as f64 / 60.0),
                frontend: frontend_visible.then_some((&mut self.frontend, &self.campaign)),
            },
        );
        let video = self.video.as_mut().unwrap();
        view.render(
            &video.texture.create_view(&Default::default()),
            &self.campaign.game,
            &self.debug,
        )?;
        video.write_frame(
            &view.renderer,
            self.frame - self.start,
            &format!(
                "Mission {} · field {:.1}",
                self.campaign.options.mission, field
            ),
        )?;
        self.frame += 1;
        if self.frame == self.end {
            self.video.take().unwrap().finish()?;
            return Ok(true);
        }
        Ok(false)
    }
}

impl ApplicationHandler for Recorder {
    fn resumed(&mut self, events: &ActiveEventLoop) {
        if self.view.is_some() {
            return;
        }
        let result = (|| {
            let mut view = View::new(events, &self.campaign.game, &self.settings)?;
            view.window
                .set_title("Tenchu — offline verified campaign recording");
            view.resize(crate::video::SIZE[0], crate::video::SIZE[1]);
            let mut chapters = vec![(
                0,
                if self.start == 0 {
                    "Boot and mission selection".into()
                } else {
                    "Campaign replay".into()
                },
            )];
            for (index, mission) in self.tape.missions.iter().enumerate() {
                let frame = video_frame(mission.updates[0].field);
                if frame > self.start && frame < self.end {
                    chapters.push((frame - self.start, format!("Mission {}", index + 1)));
                }
            }
            let chapters = Chapters::timeline(
                "Tenchu: Shinobi Gaisen — Ayame full campaign / Native PC",
                &chapters,
                self.end - self.start,
            )?;
            self.video = Some(Export::timeline(
                &self.output,
                chapters,
                &view.renderer,
                view.config.format,
            )?);
            std::fs::write(
                self.output.with_extension("settings.json"),
                serde_json::to_vec_pretty(&view.recording_settings()).map_err(|e| e.to_string())?,
            )
            .map_err(|e| e.to_string())?;
            self.view = Some(view);
            self.menu(0)
        })();
        if let Err(error) = result {
            self.error = Some(error);
            events.exit();
        }
    }

    fn window_event(&mut self, events: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        if matches!(event, WindowEvent::CloseRequested) {
            self.error = Some("Campaign recording cancelled".into());
            events.exit();
        }
    }

    fn about_to_wait(&mut self, events: &ActiveEventLoop) {
        if self.view.is_none() || self.error.is_some() {
            return;
        }
        // Fast seeking still runs every update and cosmetic tick, without rendering discarded frames.
        for _ in 0..120 {
            match self.render_frame() {
                Ok(true) => {
                    events.exit();
                    return;
                }
                Err(error) => {
                    self.error = Some(error);
                    events.exit();
                    return;
                }
                Ok(false) => (),
            }
            if self.frame >= self.start {
                break;
            }
        }
    }

    fn exiting(&mut self, _: &ActiveEventLoop) {
        self.video.take();
        if let Some(view) = self.view.take() {
            view.renderer.device.poll(wgpu::Maintain::Wait);
            drop(view);
        }
    }
}

pub fn record(
    assets: GameAssets<DiscSource>,
    disc: &Path,
    tape: Tape,
    output: &Path,
    start: u64,
    end: Option<u64>,
) -> Result<()> {
    let total = video_frame(tape.fields);
    let end = end.unwrap_or(total);
    if start >= end || end > total {
        return Err("Invalid video frame range".into());
    }
    let mut settings = Settings::default();
    settings.set_visual_preset(crate::settings::presets::SHOWCASE);
    settings.graphics.asset_pack = true;
    let options = tape.missions[0].initial.options(1)?;
    let campaign = Campaign::new(&assets, options, None)?;
    let frontend = Frontend::new(&assets, disc, &campaign, settings.frontend.clone())?;
    let debug = Debug::new(&campaign.game)?;
    let mut app = Recorder {
        assets,
        tape,
        campaign,
        frontend,
        settings,
        debug,
        menu: Menu::default(),
        performance: Performance::default(),
        view: None,
        video: None,
        output: output.into(),
        start,
        end,
        frame: 0,
        mission: 0,
        update: 0,
        loaded: false,
        completed: false,
        previous_field: 0,
        current_field: 0,
        page: None,
        error: None,
    };
    let events = EventLoop::new().map_err(|e| e.to_string())?;
    events.set_control_flow(ControlFlow::Poll);
    events.run_app(&mut app).map_err(|e| e.to_string())?;
    app.error.map_or(Ok(()), Err)
}
