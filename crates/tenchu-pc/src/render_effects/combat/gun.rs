use super::{Painter, noise};
use glam::Vec3;
use tenchu_core::{
    effects::{Effect, GunPart},
    game::Game,
};

impl Painter<'_> {
    pub(super) fn gunfire(&mut self, game: &Game, alpha: f32) {
        for (effect, source) in game
            .effects
            .active_slots()
            .iter()
            .zip(&game.effects.gun_sources)
        {
            let (Some(effect), Some(source)) = (effect, source) else {
                continue;
            };
            match effect {
                Effect::Impact(impact) => {
                    let age = f32::from(impact.count.saturating_sub(1)) + alpha.clamp(0.0, 1.0);
                    let center = Vec3::from(impact.position.map(|v| v as f32).to_array());
                    let direction = Vec3::from(source.direction.map(|v| v as f32).to_array())
                        .normalize_or_zero();
                    let muzzle = matches!(source.part, GunPart::Muzzle);
                    let flash = (1.0 - age / if muzzle { 3.0 } else { 1.8 })
                        .max(0.0)
                        .powi(2);
                    let projected =
                        direction - self.camera.z_axis * direction.dot(self.camera.z_axis);
                    let along = projected.try_normalize().unwrap_or(self.camera.y_axis);
                    let across = along.cross(self.camera.z_axis);
                    if flash > 0.0 {
                        let size = if muzzle { 150.0 } else { 55.0 };
                        for i in 0..3 {
                            let side = i as f32 - 1.0;
                            let axis = (along + across * side * 0.32).normalize();
                            self.patch_with_opacity(
                                center + direction * size * 0.5 + across * side * 18.0,
                                [
                                    axis * size * (0.6 + noise(i + 4) * 0.35),
                                    across * size * 0.12,
                                ],
                                [2.7, 1.3, 0.32],
                                true,
                                true,
                                flash * 0.8,
                            );
                        }
                        self.patch_with_opacity(
                            center + direction * 25.0,
                            [
                                self.camera.x_axis * size * 0.18,
                                self.camera.y_axis * size * 0.18,
                            ],
                            [3.5, 2.8, 1.7],
                            true,
                            true,
                            flash,
                        );
                    }

                    if !matches!(source.part, GunPart::Body) {
                        for i in 0..5 {
                            let seed = i * 17 + 3;
                            let drift = Vec3::new(
                                noise(seed) - 0.5,
                                -0.8 - noise(seed + 1),
                                noise(seed + 2) - 0.5,
                            );
                            let size = 35.0 + age * (6.0 + noise(seed + 3) * 4.0);
                            let position = center
                                + drift * age * 12.0
                                + direction * (age * 14.0 + i as f32 * 25.0);
                            self.patch_with_opacity(
                                position,
                                [self.camera.x_axis * size, self.camera.y_axis * size],
                                if muzzle {
                                    [0.32, 0.29, 0.26]
                                } else {
                                    [0.24, 0.22, 0.19]
                                },
                                false,
                                true,
                                (1.0 - age / 16.0).max(0.0) * 0.28,
                            );
                        }
                    }
                }
                Effect::Bleed(spark) if !matches!(source.part, GunPart::Muzzle) => {
                    let velocity = Vec3::from(spark.velocity.map(f32::from));
                    let position = Vec3::from(spark.position.map(|v| v as f32).to_array())
                        - velocity * (1.0 - alpha.clamp(0.0, 1.0));
                    let projected =
                        velocity - self.camera.z_axis * velocity.dot(self.camera.z_axis);
                    let along = projected.try_normalize().unwrap_or(self.camera.y_axis);
                    let across = along.cross(self.camera.z_axis);
                    let blood = matches!(source.part, GunPart::Body);
                    let life = (f32::from(spark.time) / 8.0).clamp(0.0, 1.0);
                    self.patch_with_opacity(
                        position,
                        [
                            across * if blood { 10.0 } else { 5.0 },
                            along * if blood { 22.0 } else { 40.0 },
                        ],
                        if blood {
                            [0.4, 0.008, 0.016]
                        } else {
                            [2.4, 0.9, 0.18]
                        },
                        !blood,
                        true,
                        life,
                    );
                }
                _ => {}
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        render_effects::{self, Style},
        settings,
    };

    #[test]
    #[ignore = "requires TENCHU_DISC; real gun animations render both camera paths without changing simulation, with optional TENCHU_GUN_REVIEW GPU captures"]
    fn real_gunshots_have_modern_flashes_smoke_and_finite_lights() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let library = crate::visuals::appearance::library::Library::load(&assets).unwrap();
        let echigoya = library.get("disc:128").unwrap();
        let output = std::env::var_os("TENCHU_GUN_REVIEW").map(std::path::PathBuf::from);
        if let Some(output) = &output {
            std::fs::create_dir_all(output).unwrap();
        }
        for character in 0..2 {
            let mut game = crate::session::Options {
                mission: 1,
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            game.events = Default::default();
            game.set_player_moveset(
                echigoya.character.as_ref(),
                &echigoya.clips,
                Default::default(),
            )
            .unwrap();
            let mut settings = settings::Settings::default();
            settings.set_visual_preset(settings::presets::MODERN_HIGH);
            let mut review = output
                .as_ref()
                .map(|_| crate::capture::review::Review::new(&game.mission).unwrap());
            let mut seen = [false; 3];
            for tick in 0..150 {
                game.step(if tick == 30 {
                    tenchu_core::input::SQUARE
                } else {
                    0
                })
                .unwrap();
                let source = game
                    .effects
                    .active_slots()
                    .iter()
                    .zip(&game.effects.gun_sources)
                    .find_map(|(effect, source)| {
                        if let (Some(Effect::Impact(impact)), Some(source)) = (effect, source) {
                            Some((impact.clone(), *source))
                        } else {
                            None
                        }
                    });
                let Some((impact, source)) = source else {
                    if tick > 100 {
                        assert!(crate::lighting::gunfire(&game, 0.5).is_empty());
                    }
                    continue;
                };
                let phase = if impact.count < 3 {
                    0
                } else if impact.count < 8 {
                    1
                } else {
                    2
                };
                let first = !seen[phase] && impact.count > 0;
                seen[phase] |= first;
                let center = Vec3::from(impact.position.map(|v| v as f32).to_array());
                let direction =
                    Vec3::from(source.direction.map(|v| v as f32).to_array()).normalize_or_zero();
                let mut camera = crate::debug::camera::Camera {
                    position: center + direction.cross(Vec3::Y) * 2400.0
                        - direction * 1800.0
                        - Vec3::Y * 900.0,
                    yaw: 0.0,
                    pitch: 0.0,
                    fov: 55.0,
                };
                let delta = center - camera.position;
                camera.yaw = delta.x.atan2(delta.z);
                camera.pitch = (-delta.y).atan2(delta.x.hypot(delta.z));
                let state = crate::session::tests::state(&game);
                for (index, view) in [game.rendered_view, camera.source_view()]
                    .into_iter()
                    .enumerate()
                {
                    let mut original = Vec::new();
                    super::super::draw(&mut original, &game, view, Style::default());
                    assert!(original.is_empty());
                    let mut modern = Vec::new();
                    super::super::draw(&mut modern, &game, view, Style::from(&settings));
                    if index == 1 {
                        assert!(!modern.is_empty(), "the aimed camera must show gunfire");
                    }
                    assert!(
                        modern
                            .iter()
                            .flat_map(|s| &s.vertices)
                            .all(|v| v.position.iter().all(|n| n.is_finite()))
                    );
                }
                let lights = crate::lighting::gunfire(&game, 0.5);
                if phase == 2 {
                    assert!(lights.is_empty());
                }
                assert_eq!(state, crate::session::tests::state(&game));
                if first && let (Some(review), Some(output)) = (&mut review, &output) {
                    review.configure(settings.graphics.clone());
                    let models = review.models(&game.mission).unwrap();
                    let stage = review.renderer.upload(&crate::render_mesh::stage(
                        &game.mission,
                        &game.math,
                        Some(&models),
                    ));
                    let actor = review
                        .renderer
                        .upload(&crate::render_mesh::characters_presented(
                            &game,
                            Some(&models),
                            true,
                            None,
                        ));
                    for modern in [false, true] {
                        let style = Style {
                            gunfire: modern,
                            ..Default::default()
                        };
                        let geometry = render_effects::build_presented(
                            &game,
                            false,
                            Some(camera.source_view()),
                            false,
                            Some(&models),
                            None,
                            style,
                        );
                        let effects = review.renderer.upload_effects(geometry);
                        review.lights = if modern {
                            lights.iter().map(|l| l.light).collect()
                        } else {
                            Vec::new()
                        };
                        review
                            .capture(
                                camera.matrix(1280.0 / 720.0),
                                &[&stage, &actor],
                                Some(&effects),
                                false,
                                &output.join(format!("gun-{character}-{phase}-{modern}.png")),
                            )
                            .unwrap();
                    }
                }
            }
            assert!(
                seen.into_iter().all(|v| v),
                "a real shot must flash, smoke and finish"
            );
        }
    }
}
