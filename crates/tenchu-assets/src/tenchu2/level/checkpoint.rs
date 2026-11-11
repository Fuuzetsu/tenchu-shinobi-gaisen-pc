use super::Level;

impl Level {
    pub fn capture<C: crate::checkpoint::Capture>(&self, out: &mut C) -> Result<(), C::Error> {
        let Self {
            id,
            gaisen,
            draw_distance,
            grid,
            map,
            objects,
            object_sizes,
            stumps,
            scene,
            vram,
            collision,
            background,
            shading,
        } = self;
        out.value(id)?;
        out.value(gaisen)?;
        out.value(draw_distance)?;
        out.value(grid)?;
        out.shared(map)?;
        out.shared(objects)?;
        out.shared(object_sizes)?;
        out.shared(stumps)?;
        out.value(scene)?;
        out.shared(vram)?;
        out.shared(collision)?;
        out.value(background)?;
        out.value(shading)
    }
}
