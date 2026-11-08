use super::Environment;
use crate::{
    Result,
    math::Math,
    tenchu2::{collision::Collision, visibility::View},
};
use tenchu_assets::tenchu2::level::Level;

impl Environment {
    pub fn activate_object(
        &mut self,
        index: usize,
        level: &Level,
        view: &View,
        collision: &mut Collision,
        math: &Math,
    ) -> Result<bool> {
        let object = &mut self.objects[index];
        if object.kind > 9 && object.kind != 18 && object.link != 0 {
            object.clock = object.clock.wrapping_add(1);
        }
        let visible = if let Some(model) = object.model {
            let size = *level
                .object_sizes
                .get(model)
                .ok_or("Missing Tenchu 2 scenery size")?;
            view.object(object.position, size) && level.grid.contains(object.cell)
        } else {
            false
        };
        if !visible {
            collision.remove(object.coordinate);
        } else if !matches!(object.kind, -1 | 19) {
            self.enable_collision(index, &level.scene.bounds, collision, math)?;
        }
        Ok(visible)
    }
}
