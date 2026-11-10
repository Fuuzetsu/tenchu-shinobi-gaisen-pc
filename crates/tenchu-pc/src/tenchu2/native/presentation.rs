use super::{
    campaign::{Action, Content, Update},
    render::{Drawing, StoryDrawing},
};
use crate::{
    audio::output::{Command, Output},
    renderer::Renderer,
    settings::Graphics,
};
use std::sync::Arc;
use tenchu_assets::Result;
use tenchu_core::tenchu2::campaign::Run;

mod equipment;
mod film;
mod results;
mod selection;

impl Update {
    pub fn play_audio(&self, output: &Output) {
        if self.stop_music {
            output.send(Command::Music(None, false));
        }
        if let Some(music) = &self.music {
            output.send(Command::Music(Some(music.clone()), false));
        }
        for sound in &self.sounds {
            output.send(Command::Tenchu2Effect(sound.clone()));
        }
    }
}

impl Content {
    pub fn interpolate(&mut self, enabled: bool) -> Result<()> {
        match self {
            Self::Gameplay(session) => session.interpolate(enabled),
            Self::Story { story, .. } => story.interpolate(enabled),
            _ => Ok(()),
        }
    }

    pub fn cutscene(&self) -> bool {
        matches!(
            self,
            Self::Prologue(_) | Self::Introduction(_) | Self::Movie(_) | Self::Story { .. }
        )
    }

    pub fn resume_audio(&self, output: &Output) {
        self.start_audio(output);
        let seconds = match self {
            Self::Prologue(intro) => f64::from(intro.state.tick) / 15.0,
            Self::Introduction(intro) => f64::from(intro.state.tick) / 15.0,
            Self::Movie(movie) => movie.seconds(),
            Self::Story { story, .. } => f64::from(story.state.frame.max(0)) / 30.0,
            _ => return,
        };
        output.send(Command::SeekMusic(seconds));
    }

    pub fn tick(&self) -> i64 {
        match self {
            Self::MissionSelection(selection) => i64::from(selection.state.tick),
            Self::Prologue(intro) => i64::from(intro.state.tick),
            Self::Gameplay(session) => session.world.tick as i64,
            Self::Equipment(selection) => i64::from(selection.state.tick),
            Self::Story { story, .. } => i64::from(story.state.frame),
            Self::Introduction(intro) => i64::from(intro.state.tick),
            Self::Movie(movie) => movie.frame.as_ref().map_or(0, |f| i64::from(f.number)),
            _ => 0,
        }
    }

    pub fn start_audio(&self, output: &Output) {
        output.send(Command::Clear);
        output.send(Command::Music(None, false));
        match self {
            Self::MissionSelection(selection) => {
                output.send(Command::Bank(selection.bank.clone()));
                output.send(Command::Music(Some(selection.music.clone()), false));
            }
            Self::Prologue(intro) => output.send(Command::Music(Some(intro.audio.clone()), false)),
            Self::Gameplay(session) => output.send(Command::Bank(session.bank.clone())),
            Self::Equipment(selection) => output.send(Command::Bank(selection.bank.clone())),
            Self::Results(results) => output.send(Command::Bank(results.bank.clone())),
            Self::Story { story, audio } if story.state.ending.is_none() => {
                output.send(Command::Music(audio.clone(), false));
            }
            Self::Movie(movie) => output.send(Command::Music(movie.audio.clone(), false)),
            Self::Introduction(intro)
                if intro.state.phase == tenchu_core::tenchu2::introduction::Phase::Narration =>
            {
                output.send(Command::Music(intro.audio.clone(), false));
            }
            _ => {}
        }
    }

    pub fn inspect(
        &self,
        ctx: &egui::Context,
        run: &Run,
        open: &mut bool,
        paused: &mut bool,
        error: Option<&str>,
        photo_mode: bool,
    ) -> Option<Action> {
        let mut action = None;
        if let Self::MissionSelection(selection) = self {
            action = selection::controls(selection, ctx);
        } else if let Self::Prologue(_) = self {
            egui::Area::new("jp-prologue-actions".into())
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -8.0])
                .show(ctx, |ui| {
                    ui.spacing_mut().interact_size.y = 36.0;
                    if ui.button("Skip · Start / Circle").clicked() {
                        action = Some(Action::Continue);
                    }
                });
        } else if let Self::Equipment(selection) = self {
            action = equipment::controls(selection, ctx);
        } else if let Self::Results(results) = self {
            egui::Area::new("jp-results-actions".into())
                .anchor(egui::Align2::CENTER_BOTTOM, [0.0, -8.0])
                .show(ctx, |ui| {
                    egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                        if results.values.new_best {
                            ui.label("New personal best");
                        }
                        if let Some(item) = results.values.unlocked_item {
                            ui.label(format!("Unlocked item {item}"));
                        }
                        ui.add_enabled_ui(results.exit.is_none(), |ui| {
                            ui.horizontal(|ui| {
                                if ui.button("Continue · Enter").clicked() {
                                    action = Some(Action::Continue);
                                }
                                if ui.button("Retry mission").clicked() {
                                    action = Some(Action::Retry);
                                }
                                if ui.button("Leave mission").clicked() {
                                    action = Some(Action::Quit);
                                }
                            });
                        });
                    });
                });
        } else if matches!(self, Self::Failed | Self::Complete) {
            egui::Window::new(if matches!(self, Self::Failed) {
                "Mission failed"
            } else {
                "Campaign complete"
            })
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if ui.button("Retry mission").clicked() {
                    action = Some(Action::Retry);
                }
                if ui.button("Mission selection").clicked() {
                    action = Some(Action::SelectMission);
                }
                ui.label("Esc: leave preview");
            });
        }
        if (!photo_mode && (*open || *paused)) || error.is_some() {
            egui::Area::new("native-status".into())
                .anchor(egui::Align2::RIGHT_TOP, [-12.0, 36.0])
                .show(ctx, |ui| {
                    egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                        match self {
                            Self::Gameplay(session) => {
                                let player = &session.world.actors[session.world.player].actor;
                                ui.label(format!(
                                    "JP native preview · area {} · sequence {} · tick {}",
                                    session.world.map, session.world.sequence, session.world.tick
                                ));
                                ui.label(format!(
                                    "Health {} / {} · {:?} · Ki {:?}",
                                    player.health,
                                    player.max_health,
                                    player.state,
                                    session.world.ai.global.radar
                                ));
                                if let Some(item) = player.inventory.selected {
                                    ui.label(format!(
                                        "Item {} × {}",
                                        item.index(),
                                        player.inventory.count(item)
                                    ));
                                }
                                if let tenchu_core::tenchu2::mission::Phase::Finished {
                                    outcome,
                                    ..
                                } = session.world.mission.phase
                                {
                                    ui.label(format!("Mission ended: {outcome:?}."));
                                }
                            }
                            Self::Story { story, .. } => {
                                ui.label(format!(
                                    "JP event {} · sequence {} · frame {} / {}",
                                    story.event,
                                    story.sequence,
                                    story.state.frame,
                                    story.resources.script.duration
                                ));
                            }
                            _ => {
                                ui.label(format!("JP sequence {}", run.sequence));
                            }
                        }
                        ui.label("F1: settings · F3: pause · F4: step · Backspace: fast-forward cutscenes");
                        if *paused {
                            ui.label("PAUSED");
                        }
                        if let Some(error) = error {
                            ui.colored_label(egui::Color32::LIGHT_RED, error);
                        }
                    });
                });
        }
        egui::Window::new("Native JP integration").open(open).default_size([550.0, 480.0]).vscroll(true).show(ctx, |ui| {
            ui.label("Modern lighting and some effects are still being connected.");
            ui.checkbox(paused, "Pause simulation");
            match self {
                Self::Gameplay(session) => {
                    let player = &session.world.actors[session.world.player].actor;
                    ui.label(format!("Objective: {:?}; {:?}", session.world.mission.objective, session.world.mission.phase));
                    ui.label(format!("Player position: {:?}; water: {:?}", player.position, session.world.water));
                    ui.label(format!("Unconnected events observed: {:?}", session.pending));
                    ui.separator();
                    for (index, agent) in session.world.actors.iter().enumerate() {
                        ui.label(format!("{index}: kind {:02x} · {:?} · HP {} · action {:03x} · voice {} · active {}", agent.actor.kind, agent.actor.state, agent.actor.health, agent.actor.motion.action, session.world.instances[index].voice, agent.actor.flags & 8 == 0));
                    }
                }
                Self::Story { story, audio } => {
                    ui.label(format!("{} actor tracks · disc dialogue {}", story.state.actors.len(), if audio.is_some() { "loaded" } else { "absent" }));
                    ui.label("Timed 3D effects and distortion are not connected yet.");
                    if story.state.finished(&story.resources.script) { ui.label("Scene complete."); }
                }
                Self::Introduction(intro) => {
                    ui.label(format!("Chapter introduction · {:?} · narration tick {}", intro.state.phase, intro.state.tick));
                    ui.label("Start / Circle: skip · Start + Select: leave preview");
                }
                Self::Movie(movie) => {
                    ui.label(format!("Movie frame {} · disc audio {}", movie.frame.as_ref().map_or(0, |f| f.number), if movie.audio.is_some() { "loaded" } else { "absent" }));
                    ui.label("Start / Circle: skip · Start + Select: leave preview");
                }
                _ => { ui.label(format!("Mission {} · sequence {}", run.mission, run.sequence)); }
            }
        });
        action
    }
}

pub(crate) enum ContentDrawing {
    MissionSelection(selection::Drawing),
    Prologue(Arc<crate::frontend::draw::Frame>),
    Equipment(equipment::Drawing),
    Gameplay(Box<Drawing>),
    Story(Box<StoryDrawing>),
    Introduction(Arc<crate::frontend::draw::Frame>),
    Movie {
        texture: Option<egui::TextureHandle>,
        frame: Option<Arc<tenchu_assets::movie::Frame>>,
    },
    Results(results::Drawing),
    Empty,
}

impl ContentDrawing {
    pub fn tick(&mut self, content: &Content, graphics: &crate::settings::Graphics) -> Result<()> {
        match (self, content) {
            (Self::Gameplay(drawing), Content::Gameplay(session)) => {
                drawing.tick(session, graphics)
            }
            (Self::Story(drawing), Content::Story { story, .. }) => {
                drawing.tick(story, &graphics.motion)
            }
            _ => Ok(()),
        }
    }

    pub fn interpolate(&mut self, alpha: f32) {
        match self {
            Self::Gameplay(drawing) => drawing.alpha = alpha,
            Self::Story(drawing) => drawing.alpha = alpha,
            _ => {}
        }
    }

    pub fn light_count(&self, settings: &crate::settings::Lighting) -> usize {
        match self {
            Self::Gameplay(drawing) => drawing.light_count(settings),
            Self::Story(drawing) => drawing.light_count(settings),
            _ => 0,
        }
    }

    pub fn clear_blood(&mut self) {
        if let Self::Gameplay(d) = self {
            d.clear_blood();
        }
    }

    pub fn clear_weather(&mut self, tracks_only: bool) {
        match self {
            Self::Gameplay(d) => d.clear_weather(tracks_only),
            Self::Story(d) => d.clear_weather(tracks_only),
            _ => {}
        }
    }

    pub fn weather_audio(
        &mut self,
        settings: &crate::settings::Weather,
    ) -> (tenchu_audio::Ambience, Option<tenchu_audio::ThunderCue>) {
        match self {
            Self::Gameplay(d) => d.weather_audio(settings),
            Self::Story(d) => d.weather_audio(settings),
            _ => (Default::default(), None),
        }
    }

    pub fn new(content: &Content, graphics: &Graphics, renderer: &mut Renderer) -> Result<Self> {
        match content {
            Content::MissionSelection(selection) => Ok(Self::MissionSelection(
                selection::Drawing::new(selection, graphics)?,
            )),
            Content::Prologue(intro) => Ok(Self::Prologue(film::prologue(intro))),
            Content::Equipment(selection) => {
                Ok(Self::Equipment(equipment::Drawing::new(selection)))
            }
            Content::Gameplay(session) => Ok(Self::Gameplay(Box::new(Drawing::new(
                session, graphics, renderer,
            )?))),
            Content::Story { story, .. } => Ok(Self::Story(Box::new(StoryDrawing::new(
                story, graphics, renderer,
            )?))),
            Content::Introduction(intro) => Ok(Self::Introduction(film::introduction(intro))),
            Content::Movie(_) => Ok(Self::Movie {
                texture: None,
                frame: None,
            }),
            Content::Results(results) => Ok(Self::Results(results::Drawing::new(results, None))),
            _ => Ok(Self::Empty),
        }
    }

    pub fn replace(
        &mut self,
        content: &Content,
        vram: &tenchu_assets::texture::Vram,
        graphics: &Graphics,
        renderer: &mut Renderer,
        preserve_environment: bool,
    ) -> Result<()> {
        let backdrop = if matches!(content, Content::Results(_)) {
            self.results_backdrop(renderer)?
        } else {
            None
        };
        if !preserve_environment {
            renderer.reset_vram(vram);
        }
        let mut replacement = if let Content::Results(results) = content {
            Self::Results(results::Drawing::new(results, backdrop))
        } else {
            Self::new(content, graphics, renderer)?
        };
        if preserve_environment {
            match (&mut replacement, &mut *self) {
                (Self::Gameplay(next), Self::Gameplay(old)) => next.preserve_environment(old),
                (Self::Story(next), Self::Story(old)) => next.preserve_environment(old),
                _ => {}
            }
        }
        *self = replacement;
        Ok(())
    }

    pub fn update(
        &mut self,
        content: &Content,
        renderer: &mut Renderer,
        debug: &crate::debug::Debug,
        show_hud: bool,
    ) -> Result<()> {
        match (self, content) {
            (Self::MissionSelection(drawing), Content::MissionSelection(selection)) => {
                drawing.update(selection)
            }
            (Self::Prologue(frame), Content::Prologue(intro)) => {
                *frame = film::prologue(intro);
                Ok(())
            }
            (Self::Equipment(drawing), Content::Equipment(selection)) => {
                drawing.update(selection);
                Ok(())
            }
            (Self::Gameplay(drawing), Content::Gameplay(session)) => {
                drawing.camera = debug.camera.or(debug.follow_camera);
                drawing.show_hud = show_hud && !debug.photo_mode;
                drawing.update(session, renderer, Some(debug))
            }
            (Self::Story(drawing), Content::Story { story, .. }) => {
                drawing.camera = debug.camera.or(debug.follow_camera);
                drawing.update(story, renderer, Some(debug))
            }
            (Self::Introduction(frame), Content::Introduction(intro)) => {
                *frame = film::introduction(intro);
                Ok(())
            }
            (Self::Movie { .. }, Content::Movie(_)) => Ok(()),
            (Self::Results(_), Content::Results(_))
            | (Self::Empty, Content::Failed | Content::Complete) => Ok(()),
            _ => Err("Mismatched native JP scene renderer".into()),
        }
    }

    pub fn draw(
        &self,
        content: &Content,
        target: &wgpu::TextureView,
        renderer: &mut Renderer,
    ) -> Result<()> {
        match (self, content) {
            (Self::Gameplay(drawing), Content::Gameplay(_)) => drawing.draw(target, renderer),
            (Self::Story(drawing), Content::Story { .. }) => drawing.draw(target, renderer),
            (Self::Results(_), Content::Results(_))
            | (Self::MissionSelection(_), Content::MissionSelection(_))
            | (Self::Prologue(_), Content::Prologue(_))
            | (Self::Equipment(_), Content::Equipment(_))
            | (Self::Empty, Content::Failed | Content::Complete)
            | (Self::Introduction(_), Content::Introduction(_))
            | (Self::Movie { .. }, Content::Movie(_)) => {
                let mut encoder = renderer.device.create_command_encoder(&Default::default());
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("JP campaign screen"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Clear(wgpu::Color::BLACK),
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                });
                renderer.queue.submit([encoder.finish()]);
                Ok(())
            }
            _ => Err("Mismatched native JP scene renderer".into()),
        }
    }

    pub fn paint(&mut self, content: &Content, ctx: &egui::Context) {
        match (self, content) {
            (Self::Gameplay(drawing), _) => {
                crate::debug::overlay::paint_labels(ctx, &drawing.labels)
            }
            (Self::MissionSelection(drawing), _) => {
                ctx.layer_painter(egui::LayerId::background()).add(
                    egui_wgpu::Callback::new_paint_callback(
                        film::rect(ctx),
                        crate::frontend::gpu::Paint::Frontend(drawing.frame.clone()),
                    ),
                );
                crate::fonts::paint(ctx, film::rect(ctx), &drawing.text);
            }
            (Self::Equipment(drawing), _) => {
                ctx.layer_painter(egui::LayerId::background()).add(
                    egui_wgpu::Callback::new_paint_callback(
                        film::rect(ctx),
                        crate::frontend::gpu::Paint::Frontend(drawing.frame.clone()),
                    ),
                );
                crate::fonts::paint(ctx, film::rect(ctx), &drawing.text);
            }
            (Self::Story(drawing), _) => {
                crate::fonts::paint(ctx, ctx.screen_rect(), &drawing.texts);
                crate::debug::overlay::paint_labels(ctx, &drawing.labels);
            }
            (Self::Introduction(frame) | Self::Prologue(frame), _)
            | (Self::Results(results::Drawing { frame, .. }), _) => {
                ctx.layer_painter(egui::LayerId::background()).add(
                    egui_wgpu::Callback::new_paint_callback(
                        film::rect(ctx),
                        crate::frontend::gpu::Paint::Frontend(frame.clone()),
                    ),
                );
            }
            (Self::Movie { texture, frame }, Content::Movie(movie)) => {
                if let Some(current) = &movie.frame {
                    if frame.as_ref().map(|f| f.number) != Some(current.number) {
                        let image =
                            egui::ColorImage::from_rgba_unmultiplied(current.size, &current.rgba);
                        if let Some(texture) = texture {
                            texture.set(image, egui::TextureOptions::LINEAR);
                        } else {
                            *texture = Some(ctx.load_texture(
                                "JP movie",
                                image,
                                egui::TextureOptions::LINEAR,
                            ));
                        }
                        *frame = Some(current.clone());
                    }
                    if let Some(texture) = texture {
                        ctx.layer_painter(egui::LayerId::background()).image(
                            texture.id(),
                            film::rect(ctx),
                            egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                    }
                }
            }
            _ => {}
        }
    }
}
