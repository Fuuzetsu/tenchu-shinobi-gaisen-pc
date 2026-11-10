use super::*;
use crate::{math::Matrix, tenchu2::environment::GAISEN_PITFALL};
use tenchu_assets::{
    misc,
    tenchu2::level::objects::{Bounds, Dynamic},
};

impl Assets {
    pub(super) fn import_hinged_objects(&mut self, source: &MissionAssets) -> Result<()> {
        let level = &mut self.level;
        level.scene.bounds = vec![
            Bounds {
                offset: [0; 3],
                top: 0,
                half_width: 0,
                half_depth: 0,
            };
            level.objects.len()
        ];
        self.object_sources.clear();
        let mut models = BTreeMap::new();
        for spawn in &source.misc.spawns {
            let (rotation, data, pitfall) = match spawn.kind {
                misc::Kind::Door { rotation, kind } => (rotation, &source.misc.doors[kind], false),
                misc::Kind::Pitfall { rotation, kind } => {
                    (rotation, &source.misc.pitfalls[kind], true)
                }
                _ => continue,
            };
            let trigger = level.scene.triggers.len();
            let first = level.scene.dynamic.len();
            if first + 1 > 255 {
                return Err("Too many imported Tenchu 2 hinged objects".into());
            }
            let root = Matrix {
                translation: spawn.position.into(),
                rotation: self.resources.math.rotate_yxz([0, rotation, 0]),
            };
            for (side, mesh) in data.models.iter().enumerate() {
                let model = if let Some(mesh) = mesh {
                    if let std::collections::btree_map::Entry::Vacant(entry) = models.entry(*mesh) {
                        let geometry = source
                            .common_models
                            .get(mesh)
                            .ok_or("Missing Gaisen hinged model")?;
                        let mut low = [i16::MAX; 3];
                        let mut high = [i16::MIN; 3];
                        for vertex in geometry.iter().flat_map(|g| &g.vertices) {
                            for axis in 0..3 {
                                low[axis] = low[axis].min(vertex[axis]);
                                high[axis] = high[axis].max(vertex[axis]);
                            }
                        }
                        if low.iter().zip(high).any(|(&a, b)| a > b) {
                            return Err("Empty Gaisen hinged model".into());
                        }
                        let middle = std::array::from_fn::<_, 3, _>(|i| {
                            ((i32::from(low[i]) + i32::from(high[i])) / 2) as i16
                        });
                        let half = std::array::from_fn::<_, 3, _>(|i| {
                            ((i32::from(high[i]) - i32::from(low[i])) / 2) as i16
                        });
                        let index = level.objects.len();
                        entry.insert(index);
                        level.objects.push(Part {
                            parent: -1,
                            child: -1,
                            position: [0; 3],
                            geometry: geometry.clone(),
                        });
                        level.object_sizes.push(i32::from(half[1]));
                        // The native loader uses offset[2] with cos(yaw) for X
                        // and offset[0] with -sin(yaw) for Z. Encode the leaf's
                        // center along its local X axis in both components.
                        level.scene.bounds.push(Bounds {
                            offset: [middle[0], middle[1], middle[0]],
                            top: -half[1],
                            half_width: half[0],
                            half_depth: half[2],
                        });
                        self.object_sources.insert(index, format!("common/{mesh}"));
                    }
                    Some(models[mesh])
                } else {
                    None
                };
                let width = if side == 0 { -data.width } else { data.width };
                level.scene.dynamic.push(Dynamic {
                    position: root.transform([width, 0, 0]).to_array(),
                    facing: rotation,
                    kind: if model.is_none() {
                        u16::MAX
                    } else if pitfall {
                        GAISEN_PITFALL as u16
                    } else if side == 0 {
                        2
                    } else {
                        1
                    },
                    model,
                    trigger: Some(trigger),
                });
            }
            // Keep both source leaf slots, including a missing sibling. Native
            // paired-door updates then turn a single leaf once, with its own sign.
            level.scene.triggers.push(Trigger {
                position: spawn.position,
                angle: rotation >> 4,
                kind: if pitfall { GAISEN_PITFALL } else { 1 },
                target: (first + 1) as u16 | ((first as u16) << 8),
            });
        }
        Ok(())
    }
}
