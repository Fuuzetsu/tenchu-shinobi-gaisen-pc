use super::*;
use crate::frontend::{Event, Page};
use tenchu_assets::frontend::Movie;

impl Application {
    #[cfg(target_arch = "wasm32")]
    pub(super) fn browser_ready(&mut self) -> bool {
        match crate::web::mission_ready(self.campaign.options.language.unwrap_or_default()) {
            Ok(true) => return true,
            Ok(false) => crate::web::status(
                "Preparing the campaign in the background — please stay in the menu a moment",
            ),
            Err(error) => self.pause_error(error),
        }
        false
    }
    pub(super) fn frontend_visible(&self) -> bool {
        self.frontend.is_some()
            && (matches!(self.campaign.screen, Screen::Frontend | Screen::Ending(_))
                || matches!(self.campaign.screen, Screen::Results(_))
                    && self.campaign.options.mission == 10)
    }

    pub(super) fn open_title(&mut self) {
        let Some(frontend) = &mut self.frontend else {
            return;
        };
        self.campaign.screen = Screen::Frontend;
        self.campaign.quick_launcher = false;
        if let Err(error) = frontend
            .language(
                &self.assets,
                self.campaign.options.language.unwrap_or_default(),
            )
            .and_then(|_| frontend.enter_title(&self.assets))
        {
            self.pause_error(error);
            return;
        }
        self.paused = false;
        self.faulted = false;
        self.fault = None;
        self.sync_pause();
        self.update_view();
    }

    pub(super) fn frontend_action(&mut self, action: Action) {
        #[cfg(target_arch = "wasm32")]
        if matches!(action, Action::Confirm)
            && self
                .frontend
                .as_ref()
                .is_some_and(|f| matches!(f.page, Page::Character | Page::Missions))
            && !self.browser_ready()
        {
            return;
        }
        if let Some(frontend) = &mut self.frontend
            && let Err(error) = frontend.action(action, &mut self.campaign, &self.assets)
        {
            self.pause_error(error);
        }
    }

    pub(super) fn start_ending(&mut self, events: &ActiveEventLoop) {
        if matches!(self.campaign.screen, Screen::Ending(_))
            && self.frontend.as_ref().is_some_and(|f| {
                !matches!(
                    f.page,
                    Page::Movie(Movie::Rikimaru | Movie::Ayame | Movie::Credits)
                )
            })
        {
            let frontend = self.frontend.as_mut().unwrap();
            let movie = if self.campaign.options.character == 0 {
                Movie::Rikimaru
            } else {
                Movie::Ayame
            };
            if let Err(error) = frontend
                .language(
                    &self.assets,
                    self.campaign.options.language.unwrap_or_default(),
                )
                .and_then(|_| frontend.play_movie(movie, &self.assets))
            {
                self.pause_error(error);
            }
            self.frontend_events(events);
        }
    }

    pub(super) fn frontend_events(&mut self, events: &ActiveEventLoop) {
        let Some(frontend) = &mut self.frontend else {
            return;
        };
        let pending = std::mem::take(&mut frontend.events);
        for event in pending {
            match event {
                Event::Quit => events.exit(),
                Event::QuickLauncher => {
                    self.finish_recording();
                    self.campaign.quick_launcher = true;
                    self.campaign.screen = Screen::Launcher;
                    self.campaign.cursor = usize::from(self.campaign.saved.is_none());
                    if let Some(audio) = &mut self.audio {
                        audio.screen(&self.campaign.screen);
                    }
                    self.paused = false;
                    self.sync_pause();
                    self.update_view();
                }
                Event::Launch => {
                    self.campaign.quick_launcher = false;
                    self.campaign.inventory();
                    self.save();
                    if let Some(audio) = &mut self.audio {
                        audio.screen(&self.campaign.screen);
                    }
                    self.sync_pause();
                    self.update_view();
                }
                Event::Results => {
                    if let Screen::Ending(results) =
                        std::mem::replace(&mut self.campaign.screen, Screen::Frontend)
                    {
                        self.campaign.screen = Screen::Results(results);
                    }
                    self.update_view();
                }
                Event::Preferences => {
                    self.settings.frontend = self.frontend.as_ref().unwrap().preferences.clone();
                    if !self.settings.frontend.rumble {
                        self.controls.stop_rumble();
                    }
                    if let Some(audio) = &mut self.audio {
                        audio.preferences(&self.settings.frontend);
                    }
                    if let Err(error) = self.settings_store.write(&self.settings) {
                        self.frontend.as_mut().unwrap().notice = error;
                    }
                    self.campaign.checkpoint(self.campaign.options);
                    self.save();
                }
                Event::Save => {
                    self.campaign.checkpoint(self.campaign.options);
                    if let Some(save) = self.campaign.save_request.take() {
                        match self.storage.write(&save) {
                            Ok(()) => {
                                self.campaign.saved = Some(save);
                                self.frontend.as_mut().unwrap().notice = "Campaign saved".into();
                            }
                            Err(error) => self.frontend.as_mut().unwrap().notice = error,
                        }
                    }
                }
                Event::Load => match self.storage.load() {
                    Ok(Some(save)) => {
                        self.campaign.progress = save.progress.clone();
                        self.campaign.seed = save.seed;
                        self.campaign.options.difficulty = save.selection.difficulty;
                        self.campaign.options.language = Some(save.selection.language);
                        self.campaign.saved = Some(save);
                        let frontend = self.frontend.as_mut().unwrap();
                        frontend.notice = match frontend.language(
                            &self.assets,
                            self.campaign.options.language.unwrap_or_default(),
                        ) {
                            Ok(()) => "Campaign loaded".into(),
                            Err(error) => error,
                        };
                    }
                    Ok(None) => {
                        self.frontend.as_mut().unwrap().notice = "No saved campaign yet".into()
                    }
                    Err(error) => self.frontend.as_mut().unwrap().notice = error,
                },
                event => {
                    if let Some(audio) = &mut self.audio {
                        audio.frontend(event);
                    }
                }
            }
        }
    }
}
