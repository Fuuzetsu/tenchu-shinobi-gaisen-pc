use super::{ALPHA, DEBUG, FOLIAGE, SCENERY, SCREEN, Vertex, WATER};
use glam::{Mat3A, Vec3A};

#[derive(Clone, Copy, Default)]
pub(crate) struct Bounds {
    pub shadow: Option<[Vec3A; 2]>,
    pub scenery: Option<[Vec3A; 2]>,
    pub foliage: bool,
}

impl Bounds {
    pub fn collect<'a>(vertices: impl Iterator<Item = &'a Vertex>) -> Self {
        let mut out = Self::default();
        for v in vertices {
            let p = Vec3A::from(v.position);
            if v.flags & SCENERY != 0 {
                out.scenery = Some(out.scenery.map_or([p, p], |[a, b]| [a.min(p), b.max(p)]));
            }
            if v.flags & (SCREEN | DEBUG | ALPHA | WATER) == 0 {
                out.shadow = Some(out.shadow.map_or([p, p], |[a, b]| [a.min(p), b.max(p)]));
                out.foliage |= v.flags & FOLIAGE != 0;
            }
        }
        out
    }

    pub fn merge(&mut self, other: Self) {
        for (destination, source) in [
            (&mut self.shadow, other.shadow),
            (&mut self.scenery, other.scenery),
        ] {
            if let Some([a, b]) = source {
                *destination = Some(destination.map_or([a, b], |[c, d]| [a.min(c), b.max(d)]));
            }
        }
        self.foliage |= other.foliage;
    }

    pub fn transformed(
        self,
        rotation: Mat3A,
        translation: Vec3A,
        displacement: [Vec3A; 2],
    ) -> Self {
        let transform = |bounds: [Vec3A; 2]| {
            let mut out = [Vec3A::splat(f32::INFINITY), Vec3A::splat(f32::NEG_INFINITY)];
            for i in 0..8 {
                let p = rotation
                    * Vec3A::new(
                        bounds[i & 1].x,
                        bounds[(i >> 1) & 1].y,
                        bounds[(i >> 2) & 1].z,
                    )
                    + translation;
                out = [out[0].min(p), out[1].max(p)];
            }
            let magnitude = rotation.x_axis.abs() * bounds[0].x.abs().max(bounds[1].x.abs())
                + rotation.y_axis.abs() * bounds[0].y.abs().max(bounds[1].y.abs())
                + rotation.z_axis.abs() * bounds[0].z.abs().max(bounds[1].z.abs())
                + translation.abs()
                + displacement[0].abs()
                + displacement[1].abs()
                + Vec3A::ONE;
            // Enclose roundoff in both CPU bounds and GPU multiply/add evaluation.
            let padding = magnitude * (8.0 * f32::EPSILON);
            [
                out[0] + displacement[0] - padding,
                out[1] + displacement[1] + padding,
            ]
        };
        Self {
            shadow: self.shadow.map(transform),
            scenery: self.scenery.map(transform),
            foliage: self.foliage,
        }
    }
}
