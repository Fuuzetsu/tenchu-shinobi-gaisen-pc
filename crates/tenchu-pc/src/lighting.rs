pub mod flames;
mod gaisen;
pub mod shadows;
pub mod sparks;
pub(crate) mod transients;
pub use gaisen::{fire_sources, gunfire, lightning, sources};
use glam::Vec3;

pub const MAX_LIGHTS: usize = 16;
pub const MAX_SCENE_LIGHTS: usize = 256;
pub const MAX_SHADOW_LIGHTS: usize = 8;

#[derive(Clone, Copy)]
pub struct Light {
    pub flame: bool,
    /// Burning bodies emit around their surface; nearby scenery still casts shadows.
    pub emission_radius: f32,
    pub position: Vec3,
    pub color: [f32; 3],
    pub radius: f32,
}

#[derive(Clone)]
pub struct Source {
    pub name: String,
    pub light: Light,
}

#[derive(Default)]
pub struct Sources {
    pub lamps: Vec<Source>,
    pub fires: Vec<Source>,
    pub flashes: Vec<Source>,
}

pub fn select(
    settings: &crate::settings::Lighting,
    focus: Vec3,
    debug_light: Option<Vec3>,
    input: Sources,
) -> Vec<Source> {
    if !settings.enabled || settings.count == 0 {
        return Vec::new();
    }
    let radius = f32::from(settings.radius) * 1000.0;
    let mut lights = Vec::new();
    if let Some(position) = debug_light {
        lights.push(Source {
            name: "Movable light".into(),
            light: Light {
                flame: false,
                emission_radius: 0.0,
                position,
                color: [1.0, 0.88, 0.7],
                radius,
            },
        });
    }
    let mut candidates = if settings.lamps {
        input.lamps
    } else {
        Vec::new()
    };
    for source in &mut candidates {
        source.light.radius *= radius / 12_000.0;
    }
    if settings.fires {
        candidates.extend(input.fires);
    }
    candidates.sort_by(|a, b| {
        focus
            .distance_squared(a.light.position)
            .total_cmp(&focus.distance_squared(b.light.position))
    });
    if settings.fires {
        let mut flashes = input.flashes;
        flashes.sort_by(|a, b| {
            focus
                .distance_squared(a.light.position)
                .total_cmp(&focus.distance_squared(b.light.position))
        });
        lights.extend(flashes.into_iter().take(2));
    }
    lights.extend(candidates);
    lights.truncate(MAX_SCENE_LIGHTS);
    lights
}
#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_core::game::Game;

    #[test]
    #[ignore = "requires TENCHU_DISC; approaching distant lamps does not activate or reset them"]
    fn distant_lights_do_not_switch_on_when_approached() {
        let path = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap())
                .unwrap();
        let game = crate::session::Options::default().load(&assets).unwrap();
        let lamp = Source {
            name: "Distant lantern".into(),
            light: Light {
                flame: false,
                emission_radius: 0.0,
                position: Vec3::new(90_000.0, -2000.0, 0.0),
                color: [1.0, 0.6, 0.3],
                radius: 12_000.0,
            },
        };
        let settings = crate::settings::Lighting {
            enabled: true,
            fires: false,
            count: 1,
            shadow_count: 1,
            ..Default::default()
        };
        let mut shadows = shadows::Selection::default();
        for distance in [90_000.0, 40_000.0, 27_001.0, 26_999.0, 15_000.0, 5000.0] {
            let focus = lamp.light.position - Vec3::X * distance;
            let selected = sources(&game, &settings, Some(focus), None, vec![lamp.clone()]);
            assert_eq!(selected.len(), 1, "Light missing at distance {distance}");
            assert_eq!(selected[0].light.color, lamp.light.color);
            assert_eq!(selected[0].light.radius, lamp.light.radius);
            assert_eq!(
                shadows.update(&[selected[0].light], 1, Some(1.0 / 60.0)),
                [Some((0, 1.0))],
                "Approaching a selected light must not restart its shadows"
            );
        }
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; orbiting the gameplay camera preserves light and shadow selection"]
    fn camera_orbits_do_not_swap_lamps_or_shadow_slots() {
        let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap())
                .unwrap();
        let mut game = crate::session::Options::default().load(&assets).unwrap();
        for _ in 0..40 {
            game.step(0).unwrap();
        }
        assert!(!game.cinematic.active);
        let lamps = crate::visuals::lights::Emitters::build(
            &game,
            &crate::visuals::pack::Pack::default(),
            false,
        )
        .sources(&game);
        let settings = crate::settings::Lighting {
            enabled: true,
            count: 2,
            shadow_count: 1,
            ..Default::default()
        };
        let positions = |game: &Game, camera| {
            sources(game, &settings, camera, None, lamps.clone())
                .into_iter()
                .take(usize::from(settings.count))
                .map(|source| source.light.position)
                .collect::<Vec<_>>()
        };
        let expected = positions(&game, None);
        assert_eq!(expected.len(), 2);
        let target = (expected[0] + expected[1]) * 0.5;
        game.roster.get_mut(game.player_id).position =
            target.to_array().map(|value| value.round() as i32).into();
        let expected = positions(&game, None);
        let mut camera = crate::debug::camera::Camera::from_game(&game);
        let mut camera_selected = Vec::new();
        for angle in 0..24 {
            camera.yaw = angle as f32 * std::f32::consts::TAU / 24.0;
            camera.position = target - camera.forward() * 8000.0;
            game.rendered_view = camera.source_view();
            assert_eq!(positions(&game, None), expected);
            camera_selected.push(positions(&game, Some(camera.position)));
        }
        // This scene crosses the old camera-distance ranking boundary.
        assert!(camera_selected.windows(2).any(|pair| pair[0] != pair[1]));
        // Free-camera travel reprioritizes sources without introducing a distance cutoff.
        let distant = positions(&game, Some(target + Vec3::splat(500_000.0)));
        assert_eq!(distant.len(), 2);
        assert_ne!(distant, expected);
    }
}
