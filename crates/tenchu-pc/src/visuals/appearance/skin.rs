use crate::{render_mesh::Vertex, visuals::models::rotation};
use glam::{Affine3A, Mat3, Vec3};
use serde::{Deserialize, Serialize};
use std::cell::{Cell, Ref, RefCell};
use tenchu_assets::{Result, model};
use tenchu_core::math::Matrix;

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Influence {
    pub joints: [u8; 4],
    pub weights: [f32; 4],
}

impl Influence {
    pub fn key(self) -> ([u8; 4], [u32; 4]) {
        (self.joints, self.weights.map(f32::to_bits))
    }
}

pub fn decode(bytes: &[u8]) -> Result<Vec<Influence>> {
    let mut weights: Vec<Influence> = serde_json::from_slice(bytes).map_err(|e| e.to_string())?;
    if weights.is_empty() || weights.len() > 500_000 {
        return Err("Skin needs 1–500,000 vertex weights".into());
    }
    for influence in &mut weights {
        let total: f32 = influence.weights.iter().sum();
        if influence.joints.iter().any(|&j| j >= 15)
            || influence.weights.iter().any(|w| !w.is_finite() || *w < 0.0)
            || !total.is_finite()
            || (total - 1.0).abs() > 0.001
        {
            return Err("Skin weights must sum to one and reference joints 0–14".into());
        }
        influence.weights = influence.weights.map(|w| w / total);
    }
    Ok(weights)
}

#[derive(Clone, Copy)]
pub struct Transform {
    point: Affine3A,
    normal: Mat3,
}

impl Transform {
    const IDENTITY: Self = Self {
        point: Affine3A::IDENTITY,
        normal: Mat3::IDENTITY,
    };
}

pub struct Pose {
    pub roots: Vec<Vec3>,
    parents: Vec<Option<usize>>,
    detached: Cell<u64>,
    // Refreshed before each body, shadow or gallery traversal; never simulation state.
    transforms: RefCell<Vec<Transform>>,
}

impl Pose {
    pub fn new(parts: &[model::Part]) -> Self {
        let roots = (0..parts.len())
            .map(|i| {
                let mut root = Vec3::from(parts[i].position.map(f32::from));
                let mut parent = parts[i].parent;
                while parent >= 0 {
                    let part = &parts[parent as usize];
                    root += Vec3::from(part.position.map(f32::from));
                    parent = part.parent;
                }
                root
            })
            .collect();
        Self {
            roots,
            parents: super::parents(parts),
            detached: Cell::new(0),
            transforms: RefCell::new(vec![Transform::IDENTITY; parts.len()]),
        }
    }

    pub fn set(&self, matrices: &[Matrix], severed: Option<(usize, Matrix)>) {
        let detached = severed.map_or(0, |(cut, _)| {
            (0..self.roots.len()).fold(0, |mask, joint| {
                let mut ancestor = Some(joint);
                while let Some(index) = ancestor {
                    if index == cut {
                        return mask | (1 << joint);
                    }
                    ancestor = self.parents[index];
                }
                mask
            })
        });
        self.detached.set(detached);
        let affine = |matrix: Matrix| {
            Affine3A::from_mat3_translation(
                rotation(matrix),
                Vec3::from(matrix.translation.to_array().map(|n| n as f32)),
            )
        };
        let attached =
            severed.map(|(joint, matrix)| affine(matrix) * affine(matrices[joint]).inverse());
        for (i, transform) in self.transforms.borrow_mut().iter_mut().enumerate() {
            let mut matrix = affine(matrices[i]);
            if detached & (1 << i) != 0 {
                matrix = attached.unwrap() * matrix;
            }
            let rotation = Mat3::from(matrix.matrix3);
            let position = Vec3::from(matrix.translation);
            *transform = Transform {
                point: Affine3A::from_mat3_translation(
                    rotation,
                    position - rotation * self.roots[i],
                ),
                normal: rotation.inverse().transpose(),
            };
        }
    }

    pub fn transforms(&self) -> Ref<'_, [Transform]> {
        Ref::map(self.transforms.borrow(), |transforms| transforms.as_slice())
    }

    pub fn detached(&self, joint: usize) -> bool {
        self.detached.get() & (1 << joint) != 0
    }
}

pub fn vertex(vertex: &mut Vertex, weights: Influence, root: Vec3, pose: &[Transform]) {
    let bind = Vec3::from(vertex.position) + root;
    let normal = Vec3::from(vertex.normal);
    let mut position = Vec3::ZERO;
    let mut direction = Vec3::ZERO;
    for (joint, weight) in weights.joints.into_iter().zip(weights.weights) {
        if weight == 0.0 {
            continue;
        }
        let transform = pose[usize::from(joint)];
        position += transform.point.transform_point3(bind) * weight;
        direction += transform.normal * normal * weight;
    }
    vertex.position = position.to_array();
    vertex.normal = direction.normalize_or_zero().to_array();
}

#[cfg(test)]
mod tests {
    use super::*;
    use bytemuck::Zeroable;

    #[test]
    fn blended_joint_keeps_seams_closed_across_separate_parts() {
        let parts: Vec<_> = (0..15)
            .map(|i| model::Part {
                parent: if i == 0 { -1 } else { 0 },
                child: i,
                position: if i == 1 { [10, 0, 0] } else { [0; 3] },
                geometry: vec![],
            })
            .collect();
        let pose = Pose::new(&parts);
        let mut matrices = [Matrix::default(); 15];
        matrices[1].translation = [10, 0, 0].into();
        matrices[1].rotation = [[0, -4096, 0], [4096, 0, 0], [0, 0, 4096]];
        pose.set(&matrices, None);
        let weights = Influence {
            joints: [0, 1, 0, 0],
            weights: [0.5, 0.5, 0.0, 0.0],
        };
        let original = Vertex {
            position: [10.0, 2.0, 0.0],
            normal: [0.0, -1.0, 0.0],
            ..Vertex::zeroed()
        };
        let mut a = original;
        let mut b = Vertex {
            position: [0.0, 2.0, 0.0],
            ..original
        };
        vertex(&mut a, weights, pose.roots[0], &pose.transforms());
        vertex(&mut b, weights, pose.roots[1], &pose.transforms());
        assert_eq!(a.position, [9.0, 1.0, 0.0]);
        assert_eq!(a.position, b.position);
        assert_eq!(a.normal, b.normal);
        assert!((Vec3::from(a.normal).length() - 1.0).abs() < 1e-6);
        matrices[1].rotation = Matrix::default().rotation;
        pose.set(&matrices, None);
        let mut restored = original;
        vertex(&mut restored, weights, pose.roots[0], &pose.transforms());
        assert_eq!(restored.position, original.position);
    }

    #[test]
    fn invalid_skin_weights_cannot_reach_the_pose() {
        let valid = br#"[{"joints":[0,14,0,0],"weights":[0.25,0.75,0,0]}]"#;
        assert_eq!(decode(valid).unwrap().len(), 1);
        for value in [
            r#"[{"joints":[0,15,0,0],"weights":[0.25,0.75,0,0]}]"#,
            r#"[{"joints":[0,1,0,0],"weights":[-0.25,1.25,0,0]}]"#,
            r#"[{"joints":[0,1,0,0],"weights":[0,0,0,0]}]"#,
            r#"[{"joints":[0,1,0,0],"weights":[0.5,0.75,0,0]}]"#,
            "[]",
        ] {
            assert!(decode(value.as_bytes()).is_err());
        }
    }

    #[test]
    #[ignore = "requires TENCHU_DISC and TENCHU_MODEL_DIR; exercises an authored skinned model on both heroes without changing the simulation"]
    fn custom_skin_animates_on_both_heroes_without_gameplay_changes() {
        use crate::visuals::{interpolation::History, models::Models, pack::Pack};
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        let directory = std::env::var_os("TENCHU_MODEL_DIR").unwrap();
        let bodies = Pack::load(directory.as_ref()).unwrap();
        let id = bodies.player_models.keys().next().unwrap();
        let graphics = crate::settings::Graphics::default();
        for character in 0..2 {
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            game.events = Default::default();
            let models = Models::build(&game.mission, &Pack::default(), &graphics)
                .unwrap()
                .with_player(
                    &game,
                    &format!("pack:{id}"),
                    &Pack::default(),
                    &bodies,
                    &graphics,
                    None,
                )
                .unwrap();
            assert!(models.player.as_ref().unwrap().meshes.has_skin());
            let mut history = History::default();
            history.reset(&game);
            let mut moved = false;
            let mut previous = None;
            for tick in 0..180 {
                game.step(match tick % 60 {
                    0..20 => tenchu_core::input::UP,
                    25 => tenchu_core::input::SQUARE,
                    45 => tenchu_core::input::CROSS,
                    _ => 0,
                })
                .unwrap();
                history.tick(&game);
                let before = postcard::to_allocvec(&game).unwrap();
                let frame = history.sample(game.tick, 0.5);
                let mut positions = Vec::new();
                super::super::visit_parts(
                    &game,
                    true,
                    Some(&models),
                    frame.as_ref(),
                    |key, geometry, matrix| {
                        if let crate::visuals::interpolation::Part::Actor { root, model, .. } = key
                            && root == game.player().root_model
                            && game.player().parts.contains(&model)
                        {
                            assert!(models.visit_part(key, geometry, matrix, true, 0, |t| {
                                for v in t {
                                    let position = Vec3::from(v.position);
                                    assert!(
                                        position.is_finite() && Vec3::from(v.normal).is_finite()
                                    );
                                    assert!(
                                        position.distance(Vec3::from(
                                            game.player().position.to_array().map(|n| n as f32)
                                        )) < 6000.0
                                    );
                                    positions.push(v.position);
                                }
                            }));
                        }
                    },
                );
                assert!(!positions.is_empty());
                if let Some(previous) = &previous {
                    moved |= *previous != positions;
                }
                previous = Some(positions);
                assert_eq!(before, postcard::to_allocvec(&game).unwrap());
            }
            assert!(moved);
        }
    }
}
