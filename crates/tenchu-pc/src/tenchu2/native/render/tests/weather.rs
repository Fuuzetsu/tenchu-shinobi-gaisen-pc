use super::*;

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; original map weather at a fixed gameplay state"]
fn original_weather_captures() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut graphics = Graphics::default();
    graphics.set_preset(0);
    graphics.asset_pack = false;
    for sequence in [119, 189, 28, 56] {
        let mut session = super::super::super::Session::load(&disc, sequence, 1).unwrap();
        for _ in 0..180 {
            session.frame.effects = session.world.update_effects(&session.assets).unwrap();
        }
        assert!(
            !session.frame.effects.weather.lines.is_empty()
                || !session.frame.effects.weather.sprites.is_empty()
        );
        let before = postcard::to_allocvec(&session.world).unwrap();
        let path = output.join(format!("original-weather-sequence{sequence}.png"));
        capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            Ok((drawing.scene, session.world.camera.view, Vec::new()))
        })
        .unwrap();
        assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC; source weather survives checkpoint replay and visual overrides never alter its state"]
fn original_weather_restore_and_overrides() {
    use crate::settings::WeatherMode;
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    for sequence in [28, 56] {
        let mut session = super::super::super::Session::load(&disc, sequence, 1).unwrap();
        for _ in 0..180 {
            session.frame.effects = session.world.update_effects(&session.assets).unwrap();
        }
        let saved = session.capture_state().unwrap();
        for _ in 0..45 {
            session.frame.effects = session.world.update_effects(&session.assets).unwrap();
        }
        let expected = session.capture_state().unwrap();
        session.restore_state(&saved).unwrap();
        for _ in 0..45 {
            session.frame.effects = session.world.update_effects(&session.assets).unwrap();
        }
        assert_eq!(session.capture_state().unwrap(), expected);
        let state = postcard::to_allocvec(&session.world).unwrap();
        let frame = &session.frame.effects;
        assert!(!frame.weather.lines.is_empty() || !frame.weather.sprites.is_empty());
        let mut no_weather: tenchu_core::tenchu2::effects::Frame =
            postcard::from_bytes(&postcard::to_allocvec(frame).unwrap()).unwrap();
        no_weather.weather = Default::default();
        for preset in [
            0,
            crate::settings::presets::MODERN_HIGH,
            crate::settings::presets::MODERN_ULTRA,
        ] {
            for mode in WeatherMode::ALL {
                let mut graphics = Graphics::default();
                graphics.set_preset(preset);
                graphics.weather.mode = mode;
                let mut lengths = Vec::new();
                for frame in [frame, &no_weather] {
                    let mut builder =
                        Builder::new(session.world.camera.view, &session.assets.level, &graphics);
                    builder
                        .particles(
                            super::super::particles::Particles {
                                scene: &session.world.scene,
                                view: session.world.camera.view,
                                sprites: &session.assets.sprites,
                                frame,
                            },
                            &graphics,
                            1.0,
                            false,
                        )
                        .unwrap();
                    lengths.push(builder.surfaces.len());
                }
                if mode == WeatherMode::Original {
                    assert!(
                        lengths[0] > lengths[1],
                        "sequence {sequence}, preset {preset}"
                    );
                } else {
                    assert_eq!(
                        lengths[0], lengths[1],
                        "sequence {sequence}, preset {preset}, mode {mode:?}"
                    );
                }
            }
        }
        assert_eq!(postcard::to_allocvec(&session.world).unwrap(), state);
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; native weather under Modern High/Ultra and Clear"]
fn original_weather_preset_captures() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    for sequence in [28, 56] {
        let mut session = super::super::super::Session::load(&disc, sequence, 1).unwrap();
        for _ in 0..180 {
            session.frame.effects = session.world.update_effects(&session.assets).unwrap();
        }
        for preset in [
            0,
            crate::settings::presets::MODERN_HIGH,
            crate::settings::presets::MODERN_ULTRA,
        ] {
            let mut graphics = Graphics::default();
            graphics.set_preset(preset);
            graphics.asset_pack = false;
            if preset == 0 {
                graphics.weather.mode = crate::settings::WeatherMode::Clear;
            }
            let path = output.join(format!(
                "original-weather-sequence{sequence}-preset{preset}.png"
            ));
            capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                drawing.show_hud = false;
                drawing.update(&session, renderer, None)?;
                Ok((drawing.scene, session.world.camera.view, Vec::new()))
            })
            .unwrap();
        }
    }
}
