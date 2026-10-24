use super::*;

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
    let mut splashes = 0;
    for tick in 0..2400 {
        weather.tick(tick, Vec3::ZERO, &surfaces, &settings);
        runoff = runoff.max(weather.runoff.len());
        splashes = splashes.max(weather.splashes.len());
        assert!(weather.particles.len() <= settings.particles());
        assert!(weather.runoff.len() <= RUNOFF_LIMIT && weather.splashes.len() <= SPLASH_LIMIT);
        for p in &weather.particles {
            assert!(p.position.is_finite());
            assert!(p.position.y < 1001.0);
            assert!(
                surfaces.cast(p.previous, p.position).is_none(),
                "A falling particle crossed a solid roof or floor"
            );
        }
    }
    assert!(runoff > 0 && splashes > 0);
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
