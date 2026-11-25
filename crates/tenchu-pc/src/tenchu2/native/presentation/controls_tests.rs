use super::*;
use crate::settings::MenuButtons;

fn input(events: Vec<egui::Event>) -> egui::RawInput {
    egui::RawInput {
        screen_rect: Some(egui::Rect::from_min_size(
            egui::Pos2::ZERO,
            egui::vec2(960.0, 720.0),
        )),
        events,
        ..Default::default()
    }
}

fn text_rect(output: &egui::FullOutput, label: &str) -> Option<egui::Rect> {
    fn find(shape: &egui::Shape, label: &str) -> Option<egui::Rect> {
        match shape {
            egui::Shape::Text(text) if text.galley.job.text == label => {
                Some(text.galley.rect.translate(text.pos.to_vec2()))
            }
            egui::Shape::Vec(shapes) => shapes.iter().find_map(|shape| find(shape, label)),
            _ => None,
        }
    }
    output
        .shapes
        .iter()
        .find_map(|shape| find(&shape.shape, label))
}

#[test]
fn native_completion_controls_keep_existing_actions() {
    for convention in MenuButtons::ALL {
        let (confirm, back) = convention.names();
        for switch_games in [false, true] {
            for single_stage in [false, true] {
                for failed in [false, true] {
                    let content = if failed {
                        Content::Failed
                    } else {
                        Content::Complete
                    };
                    let ctx = egui::Context::default();
                    let mut output = egui::FullOutput::default();
                    for _ in 0..3 {
                        output = ctx.run(input(Vec::new()), |ctx| {
                            content.controls(ctx, convention, switch_games, single_stage);
                        });
                    }
                    let title = match (single_stage, failed) {
                        (true, true) => "Stage failed",
                        (true, false) => "Stage complete",
                        (false, true) => "Mission failed",
                        (false, false) => "Campaign complete",
                    };
                    assert!(text_rect(&output, title).is_some(), "{title}");
                    let retry = if single_stage {
                        "Retry stage"
                    } else {
                        "Retry mission"
                    };
                    let selection = match (single_stage, switch_games) {
                        (true, true) => "Quick launcher (F6)",
                        (true, false) => "Close preview",
                        (false, _) => "Mission selection",
                    };
                    for (label, expected) in [
                        (format!("{retry} · Enter / {confirm}"), Action::Retry),
                        (selection.to_owned(), Action::SelectMission),
                        (format!("Back · {back}"), Action::Quit),
                    ] {
                        let position = text_rect(&output, &label)
                            .unwrap_or_else(|| panic!("Missing control {label}"))
                            .center();
                        let mut selected = None;
                        for pressed in [true, false] {
                            let _ = ctx.run(
                                input(vec![
                                    egui::Event::PointerMoved(position),
                                    egui::Event::PointerButton {
                                        pos: position,
                                        button: egui::PointerButton::Primary,
                                        pressed,
                                        modifiers: Default::default(),
                                    },
                                ]),
                                |ctx| {
                                    selected = content.controls(
                                        ctx,
                                        convention,
                                        switch_games,
                                        single_stage,
                                    );
                                },
                            );
                        }
                        assert_eq!(
                            std::mem::discriminant(&selected.expect(&label)),
                            std::mem::discriminant(&expected),
                            "{label}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "requires TENCHU_COMPLETION_REVIEW and GPU; captures terminal controls in integrated and standalone native play"]
fn native_completion_controls_capture() {
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_COMPLETION_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let size = [960, 720];
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer =
        pollster::block_on(Renderer::new(&adapter, format, size, &Default::default())).unwrap();
    let graphics = Graphics::default();
    renderer.configure(&graphics);
    for (name, content, switch_games, single_stage) in [
        ("imported-complete", Content::Complete, true, true),
        ("standalone-complete", Content::Complete, false, true),
        ("jp-complete", Content::Complete, true, false),
        ("jp-failed", Content::Failed, true, false),
    ] {
        let mut drawing = ContentDrawing::new(&content, &graphics, &mut renderer).unwrap();
        for convention in MenuButtons::ALL {
            let ctx = egui::Context::default();
            crate::fonts::install(&ctx);
            let mut overlay = egui_wgpu::Renderer::new(&renderer.device, format, None, 1, false);
            crate::frontend::gpu::register(&mut overlay, format);
            for second in 0..3 {
                let mut raw = input(Vec::new());
                raw.time = Some(f64::from(second));
                let frame = ctx.run(raw, |ctx| {
                    drawing.paint(&content, ctx);
                    content.controls(ctx, convention, switch_games, single_stage);
                });
                let jobs = ctx.tessellate(frame.shapes, frame.pixels_per_point);
                renderer.update_overlay_textures(
                    &mut overlay,
                    &mut Vec::new(),
                    frame.textures_delta,
                );
                if second != 2 {
                    continue;
                }
                crate::capture::render_to_png(
                    &mut renderer,
                    size,
                    format,
                    &output.join(format!("{name}-{convention:?}.png")),
                    |renderer, target| {
                        drawing.draw(&content, target, renderer)?;
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
            }
        }
    }
}
