use super::*;

#[test]
#[ignore = "requires a graphics adapter; alpha stains respect depth and texture cutouts, blend softly, and clear without altering the original pass"]
fn projected_blood_blends_without_filling_cutouts_or_occluding_foreground() {
    let mut vram = Vram::default();
    vram.words[0] = 0x7fff;
    let (mut renderer, target) = fixture(&vram);
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let source = rectangle(
        1000,
        128,
        Some(Texture {
            page: 256,
            palette: 0,
        }),
        false,
        false,
        false,
    );
    let mesh = renderer.upload(&source);
    let background = renderer.upload(&rectangle(1500, 102, None, false, false, false));
    let mut vertices = crate::visuals::surfaces::Surfaces::new(&source).project(
        &crate::visuals::surfaces::Stamp {
            center: glam::Vec3::new(0.0, 0.0, 1000.0),
            normal: glam::Vec3::NEG_Z,
            radius: 400.0,
            depth: 1.0,
            seed: 0,
            shape: None,
        },
    );
    for v in &mut vertices {
        v.opacity *= 0.5;
    }
    let count = vertices.len() as u32;
    let effect = renderer.upload_effects(render_effects::Geometry {
        vertices,
        batches: vec![render_effects::Batch {
            blend: 5,
            range: 0..count,
        }],
    });
    renderer
        .draw(&view, camera, &[&background, &mesh], None)
        .unwrap();
    let original = read_pixel(&renderer, &target);
    renderer
        .draw(&view, camera, &[&background, &mesh], Some(&effect))
        .unwrap();
    let stained = read_pixel(&renderer, &target);
    assert!(stained[0] > stained[1] + 20, "{stained:?}");
    assert!(
        stained[1] > 90 && stained[1] < 170,
        "soft alpha blend: {stained:?}"
    );
    renderer
        .draw(&view, camera, &[&background, &mesh], None)
        .unwrap();
    assert_eq!(read_pixel(&renderer, &target), original);
    let foreground = renderer.upload(&rectangle(800, 60, None, false, false, false));
    renderer
        .draw(
            &view,
            camera,
            &[&background, &mesh, &foreground],
            Some(&effect),
        )
        .unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[60; 3]);
    vram.words[0] = 0;
    renderer.update_vram(&vram);
    renderer
        .draw(&view, camera, &[&background, &mesh], Some(&effect))
        .unwrap();
    assert_eq!(&read_pixel(&renderer, &target)[..3], &[102; 3]);
}
