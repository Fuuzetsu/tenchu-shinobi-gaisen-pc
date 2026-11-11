use super::*;
use crate::tenchu2::native::{campaign::Content, presentation::ContentDrawing};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub(crate) struct State {
    pub(super) scene: SceneState,
    pub(super) motion: crate::visuals::motion::Motion,
    pub(super) wounds: Option<wounds::Wounds>,
}

#[derive(Serialize, Deserialize)]
pub(super) struct SceneState {
    environment: Option<environment::State>,
    sky: crate::visuals::sky::Clock,
}

impl State {
    pub(in crate::tenchu2::native) fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        let Self {
            scene: SceneState { environment, sky },
            motion,
            wounds,
        } = self;
        out.value(&environment.is_some())?;
        if let Some(environment) = environment {
            environment.capture(out)?;
        }
        out.value(sky)?;
        out.value(motion)?;
        out.value(wounds)
    }

    pub fn validate(&self, content: &Content) -> Result<()> {
        if !matches!(
            (content, &self.wounds),
            (Content::Gameplay(_), Some(_)) | (Content::Story { .. }, None)
        ) {
            return Err("Saved native T2 presentation belongs to another scene".into());
        }
        self.motion.validate()
    }
}

impl SceneDrawing {
    pub(super) fn capture_state(&self) -> SceneState {
        let Self {
            draw_distance: _,
            backgrounds: _,
            terrain: _,
            props: _,
            actors: _,
            effects: _,
            environment,
            lighting: _,
            sky,
        } = self;
        SceneState {
            environment: environment
                .as_ref()
                .map(environment::Environment::capture_state),
            sky: sky.clone(),
        }
    }

    pub(super) fn restore_state(
        &mut self,
        state: SceneState,
        level: &Level,
        meshes: &Meshes,
        math: &tenchu_core::math::Math,
        renderer: &mut Renderer,
    ) {
        if let Some(state) = state.environment {
            self.environment(level, meshes, math, renderer)
                .restore_state(state);
        } else {
            self.environment = None;
        }
        self.sky = state.sky;
        self.lighting = lighting::Lighting::new(level);
    }
}

impl Drawing {
    fn capture_state(&self) -> State {
        State {
            scene: self.scene.capture_state(),
            motion: self.motion.clone(),
            wounds: Some(self.wounds.clone()),
        }
    }

    fn restore_state(
        &mut self,
        state: State,
        session: &super::super::Session,
        renderer: &mut Renderer,
    ) -> Result<()> {
        self.models
            .rebind_weapons(&session.assets, &session.world)?;
        let inputs = self
            .models
            .motion_inputs(session, &renderer.graphics().motion)?;
        self.motion = state.motion;
        self.motion.rebind_inputs(&inputs);
        self.wounds = state.wounds.ok_or("Missing native T2 wound state")?;
        self.scene.restore_state(
            state.scene,
            &session.assets.level,
            &self.models.scenery,
            &session.assets.resources.math,
            renderer,
        );
        Ok(())
    }
}

impl ContentDrawing {
    pub fn capture_state(&self) -> Option<State> {
        match self {
            Self::Gameplay(drawing) => Some(drawing.capture_state()),
            Self::Story(drawing) => Some(drawing.capture_state()),
            _ => None,
        }
    }

    pub fn restore_state(
        &mut self,
        state: State,
        content: &Content,
        renderer: &mut Renderer,
    ) -> Result<()> {
        state.validate(content)?;
        match (self, content) {
            (Self::Gameplay(drawing), Content::Gameplay(session)) => {
                drawing.restore_state(state, session, renderer)
            }
            (Self::Story(drawing), Content::Story { story, .. }) => {
                drawing.restore_state(state, story, renderer)
            }
            _ => Err("Mismatched native T2 scene renderer".into()),
        }
    }
}

#[cfg(test)]
mod tests;
