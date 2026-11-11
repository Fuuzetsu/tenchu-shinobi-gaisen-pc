use super::{Surface, quad};
use glam::Vec2;
use tenchu_core::game::Game;

pub(super) fn draw(surfaces: &mut Vec<Surface>, game: &Game) {
    let Some(position) = game.hud.bandage_position(game.player()) else {
        return;
    };
    let scale = if game.hud.bandage_selected() {
        1.0
    } else {
        2.0 / 3.0
    };
    draw_at(surfaces, position.map(f32::from), scale, [1.0; 3]);
}

pub(crate) fn draw_at(
    surfaces: &mut Vec<Surface>,
    position: [f32; 2],
    scale: f32,
    shade: [f32; 3],
) {
    let center = Vec2::from_array(position);
    let rotation = Vec2::from_angle(-0.6);
    let mut rectangle = |min: [f32; 2], max: [f32; 2], color: [f32; 3]| {
        let points = [
            [min[0], min[1]],
            [max[0], min[1]],
            [min[0], max[1]],
            [max[0], max[1]],
        ]
        .map(|p| {
            let p = Vec2::from_array(p) * scale;
            let p = center
                + Vec2::new(
                    rotation.x * p.x - rotation.y * p.y,
                    rotation.y * p.x + rotation.x * p.y,
                );
            [p.x, p.y, 0.0]
        });
        surfaces.push(Surface {
            depth: 0.0,
            blend: 0,
            vertices: quad(
                points,
                [std::array::from_fn(|i| color[i] * shade[i]); 4],
                None,
                0,
            ),
        });
    };
    rectangle([-12.0, -6.0], [12.0, 6.0], [0.25, 0.20, 0.14]);
    rectangle([-11.0, -5.0], [11.0, 5.0], [0.82, 0.76, 0.59]);
    rectangle([-5.0, -4.0], [5.0, 4.0], [0.98, 0.94, 0.81]);
    for y in [-3.0, 0.0, 3.0] {
        rectangle([-4.0, y], [4.0, y + 0.4], [0.73, 0.67, 0.53]);
        for x in [-9.0, -7.0, 7.0, 9.0] {
            rectangle([x, y], [x + 0.6, y + 0.6], [0.47, 0.40, 0.28]);
        }
    }
}
