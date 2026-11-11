use super::{
    campaign::{Content, Playback},
    presentation::ContentDrawing,
    savestate,
    tools::Tools,
    transport::Observation,
};
use crate::{
    renderer::Renderer,
    rewind::{History, Timeline, recording},
    settings::Settings,
};
use std::time::{Duration, Instant};
use tenchu_assets::{ReadAt, Result};

mod recording_io;

#[derive(Clone, Copy)]
pub enum Action {
    Toggle,
    Seek(u64),
    Resume,
    Forward,
    Select([u64; 2]),
    RecordedVisuals(bool),
    Export,
    Import,
}

pub struct State<'a, S> {
    pub playback: &'a mut Playback<S>,
    pub drawing: &'a mut ContentDrawing,
    pub settings: &'a mut Settings,
    pub tools: &'a mut Tools,
    pub renderer: &'a mut Renderer,
}

#[derive(Default)]
pub struct Rewind {
    history: History,
    tick: u64,
    pending: Option<recording::Input>,
    pub active: bool,
    pub preview: bool,
    forward: bool,
    fraction: f64,
    selection: Option<[u64; 2]>,
    recorded_visuals: Option<bool>,
    source_report: Option<String>,
    pub cpu: Duration,
}

fn supported(content: &Content) -> bool {
    matches!(content, Content::Gameplay(_) | Content::Story { .. })
}

fn camera_context(tools: &Tools) -> recording::Input {
    recording::Input {
        camera: tools.debug.camera,
        camera_actor: tools.debug.camera_actor,
        light: tools.debug.light,
        light_follow: tools.debug.light_follow,
        ..Default::default()
    }
}

impl Rewind {
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    pub fn sync(&self, tools: &mut Tools) {
        tools.debug.rewind_bytes = self.history.bytes;
        tools.debug.rewind = self.timeline();
    }

    fn timeline(&self) -> Option<Timeline> {
        let start = self.history.entries.front()?.tick;
        let end = self.history.entries.back()?.tick;
        Some(Timeline {
            start,
            end,
            tick: self.tick,
            playing: self.active,
            preview: self.preview,
            selection: self.selection.map_or([start, end], |[a, b]| {
                [a.clamp(start, end), b.clamp(start, end)]
            }),
            forward: self.forward,
            recorded_visuals: self.recorded_visuals,
        })
    }

    pub fn observed(
        &mut self,
        playback: &Playback<impl ReadAt>,
        input: Observation,
        tools: &Tools,
    ) {
        if self.preview || !supported(&playback.content) {
            return;
        }
        self.tick = self.tick.saturating_add(1);
        self.pending = Some(recording::Input {
            buttons: input.buttons,
            bandage: input.bandage,
            observing_death: input.observing_death,
            native: Some(recording::NativeInput {
                navigation: input.navigation,
                walking: input.walking,
                sheathing: input.sheathing,
                initial: false,
            }),
            ..camera_context(tools)
        });
    }

    pub fn record(
        &mut self,
        playback: &Playback<impl ReadAt>,
        drawing: &ContentDrawing,
        settings: &mut Settings,
        tools: &Tools,
    ) -> Option<String> {
        if !self.preview && settings.rewind_seconds == 0 {
            self.history.clear();
            self.pending = None;
            return None;
        }
        if self.preview
            || settings.rewind_seconds == 0
            || !supported(&playback.content)
            || !self.history.due(self.tick)
        {
            return None;
        }
        let started = Instant::now();
        let result = savestate::capture_rewind(
            playback,
            drawing,
            settings,
            tools,
            &mut self.history.recorder,
        )
        .and_then(|snapshot| {
            self.history.push(
                self.tick,
                settings.rewind_seconds,
                snapshot,
                Some(self.pending.take().unwrap_or_else(|| recording::Input {
                    native: Some(recording::NativeInput {
                        initial: true,
                        walking: match &playback.content {
                            Content::Gameplay(session) => session.world.controls.walking,
                            _ => None,
                        },
                        sheathing: match &playback.content {
                            Content::Gameplay(session) => {
                                session.world.actors[session.world.player].actor.sheathing
                            }
                            _ => None,
                        },
                        navigation: match &playback.content {
                            Content::Gameplay(session) => session.navigation,
                            _ => None,
                        },
                    }),
                    ..camera_context(tools)
                })),
            )
        });
        self.cpu += started.elapsed();
        result.err().map(|error| {
            self.history.clear();
            settings.rewind_seconds = 0;
            format!("Rewind disabled: {error}")
        })
    }

    pub fn stop(&mut self) {
        self.active = false;
        self.fraction = 0.0;
    }

    pub fn resume(&mut self) -> Result<()> {
        if self.preview {
            let index = self
                .history
                .entries
                .iter()
                .position(|e| e.tick == self.tick)
                .ok_or("The selected rewind state is no longer available")?;
            self.history.branch(index)?;
        }
        self.stop();
        self.preview = false;
        self.recorded_visuals = None;
        self.source_report = None;
        self.pending = None;
        Ok(())
    }

    fn seek(
        &mut self,
        tick: u64,
        state: &mut State<'_, impl ReadAt>,
    ) -> std::result::Result<(), savestate::RestoreError> {
        let index = self
            .history
            .entries
            .iter()
            .rposition(|e| e.tick <= tick)
            .unwrap_or(0);
        let entry = self
            .history
            .entries
            .get(index)
            .ok_or_else(|| "No rewind history available".to_string())?;
        let tick = entry.tick;
        let context = if self.recorded_visuals == Some(true) {
            entry
                .input
                .as_ref()
                .map(|i| (i.camera, i.camera_actor, i.light, i.light_follow))
                .unwrap_or_default()
        } else {
            (
                state.tools.debug.camera,
                state.tools.debug.camera_actor,
                state.tools.debug.light,
                state.tools.debug.light_follow,
            )
        };
        let started = Instant::now();
        let result = self
            .history
            .snapshot(index)
            .map_err(savestate::RestoreError::from)
            .and_then(|snapshot| {
                savestate::restore_recorded(
                    state.playback,
                    snapshot,
                    state.settings,
                    state.tools,
                    state.renderer,
                    state.drawing,
                    self.recorded_visuals == Some(true),
                )
            });
        self.cpu += started.elapsed();
        result?;
        self.tick = tick;
        self.preview = true;
        self.pending = None;
        state.drawing.interpolate(1.0);
        (
            state.tools.debug.camera,
            state.tools.debug.camera_actor,
            state.tools.debug.light,
            state.tools.debug.light_follow,
        ) = context;
        self.sync(state.tools);
        Ok(())
    }

    pub fn advance(
        &mut self,
        elapsed: Duration,
        state: &mut State<'_, impl ReadAt>,
    ) -> std::result::Result<bool, savestate::RestoreError> {
        if !self.active {
            return Ok(false);
        }
        self.fraction +=
            elapsed.as_secs_f64().min(0.1) * 30.0 * f64::from(state.settings.rewind_speed) / 100.0;
        let steps = self.fraction as u64;
        if steps == 0 {
            return Ok(false);
        }
        self.fraction -= steps as f64;
        let target = if self.forward {
            self.tick.saturating_add(steps)
        } else {
            self.tick.saturating_sub(steps)
        };
        if let Err(error) = self.seek(target, state) {
            self.stop();
            return Err(error);
        }
        if self.timeline().is_some_and(|t| {
            if self.forward {
                self.tick == t.end
            } else {
                self.tick == t.start
            }
        }) {
            self.stop();
        }
        self.sync(state.tools);
        Ok(true)
    }

    pub fn apply(
        &mut self,
        action: Action,
        state: &mut State<'_, impl ReadAt>,
        report: Option<String>,
    ) -> std::result::Result<Option<String>, savestate::RestoreError> {
        match action {
            Action::Toggle if self.active => self.stop(),
            Action::Toggle | Action::Forward => {
                let forward = matches!(action, Action::Forward);
                let available = self.history.entries.iter().any(|e| {
                    if forward {
                        e.tick > self.tick
                    } else {
                        e.tick < self.tick
                    }
                });
                if !available {
                    return Err("No recorded frame in that direction. Enable rewind history under F1 → Simulation.".to_string().into());
                }
                self.seek(self.tick, state)?;
                self.forward = forward;
                self.active = true;
                self.fraction = 0.0;
            }
            Action::Seek(tick) => {
                self.seek(tick, state)?;
                self.stop();
            }
            Action::Resume => self.resume()?,
            Action::Select(range) => {
                self.selection = Some([range[0].min(range[1]), range[0].max(range[1])])
            }
            Action::RecordedVisuals(value) => {
                let old = self.recorded_visuals.replace(value);
                if let Err(error) = self.seek(self.tick, state) {
                    self.recorded_visuals = old;
                    return Err(error);
                }
            }
            Action::Export => {
                self.stop();
                return self
                    .export(report.unwrap_or_default())
                    .map(Some)
                    .map_err(Into::into);
            }
            Action::Import => {
                self.stop();
                return self.import(state);
            }
        }
        self.sync(state.tools);
        Ok(None)
    }
}

#[cfg(test)]
mod tests;
