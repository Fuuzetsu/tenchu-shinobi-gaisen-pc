use super::*;

impl Builder {
    pub(super) fn original_weather(
        &mut self,
        frame: &tenchu_core::tenchu2::effects::weather::Frame,
        sprites: &tenchu_assets::tenchu2::effects::Sprites,
        view: Matrix,
        reproject: bool,
    ) {
        for line in &frame.lines {
            let points = if reproject {
                line.points.map(|point| self.project(point))
            } else {
                line.screen.map(|p| {
                    [
                        f32::from(p[0]),
                        f32::from(p[1]),
                        f32::from(line.depth) * 16.0,
                    ]
                })
            };
            let [a, b] = points;
            if a[2] <= 0.0 || b[2] <= 0.0 {
                continue;
            }
            let delta = glam::Vec2::new(b[0] - a[0], b[1] - a[1]);
            let offset = glam::Vec2::new(-delta.y, delta.x).normalize_or_zero() * 0.5;
            if offset == glam::Vec2::ZERO {
                continue;
            }
            let points = [
                [
                    a[0] - offset.x,
                    a[1] - offset.y,
                    render_effects::depth(a[2]),
                ],
                [
                    b[0] - offset.x,
                    b[1] - offset.y,
                    render_effects::depth(b[2]),
                ],
                [
                    a[0] + offset.x,
                    a[1] + offset.y,
                    render_effects::depth(a[2]),
                ],
                [
                    b[0] + offset.x,
                    b[1] + offset.y,
                    render_effects::depth(b[2]),
                ],
            ];
            let color = line.color.map(|v| f32::from(v) / 255.0);
            self.surfaces.push(Surface {
                depth: a[2],
                blend: 2,
                vertices: render_effects::quad(points, [color, [0.0; 3], color, [0.0; 3]], None, 2),
            });
        }
        for sprite in &frame.sprites {
            self.particle_sprite(
                sprite,
                sprites.effects[usize::from(sprite.image)],
                view,
                reproject,
            );
        }
        if let Some(intensity) = frame.flash {
            self.screen_sprite(
                [0.0, 0.0, 64.0],
                [320.0, 240.0],
                [1.0; 2],
                0,
                [f32::from(intensity) / 255.0; 3],
                None,
                2,
            );
            for vertex in &mut self.surfaces.last_mut().unwrap().vertices {
                vertex.flags |= render_mesh::SCREEN_WIDE;
            }
        }
    }
}
