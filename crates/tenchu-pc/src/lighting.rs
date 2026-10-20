use glam::Vec3;
use tenchu_assets::misc::Kind;
use tenchu_core::game::Game;

pub const MAX_LIGHTS: usize = 16;
pub const MAX_SHADOW_LIGHTS: usize = 4;

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

pub fn sources(
    game: &Game,
    settings: &crate::settings::Lighting,
    camera: Vec3,
    debug_light: Option<Vec3>,
    lamps: Vec<Source>,
) -> Vec<Source> {
    if !settings.enabled {
        return Vec::new();
    }
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
        candidates.extend(game.misc.objects.iter().filter_map(|object| {
            if object.mode != 0 || !matches!(object.kind, Kind::Fire | Kind::Bonfire(_)) {
                return None;
            }
            let position = Vec3::from(object.position.map(|v| v as f32)) - Vec3::Y * 400.0;
            Some(Source {
                name: "Fire".into(),
                light: Light {
                    position,
                    color: [1.0, 0.57, 0.25],
                    radius,
                },
            })
        }));
    }
    candidates.retain(|s| {
        camera.distance_squared(s.light.position) < (s.light.radius + 15_000.0).powi(2)
    });
    candidates.sort_by(|a, b| {
        camera
            .distance_squared(a.light.position)
            .total_cmp(&camera.distance_squared(b.light.position))
    });
    lights.extend(candidates);
    lights.truncate(usize::from(settings.count).min(MAX_LIGHTS));
    lights
}
