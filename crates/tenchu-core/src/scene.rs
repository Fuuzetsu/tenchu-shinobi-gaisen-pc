use crate::{Result, math::Matrix};

pub const COLLIDE: u16 = 0x4000;
pub const CONFLICT: u16 = 0x8000;
pub const NO_CULL: u16 = 2;
pub type ModelId = usize;

#[derive(Clone, Debug)]
pub struct Model {
    pub parent: Option<ModelId>,
    pub local: Matrix,
    pub cached: Matrix,
    pub flags: u32,
    pub attribute: u16,
    pub conflict: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct Scene {
    pub models: Vec<Model>,
    pub epoch: u32,
}

impl Default for Scene {
    fn default() -> Self {
        let mut scene = Self {
            models: Vec::new(),
            epoch: 1,
        };
        scene.add(None, Matrix::default());
        scene
    }
}

impl Scene {
    pub fn add(&mut self, parent: Option<ModelId>, local: Matrix) -> ModelId {
        let id = self.models.len();
        self.models.push(Model {
            parent,
            local,
            cached: Matrix::default(),
            flags: 0,
            attribute: 0,
            conflict: None,
        });
        id
    }

    pub fn update(&mut self, id: ModelId, local: Matrix) {
        let model = &mut self.models[id];
        model.local = local;
        model.flags = 0;
    }

    pub fn set_translation(&mut self, id: ModelId, position: [i32; 3]) {
        self.models[id].local.translation = position;
    }

    pub fn world(&mut self, id: ModelId) -> Result<Matrix> {
        let mut path = Vec::new();
        let mut current = id;
        let mut dirty: Option<usize> = None;
        let (mut world, count) = loop {
            if path.contains(&current) {
                return Err("Cyclic coordinate hierarchy".into());
            }
            path.push(current);
            let model = self.models.get(current).ok_or("Missing coordinate")?;
            let depth = path.len() - 1;
            let Some(parent) = model.parent else {
                if model.flags == 0 || model.flags == self.epoch {
                    let local = model.local;
                    self.models[current].cached = local;
                    self.models[current].flags = self.epoch;
                    break (local, depth);
                }
                if let Some(depth) = dirty {
                    break (self.models[path[depth + 1]].cached, depth + 1);
                }
                return Ok(self.models[id].cached);
            };
            if model.flags == self.epoch {
                break (model.cached, depth);
            }
            if model.flags == 0 {
                dirty = Some(depth);
            }
            current = parent;
        };
        for &id in path[..count].iter().rev() {
            let model = &mut self.models[id];
            world = world.compose(&model.local);
            model.cached = world;
            model.flags = self.epoch;
        }
        Ok(world)
    }

    pub fn finish_tick(&mut self) {
        self.epoch = self.epoch.wrapping_add(1).max(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn translated(x: i32) -> Matrix {
        Matrix {
            translation: [x, 0, 0],
            ..Matrix::default()
        }
    }

    #[test]
    fn current_child_cache_survives_parent_update_until_the_next_logic_tick() {
        let mut scene = Scene::default();
        let root = scene.add(Some(0), translated(10));
        let child = scene.add(Some(root), translated(2));
        assert_eq!(scene.world(child).unwrap().translation[0], 12);
        scene.update(root, translated(100));
        assert_eq!(scene.world(child).unwrap().translation[0], 12);
        scene.finish_tick();
        assert_eq!(scene.world(child).unwrap().translation[0], 102);
        scene.set_translation(root, [200, 0, 0]);
        scene.finish_tick();
        assert_eq!(scene.world(child).unwrap().translation[0], 102);
        scene.models[child].flags = 0;
        assert_eq!(scene.world(child).unwrap().translation[0], 102);
        scene.models[root].flags = 0;
        scene.finish_tick();
        assert_eq!(scene.world(child).unwrap().translation[0], 202);
        scene.epoch = u32::MAX;
        scene.finish_tick();
        assert_eq!(scene.epoch, 1);
    }
}
