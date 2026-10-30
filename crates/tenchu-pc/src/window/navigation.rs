use super::*;

impl Application {
    pub(super) fn navigation_enabled(&self) -> bool {
        self.settings.navigation.camera_relative
            && self.campaign.playing()
            && self.demo.is_none()
            && self.benchmarks.run.is_none()
    }

    pub(super) fn navigation_camera(
        &mut self,
        alpha: f32,
        elapsed: f32,
    ) -> Option<crate::debug::camera::Camera> {
        let game = &self.campaign.game;
        if !self.navigation_enabled()
            || self.debug.camera.is_some()
            || !game.player_can_orbit()
            || game.player().pad.held & tenchu_core::input::L1 != 0
        {
            return None;
        }
        self.orbit.observe(game);
        let view = self.view.as_mut()?;
        Some(self.orbit.camera(
            alpha,
            elapsed,
            &self.settings.navigation,
            view.navigation_surfaces(game),
        ))
    }

    pub(super) fn advance_navigation(&mut self, elapsed: f32) {
        let recenter = self.controls.take_recenter();
        if !self.navigation_enabled() {
            if self.orbit.looking {
                self.release_pointer();
            }
            self.orbit = Default::default();
            self.debug.follow_camera = None;
            return;
        }
        self.orbit.observe(&self.campaign.game);
        if !self.campaign.game.player_can_orbit() && self.orbit.looking {
            self.release_pointer();
        }
        if self.focused
            && !self.menu.open
            && !self.is_suspended()
            && self.debug.camera.is_none()
            && self.campaign.game.player_can_orbit()
            && self.campaign.game.player().pad.held & tenchu_core::input::L1 == 0
        {
            if recenter {
                self.orbit.recenter(&self.campaign.game);
            }
            self.orbit.analog_look(
                self.controls.sticks()[1],
                elapsed,
                &self.settings.navigation,
            );
        }
        self.navigation_camera(1.0, if self.is_suspended() { 0.0 } else { elapsed });
    }

    pub(super) fn navigation_input(&mut self, buttons: u16) -> u16 {
        self.campaign.game.navigation = Default::default();
        if !self.navigation_enabled()
            || self.campaign.game.cinematic.active
            || self.debug.camera.is_some()
        {
            return buttons;
        }
        self.stop_edited_recording();
        self.orbit.observe(&self.campaign.game);
        let sticks = self.controls.sticks();
        let analog = if !self.focused || self.menu.open || self.randomizer.open {
            glam::Vec2::ZERO
        } else if buttons & tenchu_core::input::L1 != 0 || self.campaign.game.player().aiming {
            sticks[0] + sticks[1]
        } else {
            sticks[0]
        };
        let buttons = if self.controls.held(KeyCode::ControlLeft) {
            buttons | tenchu_core::input::R1
        } else {
            buttons
        };
        let (buttons, heading) = self.orbit.movement(
            &self.campaign.game,
            buttons,
            analog,
            &self.settings.navigation,
        );
        let camera = if buttons & tenchu_core::input::L1 == 0 {
            self.navigation_camera(1.0, 0.0)
                .map(crate::navigation::viewpoint)
        } else {
            None
        };
        self.campaign.game.navigation =
            tenchu_core::game::navigation::Navigation { heading, camera };
        buttons
    }
}
