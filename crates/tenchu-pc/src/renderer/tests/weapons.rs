use super::*;

#[test]
#[ignore = "requires a GPU; metal reflections change with light position, leave matte surfaces alone, and coating coverage fades without white borders"]
fn weapon_reflections_and_blood_are_isolated_to_marked_surfaces() {
    use glam::Vec3;
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let mut graphics = crate::settings::Graphics {
        fog: false,
        ..Default::default()
    };
    graphics.lighting.enabled = true;
    graphics.lighting.ambient = 30;
    graphics.lighting.shadows = false;
    let plain = rectangle(1000, 80, None, false, false, true);
    let matte = renderer.upload(&plain);
    let mut metal = plain.clone();
    for vertex in &mut metal {
        vertex.flags |= render_mesh::WEAPON_METAL;
    }
    let metal = renderer.upload(&metal);
    let mut leaves = plain.clone();
    for v in &mut leaves {
        v.flags |= render_mesh::FOLIAGE | render_mesh::WEAPON_METAL;
    }
    let leaves = renderer.upload(&leaves);
    let light = crate::lighting::Light {
        position: Vec3::new(0.0, -300.0, 0.0),
        color: [1.0, 0.8, 0.6],
        radius: 3000.0,
        flame: false,
        emission_radius: 0.0,
    };
    let draw = |renderer: &mut Renderer, mesh: &Mesh, light: crate::lighting::Light| {
        renderer
            .draw_lit(
                &view,
                camera,
                SceneMeshes::new(&[], &[mesh]),
                None,
                &[light],
                &[],
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    renderer.configure(&graphics);
    let original = draw(&mut renderer, &matte, light);
    assert_eq!(draw(&mut renderer, &metal, light), original);
    graphics.weapon_reflections = 100;
    renderer.configure(&graphics);
    assert_eq!(draw(&mut renderer, &matte, light), original);
    assert_eq!(
        draw(&mut renderer, &leaves, light),
        original,
        "Foliage weights must not enable weapon reflections"
    );
    let reflected = draw(&mut renderer, &metal, light);
    assert_ne!(reflected, original);
    assert_ne!(
        draw(
            &mut renderer,
            &metal,
            crate::lighting::Light {
                position: Vec3::new(500.0, 0.0, 200.0),
                ..light
            }
        ),
        reflected
    );
    let draw_coating = |renderer: &mut Renderer, amount: f32, flow: [f32; 2], wet: f32| {
        let mut coating = plain.clone();
        for v in &mut coating {
            v.position[2] -= 1.0;
            v.color = [0.31, 0.006, 0.012];
            v.uv = [v.position[0] * 0.008, v.position[1] * 0.006];
            v.flags = render_mesh::WEAPON_COAT
                | render_mesh::STAIN
                | render_mesh::ALPHA
                | render_mesh::DOUBLE_SIDED;
            v.opacity = amount;
            v.page = flow[0].to_bits();
            v.palette = flow[1].to_bits();
            v.material = wet.to_bits();
        }
        let effects = renderer.upload_effects(render_effects::Geometry {
            vertices: coating,
            batches: vec![render_effects::Batch {
                blend: 5,
                range: 0..6,
            }],
        });
        renderer
            .draw_lit(
                &view,
                camera,
                SceneMeshes::new(&[], &[&matte]),
                Some(&effects),
                &[light],
                &[],
            )
            .unwrap();
        read_pixels(renderer, &target)
    };
    let mut changed = Vec::new();
    for amount in [0.0, 0.3, 0.7, 1.0] {
        let pixels = draw_coating(&mut renderer, amount, [0.0; 2], 0.0);
        let count = pixels
            .chunks_exact(4)
            .zip(original.chunks_exact(4))
            .filter(|(a, b)| a[1] < b[1])
            .count();
        if amount == 0.0 {
            assert_eq!(pixels, original);
        }
        changed.push(count);
        assert!(
            pixels
                .chunks_exact(4)
                .zip(original.chunks_exact(4))
                .all(|(a, b)| a[1] <= b[1] && a[2] <= b[2])
        );
    }
    assert!(changed.windows(2).all(|c| c[1] > c[0]), "{changed:?}");
    let dry = draw_coating(&mut renderer, 0.7, [0.0; 2], 0.0);
    assert_eq!(dry, draw_coating(&mut renderer, 0.7, [0.2, -0.6], 0.0));
    let wet = draw_coating(&mut renderer, 0.7, [0.0; 2], 1.0);
    let flowed = draw_coating(&mut renderer, 0.7, [0.2, -0.6], 1.0);
    assert_ne!(wet, dry);
    assert_ne!(wet, flowed);
    assert_eq!(draw_coating(&mut renderer, 0.0, [0.2, -0.6], 1.0), original);
}
