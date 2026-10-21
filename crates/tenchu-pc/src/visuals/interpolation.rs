use crate::render_mesh;
use glam::{Mat3, Mat4, Quat, Vec3};
use std::collections::BTreeMap;
use tenchu_core::{game::Game, math::Matrix};

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Part {
    Actor {
        root: usize,
        model: usize,
        kind: i16,
        inspection: bool,
    },
    Hinge {
        object: usize,
        part: usize,
        mesh: usize,
    },
}

struct Snapshot {
    tick: u64,
    parts: BTreeMap<Part, Matrix>,
    roots: BTreeMap<usize, [i32; 3]>,
    view: Matrix,
    cinematic: (bool, i16),
    camera_mode: tenchu_core::camera::Mode,
}

impl Snapshot {
    fn new(game: &Game) -> Self {
        let mut parts = BTreeMap::new();
        for inspection in [false, true] {
            render_mesh::visit_character_parts(game, inspection, |key, _, matrix| {
                parts.insert(key, matrix);
            });
        }
        render_mesh::visit_hinged_parts(game, |key, _, matrix| {
            parts.insert(key, matrix);
        });
        Self {
            tick: game.tick,
            parts,
            roots: game
                .roster
                .order
                .iter()
                .map(|&id| {
                    let a = game.roster.get(id);
                    (a.root_model, a.position)
                })
                .collect(),
            view: game.rendered_view,
            cinematic: (game.cinematic.active, game.cinematic.sequence),
            camera_mode: game.camera.mode,
        }
    }
}

#[derive(Default)]
pub struct History {
    previous: Option<Snapshot>,
    current: Option<Snapshot>,
}

pub struct Frame {
    parts: BTreeMap<Part, Matrix>,
    hinges: Vec<(usize, Matrix, Matrix)>,
    pub view: Matrix,
}

impl History {
    pub fn reset(&mut self, game: &Game) {
        self.previous = None;
        self.current = Some(Snapshot::new(game));
    }

    pub fn tick(&mut self, game: &Game) {
        let next = Snapshot::new(game);
        self.previous = self
            .current
            .take()
            .filter(|s| s.tick + 1 == game.tick && s.cinematic == next.cinematic);
        self.current = Some(next);
    }

    pub fn sample(&self, tick: u64, alpha: f32) -> Option<Frame> {
        let current = self.current.as_ref().filter(|s| s.tick == tick)?;
        let previous = self.previous.as_ref()?;
        let alpha = alpha.clamp(0.0, 1.0);
        if alpha >= 1.0 {
            return None;
        }
        let mut parts = BTreeMap::new();
        let mut hinges = Vec::new();
        for (&key, &matrix) in &current.parts {
            let continuous = match key {
                Part::Actor { root, .. } => previous
                    .roots
                    .get(&root)
                    .zip(current.roots.get(&root))
                    .is_some_and(|(a, b)| close(*a, *b, 2000.0)),
                Part::Hinge { .. } => true,
            };
            let pose = previous
                .parts
                .get(&key)
                .filter(|_| continuous)
                .map_or(matrix, |&before| blend(before, matrix, alpha));
            parts.insert(key, pose);
            if let Part::Hinge { mesh, .. } = key {
                hinges.push((mesh, matrix, pose));
            }
        }
        // Script cameras can cut even between nearby viewpoints. Keep their authored cuts exact.
        let view = if current.cinematic.0 || previous.camera_mode != current.camera_mode {
            current.view
        } else {
            let before = matrix4(previous.view).inverse();
            let after = matrix4(current.view).inverse();
            if before.w_axis.truncate().distance(after.w_axis.truncate()) > 3000.0
                || rotation(before).dot(rotation(after)).abs() < 0.9238795
            {
                current.view
            } else {
                source_matrix(blend_affine(before, after, alpha).inverse())
            }
        };
        Some(Frame {
            parts,
            hinges,
            view,
        })
    }
}

impl Frame {
    pub fn transform(frame: Option<&Self>, key: Part, original: Matrix) -> Matrix {
        frame
            .and_then(|f| f.parts.get(&key))
            .copied()
            .unwrap_or(original)
    }

    pub fn hinged(&self, mesh: usize, original: Matrix) -> Matrix {
        self.hinges
            .iter()
            .find(|(kind, matrix, _)| *kind == mesh && *matrix == original)
            .map_or(original, |(_, _, matrix)| *matrix)
    }
}

fn close(a: [i32; 3], b: [i32; 3], radius: f32) -> bool {
    Vec3::from(a.map(|v| v as f32)).distance_squared(Vec3::from(b.map(|v| v as f32)))
        <= radius * radius
}

fn matrix4(matrix: Matrix) -> Mat4 {
    let mut out = Mat4::from_mat3(super::models::rotation(matrix));
    out.w_axis = Vec3::from(matrix.translation.map(|v| v as f32)).extend(1.0);
    out
}

fn rotation(matrix: Mat4) -> Quat {
    matrix.to_scale_rotation_translation().1.normalize()
}

fn blend_affine(a: Mat4, b: Mat4, alpha: f32) -> Mat4 {
    if a.determinant().abs() < 0.000001 || b.determinant().abs() < 0.000001 {
        return b;
    }
    let (sa, qa, pa) = a.to_scale_rotation_translation();
    let (sb, qb, pb) = b.to_scale_rotation_translation();
    if !qa.is_finite()
        || !qb.is_finite()
        || sa.abs().min_element() < 0.0001
        || sb.abs().min_element() < 0.0001
    {
        return b;
    }
    Mat4::from_scale_rotation_translation(
        sa.lerp(sb, alpha),
        qa.normalize().slerp(qb.normalize(), alpha),
        pa.lerp(pb, alpha),
    )
}

fn source_matrix(matrix: Mat4) -> Matrix {
    let rotation = Mat3::from_mat4(matrix).to_cols_array();
    Matrix {
        rotation: std::array::from_fn(|row| {
            std::array::from_fn(|col| {
                (rotation[col * 3 + row] * 4096.0)
                    .round()
                    .clamp(i16::MIN as f32, i16::MAX as f32) as i16
            })
        }),
        translation: matrix
            .w_axis
            .truncate()
            .to_array()
            .map(|v| v.round() as i32),
    }
}

fn blend(a: Matrix, b: Matrix, alpha: f32) -> Matrix {
    if alpha <= 0.0 {
        a
    } else if alpha >= 1.0 {
        b
    } else {
        source_matrix(blend_affine(matrix4(a), matrix4(b), alpha))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rotations_keep_rigid_shape_and_take_the_short_path_across_wrap() {
        let pose = |degrees: f32, x: f32| {
            source_matrix(Mat4::from_scale_rotation_translation(
                Vec3::splat(1.5),
                Quat::from_rotation_y(degrees.to_radians()),
                Vec3::new(x, -200.0, 1000.0),
            ))
        };
        let a = pose(170.0, 0.0);
        let b = pose(-170.0, 100.0);
        assert_eq!(blend(a, b, 0.0), a);
        assert_eq!(blend(a, b, 1.0), b);
        let middle = matrix4(blend(a, b, 0.5));
        assert!((middle.x_axis.truncate().length() - 1.5).abs() < 0.001);
        assert!((middle.y_axis.truncate().length() - 1.5).abs() < 0.001);
        assert!(middle.x_axis.x < -1.49);
        assert_eq!(middle.w_axis.x, 50.0);
        let flat = Matrix {
            rotation: [[0; 3]; 3],
            ..b
        };
        assert_eq!(blend(flat, b, 0.5), b);
    }

    #[test]
    fn replacements_teleports_and_camera_cuts_do_not_morph_between_unrelated_states() {
        let key = Part::Actor {
            root: 1,
            model: 2,
            kind: 3,
            inspection: false,
        };
        let pose = |x| Matrix {
            translation: [x, 0, 0],
            ..Default::default()
        };
        let snapshot = |tick, x| Snapshot {
            tick,
            parts: BTreeMap::from([(key, pose(x))]),
            roots: BTreeMap::from([(1, [x, 0, 0])]),
            view: pose(-x),
            cinematic: (false, 0),
            camera_mode: tenchu_core::camera::Mode::Normal,
        };
        let mut history = History {
            previous: Some(snapshot(1, 0)),
            current: Some(snapshot(2, 100)),
        };
        let middle = history.sample(2, 0.5).unwrap();
        assert_eq!(Frame::transform(Some(&middle), key, pose(100)), pose(50));
        assert_eq!(middle.view, pose(-50));
        let replacement = Part::Actor {
            root: 1,
            model: 2,
            kind: 4,
            inspection: false,
        };
        assert_eq!(
            Frame::transform(Some(&middle), replacement, pose(100)),
            pose(100)
        );
        assert!(history.sample(3, 0.5).is_none());
        assert!(history.sample(2, 1.0).is_none());
        history.current.as_mut().unwrap().cinematic.0 = true;
        assert_eq!(history.sample(2, 0.5).unwrap().view, pose(-100));
        history.current = Some(snapshot(2, 10_000));
        let snap = history.sample(2, 0.5).unwrap();
        assert_eq!(
            Frame::transform(Some(&snap), key, pose(10_000)),
            pose(10_000)
        );
        assert_eq!(snap.view, pose(-10_000));
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; sampled body/weapon poses and shadows change between ticks without changing any gameplay state"]
    fn live_pose_sampling_is_visual_only_and_preserves_current_frame_endpoints() {
        let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for character in 0..2 {
            let mut game = crate::session::Options {
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            let mut history = History::default();
            history.reset(&game);
            for tick in 0..45 {
                game.step(if tick == 35 {
                    tenchu_core::input::SQUARE
                } else {
                    0
                })
                .unwrap();
                history.tick(&game);
            }
            let state = crate::session::tests::state(&game);
            let original = render_mesh::characters(&game, None);
            let positions = |vertices: Vec<render_mesh::Vertex>| {
                vertices.into_iter().map(|v| v.position).collect::<Vec<_>>()
            };
            let start = history.sample(game.tick, 0.0).unwrap();
            let middle = history.sample(game.tick, 0.5).unwrap();
            assert_ne!(
                positions(render_mesh::characters_presented(
                    &game,
                    None,
                    false,
                    Some(&middle)
                )),
                positions(original)
            );
            assert_ne!(
                positions(render_mesh::shadow_models_presented(
                    &game,
                    None,
                    Some(&start)
                )),
                positions(render_mesh::shadow_models_presented(
                    &game,
                    None,
                    Some(&middle)
                ))
            );
            let mut moving_weapons = 0;
            for weapon in game.player().weapons[..2].iter().flatten() {
                let key = Part::Actor {
                    root: game.player().root_model,
                    model: weapon.model,
                    kind: weapon.kind,
                    inspection: false,
                };
                if history
                    .previous
                    .as_ref()
                    .unwrap()
                    .parts
                    .get(&key)
                    .is_some_and(|m| *m != weapon.rendered)
                {
                    assert_ne!(
                        Frame::transform(Some(&middle), key, weapon.rendered),
                        weapon.rendered
                    );
                    moving_weapons += 1;
                }
            }
            assert!(
                moving_weapons > 0,
                "character {character} has no interpolated weapon"
            );
            for alpha in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let frame = history.sample(game.tick, alpha);
                let _ = render_effects(&game, frame.as_ref());
                let _ = render_mesh::characters_presented(&game, None, true, frame.as_ref());
            }
            assert_eq!(state, crate::session::tests::state(&game));
            let exact = render_mesh::characters_presented(
                &game,
                None,
                false,
                history.sample(game.tick, 1.0).as_ref(),
            );
            assert!(
                bytemuck::cast_slice::<_, u8>(&exact)
                    == bytemuck::cast_slice::<_, u8>(&render_mesh::characters(&game, None))
            );
            history.reset(&game);
            assert!(history.sample(game.tick, 0.0).is_none());
        }
    }

    fn render_effects(game: &Game, frame: Option<&Frame>) -> crate::render_effects::Geometry {
        crate::render_effects::build_presented(game, true, None, true, None, frame)
    }
}
