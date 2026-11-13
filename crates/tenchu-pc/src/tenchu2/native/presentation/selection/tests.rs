use super::*;

fn capture(path: &std::path::Path, canvas: &Canvas) {
    let mut encoder = png::Encoder::new(std::fs::File::create(path).unwrap(), 320, 240);
    encoder.set_color(png::ColorType::Rgba);
    encoder.set_depth(png::BitDepth::Eight);
    encoder
        .write_header()
        .unwrap()
        .write_image_data(&canvas.pixels)
        .unwrap();
}

#[test]
#[ignore = "requires TENCHU2_DISC; the turning backdrop cannot erase limbs from any settled selector portrait"]
fn selection_portraits_remain_in_front_of_the_backdrop() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut selection = Selection::load(&disc, Default::default(), true).unwrap();
    let output = std::env::var_os("TENCHU2_SELECTION_REVIEW").map(std::path::PathBuf::from);
    if let Some(path) = &output {
        std::fs::create_dir_all(path).unwrap();
    }
    let mut missing = [0; 3];
    for (hero, missing) in missing.iter_mut().enumerate() {
        if hero != 0 {
            selection
                .step(0, Some(Action::MissionSelection(Edit::Hero(true))))
                .unwrap();
        }
        for tick in 0..150 {
            selection.step(0, None).unwrap();
            if selection.state.hero != selection.state.displayed || selection.state.fade != 0 {
                continue;
            }
            let before = postcard::to_allocvec(&(&selection.state, &selection.motions)).unwrap();
            let mut actual = Canvas::new(320);
            actual.sprite(Sprite::new(&selection.resources.images[0], 0.0, 0.0));
            characters(&selection, &mut actual).unwrap();
            let mut expected = Canvas::new(320);
            expected.sprite(Sprite::new(&selection.resources.images[0], 0.0, 0.0));
            panel(&selection, &mut expected);
            let backdrop = selection.resources.images[1].pixels.clone();
            selection.resources.images[1].pixels.fill(0);
            characters(&selection, &mut expected).unwrap();
            selection.resources.images[1].pixels = backdrop;
            *missing += actual
                .pixels
                .chunks_exact(4)
                .zip(expected.pixels.chunks_exact(4))
                .filter(|(a, b)| a != b)
                .count();
            assert!(
                before == postcard::to_allocvec(&(&selection.state, &selection.motions)).unwrap()
            );
            if let Some(path) = &output
                && [15, 30, 75, 149].contains(&tick)
            {
                capture(&path.join(format!("hero-{hero}-tick-{tick}.png")), &actual);
                capture(
                    &path.join(format!("hero-{hero}-tick-{tick}-complete.png")),
                    &expected,
                );
            }
        }
    }
    eprintln!("Portrait pixels hidden by the backdrop over three idle sequences: {missing:?}");
    assert_eq!(missing, [0; 3]);
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU2_SELECTION_REVIEW and GPU; capture idle and turning native selector portraits"]
fn selection_portraits_gpu_review() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut selection = Selection::load(&disc, Default::default(), true).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU2_SELECTION_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let size = [1280, 960];
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = pollster::block_on(crate::renderer::Renderer::new(
        &adapter,
        format,
        size,
        &Default::default(),
    ))
    .unwrap();
    let mut overlay = egui_wgpu::Renderer::new(&renderer.device, format, None, 1, false);
    crate::frontend::gpu::register(&mut overlay, format);
    let ctx = egui::Context::default();
    crate::fonts::install(&ctx);
    let rect =
        egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size[0] as f32, size[1] as f32));
    for hero in 0..4 {
        if hero != 0 {
            selection
                .step(0, Some(Action::MissionSelection(Edit::Hero(true))))
                .unwrap();
        }
        for tick in 0..150 {
            selection.step(0, None).unwrap();
            if ![0, 2, 4, 6, 15, 75, 149].contains(&tick) {
                continue;
            }
            let before = postcard::to_allocvec(&(&selection.state, &selection.motions)).unwrap();
            let (frame, text) = build(&selection, Arc::default(), true).unwrap();
            let path = output.join(format!("gpu-hero-{hero}-tick-{tick}.png"));
            crate::capture::render_to_png(
                &mut renderer,
                size,
                format,
                &path,
                |renderer, target| {
                    let shapes = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(rect),
                            ..Default::default()
                        },
                        |ctx| {
                            let painter = ctx.layer_painter(egui::LayerId::background());
                            painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
                            painter.add(egui_wgpu::Callback::new_paint_callback(
                                rect,
                                crate::frontend::gpu::Paint::Frontend(frame.clone()),
                            ));
                            crate::fonts::paint(ctx, rect, &text);
                        },
                    );
                    let jobs = ctx.tessellate(shapes.shapes, shapes.pixels_per_point);
                    renderer.update_overlay_textures(
                        &mut overlay,
                        &mut Vec::new(),
                        shapes.textures_delta,
                    );
                    renderer.draw_overlay(
                        target,
                        &mut overlay,
                        &jobs,
                        &egui_wgpu::ScreenDescriptor {
                            size_in_pixels: size,
                            pixels_per_point: 1.0,
                        },
                    );
                    Ok(())
                },
            )
            .unwrap();
            assert!(
                before == postcard::to_allocvec(&(&selection.state, &selection.motions)).unwrap()
            );
        }
    }
}
