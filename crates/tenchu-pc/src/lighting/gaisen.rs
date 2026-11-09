use super::{Light, Source, Sources, select, transients};
use glam::Vec3;
use tenchu_assets::misc::Kind;
use tenchu_core::game::Game;

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
                        flame: false,
                        emission_radius: 0.0,
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
                    flame: false,
                    emission_radius: 0.0,
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
    if !settings.enabled || settings.count == 0 {
        return Vec::new();
    }
    // Orbiting the player must not reshuffle shadow priority.
    let focus = free_camera.unwrap_or_else(|| {
        if game.cinematic.active {
            crate::debug::camera::Camera::from_game(game).position
        } else {
            Vec3::from(game.player().position.map(|v| v as f32).to_array())
        }
    });
    let radius = f32::from(settings.radius) * 1000.0;
    select(
        settings,
        focus,
        debug_light,
        Sources {
            lamps,
            fires: if settings.fires {
                fire_sources(game, radius).collect()
            } else {
                Vec::new()
            },
            flashes: if settings.fires {
                transients::sources(game, radius)
            } else {
                Vec::new()
            },
        },
    )
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
                flame: true,
                emission_radius: 0.0,
                position,
                color: [1.0, 0.57, 0.25],
                radius,
            },
        })
    })
}
