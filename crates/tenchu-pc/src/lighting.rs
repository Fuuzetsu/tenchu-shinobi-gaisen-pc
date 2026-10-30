pub mod shadows;
mod transients;
use glam::Vec3;
use tenchu_assets::misc::Kind;
use tenchu_core::game::Game;

pub const MAX_LIGHTS: usize = 16;
pub const MAX_SHADOW_LIGHTS: usize = 8;

#[derive(Clone, Copy)]
pub struct Light {
    pub position: Vec3,
    pub color: [f32; 3],
    pub radius: f32,
}

#[derive(Clone)]
pub struct Source {
    pub name: String,
    pub light: Light,
}

pub fn lightning(game: &Game) -> Vec<Source> {
    let mut sources: Vec<Source> = Vec::new();
    for bolt in &game.effects.bolts {
        for point in [bolt.end, bolt.start] {
            let position = Vec3::from(point.map(|v| v as f32).to_array());
            if sources
                .iter()
                .all(|s| s.light.position.distance_squared(position) > 2500.0_f32.powi(2))
            {
                sources.push(Source {
                    name: "Lightning".into(),
                    light: Light {
                        position,
                        color: [0.45, 0.9, 2.0],
                        radius: 5000.0,
                    },
                });
                if sources.len() == 2 {
                    return sources;
                }
            }
        }
    }
    sources
}

pub fn gunfire(game: &Game, alpha: f32) -> Vec<Source> {
    use tenchu_core::effects::{Effect, GunPart};
    game.effects
        .active_slots()
        .iter()
        .zip(&game.effects.gun_sources)
        .filter_map(|(effect, source)| {
            let (Some(Effect::Impact(impact)), Some(source)) = (effect, source) else {
                return None;
            };
            if !matches!(source.part, GunPart::Muzzle) {
                return None;
            }
            let age = f32::from(impact.count.saturating_sub(1)) + alpha.clamp(0.0, 1.0);
            let strength = (1.0 - age / 3.0).max(0.0).powi(2);
            (strength > 0.0).then(|| Source {
                name: "Gun muzzle".into(),
                light: Light {
                    position: Vec3::from(impact.position.map(|v| v as f32).to_array()),
                    color: [2.0 * strength, 0.9 * strength, 0.25 * strength],
                    radius: 4000.0,
                },
            })
        })
        .take(2)
        .collect()
}

pub fn sources(
    game: &Game,
    settings: &crate::settings::Lighting,
    free_camera: Option<Vec3>,
    debug_light: Option<Vec3>,
    lamps: Vec<Source>,
) -> Vec<Source> {
    if !settings.enabled {
        return Vec::new();
    }
    // Orbiting the player must not reshuffle the limited light and shadow slots.
    let focus = free_camera.unwrap_or_else(|| {
        if game.cinematic.active {
            crate::debug::camera::Camera::from_game(game).position
        } else {
            Vec3::from(game.player().position.map(|v| v as f32).to_array())
        }
    });
    let radius = f32::from(settings.radius) * 1000.0;
    let mut lights = Vec::new();
    if let Some(position) = debug_light {
        lights.push(Source {
            name: "Movable light".into(),
            light: Light {
                position,
                color: [1.0, 0.88, 0.7],
                radius,
            },
        });
    }
    let mut candidates = if settings.lamps { lamps } else { Vec::new() };
    for source in &mut candidates {
        source.light.radius *= radius / 12_000.0;
    }
    if settings.fires {
        candidates.extend(fire_sources(game, radius));
    }
    candidates
        .retain(|s| focus.distance_squared(s.light.position) < (s.light.radius + 15_000.0).powi(2));
    candidates.sort_by(|a, b| {
        focus
            .distance_squared(a.light.position)
            .total_cmp(&focus.distance_squared(b.light.position))
    });
    if settings.fires {
        let mut flashes = transients::sources(game, radius);
        flashes.retain(|s| {
            focus.distance_squared(s.light.position) < (s.light.radius + 15000.0).powi(2)
        });
        flashes.sort_by(|a, b| {
            focus
                .distance_squared(a.light.position)
                .total_cmp(&focus.distance_squared(b.light.position))
        });
        lights.extend(flashes.into_iter().take(2));
    }
    lights.extend(candidates);
    lights.truncate(usize::from(settings.count).min(MAX_LIGHTS));
    lights
}

pub fn fire_sources(game: &Game, radius: f32) -> impl Iterator<Item = Source> + '_ {
    game.misc.objects.iter().filter_map(move |object| {
        if object.mode != 0 || !matches!(object.kind, Kind::Fire | Kind::Bonfire(_)) {
            return None;
        }
        let position = Vec3::from(object.position.map(|v| v as f32).to_array()) - Vec3::Y * 400.0;
        Some(Source {
            name: "Fire".into(),
            light: Light {
                position,
                color: [1.0, 0.57, 0.25],
                radius,
            },
        })
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        // Free-camera travel still selects lights near the inspected scene.
        assert!(positions(&game, Some(target + Vec3::splat(500_000.0))).is_empty());
    }
}
