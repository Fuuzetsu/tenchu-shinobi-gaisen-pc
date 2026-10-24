use super::*;
use bytemuck::Zeroable;

#[test]
fn waterfall_curves_keep_boundary_uvs_and_respect_geometry_budget() {
    let points = [
        [-1000.0, -2000.0, 0.0],
        [1000.0, -2000.0, 0.0],
        [-1000.0, -2000.0, 1000.0],
        [1000.0, -2000.0, 1000.0],
        [-1000.0, 0.0, 1000.0],
        [1000.0, 0.0, 1000.0],
    ];
    let source: Vec<_> = [0, 1, 2, 1, 3, 2, 2, 3, 4, 3, 5, 4]
        .into_iter()
        .map(|i| Vertex {
            position: points[i],
            uv: [i as f32 % 2.0, (i / 2) as f32 / 2.0],
            material: 1,
            ..Vertex::zeroed()
        })
        .collect();
    let materials = vec![Material {
        id: "fall".into(),
        region: None,
        image: std::sync::Arc::new(super::super::pixels::Image {
            size: [1, 1],
            rgba: vec![255; 4],
        }),
    }];
    let rules = BTreeMap::from([("fall".into(), Kind::Waterfall)]);
    let mut curved = source.clone();
    let mut budget = 10000;
    prepare(
        &mut curved,
        &materials,
        &rules,
        &settings::Water::default(),
        &mut budget,
    );
    assert_eq!(curved.len(), source.len() * 16);
    assert_eq!(budget + curved.len(), 10000);
    for original in &source {
        assert!(
            curved
                .iter()
                .any(|v| v.position == original.position && v.uv == original.uv)
        );
    }
    assert!(curved.iter().all(|v| Vec3::from(v.position).is_finite()
        && Vec3::from(v.normal).is_finite()
        && v.uv.iter().all(|&u| (0.0..=1.0).contains(&u))));
    assert!(
        curved
            .iter()
            .any(|v| v.position[1] < -2001.0 || v.position[2] > 1001.0)
    );
    for vertex in &curved {
        let p = Vec3::from(vertex.position);
        let distance = source
            .chunks_exact(3)
            .map(|t| p.distance(super::super::surfaces::closest(&[t[0], t[1], t[2]], p)))
            .fold(f32::INFINITY, f32::min);
        assert!(distance <= 250.001);
    }
    let spray = Spray::new(&curved);
    assert!(!spray.edges.is_empty());
    assert!(spray.edges.iter().any(|e| e.impact));
    assert!(
        spray
            .edges
            .iter()
            .filter(|e| e.impact)
            .all(|e| e.a.y == 0.0 && e.b.y == 0.0)
    );
    assert!(
        spray
            .edges
            .iter()
            .all(|e| e.a.y > -2000.0 || e.b.y > -2000.0)
    );
    let camera = Mat4::perspective_rh(1.0, 1.0, 10.0, 100000.0)
        * Mat4::look_at_rh(Vec3::new(0.0, -1000.0, -3000.0), Vec3::ZERO, Vec3::NEG_Y);
    let settings = settings::Water {
        enabled: true,
        ..Default::default()
    };
    let first = spray.geometry(0.0, camera, &settings);
    let second = spray.geometry(1.0, camera, &settings);
    assert!(!first.vertices.is_empty());
    assert!(first.vertices.len() <= 32 * 20 * 6);
    assert_ne!(first.vertices[0].position, second.vertices[0].position);
    let mut limited = source.clone();
    prepare(
        &mut limited,
        &materials,
        &rules,
        &settings::Water::default(),
        &mut 0,
    );
    assert_eq!(limited.len(), source.len());
    for (before, after) in source.iter().zip(limited) {
        assert_eq!(before.position, after.position);
    }
}
