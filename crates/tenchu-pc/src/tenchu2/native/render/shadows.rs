use super::*;
use tenchu_core::tenchu2::effects::shadow::{IMAGE, Shadow};

impl Builder {
    pub(super) fn shadows(&mut self, world: &World, assets: &Assets, source_clip: bool) {
        let view = world.view();
        for (agent, instance) in world.actors.iter().zip(&world.instances) {
            let actor = &agent.actor;
            if actor.flags & 0x100008 != 0x100000 || actor.health <= 0 || actor.kind & 0xf0 == 0x70
            {
                continue;
            }
            let Some(shadow) = Shadow::new(&instance.feet, actor.width, &assets.resources.math)
            else {
                continue;
            };
            if source_clip && !shadow.visible(view.matrix, view.distance) {
                continue;
            }
            self.shadow(&shadow, assets.sprites.effects[usize::from(IMAGE)]);
        }
    }

    pub(super) fn shadow(&mut self, shadow: &Shadow, image: tenchu_assets::texture::Image) {
        let points = shadow.corners.map(|p| {
            shadow
                .position
                .wrapping_add(p.map(i32::from))
                .map(|v| v as f32)
                .to_array()
        });
        let mut vertices = render_effects::quad(points, [[1.0; 3]; 4], Some(image), 3);
        for vertex in &mut vertices {
            // The source OT places these over coplanar floors, including blended T1 faces.
            vertex.position[1] -= 1.0;
            vertex.flags &= !render_mesh::SCREEN;
            vertex.flags |= render_mesh::UNLIT | render_mesh::DOUBLE_SIDED;
        }
        self.surface(vertices, 3);
    }
}

#[cfg(test)]
mod tests;
