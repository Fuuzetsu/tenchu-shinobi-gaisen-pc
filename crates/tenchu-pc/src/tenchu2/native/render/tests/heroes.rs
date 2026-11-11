use super::*;
use crate::visuals::appearance::library::Library;
use tenchu_assets::tenchu2::characters;
use tenchu_core::{math::Math, motion::Pose, scene::Scene};

fn vertices(meshes: &Meshes, part: &Part, matrix: Matrix) -> Vec<Vertex> {
    let mut out = Vec::new();
    meshes.visit(&part.geometry, matrix, false, 0, None, |t| out.extend(t));
    out
}

fn bounds(vertices: &[Vertex]) -> (Vec3, Vec3) {
    vertices.iter().fold(
        (Vec3::splat(f32::INFINITY), Vec3::splat(f32::NEG_INFINITY)),
        |(lo, hi), v| {
            let p = Vec3::from(v.position);
            (lo.min(p), hi.max(p))
        },
    )
}

#[test]
#[ignore = "requires both discs; all T2 hero outfits retain bounds, joint lengths and finite skinning in native and imported bone orders"]
fn high_detail_heroes_preserve_both_rig_orders_and_leave_other_actors_alone() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&gaisen).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let math = Math {
        tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
    };
    for name in [
        "rikimaru", "rikia", "ayame", "ayamea", "tatumaru", "tatua", "tatua_m", "tatu2", "tatu2_m",
        "riki2", "rikia2", "riki3", "rikia3", "siun",
    ] {
        let native = characters::rig(&disc, name).unwrap();
        let imported = library.get(&format!("tenchu2-jp:{name}"));
        for use_native in [false, true] {
            let (parts, vram) = if use_native {
                (native.parts.as_slice(), &native.vram)
            } else {
                let Some(imported) = imported else { continue };
                (imported.parts.as_slice(), imported.vram.as_ref())
            };
            let original_parts = parts.to_vec();
            let original_vram = vram.clone();
            let sources = || {
                if use_native {
                    sources(&format!("actors/{name}"), parts, Role::Actor)
                } else {
                    imported.unwrap().sources()
                }
            };
            let mut graphics = Graphics {
                asset_pack: false,
                ..Default::default()
            };
            let mut original = build(vram, sources(), &graphics).unwrap();
            graphics.models.heroes = true;
            let mut refined = build(vram, sources(), &graphics).unwrap();
            original.skin(parts);
            refined.skin(parts);
            assert_eq!(original.has_skin(), refined.has_skin());
            let mut scene = Scene::default();
            let root = scene.add(None, Matrix::default());
            let joints = Pose::new(parts).unwrap().bind(&mut scene, root, &math);
            let mut matrices: Vec<_> = joints
                .into_iter()
                .map(|joint| scene.world(joint).unwrap())
                .collect();
            let mut total = 0;
            for pass in 0..2 {
                if pass == 1 {
                    for (i, matrix) in matrices.iter_mut().enumerate() {
                        matrix.rotation = math.rotate_yxz([i as i16 * 37, i as i16 * 128, 0]);
                    }
                }
                original.pose_skin(&matrices, None);
                refined.pose_skin(&matrices, None);
                for (index, (part, &matrix)) in parts.iter().zip(&matrices).enumerate() {
                    let a = vertices(&original, part, matrix);
                    let b = vertices(&refined, part, matrix);
                    if name == "siun" || index >= 15 {
                        assert_eq!(
                            bytemuck::cast_slice::<_, u8>(&a),
                            bytemuck::cast_slice::<_, u8>(&b)
                        );
                    } else {
                        assert_eq!(
                            b.len(),
                            a.len() * 64,
                            "{name} native={use_native} part={index}"
                        );
                    }
                    assert!(b.iter().all(|v| Vec3::from(v.position).is_finite()
                        && Vec3::from(v.normal).is_finite()
                        && v.uv.iter().all(|n| n.is_finite())));
                    if pass == 0 && !a.is_empty() {
                        let (lo, hi) = bounds(&a);
                        let (low, high) = bounds(&b);
                        assert!(
                            (lo.y - low.y).abs() < 0.02 && (hi.y - high.y).abs() < 0.02,
                            "{name} native={use_native} part={index}: joint length changed"
                        );
                        assert!(
                            low.cmpge(lo - Vec3::splat(0.02)).all()
                                && high.cmple(hi + Vec3::splat(0.02)).all(),
                            "{name} native={use_native} part={index}: inflated bounds"
                        );
                        total += b.len() / 3;
                    }
                }
            }
            assert!(
                total > 0 && total < 100_000,
                "{name}: unbounded refinement {total}"
            );
            assert_eq!(parts, original_parts);
            assert!(vram == &original_vram);
            eprintln!("{name} native={use_native}: {total} triangles");
        }
    }
}
