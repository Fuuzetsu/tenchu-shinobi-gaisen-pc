use crate::{
    render_effects::{Batch, Geometry},
    render_mesh::{DEBUG, DOUBLE_SIDED, SCREEN, Vertex, XRAY},
};
use glam::{Mat4, Vec3, Vec4};

pub struct Overlay {
    pub geometry: Geometry,
    pub labels: Vec<Label>,
}

pub enum Anchor {
    Projected([f32; 2]),
    TopLeft([f32; 2]),
    BottomLeft([f32; 2]),
}

pub struct Label(pub Anchor, pub String, pub [f32; 3]);

pub struct Builder {
    pub labels: Vec<Label>,
    vertices: Vec<Vertex>,
    matrix: Mat4,
    aspect: f32,
    xray: bool,
}

fn vertex(position: [f32; 3], color: [f32; 3], flags: u32) -> Vertex {
    Vertex {
        material: 0,
        normal: [0.0; 3],
        opacity: 1.0,
        position,
        color,
        flags,
        uv: [0.0; 2],
        page: 0,
        palette: 0,
    }
}

impl Builder {
    pub fn new(matrix: Mat4, aspect: f32, xray: bool) -> Self {
        Self {
            vertices: Vec::new(),
            labels: Vec::new(),
            matrix,
            aspect,
            xray,
        }
    }

    pub fn finish(self) -> Overlay {
        let count = self.vertices.len() as u32;
        Overlay {
            geometry: Geometry {
                vertices: self.vertices,
                batches: vec![Batch {
                    blend: 0,
                    range: 0..count,
                }],
            },
            labels: self.labels,
        }
    }

    pub fn surface(&mut self, corners: [Vec3; 4], color: [f32; 3]) {
        let flags = DOUBLE_SIDED | DEBUG | if self.xray { XRAY } else { 0 };
        self.vertices
            .extend([0, 1, 2, 1, 3, 2].map(|i| vertex(corners[i].to_array(), color, flags)));
    }

    pub fn collider(&mut self, center: Vec3, half: Vec3, hit: bool) {
        self.box_at(
            center - half,
            center + half,
            if hit {
                [1.0, 0.7, 0.15]
            } else {
                [0.25, 0.9, 1.0]
            },
        );
    }

    pub fn actor(&mut self, actor: super::inspection::Actor, selected: usize) {
        if let Some(point) = self.project(actor.position - Vec3::Y * (actor.height + 300.0)) {
            self.labels.push(Label(
                Anchor::Projected(point),
                format!("#{} {:02x} HP {}", actor.id, actor.kind, actor.health),
                if actor.suspended {
                    [0.5; 3]
                } else if actor.id == selected {
                    [1.0, 0.85, 0.2]
                } else {
                    [0.7, 0.95, 1.0]
                },
            ));
        }
    }

    pub fn inspection(&mut self, lines: impl IntoIterator<Item = String>) {
        for (i, text) in lines.into_iter().enumerate() {
            self.labels.push(Label(
                Anchor::TopLeft([10.0, 36.0 + i as f32 * 22.0]),
                text,
                [0.7, 0.95, 1.0],
            ));
        }
    }

    pub fn line(&mut self, a: Vec3, b: Vec3, color: [f32; 3]) {
        let mut a = self.matrix * a.extend(1.0);
        let mut b = self.matrix * b.extend(1.0);
        for plane in [
            Vec4::new(0.0, 0.0, 1.0, 0.0),
            Vec4::new(0.0, 0.0, -1.0, 1.0),
        ] {
            let da = a.dot(plane);
            let db = b.dot(plane);
            if da < 0.0 && db < 0.0 {
                return;
            }
            if da < 0.0 {
                a = a.lerp(b, da / (da - db));
            }
            if db < 0.0 {
                b = b.lerp(a, db / (db - da));
            }
        }
        let to_screen = |p: Vec4| {
            let p = p.truncate() / p.w;
            Vec3::new(
                p.x * self.aspect * 120.0,
                -p.y * 120.0,
                if self.xray {
                    0.0
                } else {
                    (p.z - 0.00001).max(0.0)
                },
            )
        };
        let [a, b] = [a, b].map(to_screen);
        let d = (b - a).truncate().normalize_or_zero() * 0.35;
        let offset = Vec3::new(-d.y, d.x, 0.0);
        let points = [a - offset, b - offset, a + offset, b + offset];
        self.vertices.extend(
            [0, 1, 2, 1, 3, 2].map(|i| vertex(points[i].to_array(), color, SCREEN | DEBUG)),
        );
    }

    pub fn box_at(&mut self, min: Vec3, max: Vec3, color: [f32; 3]) {
        let points: [Vec3; 8] = std::array::from_fn(|i| {
            Vec3::new(
                if i & 1 == 0 { min.x } else { max.x },
                if i & 2 == 0 { min.y } else { max.y },
                if i & 4 == 0 { min.z } else { max.z },
            )
        });
        for i in 0..8 {
            for bit in [1, 2, 4] {
                if i & bit == 0 {
                    self.line(points[i], points[i | bit], color);
                }
            }
        }
    }

    pub fn project(&self, point: Vec3) -> Option<[f32; 2]> {
        let p = self.matrix * point.extend(1.0);
        let ndc = p.truncate() / p.w;
        (p.w > 0.0 && (0.0..=1.0).contains(&ndc.z) && ndc.x.abs() < 0.9 && ndc.y.abs() < 0.9)
            .then_some([(ndc.x + 1.0) * 0.5, (1.0 - ndc.y) * 0.5])
    }
}

pub fn paint_labels(ctx: &egui::Context, labels: &[crate::debug::Label]) {
    use crate::debug::{Anchor, Label};
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    for Label(anchor, text, color) in labels {
        let origin = match *anchor {
            Anchor::Projected([x, y]) => {
                screen.min + egui::vec2(x * screen.width(), y * screen.height())
            }
            Anchor::TopLeft([x, y]) => screen.min + egui::vec2(x, y),
            Anchor::BottomLeft([x, y]) => screen.left_bottom() + egui::vec2(x, -y),
        };
        let color = color.map(|v| (v * 255.0).clamp(0.0, 255.0) as u8);
        let color = egui::Color32::from_rgb(color[0], color[1], color[2]);
        let text = painter.layout_no_wrap(text.clone(), egui::FontId::monospace(15.0), color);
        let origin = egui::pos2(
            origin.x.min(screen.right() - text.size().x - 5.0).max(5.0),
            origin.y,
        );
        painter.rect_filled(
            egui::Rect::from_min_size(origin, text.size()).expand(3.0),
            3.0,
            egui::Color32::from_black_alpha(175),
        );
        painter.galley(origin, text, color);
    }
}
