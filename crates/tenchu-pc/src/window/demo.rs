use super::*;
use crate::debug;
use crate::demo::{self, Feature, Shot};
use glam::Vec3;
use std::time::Duration;

impl Application {
    pub(super) fn start_demo_clip(&mut self) -> Result<()> {
        let Some(player) = &mut self.demo else {
            return Ok(());
        };
        let clip = player.clip().clone();
        let options = Options {
            mission: clip.mission,
            character: clip.character,
            language: self.campaign.options.language,
            ..Default::default()
        };
        let mut game = options.load(&self.assets)?;
        if clip.feature != Feature::Cinematics {
            game.events = Default::default();
            for _ in 0..30 {
                game.step(0)?;
            }
        }
        self.settings = Settings {
            volume: self.settings.volume,
            model_directory: self.settings.model_directory.clone(),
            show_hud: false,
            pause_on_defocus: false,
            ..Default::default()
        };
        self.settings.graphics.set_preset(clip.preset);
        self.settings.graphics.asset_pack = std::env::var_os("TENCHU_ASSET_PACK").is_some();
        self.debug = debug::Debug::new(&game)?;
        let view = self.view.as_mut().ok_or("Demo window is not ready")?;
        view.reload(&game, &self.settings)?;
        let origin = Vec3::from(game.player().position.map(|v| v as f32).to_array());
        let mut scene = demo::Scene {
            target: origin - Vec3::Y * 900.0,
            yaw: debug::camera::Camera::from_game(&game).yaw,
            player_start: game.player().position,
            samples: view.renderer.sample_counts.clone(),
            bodies: view.player_models.clone(),
            ..Default::default()
        };
        // Prefer different silhouettes over the player's own duplicate disc entry.
        let own = format!("disc:{}", game.player().kind);
        scene.bodies.retain(|(id, _)| *id != own);
        match clip.shot {
            Shot::Lamp => {
                let lamp = view
                    .nearest_lamp(&game, origin)
                    .ok_or_else(|| format!("Demo '{}' needs a lantern in this stage", clip.id))?;
                scene.target = lamp.light.position;
                scene.distance = 7000.0;
                scene.pitch = -0.30;
                let toward = Vec3::new(scene.yaw.sin(), 0.0, scene.yaw.cos());
                for offset in [
                    -toward * 2000.0,
                    toward * 2000.0,
                    Vec3::X * 2000.0,
                    Vec3::NEG_X * 2000.0,
                ] {
                    let target = (scene.target + offset)
                        .to_array()
                        .map(|v| v.round() as i32)
                        .into();
                    if game.teleport_player(target, true).is_ok() {
                        break;
                    }
                }
                scene.player_start = game.player().position;
            }
            Shot::Guard => {
                let far = clip.feature == Feature::EnemySimulation;
                let actor = game
                    .roster
                    .order
                    .iter()
                    .copied()
                    .filter(|&id| {
                        let actor = game.roster.get(id);
                        id != game.player_id
                            && actor.health > 0
                            && actor.attribute & tenchu_core::ids::ATTR_CUSTOMAI != 0
                            && actor.patrol.is_some()
                    })
                    .min_by(|&a, &b| {
                        let distance = |id| {
                            Vec3::from(game.roster.get(id).position.map(|v| v as f32).to_array())
                                .distance_squared(origin)
                        };
                        let order = distance(a).total_cmp(&distance(b));
                        if far { order.reverse() } else { order }
                    })
                    .ok_or_else(|| format!("Demo '{}' needs a patrol actor", clip.id))?;
                scene.actor = Some(actor);
                scene.distance = if far { 10000.0 } else { 14000.0 };
                scene.pitch = -0.75;
                self.debug.selected_actor = actor;
                if !far {
                    self.settings.keep_enemies_active = true;
                    game.activation.keep_all_active = true;
                }
            }
            Shot::Wide => {
                scene.distance = 20000.0;
                scene.pitch = -0.65;
            }
            Shot::Player => {
                scene.distance = 4800.0;
                scene.pitch = -0.25;
                scene.yaw += 2.2;
            }
            Shot::Cinematic => {}
        }
        player.scene = scene;
        demo::animate(player, &mut self.settings, &mut self.debug);
        self.debug.camera = demo::camera(player, &game);
        game.activation.keep_all_active = self.settings.keep_enemies_active;
        game.intelligence.sight_percent = self.debug.sight_percent;
        self.campaign.game = game;
        self.campaign.options = options;
        self.campaign.screen = Screen::Playing;
        self.campaign.save_request = None;
        self.menu.open = false;
        self.paused = false;
        self.faulted = false;
        self.fault = None;
        self.controls.clear();
        self.clock = GameClock::default();
        self.performance = Default::default();
        view.settings(&self.settings, &self.campaign.game)?;
        view.reset_interpolation(&self.campaign.game);
        if let Some(audio) = &mut self.audio {
            audio.reload(&self.assets, &self.campaign.game)?;
            audio.screen(&self.campaign.screen);
            audio.pause(false);
        }
        println!(
            "Demo {}/{}: {} — {}",
            player.index + 1,
            player.playlist.len(),
            clip.id,
            clip.title
        );
        self.last_frame = Instant::now();
        self.update_view();
        Ok(())
    }

    pub(super) fn advance_demo(&mut self, elapsed: Duration) {
        if self.is_suspended() || self.faulted {
            return;
        }
        self.demo
            .as_mut()
            .unwrap()
            .elapse(elapsed.min(Duration::from_millis(100)));
        while let Some(player) = &mut self.demo {
            let Some(ticks) = player.step() else {
                break;
            };
            let first = player.simulation_ticks - ticks as u64;
            let inputs = (0..ticks)
                .map(|i| player.buttons(first + i as u64))
                .collect::<Vec<_>>();
            let previous = self.settings.clone();
            demo::animate(player, &mut self.settings, &mut self.debug);
            self.campaign.game.activation.keep_all_active = self.settings.keep_enemies_active;
            self.campaign.game.intelligence.sight_percent = self.debug.sight_percent;
            if let Some(audio) = &mut self.audio {
                audio.fast_forward(player.speed() != 100);
            }
            for buttons in inputs {
                if !self.game_tick(buttons) {
                    break;
                }
            }
            if self.faulted {
                return;
            }
            let player = self.demo.as_mut().unwrap();
            if player.clip().feature == Feature::Teleport {
                let frames = u64::from(player.clip().seconds) * 60;
                if player.frame == frames / 3 || player.frame == frames * 2 / 3 {
                    let start = player.scene.player_start;
                    let target = if player.frame == frames / 3 {
                        tenchu_core::math::Position::new(start.x + 1500, start.y, start.z)
                    } else {
                        start
                    };
                    if let Err(error) = self.campaign.game.teleport_player(target, true) {
                        self.pause_error(error);
                        return;
                    }
                    if let Some(view) = &mut self.view {
                        view.reset_interpolation(&self.campaign.game);
                    }
                }
            }
            self.debug.camera = demo::camera(player, &self.campaign.game);
            if self.settings != previous
                && let Some(view) = &mut self.view
                && let Err(error) = view.settings(&self.settings, &self.campaign.game)
            {
                self.pause_error(error);
                return;
            }
            if player.ended() {
                if player.next() {
                    if let Err(error) = self.start_demo_clip() {
                        self.pause_error(error);
                    }
                } else if let Some(audio) = &mut self.audio {
                    audio.pause(true);
                }
                break;
            }
        }
    }

    pub(super) fn demo_action(&mut self, action: demo::overlay::Action) {
        use demo::overlay::Action;
        if matches!(action, Action::Pause) {
            self.set_paused(!self.paused);
            return;
        }
        let Some(player) = &mut self.demo else {
            return;
        };
        match action {
            Action::Previous => player.reset(player.index.saturating_sub(1)),
            Action::Next => {
                if !player.next() {
                    return;
                }
            }
            Action::Restart => player.reset(player.index),
            Action::Solo(index) => {
                player.playlist = vec![index];
                player.reset(0);
            }
            Action::Selected => {
                player.playlist = player
                    .selected
                    .iter()
                    .enumerate()
                    .filter_map(|(i, &s)| s.then_some(i))
                    .collect();
                player.reset(0);
            }
            Action::Exit => {
                self.demo_exit = true;
                return;
            }
            Action::Pause => unreachable!(),
        }
        player.library = false;
        self.menu.open = false;
        if let Err(error) = self.start_demo_clip() {
            self.pause_error(error);
        }
    }

    pub(super) fn demo_key(&mut self, code: KeyCode, pressed: bool) -> bool {
        use demo::overlay::Action;
        if self.demo.is_none() || self.menu.open {
            return false;
        }
        if !pressed {
            return true;
        }
        let action = match code {
            KeyCode::Space | KeyCode::KeyP | KeyCode::F3 => Some(Action::Pause),
            KeyCode::ArrowLeft => Some(Action::Previous),
            KeyCode::ArrowRight => Some(Action::Next),
            KeyCode::Home => Some(Action::Restart),
            KeyCode::Escape => Some(Action::Exit),
            KeyCode::Tab => {
                let player = self.demo.as_mut().unwrap();
                player.library = !player.library;
                self.sync_pause();
                None
            }
            KeyCode::KeyH => {
                let player = self.demo.as_mut().unwrap();
                player.captions = !player.captions;
                None
            }
            KeyCode::KeyV => {
                let player = self.demo.as_mut().unwrap();
                player.transport = !player.transport;
                None
            }
            _ => None,
        };
        if let Some(action) = action {
            self.demo_action(action);
        }
        true
    }
}
