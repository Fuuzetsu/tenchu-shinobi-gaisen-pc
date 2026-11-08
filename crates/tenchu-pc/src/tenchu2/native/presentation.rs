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

mod film;

impl Update {
    pub fn play_audio(&self, output: &Output) {
        if self.stop_music {
            output.send(Command::Music(None, false));
        }
        if let Some(music) = &self.music {
            output.send(Command::Music(Some(music.clone()), false));
        }
        for &sound in &self.sounds {
            output.send(Command::Tenchu2Effect(sound));
        }
    }
}

impl Content {
    pub fn tick(&self) -> i64 {
        match self {
            Self::Gameplay(session) => session.world.tick as i64,
            Self::Story { story, .. } => i64::from(story.state.frame),
            Self::Introduction(intro) => i64::from(intro.state.tick),
            Self::Movie(movie) => movie.frame.as_ref().map_or(0, |f| i64::from(f.number)),
            _ => 0,
        }
    }

    pub fn start_audio(&self, output: &Output) {
        output.send(Command::Clear);
        match self {
            Self::Gameplay(session) => output.send(Command::Bank(session.bank.clone())),
            Self::Story { audio, .. } => output.send(Command::Music(audio.clone(), false)),
            Self::Movie(movie) => output.send(Command::Music(movie.audio.clone(), false)),
            _ => output.send(Command::Music(None, false)),
        }
    }

    pub fn inspect(
        &self,
        ctx: &egui::Context,
        run: &Run,
        open: &mut bool,
        paused: &mut bool,
        error: Option<&str>,
    ) -> Option<Action> {
        let mut action = None;
        if matches!(self, Self::Results | Self::Failed | Self::Complete) {
            egui::Window::new(match self {
                Self::Results => "Mission complete",
                Self::Failed => "Mission failed",
                _ => "Campaign complete",
            })
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                if let Self::Results = self
                    && let Some(results) = &run.results
                {
                    ui.heading(format!(
                        "Score {} · rank {} / 6",
                        results.record.score, results.record.rank
                    ));
                    ui.label(format!(
                        "Time {:.1}s",
                        f64::from(results.record.time_tenths) / 10.0
                    ));
                    for (name, value) in [
                        ("Stealth kills", results.summary.kills.stealth_kills),
                        ("Combat kills", results.summary.kills.combat_kills),
                        ("Detections", results.summary.detections),
                        ("Innocents killed", results.summary.kills.innocents),
                    ] {
                        ui.label(format!("{name}: {value}"));
                    }
                    if results.new_best {
                        ui.label("New personal best");
                    }
                    if let Some(item) = results.unlocked_item {
                        ui.label(format!("Unlocked item {item}"));
                    }
                    ui.separator();
                    if ui.button("Continue · Enter").clicked() {
                        action = Some(Action::Continue);
                    }
                }
                if ui.button("Retry mission").clicked() {
                    action = Some(Action::Retry);
                }
                ui.label("Esc: leave preview");
            });
        }
        if matches!(self, Self::Gameplay(_)) || *open || *paused || error.is_some() {
            egui::Area::new("native-status".into())
                .fixed_pos(egui::pos2(12.0, 12.0))
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
                        ui.label("F1: diagnostics · F3: pause · F4: step · Esc: exit");
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
            ui.label("The original HUD, equipment/results artwork and some effects are still being connected.");
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
    Gameplay(Box<Drawing>),
    Story(Box<StoryDrawing>),
    Introduction(Arc<crate::frontend::draw::Frame>),
    Movie {
        texture: Option<egui::TextureHandle>,
        frame: Option<u32>,
    },
    Empty,
}

impl ContentDrawing {
    pub fn new(content: &Content, graphics: &Graphics, renderer: &mut Renderer) -> Result<Self> {
        match content {
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
            _ => Ok(Self::Empty),
        }
    }

    pub fn update(&mut self, content: &Content, renderer: &mut Renderer) -> Result<()> {
        match (self, content) {
            (Self::Gameplay(drawing), Content::Gameplay(session)) => {
                drawing.update(session, renderer)
            }
            (Self::Story(drawing), Content::Story { story, .. }) => drawing.update(story, renderer),
            (Self::Introduction(frame), Content::Introduction(intro)) => {
                *frame = film::introduction(intro);
                Ok(())
            }
            (Self::Movie { .. }, Content::Movie(_)) => Ok(()),
            (Self::Empty, Content::Results | Content::Failed | Content::Complete) => Ok(()),
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
            (Self::Gameplay(drawing), Content::Gameplay(session)) => {
                drawing.draw(session.world.camera.view, target, renderer)
            }
            (Self::Story(drawing), Content::Story { story, .. }) => {
                drawing.draw(story.state.view, target, renderer)
            }
            (Self::Empty, Content::Results | Content::Failed | Content::Complete)
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
            (Self::Story(drawing), _) => {
                crate::fonts::paint(ctx, ctx.screen_rect(), &drawing.texts)
            }
            (Self::Introduction(frame), _) => {
                ctx.layer_painter(egui::LayerId::background()).add(
                    egui_wgpu::Callback::new_paint_callback(
                        film::rect(ctx),
                        crate::frontend::gpu::Paint::Frontend(frame.clone()),
                    ),
                );
            }
            (Self::Movie { texture, frame }, Content::Movie(movie)) => {
                if let Some(current) = &movie.frame {
                    if *frame != Some(current.number) {
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
                        *frame = Some(current.number);
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
