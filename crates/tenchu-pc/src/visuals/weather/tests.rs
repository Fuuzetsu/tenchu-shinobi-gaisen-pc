use super::*;

#[test]
fn changing_wind_redirects_existing_rain_and_snow_and_rechecks_shelter() {
    let surfaces = surfaces();
    for mode in [WeatherMode::Rain, WeatherMode::Snow] {
        let mut weather = Weather::default();
        let mut settings = Settings {
            mode,
            amount: 100,
            wind: 100,
            heading: 90,
            ..Default::default()
        };
        for tick in 0..90 {
            weather.tick(tick, Vec3::ZERO, &surfaces, &settings);
        }
        assert!(weather.particles.len() > 50);
        let wet = weather.wet;
        let snow = weather.snow;
        for (tick, heading) in [(90, 270), (91, 0), (92, 180)] {
            let old: Vec<_> = weather.particles.iter().map(|p| p.position).collect();
            settings.heading = heading;
            weather.tick(tick, Vec3::ZERO, &surfaces, &settings);
            assert!(
                weather.particles.iter().any(|p| old.contains(&p.previous)),
                "Wind changes should redirect existing drops, not clear them all"
            );
            for p in &weather.particles {
                assert!(
                    p.velocity
                        .normalize()
                        .abs_diff_eq(settings.direction(), 0.00001)
                );
                assert!(surfaces.exposed(p.position, p.velocity));
                assert!(
                    surfaces.cast(p.previous, p.position).is_none(),
                    "A changed wind sent a particle through a roof/wall"
                );
            }
        }
        assert!(weather.wet >= wet - 0.001 && weather.snow >= snow - 0.002);
        settings.wind = 0;
        weather.tick(93, Vec3::ZERO, &surfaces, &settings);
        assert!(
            weather
                .particles
                .iter()
                .all(|p| p.velocity.x == 0.0 && p.velocity.z == 0.0)
        );
    }
}

fn surfaces() -> Surfaces {
    let mut vertices = Vec::new();
    for points in [
        [
            [-500000.0, 1000.0, -500000.0],
            [500000.0, 1000.0, -500000.0],
            [-500000.0, 1000.0, 500000.0],
            [500000.0, 1000.0, 500000.0],
        ],
        [
            [-6000.0, -2000.0, -6000.0],
            [6000.0, -7000.0, -6000.0],
            [-6000.0, -2000.0, 6000.0],
            [6000.0, -7000.0, 6000.0],
        ],
    ] {
        vertices.extend([0, 1, 2, 1, 3, 2].map(|i| Vertex {
            position: points[i],
            ..Vertex::zeroed()
        }));
    }
    Surfaces::new(&vertices)
}

#[test]
fn precipitation_stops_at_roofs_and_stays_bounded_through_weather_changes() {
    let surfaces = surfaces();
    let mut weather = Weather::default();
    let mut settings = Settings {
        mode: WeatherMode::Rain,
        amount: 100,
        ..Default::default()
    };
    let mut runoff = 0;
    let mut eaves = 0;
    let mut falling = 0;
    let mut splashes = 0;
    for tick in 0..2400 {
        weather.tick(tick, Vec3::ZERO, &surfaces, &settings);
        runoff = runoff.max(weather.runoff.len());
        eaves = eaves.max(weather.eaves.len());
        falling = falling.max(weather.runoff.iter().filter(|p| p.hit.is_none()).count());
        splashes = splashes.max(weather.splashes.len());
        assert!(weather.particles.len() <= settings.particles());
        assert!(weather.runoff.len() <= RUNOFF_LIMIT && weather.splashes.len() <= SPLASH_LIMIT);
        assert!(weather.eaves.len() <= EAVE_LIMIT);
        for p in &weather.particles {
            assert!(p.position.is_finite());
            assert!(p.position.y < 1001.0);
            assert!(
                surfaces.cast(p.previous, p.position).is_none(),
                "A falling particle crossed a solid roof or floor"
            );
        }
    }
    assert!(runoff > 0 && splashes > 0 && eaves > 0 && falling > 0);
    assert!(weather.sample(&settings, 1.0).wet > 0.6);
    settings.mode = WeatherMode::Snow;
    for tick in 2400..4200 {
        weather.tick(tick, Vec3::ZERO, &surfaces, &settings);
    }
    assert!(weather.runoff.is_empty() && weather.splashes.is_empty());
    assert!(weather.sample(&settings, 1.0).snow > 0.5);
    settings.mode = WeatherMode::Clear;
    weather.tick(4200, Vec3::ZERO, &surfaces, &settings);
    assert!(weather.particles.is_empty());
    let before = weather.sample(&settings, 1.0).snow;
    for tick in 4201..4500 {
        weather.tick(tick, Vec3::ZERO, &surfaces, &settings);
    }
    assert!(weather.sample(&settings, 1.0).snow < before);
    settings.mode = WeatherMode::Original;
    weather.tick(4500, Vec3::ZERO, &surfaces, &settings);
    assert_eq!(weather.sample(&settings, 1.0).wet, 0.0);
    assert_eq!(weather.sample(&settings, 1.0).snow, 0.0);
}

#[test]
fn rendering_cadence_is_read_only_and_teleports_discard_old_particles() {
    let surfaces = surfaces();
    let settings = Settings {
        mode: WeatherMode::Snow,
        ..Default::default()
    };
    let mut a = Weather::default();
    let mut b = Weather::default();
    for tick in 0..90 {
        a.tick(tick, Vec3::ZERO, &surfaces, &settings);
        b.tick(tick, Vec3::ZERO, &surfaces, &settings);
        for alpha in [0.0, 0.1, 0.3, 0.7, 1.0] {
            let mesh = a.geometry(Mat4::IDENTITY, alpha, &settings);
            assert!(mesh.vertices.len() <= settings.particles() * 6);
        }
        assert_eq!(a.seed, b.seed);
        for (a, b) in a.particles.iter().zip(&b.particles) {
            assert_eq!(a.position, b.position);
        }
    }
    a.tick(90, Vec3::new(100000.0, 0.0, 0.0), &surfaces, &settings);
    assert!(a.particles.iter().all(|p| p.previous == p.position));
}

#[test]
fn mesh_changes_preserve_accumulation_and_distant_precipitation_is_bounded() {
    let surfaces = surfaces();
    let settings = Settings {
        mode: WeatherMode::Snow,
        ..Default::default()
    };
    let mut weather = Weather::default();
    for tick in 0..900 {
        weather.tick(tick, Vec3::ZERO, &surfaces, &settings);
    }
    assert!(
        weather
            .particles
            .iter()
            .any(|p| p.position.x.abs() > 40000.0)
    );
    let snow = weather.sample(&settings, 1.0).snow;
    weather.invalidate_surfaces();
    assert!(weather.particles.is_empty());
    assert_eq!(weather.sample(&settings, 1.0).snow, snow);
    weather.tick(900, Vec3::ZERO, &surfaces, &settings);
    assert!(weather.sample(&settings, 1.0).snow >= snow);
    assert!(weather.particles.len() <= settings.particles());
}

#[test]
fn wind_can_enter_under_eaves_and_walls_block_upwind_trajectories() {
    let points = [
        [-500.0, -1000.0, -500.0],
        [500.0, -1000.0, -500.0],
        [-500.0, -1000.0, 500.0],
        [500.0, -1000.0, 500.0],
    ];
    let mut vertices: Vec<_> = [0, 1, 2, 1, 3, 2]
        .map(|i| Vertex {
            position: points[i],
            ..Vertex::zeroed()
        })
        .into();
    let roof = Surfaces::new(&vertices);
    let sheltered = Vec3::ZERO;
    let wind = Vec3::new(2.0, 1.0, 0.0);
    assert!(!roof.exposed(sheltered, Vec3::Y));
    assert!(roof.exposed(sheltered, wind));
    let points = [
        [-700.0, -2000.0, -1000.0],
        [-700.0, 500.0, -1000.0],
        [-700.0, -2000.0, 1000.0],
        [-700.0, 500.0, 1000.0],
    ];
    vertices.extend([0, 1, 2, 1, 3, 2].map(|i| Vertex {
        position: points[i],
        ..Vertex::zeroed()
    }));
    let walled = Surfaces::new(&vertices);
    assert!(!walled.exposed(sheltered, wind));
    let open = Vec3::new(0.0, 0.0, 750.0);
    assert!(walled.exposed(open, Vec3::Y));
    assert!(!walled.exposed(open, wind));
}

#[test]
fn pooling_speed_changes_build_up_but_not_pause_drying_or_particle_budget() {
    let surfaces = surfaces();
    let mut settings = Settings {
        mode: WeatherMode::Rain,
        amount: 100,
        ..Default::default()
    };
    let mut slow = Weather::default();
    let mut fast = Weather::default();
    for tick in 0..300 {
        settings.pooling = 100;
        slow.tick(tick, Vec3::ZERO, &surfaces, &settings);
        settings.pooling = 1000;
        fast.tick(tick, Vec3::ZERO, &surfaces, &settings);
    }
    assert!(fast.puddles > 0.95 && slow.puddles < 0.05);
    let before = fast.sample(&settings, 0.0);
    for alpha in [0.0, 0.3, 0.7, 1.0] {
        fast.tick(299, Vec3::ZERO, &surfaces, &settings);
        let _ = fast.geometry(Mat4::IDENTITY, alpha, &settings);
    }
    assert_eq!(before.puddles, fast.sample(&settings, 0.0).puddles);
    fast.invalidate_surfaces();
    assert_eq!(before.puddles, fast.sample(&settings, 0.0).puddles);
    settings.mode = WeatherMode::Clear;
    for tick in 300..900 {
        fast.tick(tick, Vec3::ZERO, &surfaces, &settings);
    }
    assert!(fast.puddles < 1.0 && fast.puddles > 0.8);
    assert!(fast.particles.is_empty() && fast.eaves.is_empty());
    settings.mode = WeatherMode::Original;
    fast.tick(900, Vec3::ZERO, &surfaces, &settings);
    assert_eq!(fast.puddles, 0.0);
}

#[test]
fn hidden_rain_streaks_keep_roof_drips_and_runoff_toggle_clears_them() {
    let surfaces = surfaces();
    let mut settings = Settings {
        mode: WeatherMode::Rain,
        amount: 100,
        quality: 0,
        streaks: 0,
        pooling: 1000,
        ..Default::default()
    };
    let mut weather = Weather::default();
    for tick in 0..900 {
        weather.tick(tick, Vec3::ZERO, &surfaces, &settings);
    }
    assert!(!weather.runoff.is_empty() && !weather.eaves.is_empty());
    assert!(
        weather
            .runoff
            .iter()
            .all(|p| p.position.is_finite() && p.velocity.is_finite())
    );
    let camera = Mat4::perspective_rh(1.0, 1.0, 10.0, 100000.0)
        * Mat4::look_at_rh(
            Vec3::new(-10000.0, -5000.0, -10000.0),
            Vec3::ZERO,
            Vec3::NEG_Y,
        );
    let geometry = weather.geometry(camera, 0.5, &settings);
    assert!(geometry.vertices.iter().any(|v| v.opacity > 0.3));
    assert!(geometry.vertices.len() <= (RUNOFF_LIMIT + SPLASH_LIMIT * 8) * 6);
    settings.runoff = false;
    weather.tick(900, Vec3::ZERO, &surfaces, &settings);
    assert!(weather.runoff.is_empty() && weather.eaves.is_empty());
    assert!(
        weather
            .geometry(camera, 0.5, &settings)
            .vertices
            .iter()
            .all(|v| v.opacity == 0.0)
    );
}
