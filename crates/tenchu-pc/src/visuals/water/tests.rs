use super::*;
use bytemuck::Zeroable;

#[test]
fn source_water_uses_face_orientation_and_respects_artwork_overrides() {
    let mut material = Material {
        id: "source-water".into(),
        region: None,
        image: std::sync::Arc::new(super::super::pixels::Image {
            size: [1, 1],
            rgba: vec![255; 4],
        }),
        animation: Some(tenchu_assets::texture::TextureScroll {
            image: tenchu_assets::texture::Image {
                page: 0,
                palette: 0,
                uv: [0; 2],
                size: [64; 2],
            },
            velocity: [-4, -8],
            delay: 5,
            water: true,
        }),
    };
    let source: Vec<_> = [Vec3::ZERO, Vec3::X, Vec3::Z, Vec3::ZERO, Vec3::X, Vec3::Y]
        .map(|p| Vertex {
            position: p.to_array(),
            material: 1,
            ..Vertex::zeroed()
        })
        .into();
    let settings = settings::Water {
        detail: 0,
        ..Default::default()
    };
    for (authored_water, override_kind) in [(true, None), (false, None), (true, Some(Kind::Off))] {
        material.animation.as_mut().unwrap().water = authored_water;
        let mut result = source.clone();
        let rules = override_kind
            .map(|kind| (material.id.clone(), kind))
            .into_iter()
            .collect();
        prepare(
            &mut result,
            std::slice::from_ref(&material),
            &rules,
            &settings,
            &mut 0,
        );
        if authored_water && override_kind.is_none() {
            assert!(result[..3].iter().all(|v| v.flags == render_mesh::WATER));
            assert!(
                result[3..]
                    .iter()
                    .all(|v| v.flags == render_mesh::WATER | render_mesh::WATERFALL)
            );
        } else {
            assert_eq!(
                bytemuck::cast_slice::<_, u8>(&result),
                bytemuck::cast_slice::<_, u8>(&source)
            );
        }
    }
}

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
        animation: None,
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
