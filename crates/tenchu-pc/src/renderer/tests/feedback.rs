use super::*;

#[test]
#[ignore = "requires a graphics adapter; verifies original frame mixing, paused redraws and history invalidation"]
fn story_feedback_blends_once_per_tick_and_discards_stale_history() {
    let (mut renderer, target) = fixture(&Vram::default());
    let view = target.create_view(&Default::default());
    let camera = game_camera(Matrix::default(), 1.0);
    let dark = renderer.upload(&rectangle(1000, 40, None, false, false, false));
    let light = renderer.upload(&rectangle(1000, 160, None, false, false, false));
    for samples in renderer.sample_counts.clone() {
        for post in [false, true] {
            let mut graphics = crate::settings::Graphics {
                samples,
                ..Default::default()
            };
            graphics.post.enabled = post;
            renderer.configure(&graphics);
            renderer.feedback = None;
            renderer.draw(&view, camera, &[&dark], None).unwrap();
            let a = read_pixel(&renderer, &target);
            renderer.draw(&view, camera, &[&light], None).unwrap();
            let b = read_pixel(&renderer, &target);
            assert_ne!(a, b);
            for additive in [false, true] {
                let request = |scene, tick| {
                    Some(super::super::feedback::Request {
                        scene,
                        tick,
                        additive: Some(additive),
                    })
                };
                renderer.feedback = request(1, 0);
                renderer.draw(&view, camera, &[&dark], None).unwrap();
                assert_eq!(read_pixel(&renderer, &target), a);
                renderer.feedback = request(1, 1);
                renderer.draw(&view, camera, &[&light], None).unwrap();
                let blended = read_pixel(&renderer, &target);
                for axis in 0..3 {
                    let expected = if additive {
                        f32::from(b[axis]) + f32::from(a[axis]) * 0.75
                    } else {
                        (f32::from(a[axis]) + f32::from(b[axis])) * 0.5
                    };
                    assert!((f32::from(blended[axis]) - expected.min(255.0)).abs() <= 1.0);
                }
                for _ in 0..3 {
                    renderer.draw(&view, camera, &[&light], None).unwrap();
                    assert_eq!(read_pixel(&renderer, &target), blended);
                }
                for (scene, tick) in [(1, 10), (2, 11), (2, 2)] {
                    renderer.feedback = request(scene, tick);
                    renderer.draw(&view, camera, &[&dark], None).unwrap();
                    assert_eq!(read_pixel(&renderer, &target), a);
                }
            }
        }
    }
}
