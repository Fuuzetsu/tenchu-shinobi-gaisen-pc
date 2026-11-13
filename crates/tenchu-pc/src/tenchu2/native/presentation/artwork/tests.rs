use super::super::{Content, ContentDrawing, text_tests::state};
use crate::{renderer::Renderer, settings::Settings, tenchu2::native};
use tenchu_core::tenchu2::{introduction::Frame, progress};

fn capture(
    drawing: &mut ContentDrawing,
    content: &Content,
    renderer: &mut Renderer,
    path: &std::path::Path,
) {
    let size = renderer.size();
    let format = renderer.format();
    let ctx = egui::Context::default();
    crate::fonts::install(&ctx);
    let mut overlay = egui_wgpu::Renderer::new(&renderer.device, format, None, 1, false);
    crate::frontend::gpu::register(&mut overlay, format);
    for _ in 0..2 {
        let frame = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(size[0] as f32, size[1] as f32),
                )),
                ..Default::default()
            },
            |ctx| drawing.paint(content, ctx),
        );
        let jobs = ctx.tessellate(frame.shapes, frame.pixels_per_point);
        renderer.update_overlay_textures(&mut overlay, &mut Vec::new(), frame.textures_delta);
        crate::capture::render_to_png(renderer, size, format, path, |renderer, target| {
            drawing.draw(content, target, renderer)?;
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
        })
        .unwrap();
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU2_ASSET_PACK, TENCHU2_UI_REVIEW and GPU; HD menus, both button conventions, paused rebuilds and exact original restoration"]
fn native_hd_interface_restores_original_and_preserves_state() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU2_UI_REVIEW").unwrap())
        .join("screenshots");
    std::fs::create_dir_all(&output).unwrap();
    let mut title = native::title::Title::load(&disc).unwrap();
    for _ in 0..75 {
        title.step(0, None);
    }
    title.step(0x2000, None);
    let mut selection =
        native::selection::Selection::load(&disc, Default::default(), false).unwrap();
    selection.state.fade = 0;
    let mut equipment = native::equipment::Selection::load(
        &disc,
        0,
        2,
        progress::Stock {
            items: [Some(12); 21],
        },
    )
    .unwrap();
    equipment.state.fade = 0;
    equipment.state.description = 3;
    let mut summary = progress::Summary {
        time_tenths: 1234,
        detections: 3,
        effort: 124,
        ..Default::default()
    };
    let results = native::results::Results::load(
        &disc,
        progress::Results {
            record: summary.rank(13),
            summary,
            best: Default::default(),
            new_best: true,
            unlocked_item: None,
        },
    )
    .unwrap();
    let mut prologue = native::film::Prologue::load(&disc).unwrap();
    prologue.state.frame = Frame {
        columns: vec![(0, -100, 128), (15, 20, 128)],
        ..Default::default()
    };
    let mut cases = vec![
        ("title".to_owned(), Content::Title(Box::new(title))),
        (
            "selection".into(),
            Content::MissionSelection(Box::new(selection)),
        ),
        ("equipment".into(), Content::Equipment(Box::new(equipment))),
        ("results".into(), Content::Results(Box::new(results))),
        ("prologue".into(), Content::Prologue(Box::new(prologue))),
    ];
    for resource in [2, 3, 4, 8, 10, 11] {
        let mut intro = native::film::Introduction::load(&disc, resource).unwrap();
        intro.state.frame = Frame {
            columns: vec![(0, -100, 128), (15, 80, 128)],
            ..Default::default()
        };
        cases.push((
            format!("intro-{resource}"),
            Content::Introduction(Box::new(intro)),
        ));
    }
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        [1280, 720],
        &Default::default(),
    ))
    .unwrap();
    let mut settings = Settings::default();
    settings.graphics.asset_pack = false;
    settings.sharp_text = true;
    for (name, content) in cases {
        let before = state(&content);
        for circle_confirm in [true, false] {
            settings.circle_confirm = circle_confirm;
            settings.graphics.asset_pack = false;
            let mut drawing =
                ContentDrawing::new(&content, &settings.graphics, &mut renderer).unwrap();
            let mut captures = Vec::new();
            for (i, hd) in [false, true, false].into_iter().enumerate() {
                settings.graphics.asset_pack = hd;
                renderer.configure(&settings.graphics);
                drawing
                    .replace(
                        &content,
                        &Default::default(),
                        &settings.graphics,
                        &mut renderer,
                        true,
                    )
                    .unwrap();
                drawing
                    .update(&content, &mut renderer, &Default::default(), &settings)
                    .unwrap();
                let frame = match &drawing {
                    ContentDrawing::Title(d) => &d.frame,
                    ContentDrawing::MissionSelection(d) => &d.frame,
                    ContentDrawing::Equipment(d) => &d.frame,
                    ContentDrawing::Results(d) => &d.frame,
                    ContentDrawing::Prologue(d) | ContentDrawing::Introduction(d) => &d.frame,
                    _ => unreachable!(),
                };
                assert_eq!(
                    frame.images.keys().any(|id| id.ends_with(":hd")),
                    hd,
                    "{name}: replacement not bound"
                );
                let path = output.join(format!("{name}-circle-{circle_confirm}-{i}-hd-{hd}.png"));
                capture(&mut drawing, &content, &mut renderer, &path);
                captures.push(std::fs::read(path).unwrap());
                assert!(
                    state(&content) == before,
                    "{name}: presentation changed game state"
                );
            }
            assert!(captures[0] != captures[1], "{name}: no visible HD change");
            assert!(
                captures[0] == captures[2],
                "{name}: original did not restore"
            );
        }
    }
}
