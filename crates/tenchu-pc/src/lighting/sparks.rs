use super::{Light, Source};
use glam::Vec3;
use tenchu_core::{effects::Effect, game::Game};

pub fn sources(
    game: &Game,
    alpha: f32,
    density: u16,
    extra: impl IntoIterator<Item = Light>,
) -> Vec<Source> {
    if density == 0 {
        return Vec::new();
    }
    let alpha = alpha.clamp(0.0, 1.0);
    let particles = game
        .effects
        .active_slots()
        .iter()
        .enumerate()
        .filter_map(|(slot, e)| {
            let Some(Effect::Bleed(spark)) = e else {
                return None;
            };
            if game.effects.gun_sources[slot].is_some()
                || !crate::render_effects::metal_spark(spark.color)
            {
                return None;
            }
            let strength = ((f32::from(spark.time) + 1.0 - alpha) / 6.0).clamp(0.0, 1.0)
                * f32::from(density.min(300))
                / 100.0
                * 0.12;
            Some(Light {
                flame: false,
                emission_radius: 0.0,
                position: Vec3::from(spark.position.to_array().map(|v| v as f32))
                    - Vec3::from(spark.velocity.map(f32::from)) * (1.0 - alpha),
                color: if spark.color[2] != 0 {
                    [0.4, 0.65, 1.0]
                } else {
                    [1.0, 0.48, 0.1]
                }
                .map(|v| v * strength),
                radius: 2400.0,
            })
        });
    clusters(particles.chain(extra))
}

pub(crate) fn clusters(particles: impl IntoIterator<Item = Light>) -> Vec<Source> {
    let mut groups: Vec<Light> = Vec::new();
    for light in particles {
        let color = Vec3::from(light.color);
        let weight = color.element_sum();
        if weight <= 0.0 {
            continue;
        }
        if let Some(group) = groups
            .iter_mut()
            .find(|g| g.position.distance_squared(light.position) < 800.0_f32.powi(2))
        {
            let previous = Vec3::from(group.color);
            group.position = (group.position * previous.element_sum() + light.position * weight)
                / (previous.element_sum() + weight);
            group.color = (previous + color).to_array();
        } else {
            groups.push(light);
        }
    }
    groups.sort_by(|a, b| {
        Vec3::from(b.color)
            .element_sum()
            .total_cmp(&Vec3::from(a.color).element_sum())
    });
    groups
        .into_iter()
        .take(2)
        .map(|mut light| {
            let color = Vec3::from(light.color);
            light.color = (color / color.max_element().max(1.0)).to_array();
            Source {
                name: "Flying sparks".into(),
                light,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn moving_sparks_keep_bounded_lights_without_inflating_brightness() {
        let lights = |offset: Vec3, strength| {
            clusters((0..200).map(|i| Light {
                flame: false,
                emission_radius: 0.0,
                position: offset + Vec3::new(i as f32 * 12.0, 0.0, 0.0),
                color: [strength, strength * 0.5, 0.0],
                radius: 2400.0,
            }))
        };
        let before = lights(Vec3::ZERO, 0.1);
        let after = lights(Vec3::Y * 300.0, 0.1);
        assert_eq!(before.len(), 2);
        for (a, b) in before.iter().zip(&after) {
            assert!((b.light.position - a.light.position - Vec3::Y * 300.0).length() < 0.01);
            assert!(b.light.color.iter().all(|c| *c <= 1.0));
        }
        assert!(lights(Vec3::ZERO, 0.0).is_empty());
    }
}
