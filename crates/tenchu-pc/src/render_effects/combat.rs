mod gun;

use super::{
    Style, Surface, quad,
    style::{blood_color, lightning_spark},
};
use crate::{render_mesh, visuals::models::rotation};
use glam::{Mat3, Vec3};
use tenchu_core::{effects::Effect, game::Game, math::Matrix};

fn noise(seed: u32) -> f32 {
    let mut n = seed.wrapping_mul(0x45d9f3b);
    n = (n ^ (n >> 16)).wrapping_mul(0x45d9f3b);
    ((n ^ (n >> 16)) & 65535) as f32 / 65535.0
}

struct Painter<'a> {
    surfaces: &'a mut Vec<Surface>,
    camera: Mat3,
    view: Matrix,
}

impl Painter<'_> {
    fn patch(&mut self, center: Vec3, axes: [Vec3; 2], color: [f32; 3], glow: bool, rounded: bool) {
        self.patch_with_opacity(
            center,
            axes,
            color,
            glow,
            rounded,
            if glow { 1.0 } else { 0.9 },
        );
    }

    fn patch_with_opacity(
        &mut self,
        center: Vec3,
        axes: [Vec3; 2],
        color: [f32; 3],
        glow: bool,
        rounded: bool,
        opacity: f32,
    ) {
        let depth = center.dot(self.camera.z_axis) + self.view.translation.z as f32;
        if depth < 50.0 {
            return;
        }
        let [right, up] = axes;
        let mut vertices = quad(
            [
                center - right - up,
                center + right - up,
                center - right + up,
                center + right + up,
            ]
            .map(|p| p.to_array()),
            [color; 4],
            None,
            0,
        );
        for (vertex, i) in vertices.iter_mut().zip([0, 1, 2, 1, 3, 2]) {
            vertex.uv = [
                if i & 1 == 0 { -1.0 } else { 1.0 },
                if !rounded {
                    0.0
                } else if i & 2 == 0 {
                    -1.0
                } else {
                    1.0
                },
            ];
            vertex.opacity = opacity;
            vertex.flags = render_mesh::ALPHA
                | render_mesh::DOUBLE_SIDED
                | render_mesh::FOG
                | render_mesh::SOFT
                | if glow { render_mesh::UNLIT } else { 0 };
        }
        self.surfaces.push(Surface {
            depth,
            blend: 5,
            vertices,
        });
    }

    fn arc(&mut self, a: Vec3, b: Vec3, width: f32, brightness: f32) {
        let along = (b - a) * 0.55;
        let across = (b - a).cross(self.camera.z_axis).normalize_or_zero();
        if across == Vec3::ZERO {
            return;
        }
        let center = (a + b) * 0.5;
        self.patch(
            center,
            [across * width * 4.0, along],
            [0.13 * brightness, 0.3 * brightness, brightness],
            true,
            false,
        );
        self.patch(
            center,
            [across * width, along],
            [1.8 * brightness, 2.1 * brightness, 2.5 * brightness],
            true,
            false,
        );
    }

    fn blood(&mut self, game: &Game, alpha: f32, gunfire: bool) {
        for (slot, effect) in game.effects.active_slots().iter().enumerate() {
            if gunfire && game.effects.gun_sources[slot].is_some() {
                continue;
            }
            let (position, velocity, count, size) = match effect {
                Some(Effect::Blood(blood)) if blood.mode == 0 => (
                    blood.position,
                    blood.velocity,
                    5,
                    (blood.scale as f32 / 8192.0).clamp(0.5, 2.0) * 19.0,
                ),
                Some(Effect::Bleed(bleed)) if blood_color(bleed.color) => {
                    (bleed.position, bleed.velocity, 1, 11.0)
                }
                _ => continue,
            };
            let velocity = Vec3::from(velocity.map(f32::from));
            let position = Vec3::from(position.map(|n| n as f32).to_array())
                - velocity * (1.0 - alpha.clamp(0.0, 1.0));
            let projected = velocity - self.camera.z_axis * velocity.dot(self.camera.z_axis);
            let along = projected.try_normalize().unwrap_or(self.camera.y_axis);
            let across = along.cross(self.camera.z_axis);
            for i in 0..count {
                let seed = (slot as u32 * 37) + i * 13;
                let radius = size * (0.45 + noise(seed));
                let center = position
                    + Vec3::new(
                        noise(seed + 1) - 0.5,
                        noise(seed + 2) - 0.5,
                        noise(seed + 3) - 0.5,
                    ) * size
                        * 3.0
                    - velocity * (noise(seed + 4) * 1.4);
                self.patch(
                    center,
                    [
                        across * radius,
                        along * (radius + projected.length() * 0.22),
                    ],
                    [0.30 + noise(seed + 3) * 0.16, 0.006, 0.012],
                    false,
                    true,
                );
            }
        }
        for (index, burst) in game
            .effects
            .billboards
            .iter()
            .enumerate()
            .filter(|(_, b)| matches!(b.graphic, tenchu_core::effects::Graphic::Impact(2)))
        {
            let center = Vec3::from(burst.position.map(|v| v as f32).to_array());
            let size = (burst.scale[0] as f32 / 300.0).clamp(20.0, 90.0);
            for i in 0..5 {
                let seed = index as u32 * 31 + i * 7;
                let offset = self.camera.x_axis * (noise(seed) - 0.5)
                    + self.camera.y_axis * (noise(seed + 1) - 0.5);
                self.patch_with_opacity(
                    center + offset * size * 2.0,
                    [
                        self.camera.x_axis * size * 0.45,
                        self.camera.y_axis * size * 0.6,
                    ],
                    [0.38, 0.008, 0.016],
                    false,
                    true,
                    0.4,
                );
            }
        }
    }

    fn lightning(&mut self, game: &Game, alpha: f32) {
        for (index, bolt) in game.effects.bolts.iter().take(32).enumerate() {
            let mut previous = Vec3::from(bolt.start.map(|n| n as f32).to_array());
            for (segment, point) in bolt
                .points
                .iter()
                .skip(1)
                .chain([&bolt.end])
                .take(512)
                .enumerate()
            {
                let point = Vec3::from(point.map(|n| n as f32).to_array());
                self.arc(previous, point, 12.0, 1.0);
                if segment % 4 == 2 {
                    let seed = (game.tick as u32)
                        .wrapping_mul(31)
                        .wrapping_add(index as u32 * 71 + segment as u32);
                    let direction = Vec3::new(
                        noise(seed) - 0.5,
                        noise(seed + 1) - 0.5,
                        noise(seed + 2) - 0.5,
                    );
                    let branch = point + direction * 550.0;
                    self.arc(point, branch, 6.0, 0.65);
                }
                previous = point;
            }
            let end = Vec3::from(bolt.end.map(|n| n as f32).to_array());
            self.patch(
                end,
                [self.camera.x_axis * 140.0, self.camera.y_axis * 140.0],
                [0.25, 0.6, 1.6],
                true,
                true,
            );
            for i in 0..7 {
                let seed = (game.tick as u32)
                    .wrapping_mul(43)
                    .wrapping_add(index as u32 * 97 + i * 11);
                let direction = Vec3::new(
                    noise(seed) - 0.5,
                    noise(seed + 1) - 0.5,
                    noise(seed + 2) - 0.5,
                )
                .normalize_or_zero();
                let offset = direction * (80.0 + noise(seed + 3) * 250.0);
                self.arc(end + offset * 0.2, end + offset, 5.0, 0.8);
            }
        }
        for effect in game.effects.active_slots().iter().flatten() {
            if let Effect::Bleed(spark) = effect
                && lightning_spark(spark.color)
            {
                let velocity = Vec3::from(spark.velocity.map(f32::from));
                let position = Vec3::from(spark.position.map(|v| v as f32).to_array())
                    - velocity * (1.0 - alpha.clamp(0.0, 1.0));
                let brightness = (f32::from(spark.time) / 10.0).min(1.0);
                self.arc(position - velocity * 0.8, position, 7.0, brightness);
            }
        }
    }
}

pub(super) fn draw(surfaces: &mut Vec<Surface>, game: &Game, view: Matrix, style: Style) {
    let mut painter = Painter {
        surfaces,
        camera: rotation(view).transpose(),
        view,
    };
    if style.blood.hit == crate::settings::BloodStyle::Modern {
        painter.blood(game, style.alpha, style.gunfire);
    }
    if style.lightning {
        painter.lightning(game, style.alpha);
    }
    if style.gunfire {
        painter.gunfire(game, style.alpha);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_core::{effects, math::Position, terrain::Cursor};

    #[test]
    #[ignore = "requires TENCHU_DISC; overflowing the retail effect pool must not leave permanent modern particles or lights"]
    fn discarded_effects_do_not_leave_particles_or_lights() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let mut game = crate::session::Options::default().load(&assets).unwrap();
        game.effects = Default::default();
        let position = Position::new(0, 0, 2000);
        let style = Style {
            blood: crate::settings::Blood::modern(),
            gunfire: true,
            ..Default::default()
        };
        for _ in 0..201 {
            game.effects.bleed(position, [10, 20, 30], 20, [128, 0, 0]);
        }
        let mut surfaces = Vec::new();
        draw(&mut surfaces, &game, Matrix::default(), style);
        assert!(!surfaces.is_empty(), "live blood still renders");
        let mut rng = game.random;
        game.effects.gun_effects(
            effects::GunEffect {
                part: effects::GunPart::Muzzle,
                direction: Position::new(0, 0, 4096),
            },
            |effects| effects.impact(position, 4096, 1, &mut rng),
        );
        assert!(game.effects.slots[200].is_some());
        assert!(crate::lighting::gunfire(&game, 0.5).is_empty());
        game.effects.slots[..200].fill(None);
        surfaces.clear();
        draw(&mut surfaces, &game, Matrix::default(), style);
        assert!(surfaces.is_empty(), "overflow gunfire never renders");
        game.effects.slots[200] = Some(Effect::Bleed(effects::Bleed {
            position,
            velocity: [10, 20, 30],
            color: [128, 0, 0],
            time: 20,
            mode: 0,
        }));
        game.effects.gun_sources[200] = None;
        draw(&mut surfaces, &game, Matrix::default(), style);
        assert!(surfaces.is_empty(), "overflow blood never renders");
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; optional TENCHU_COMBAT_REVIEW renders native GPU comparisons"]
    fn combat_styles_preserve_simulation_and_work_through_both_cameras() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let mut game = crate::session::Options::default().load(&assets).unwrap();
        game.events = Default::default();
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        let origin = game.player().position;
        let start = origin.wrapping_add(Position::new(-200, -1300, -300));
        let end = origin.wrapping_add(Position::new(900, -600, 500));
        let camera = crate::debug::camera::Camera {
            position: Vec3::from(origin.map(|v| v as f32).to_array())
                + Vec3::new(0.0, -1700.0, -5000.0),
            yaw: 0.0,
            pitch: -0.12,
            fov: 55.0,
        };
        let view = effects::View {
            position: Position::ZERO,
            matrix: camera.source_view(),
        };
        game.effects
            .blood(
                start,
                12,
                40,
                &mut game.random,
                &mut effects::Environment {
                    terrain: &game.mission.terrain,
                    cursor: &mut Cursor::new(&game.mission.terrain),
                    clock: game.clock,
                    owner_position: origin,
                    reference: origin,
                },
            )
            .unwrap();
        for _ in 0..4 {
            game.step(0).unwrap();
        }
        game.effects.lightning(
            start,
            end,
            [100, 100, 200],
            &game.math,
            view,
            &mut game.random,
        );
        let state = crate::session::tests::state(&game);
        let style = Style {
            blood: crate::settings::Blood::modern(),
            lightning: true,
            ..Default::default()
        };
        for view in [game.rendered_view, camera.source_view()] {
            let mut original = Vec::new();
            draw(&mut original, &game, view, Style::default());
            assert!(original.is_empty());
            let mut modern = Vec::new();
            draw(&mut modern, &game, view, style);
            assert!(
                modern
                    .iter()
                    .any(|s| s.vertices[0].flags & render_mesh::UNLIT != 0)
            );
            assert!(
                modern
                    .iter()
                    .any(|s| s.vertices[0].flags & render_mesh::UNLIT == 0)
            );
            assert!(modern.len() < 300);
            assert!(modern.iter().flat_map(|s| &s.vertices).all(|v| {
                v.position.iter().all(|n| n.is_finite())
                    && v.flags & (render_mesh::SCREEN | render_mesh::TEXTURED) == 0
                    && v.flags & render_mesh::SOFT != 0
            }));
            let mut again = Vec::new();
            draw(&mut again, &game, view, style);
            for (a, b) in modern.iter().zip(&again) {
                assert_eq!(
                    bytemuck::cast_slice::<_, u8>(&a.vertices),
                    bytemuck::cast_slice::<_, u8>(&b.vertices)
                );
            }
        }
        let lights = crate::lighting::lightning(&game);
        assert!(!lights.is_empty() && lights.len() <= 2);
        assert_eq!(state, crate::session::tests::state(&game));
        if let Some(directory) = std::env::var_os("TENCHU_COMBAT_REVIEW") {
            let directory = std::path::PathBuf::from(directory);
            std::fs::create_dir_all(&directory).unwrap();
            let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
            let mut settings = crate::settings::Settings::default();
            settings
                .graphics
                .set_preset(crate::settings::presets::MODERN_HIGH);
            let models = review.models(&game.mission).unwrap();
            let stage = review.renderer.upload(&crate::render_mesh::stage(
                &game.mission,
                &game.math,
                Some(&models),
            ));
            let actors = review
                .renderer
                .upload(&crate::render_mesh::characters_presented(
                    &game,
                    Some(&models),
                    true,
                    None,
                ));
            for (label, style) in [("original", Style::default()), ("modern", style)] {
                review.configure(settings.graphics.clone());
                review.lights = if style.lightning {
                    lights.iter().map(|s| s.light).collect()
                } else {
                    Vec::new()
                };
                let geometry = super::super::build_presented(
                    &game,
                    false,
                    Some(camera.source_view()),
                    false,
                    Some(&models),
                    None,
                    style,
                );
                let effects = review.renderer.upload_effects(geometry);
                review
                    .capture(
                        camera.matrix(1280.0 / 720.0),
                        &[&stage, &actors],
                        Some(&effects),
                        true,
                        &directory.join(format!("combat-{label}.png")),
                    )
                    .unwrap();
            }
        }
        game.step(0).unwrap();
        assert!(
            crate::lighting::lightning(&game).is_empty(),
            "light must end with the actual bolt"
        );
    }
}
