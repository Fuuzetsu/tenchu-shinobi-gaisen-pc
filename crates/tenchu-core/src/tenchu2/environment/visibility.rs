use super::Environment;
use crate::{
    Result,
    math::{Math, Position},
    tenchu2::{collision::Collision, visibility::View},
};
use tenchu_assets::tenchu2::level::{Level, objects::StaticKind};

#[derive(Clone, Debug, Default)]
pub struct Visible {
    pub statics: Vec<usize>,
    pub dynamic: Vec<usize>,
}

impl Environment {
    pub fn activate(
        &mut self,
        level: &Level,
        view: &View,
        collision: &mut Collision,
        math: &Math,
    ) -> Result<Visible> {
        let mut visible = Visible::default();
        for (index, object) in level.scene.statics.iter().enumerate() {
            let position = Position::from(object.position);
            let show = match object.kind {
                StaticKind::Model(model) => view.object(position, level.object_sizes[model]),
                StaticKind::Effect(kind) => view.effect(position, kind, math),
            };
            if show {
                visible.statics.push(index);
            }
        }
        for index in 0..self.objects.len() {
            let object = &mut self.objects[index];
            if object.kind > 9 && object.kind != 18 && object.link != 0 {
                object.clock = object.clock.wrapping_add(1);
            }
            let show = if let Some(model) = object.model {
                let size = *level
                    .object_sizes
                    .get(model)
                    .ok_or("Missing Tenchu 2 scenery size")?;
                view.object(object.position, size) && level.grid.contains(object.cell)
            } else {
                false
            };
            if !show {
                collision.remove(object.coordinate);
            } else {
                if !matches!(object.kind, -1 | 19) {
                    self.enable_collision(index, &level.scene.bounds, collision, math)?;
                }
                visible.dynamic.push(index);
            }
        }
        Ok(visible)
    }
}
