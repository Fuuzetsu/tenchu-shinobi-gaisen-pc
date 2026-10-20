use glam::Vec3;
use tenchu_assets::misc::Kind;
use tenchu_core::game::Game;

#[derive(Clone, Copy)]
pub struct Light {
    pub position: Vec3,
    pub color: [f32; 3],
    pub radius: f32,
}

pub fn sources(
    game: &Game,
    settings: &crate::settings::Lighting,
    camera: Vec3,
    debug_light: Option<Vec3>,
) -> Vec<Light> {
    if !settings.enabled {
        return Vec::new();
    }
    let radius = f32::from(settings.radius) * 1000.0;
    let mut lights = Vec::new();
    if let Some(position) = debug_light {
        lights.push(Light {
            position,
            color: [1.0, 0.88, 0.7],
            radius,
        });
    }
    if settings.fires {
        let mut fires: Vec<_> = game
            .misc
            .objects
            .iter()
            .filter_map(|object| {
                if object.mode != 0 || !matches!(object.kind, Kind::Fire | Kind::Bonfire(_)) {
                    return None;
                }
                let position = Vec3::from(object.position.map(|v| v as f32)) - Vec3::Y * 400.0;
                let distance = camera.distance_squared(position);
                (distance < (radius + 15000.0).powi(2)).then_some((distance, position))
            })
            .collect();
        fires.sort_by(|a, b| a.0.total_cmp(&b.0));
        for (_, position) in fires {
            if lights.len() >= usize::from(settings.count) {
                break;
            }
            lights.push(Light {
                position,
                color: [1.0, 0.57, 0.25],
                radius,
            });
        }
    }
    lights.truncate(usize::from(settings.count));
    lights
}
