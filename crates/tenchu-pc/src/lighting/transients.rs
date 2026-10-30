use super::{Light, Source};
use glam::Vec3;
use tenchu_core::{effects::Effect, game::Game, items::Processor};

pub(super) fn sources(game: &Game, radius: f32) -> Vec<Source> {
    let mut lights: Vec<Source> = Vec::new();
    let mut add = |name: &str, position: Vec3, strength: f32, reach: f32| {
        if strength <= 0.01 {
            return;
        }
        let light = Light {
            position,
            color: [strength, strength * 0.43, strength * 0.12],
            radius: radius * reach,
        };
        if let Some(cluster) = lights
            .iter_mut()
            .find(|s| s.light.position.distance_squared(position) < 2000.0_f32.powi(2))
        {
            if strength > cluster.light.color[0] {
                cluster.light = light;
                cluster.name = name.into();
            }
        } else {
            lights.push(Source {
                name: name.into(),
                light,
            });
        }
    };
    for effect in game.effects.slots.iter().flatten() {
        let (name, effect, strength, reach) = match effect {
            Effect::Explosion(e) => (
                "Explosion",
                e,
                match e.mode {
                    0 => 3.0,
                    1 => 2.0,
                    _ => 1.4 * f32::from(e.time) / 5.0,
                },
                0.9,
            ),
            Effect::Ember(e) => (
                "Burning embers",
                e,
                0.35 * if e.mode == 0 {
                    1.0
                } else {
                    f32::from(e.time) / 30.0
                },
                0.45,
            ),
            _ => continue,
        };
        add(
            name,
            Vec3::from(effect.position.map(|n| n as f32).to_array())
                - Vec3::Y * (effect.scale as f32 / 4096.0 * 100.0).clamp(200.0, 1000.0),
            strength,
            reach,
        );
    }
    for item in &game.items.slots {
        if item.processor() != Some(Processor::Napalm) || item.source_mode() != 1 {
            continue;
        }
        if let Some(model) = game.scene.models.get(item.model) {
            let strength = 1.8 * (1.0 - f32::from(item.napalm.count) / 22.0).max(0.0);
            add(
                "Kaen flame",
                Vec3::from(model.local.translation.map(|n| n as f32).to_array()),
                strength,
                0.6,
            );
        }
    }
    lights
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; actual thrown explosives and kaen emit light only during their effects"]
    fn item_lights_follow_the_actual_effect_lifecycle() {
        let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for (kind, name) in [(4, "Explosion"), (17, "Kaen flame")] {
            let mut inventory = [0; 20];
            inventory[0] = 255;
            inventory[kind] = 2;
            let mut game = crate::session::Options::default()
                .load_equipped(&assets, 1, inventory)
                .unwrap();
            let mut seen = false;
            let mut lit_ticks = 0;
            let settings = crate::settings::Lighting {
                enabled: true,
                count: 1,
                ..Default::default()
            };
            for tick in 0..400 {
                game.step(match tick {
                    80 => tenchu_core::input::R2,
                    82 => tenchu_core::input::TRIANGLE,
                    _ => 0,
                })
                .unwrap();
                let state = crate::session::tests::state(&game);
                let lights = sources(&game, 12000.0);
                if lights.iter().any(|source| source.name == name) {
                    lit_ticks += 1;
                    if lit_ticks == 5
                        && let Some(output) = std::env::var_os("TENCHU_LIGHT_REVIEW")
                    {
                        let output = std::path::PathBuf::from(output);
                        std::fs::create_dir_all(&output).unwrap();
                        crate::capture::frame(
                            &game,
                            &output.join(format!("item-{kind}.png")),
                            &assets,
                        )
                        .unwrap();
                    }
                    seen = true;
                    let selected = super::super::sources(
                        &game,
                        &settings,
                        None,
                        None,
                        vec![Source {
                            name: "Nearby lantern".into(),
                            light: Light {
                                position: Vec3::from(
                                    game.player().position.map(|n| n as f32).to_array(),
                                ),
                                color: [1.0; 3],
                                radius: 12000.0,
                            },
                        }],
                    );
                    assert_ne!(
                        selected[0].name, "Nearby lantern",
                        "flashes must not be crowded out by static lamps"
                    );
                }
                assert_eq!(state, crate::session::tests::state(&game));
                assert!(
                    lights
                        .iter()
                        .all(|s| s.light.position.is_finite() && s.light.radius > 0.0)
                );
            }
            assert!(seen, "{name} was never lit");
            assert!(
                sources(&game, 12000.0).is_empty(),
                "{name} light outlived the effect"
            );
        }
    }
}
