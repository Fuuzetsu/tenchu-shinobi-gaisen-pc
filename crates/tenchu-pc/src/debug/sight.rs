use crate::{render_mesh::Vertex, visuals::surfaces::Surfaces};
use glam::Vec3;

pub(super) struct Occluders(Surfaces);
impl Occluders {
    pub fn new(vertices: &[Vertex]) -> Self {
        Self(Surfaces::new(vertices))
    }
    pub fn endpoint(&self, origin: Vec3, target: Vec3) -> Vec3 {
        self.0
            .cast(origin, target)
            .map_or(target, |hit| hit.position)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;
    #[test]
    fn sight_outline_stops_at_walls_and_keeps_clear_rays() {
        let occluders = Occluders::new(
            &[
                Vec3::new(1000.0, -1000.0, -1000.0),
                Vec3::new(1000.0, 1000.0, -1000.0),
                Vec3::new(1000.0, 0.0, 1000.0),
            ]
            .map(|p| Vertex {
                position: p.to_array(),
                ..Vertex::zeroed()
            }),
        );
        assert_eq!(
            occluders.endpoint(Vec3::ZERO, Vec3::X * 5000.0),
            Vec3::X * 1000.0
        );
        assert_eq!(
            occluders.endpoint(Vec3::ZERO, Vec3::NEG_X * 5000.0),
            Vec3::NEG_X * 5000.0
        );
        assert_eq!(
            occluders.endpoint(Vec3::ZERO, Vec3::X * 500.0),
            Vec3::X * 500.0
        );
    }
}
