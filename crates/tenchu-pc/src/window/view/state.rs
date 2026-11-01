use super::*;
use crate::visuals::{blood, contact, death, footprints, sky, weather};

#[derive(serde::Serialize, serde::Deserialize)]
pub(in crate::window) struct State {
    blood: blood::Blood,
    sky: sky::Clock,
    weather: weather::Weather,
    footprints: footprints::Footprints,
    contact: contact::Contact,
    deaths: death::Deaths,
}

impl View {
    pub(in crate::window) fn capture_state(&self) -> State {
        State {
            blood: self.blood.clone(),
            sky: self.sky.clone(),
            weather: self.weather.clone(),
            footprints: self.footprints.clone(),
            contact: self.contact.clone(),
            deaths: self.deaths.clone(),
        }
    }

    pub(in crate::window) fn restore_state(&mut self, state: State, game: &Game) {
        self.blood = state.blood;
        self.sky = state.sky;
        self.weather = state.weather;
        self.footprints = state.footprints;
        self.contact = state.contact;
        self.deaths = state.deaths;
        self.reset_interpolation(game);
        self.dirty = true;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visuals::surfaces::Surfaces;
    use glam::Vec3;

    fn tick(
        state: &mut State,
        game: &Game,
        models: &Models,
        surfaces: &Surfaces,
        settings: &Settings,
    ) {
        let eye = Vec3::from(game.player().position.map(|n| n as f32).to_array());
        state.sky.tick(game.tick, &settings.graphics.sky);
        state
            .weather
            .tick(game.tick, eye, surfaces, &settings.graphics.weather);
        let weather = state.weather.sample(&settings.graphics.weather, 1.0);
        state.deaths.tick(game, 100).unwrap();
        state.blood.tick(
            game,
            models,
            surfaces,
            &settings.graphics.blood,
            state.deaths.frame.as_ref(),
        );
        state.footprints.tick(
            game,
            models,
            surfaces,
            &settings.graphics.weather,
            weather.snow,
            eye,
        );
        state
            .contact
            .tick(game, surfaces, &settings.graphics, weather, eye);
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; snapshot alternate death, persistent blood, snow and contact effects while they are active"]
    fn savestates_restore_visual_effects_without_changing_their_continuation() {
        let mut app = crate::window::tests::application(Options::default());
        let mut settings = Settings::default();
        settings.graphics.blood = crate::settings::Blood::modern();
        settings.graphics.sky.enabled = true;
        settings.graphics.sky.speed = 60;
        settings.graphics.weather.mode = crate::settings::WeatherMode::Snow;
        settings.graphics.weather.snow = 100;
        let game = &mut app.campaign.game;
        game.events = Default::default();
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        let models = Models::build(&game.mission, &Default::default(), &settings.graphics).unwrap();
        let surfaces = Surfaces::new(&render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        let mut state = State {
            blood: Default::default(),
            sky: Default::default(),
            weather: Default::default(),
            footprints: Default::default(),
            contact: Default::default(),
            deaths: death::Deaths::load(&app.assets).unwrap(),
        };
        let eye = Vec3::from(game.player().position.map(|n| n as f32).to_array());
        state
            .weather
            .prewarm(game.tick, eye, &surfaces, &settings.graphics.weather, 20);
        state.contact.canopies = models.canopies(game);
        for _ in 0..60 {
            game.step(tenchu_core::input::UP).unwrap();
            tick(&mut state, game, &models, &surfaces, &settings);
        }
        let player = game.roster.get_mut(game.player_id);
        player.health = 0;
        player.status = tenchu_core::actor::State::Dead;
        player.motion.change(
            tenchu_core::ids::MOT_DEAD,
            &game.mission.characters[player.definition].motions,
            &game.mission.common_motions,
            &game.mission.clips,
        );
        for _ in 0..12 {
            game.step(0).unwrap();
            tick(&mut state, game, &models, &surfaces, &settings);
        }
        assert!(state.deaths.frame.is_some());
        assert!(state.weather.sample(&settings.graphics.weather, 1.0).snow > 0.0);
        let bytes = crate::savestate::encode(&state).unwrap();
        let mut restored: State = crate::savestate::decode(&bytes).unwrap();
        assert_eq!(
            postcard::to_allocvec(&state).unwrap(),
            postcard::to_allocvec(&restored).unwrap()
        );
        for _ in 0..240 {
            game.step(0).unwrap();
            for s in [&mut state, &mut restored] {
                tick(s, game, &models, &surfaces, &settings);
            }
        }
        assert!(state.blood.pages().next().is_some());
        assert_eq!(
            postcard::to_allocvec(&state).unwrap(),
            postcard::to_allocvec(&restored).unwrap()
        );
    }
}
