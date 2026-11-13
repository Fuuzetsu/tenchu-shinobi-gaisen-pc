use super::*;
use tenchu_core::hud::Sprite;

pub(super) fn candle(sprite: &Sprite, frame: usize) -> Surface {
    let mut vertices = Vec::new();
    let [width, height] = sprite.image.size.map(f32::from);
    let [sx, sy] = sprite.scale.map(|v| f32::from(v) / 4096.0);
    let point = |[x, y]: [f32; 2], color: [f32; 3]| Vertex {
        position: [
            f32::from(sprite.xy[0]) + (x * width - f32::from(sprite.origin[0])) * sx,
            f32::from(sprite.xy[1]) + (y * height - f32::from(sprite.origin[1])) * sy,
            f32::from(sprite.depth) * 0.000001,
        ],
        color: std::array::from_fn(|i| color[i] * f32::from(sprite.color[i]) / 128.0),
        flags: SCREEN,
        opacity: 1.0,
        ..bytemuck::Zeroable::zeroed()
    };
    let mut polygon = |points: &[[f32; 2]], color: [f32; 3]| {
        let point = |[x, y]: [f32; 2]| point([0.5 + (x - 0.5) * 0.4, y], color);
        for i in 1..points.len() - 1 {
            vertices.extend([point(points[0]), point(points[i]), point(points[i + 1])]);
        }
    };
    polygon(
        &[[0.25, 0.96], [0.75, 0.96], [0.72, 0.99], [0.28, 0.99]],
        [0.16, 0.10, 0.06],
    );
    polygon(
        &[[0.31, 0.47], [0.68, 0.47], [0.70, 0.96], [0.30, 0.96]],
        [0.57, 0.43, 0.25],
    );
    polygon(
        &[[0.36, 0.47], [0.64, 0.47], [0.64, 0.95], [0.36, 0.95]],
        [0.91, 0.83, 0.64],
    );
    polygon(
        &[[0.39, 0.50], [0.47, 0.50], [0.47, 0.93], [0.39, 0.93]],
        [1.0, 0.95, 0.78],
    );
    polygon(
        &[
            [0.32, 0.48],
            [0.42, 0.48],
            [0.42, 0.63],
            [0.39, 0.66],
            [0.36, 0.62],
        ],
        [0.97, 0.90, 0.73],
    );
    polygon(
        &[
            [0.57, 0.48],
            [0.66, 0.48],
            [0.64, 0.57],
            [0.61, 0.59],
            [0.58, 0.56],
        ],
        [0.98, 0.91, 0.74],
    );
    polygon(
        &[[0.47, 0.42], [0.51, 0.42], [0.53, 0.51], [0.49, 0.51]],
        [0.15, 0.08, 0.04],
    );
    let lean = [-0.055, 0.0, 0.055][frame % 3];
    for (scale, center, edge) in [
        (1.0, [1.0, 0.65, 0.12], [0.95, 0.25, 0.025]),
        (0.70, [1.0, 0.98, 0.73], [1.0, 0.77, 0.22]),
        (0.36, [1.0, 1.0, 0.95], [1.0, 0.95, 0.65]),
    ] {
        let flame = |angle: f32| {
            let (sin, cos) = angle.sin_cos();
            let y = (1.0 + cos) * 0.5;
            [
                0.50 + (sin * 0.19 * (0.25 + y * 0.75) + lean * (1.0 - y).powi(2)) * scale,
                0.46 - (1.0 - y) * (0.36 + lean.abs()) * scale,
            ]
        };
        for segment in 0..48 {
            vertices.extend([
                point([0.5, 0.40 - scale * 0.06], center),
                point(flame(segment as f32 * std::f32::consts::TAU / 48.0), edge),
                point(
                    flame((segment + 1) as f32 * std::f32::consts::TAU / 48.0),
                    edge,
                ),
            ]);
        }
    }
    Surface {
        depth: f32::from(sprite.depth),
        blend: 0,
        vertices,
    }
}
