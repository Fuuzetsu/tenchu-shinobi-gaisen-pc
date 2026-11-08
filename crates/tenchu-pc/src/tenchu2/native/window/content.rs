use super::*;

pub enum Content {
    Gameplay(Box<Session>),
    Story {
        story: Box<Story>,
        audio: Option<Arc<Clip>>,
    },
}

impl Content {
    pub fn story_finished(&self) -> bool {
        matches!(self, Self::Story { story, .. } if story.state.finished(&story.resources.script))
    }

    pub fn vram(&self) -> &Vram {
        match self {
            Self::Gameplay(session) => &session.assets.sprites.vram,
            Self::Story { story, .. } => &story.level.vram,
        }
    }

    pub fn tick(&self) -> i64 {
        match self {
            Self::Gameplay(session) => session.world.tick as i64,
            Self::Story { story, .. } => i64::from(story.state.frame),
        }
    }

    pub fn step(&mut self, buttons: u16) -> Result<Vec<Sound>> {
        match self {
            Self::Gameplay(session) => session.step(buttons),
            Self::Story { story, .. } => {
                story.step(true)?;
                Ok(Vec::new())
            }
        }
    }

    pub fn start_audio(&self, output: &Output) {
        match self {
            Self::Gameplay(session) => output.send(Command::Bank(session.bank.clone())),
            Self::Story { audio, .. } => output.send(Command::Music(audio.clone(), false)),
        }
    }

    pub fn inspect(
        &self,
        ctx: &egui::Context,
        open: &mut bool,
        paused: &mut bool,
        error: Option<&str>,
    ) {
        if matches!(self, Self::Gameplay(_)) || *open || *paused || error.is_some() {
            egui::Area::new("native-status".into()).fixed_pos(egui::pos2(12.0, 12.0)).show(ctx, |ui| {
                egui::Frame::dark_canvas(ui.style()).show(ui, |ui| {
                    match self {
                        Self::Gameplay(session) => {
                            let player = &session.world.actors[session.world.player].actor;
                            ui.label(format!("JP native preview · area {} · sequence {} · tick {}", session.world.map, session.world.sequence, session.world.tick));
                            ui.label(format!("Health {} / {} · {:?} · Ki {:?}", player.health, player.max_health, player.state, session.world.ai.global.radar));
                            if let Some(item) = player.inventory.selected { ui.label(format!("Item {} × {}", item.index(), player.inventory.count(item))); }
                            if let tenchu_core::tenchu2::mission::Phase::Finished { outcome, .. } = session.world.mission.phase {
                                ui.label(format!("Mission ended: {outcome:?}."));
                                if let Some(results) = &session.world.results {
                                    ui.label(format!("Score {} · rank {} / 6 · time {:.1}s", results.record.score, results.record.rank, results.record.time_tenths as f64 / 10.0));
                                    ui.label(format!("Stealth kills {} · combat kills {} · detections {} · innocents {}", results.summary.kills.stealth_kills, results.summary.kills.combat_kills, results.summary.detections, results.summary.kills.innocents));
                                    if let Some(item) = results.unlocked_item { ui.label(format!("Unlocked item {item}")); }
                                } else { ui.label("The next story screen is not connected yet."); }
                            }
                        }
                        Self::Story { story, .. } => {
                            ui.label(format!("JP event {} · sequence {} · frame {} / {}", story.event, story.sequence, story.state.frame, story.resources.script.duration));
                        }
                    }
                    ui.label("F1: diagnostics · F3: pause · F4: step · Esc: exit");
                    if *paused { ui.label("PAUSED"); }
                    if let Some(error) = error { ui.colored_label(egui::Color32::LIGHT_RED, error); }
                });
            });
        }
        egui::Window::new("Native JP integration").open(open).default_size([550.0, 480.0]).vscroll(true).show(ctx, |ui| {
            ui.label("The original HUD, campaign transitions and some effects are still being connected.");
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
                    ui.label("Timed 3D effects, distortion and the following sequence are not connected yet.");
                    if story.state.finished(&story.resources.script) { ui.label("Scene complete."); }
                }
            }
        });
    }
}

pub(super) enum ContentDrawing {
    Gameplay(Box<Drawing>),
    Story(Box<StoryDrawing>),
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
        }
    }

    pub fn update(&mut self, content: &Content, renderer: &mut Renderer) -> Result<()> {
        match (self, content) {
            (Self::Gameplay(drawing), Content::Gameplay(session)) => {
                drawing.update(session, renderer)
            }
            (Self::Story(drawing), Content::Story { story, .. }) => drawing.update(story, renderer),
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
            _ => Err("Mismatched native JP scene renderer".into()),
        }
    }
}
