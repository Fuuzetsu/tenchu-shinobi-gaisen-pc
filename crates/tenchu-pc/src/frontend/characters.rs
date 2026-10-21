use super::raster::{Blend, Canvas, Vertex};
use crate::source::FileSource;
use glam::{Vec2, Vec3};
use tenchu_assets::{
    Result,
    frontend::Tables,
    game::GameAssets,
    model::{self, Geometry, Part},
    motion::{self, Clip},
    texture::Vram,
};
use tenchu_core::{
    math::{Math, Matrix, Position},
    motion::Motion,
};

pub struct Characters {
    models: [Vec<Part>; 2],
    weapons: [Vec<Geometry>; 2],
    motions: [Motion; 2],
    clips: Vec<Clip>,
    vram: Vram,
    pub selected: usize,
    pub confirming: bool,
}

impl Characters {
    pub fn load(assets: &GameAssets<FileSource>, tables: &Tables) -> Result<Self> {
        let models = [
            model::archive(&assets.asset("HUMAN/RIKIMARU.MAD")?)?,
            model::archive(&assets.asset("HUMAN/AYAME.MAD")?)?,
        ];
        let mut vram = Vram::default();
        for name in ["HUMAN/RIKIMARU.TPD", "HUMAN/AYAME.TPD"] {
            vram.upload_tpd(&assets.asset(name)?)?;
        }
        let motions = [Motion::new(&models[0])?, Motion::new(&models[1])?];
        let mut result = Self {
            models,
            motions,
            weapons: [
                model::tmd_model(&assets.asset("HUMAN/WEAPON/NINJA.TMD")?)?,
                model::tmd_model(&assets.asset("HUMAN/WEAPON/KODATI.TMD")?)?,
            ],
            clips: motion::parse(&assets.asset("HUMAN/MOTION/STAGE9.AMD")?)?,
            vram,
            selected: 0,
            confirming: false,
        };
        result.reset(tables);
        Ok(result)
    }
    pub fn reset(&mut self, tables: &Tables) {
        self.confirming = false;
        self.selected = 0;
        for (i, motion) in self.motions.iter_mut().enumerate() {
            motion.action = -1;
            motion.change(
                if i == 0 { 0xf04 } else { 0x501 },
                &tables.motions[i],
                &[],
                &self.clips,
            );
            motion.count = 0;
        }
    }
    pub fn select(&mut self, selected: usize, tables: &Tables) {
        self.selected = selected;
        for (i, motion) in self.motions.iter_mut().enumerate() {
            motion.change(
                if i == selected { 0xf04 } else { 0x501 },
                &tables.motions[i],
                &[],
                &self.clips,
            );
        }
    }
    pub fn confirm(&mut self, tables: &Tables) {
        self.confirming = true;
        for (i, motion) in self.motions.iter_mut().enumerate() {
            motion.change(
                if i == self.selected { 0x80e } else { 0xf04 },
                &tables.motions[i],
                &[],
                &self.clips,
            );
        }
    }
    pub fn tick(&mut self, math: &Math) -> Result<Option<i16>> {
        for motion in &mut self.motions {
            motion.step_scalar(&self.clips, &math.tables.hermite)?;
            if motion.loops > 0 {
                motion.loops = -1;
            }
        }
        Ok((self.confirming && self.motions[self.selected].count == 24).then_some(82))
    }
    pub fn done(&self) -> bool {
        self.confirming && self.motions[self.selected].loops < 0
    }

    pub fn draw(&self, canvas: &mut Canvas, math: &Math) {
        let mut triangles = Vec::new();
        for (i, model) in self.models.iter().enumerate() {
            let pose = self.motions[i].pose.matrices(
                math,
                Matrix {
                    translation: Position::new(if i == 0 { -1100 } else { 1100 }, 800, 4000),
                    ..Matrix::default()
                },
            );
            for (part, matrix) in model.iter().zip(&pose) {
                for geometry in &part.geometry {
                    collect(geometry, *matrix, &mut triangles);
                }
            }
            let motion = &self.motions[i];
            if self.confirming && i == self.selected && (motion.count >= 24 || motion.loops < 0) {
                for hand in if i == 0 { &[13][..] } else { &[13, 14][..] } {
                    if let Some(matrix) = pose.get(*hand) {
                        for geometry in &self.weapons[i] {
                            collect(geometry, *matrix, &mut triangles);
                        }
                    }
                }
            }
        }
        triangles.sort_by(|a, b| b.0.total_cmp(&a.0));
        for (_, vertices, texture, translucent) in triangles {
            let blend = if translucent {
                match texture.map_or(0, |t| (t.page >> 5) & 3) {
                    0 => Blend::Average,
                    1 => Blend::Add,
                    2 => Blend::Subtract,
                    _ => Blend::Quarter,
                }
            } else {
                Blend::Opaque
            };
            canvas.triangle(vertices, blend, |uv| {
                texture.map_or(0x7fff, |texture| {
                    self.vram
                        .texel(texture.page, texture.palette, [uv.x as u16, uv.y as u16])
                })
            });
        }
    }
}

type Triangle = (f32, [Vertex; 3], Option<model::Texture>, bool);

fn collect(geometry: &Geometry, matrix: Matrix, triangles: &mut Vec<Triangle>) {
    for face in &geometry.faces {
        let mut corners = Vec::new();
        let mut depth = 0.0;
        for corner in &face.corners {
            let p = matrix.transform(geometry.vertices[usize::from(corner.vertex)]);
            if p.z <= 0 {
                break;
            }
            let z = p.z as f32;
            depth += z;
            corners.push(Vertex {
                position: Vec2::new(
                    160.0 + p.x as f32 * 300.0 / z,
                    120.0 + p.y as f32 * 300.0 / z,
                ),
                uv: Vec2::from_array(corner.uv.map(f32::from)),
                color: Vec3::from_array(corner.color.map(f32::from))
                    * if corner.normal.is_some() { 0.5 } else { 1.0 },
            });
        }
        if corners.len() != face.corners.len() {
            continue;
        }
        depth /= corners.len() as f32;
        triangles.push((
            depth,
            [corners[0], corners[1], corners[2]],
            face.texture,
            face.translucent,
        ));
        if corners.len() == 4 {
            triangles.push((
                depth,
                [corners[1], corners[3], corners[2]],
                face.texture,
                face.translucent,
            ));
        }
    }
}
