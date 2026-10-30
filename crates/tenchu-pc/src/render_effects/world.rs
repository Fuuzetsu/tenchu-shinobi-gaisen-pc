use super::{Style, Surface, quad};
use crate::{render_mesh, visuals::models::rotation};
use glam::Vec3;
use tenchu_core::{effects::Graphic, game::Game, math::Matrix};

pub(super) fn material(graphic: &Graphic) -> (Option<usize>, u8, bool) {
    match *graphic {
        Graphic::Sprite { image, blend } => (Some(usize::from(image)), blend, false),
        Graphic::Explosion(index) => (Some(7 + usize::from(index)), 2, false),
        Graphic::Smoke(index) => (Some([6, 58][usize::from(index)]), 2, false),
        Graphic::Splash => (Some(14), 2, true),
        Graphic::Bleed | Graphic::GunBleed => (None, 0, false),
        Graphic::Flash(index) => (Some(40 + index as usize), 2, false),
        Graphic::Impact(index) | Graphic::GunImpact(index) => {
            (Some([15, 11, 12, 59, 60][usize::from(index)]), 2, false)
        }
        Graphic::Blood {
            sprite,
            stain,
            fading,
            ..
        } => (
            Some([0, 2, 4, 56][usize::from(sprite)] + usize::from(stain)),
            if stain {
                3
            } else if fading {
                2
            } else {
                0
            },
            false,
        ),
    }
}

pub(super) fn flags(graphic: &Graphic, color: [u8; 3]) -> u32 {
    if matches!(graphic, Graphic::Splash)
        || matches!(graphic, Graphic::Bleed | Graphic::GunBleed)
            && color[2] > color[0]
            && u16::from(color[0]) * 5 > u16::from(color[2]) * 4
    {
        render_mesh::ORIGINAL_SPLASH
    } else if matches!(
        graphic,
        Graphic::Sprite {
            image: 55,
            blend: 2
        }
    ) {
        render_mesh::ORIGINAL_WEATHER
    } else {
        0
    }
}

fn depth(view: Matrix, point: Vec3) -> f32 {
    point.dot(Vec3::from(view.rotation[2].map(|v| f32::from(v) / 4096.0)))
        + view.translation.z as f32
}

pub(super) fn trail(
    surfaces: &mut Vec<Surface>,
    trail: &tenchu_core::actor::trail::Trail,
    image: tenchu_assets::texture::Image,
    view: Matrix,
) {
    let mut previous = 127;
    for i in 1..trail.count {
        let shade = ((trail.count - i) * 127 / trail.count) as u8;
        let points = [
            trail.world_points[i][0],
            trail.world_points[i - 1][0],
            trail.world_points[i][1],
            trail.world_points[i - 1][1],
        ]
        .map(|p| p.map(|v| v as f32).to_array());
        let z = points.iter().map(|&p| depth(view, p.into())).sum::<f32>() / 4.0;
        if z >= 50.0 {
            let colors = [shade, previous, shade, previous].map(|v| [f32::from(v) / 128.0; 3]);
            let mut vertices = quad(points, colors, Some(image), 2);
            for vertex in &mut vertices {
                vertex.flags = (vertex.flags & !render_mesh::SCREEN)
                    | render_mesh::DOUBLE_SIDED
                    | render_mesh::UNLIT;
            }
            surfaces.push(Surface {
                depth: z,
                blend: 2,
                vertices,
            });
        }
        previous = shade;
    }
}

pub(super) fn sprites(surfaces: &mut Vec<Surface>, game: &Game, view: Matrix, style: Style) {
    let camera = rotation(view).transpose();
    for sprite in game
        .effects
        .billboards
        .iter()
        .filter(|sprite| !style.replaces(&sprite.graphic, sprite.color))
    {
        let (image, blend, bottom_origin) = material(&sprite.graphic);
        let image = image.map(|i| game.mission.images[i]);
        let size = if let Some(image) = image {
            std::array::from_fn(|i| f32::from(image.size[i]) * sprite.scale[i] as f32 / 1024.0)
        } else {
            sprite.scale.map(|v| v as f32)
        };
        let origin = [
            size[0] * 0.5,
            size[1] * if bottom_origin { 1.0 } else { 0.5 },
        ];
        let center = Vec3::from(sprite.position.map(|v| v as f32).to_array());
        let z = depth(view, center);
        if z < 50.0 {
            continue;
        }
        let (sin, cos) = (sprite.rotation as f32 / 4096.0).to_radians().sin_cos();
        let points = std::array::from_fn(|i| {
            let x = (i & 1) as f32 * size[0] - origin[0];
            let y = (i >> 1) as f32 * size[1] - origin[1];
            (center + camera * Vec3::new(x * cos - y * sin, x * sin + y * cos, 0.0)).to_array()
        });
        let color = sprite
            .color
            .map(|v| f32::from(v) / if image.is_some() { 128.0 } else { 255.0 });
        let mut vertices = quad(points, [color; 4], image, blend);
        for vertex in &mut vertices {
            vertex.flags = (vertex.flags & !render_mesh::SCREEN)
                | render_mesh::DOUBLE_SIDED
                | render_mesh::UNLIT
                | flags(&sprite.graphic, sprite.color);
        }
        surfaces.push(Surface {
            depth: z,
            blend,
            vertices,
        });
    }
}

pub(super) fn line(
    surfaces: &mut Vec<Surface>,
    points: [Vec3; 2],
    color: [u8; 3],
    blend: u8,
    view: Matrix,
) {
    let [a, b] = points;
    let z = depth(view, (a + b) * 0.5);
    if z < 50.0 {
        return;
    }
    let forward = rotation(view).transpose() * Vec3::Z;
    let side = (b - a).cross(forward).normalize_or_zero() * (z / 600.0).max(1.0);
    let mut vertices = quad(
        [a - side, b - side, a + side, b + side].map(|p| p.to_array()),
        [color.map(|v| f32::from(v) / 255.0); 4],
        None,
        blend,
    );
    for vertex in &mut vertices {
        vertex.flags =
            (vertex.flags & !render_mesh::SCREEN) | render_mesh::DOUBLE_SIDED | render_mesh::UNLIT;
    }
    surfaces.push(Surface {
        depth: z,
        blend,
        vertices,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_core::{effects, math::Position, terrain::Cursor};

    #[test]
    #[ignore = "requires TENCHU_DISC"]
    fn custom_camera_recovers_culled_item_effects_without_changing_game_state() {
        let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let mut game = crate::session::Options::default().load(&assets).unwrap();
        game.effects = effects::Effects::default();
        let point = Position::new(0, -1000, 3000);
        game.effects.explosion(point, [0; 3], &mut game.random);
        game.effects
            .smoke_single(point, [0; 3], 60, &mut game.random)
            .unwrap();
        let hidden = effects::View {
            position: Position::ZERO,
            matrix: Matrix {
                rotation: game.math.rotate_yxz([0, 2048, 0]),
                ..Matrix::default()
            },
        };
        game.effects
            .step(
                &mut game.random,
                hidden,
                &mut game.scene,
                &mut effects::Environment {
                    terrain: &game.mission.terrain,
                    cursor: &mut Cursor::new(&game.mission.terrain),
                    clock: 1,
                    owner_position: point,
                    reference: point,
                },
            )
            .unwrap();
        game.effects.lightning(
            point,
            Position::new(1000, -1000, 3000),
            [100, 100, 200],
            &game.math,
            hidden,
            &mut game.random,
        );
        assert!(game.effects.drawings.is_empty() && game.effects.lines.is_empty());
        assert!(
            game.effects
                .billboards
                .iter()
                .any(|b| matches!(b.graphic, Graphic::Explosion(_)))
        );
        assert!(
            game.effects
                .billboards
                .iter()
                .any(|b| matches!(b.graphic, Graphic::Smoke(_)))
        );
        let before = crate::session::tests::state(&game);
        let geometry =
            crate::render_effects::build_for_camera(&game, false, Some(Matrix::default()));
        let sprites: Vec<_> = geometry
            .vertices
            .iter()
            .filter(|v| v.flags & render_mesh::UNLIT != 0)
            .collect();
        assert!(sprites.len() >= game.effects.billboards.len() * 6);
        assert!(sprites.iter().any(|v| v.flags & render_mesh::TEXTURED != 0));
        assert!(sprites.iter().all(
            |v| v.flags & render_mesh::SCREEN == 0 && v.position.iter().all(|p| p.is_finite())
        ));
        assert_eq!(crate::session::tests::state(&game), before);
        let hidden_geometry =
            crate::render_effects::build_for_camera(&game, false, Some(hidden.matrix));
        assert!(
            !hidden_geometry
                .vertices
                .iter()
                .any(|v| v.flags & render_mesh::UNLIT != 0)
        );
    }
}
