use super::{Content, ContentDrawing};
use crate::{renderer::Renderer, settings::Settings, tenchu2::native};
use tenchu_core::tenchu2::{introduction::Frame, progress};

pub(super) fn state(content: &Content) -> Vec<u8> {
    match content {
        Content::Title(value) => postcard::to_allocvec(&(
            value.state.choice,
            value.state.fade,
            value.state.color,
            value.state.tick,
            value.state.active,
            value.state.attract,
            value.state.idle,
            value.state.finished,
        )),
        Content::MissionSelection(value) => postcard::to_allocvec(&value.state),
        Content::Equipment(value) => postcard::to_allocvec(&value.state),
        Content::Results(value) => postcard::to_allocvec(&value.values),
        Content::Introduction(value) => postcard::to_allocvec(&value.state),
        Content::Prologue(value) => postcard::to_allocvec(&value.state),
        _ => unreachable!(),
    }
    .unwrap()
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU2_HUD_REVIEW and GPU; menu preference updates disc legends and PC prompts while paused without changing state"]
fn native_menu_convention_updates_legends_and_restores_original_artwork() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU2_HUD_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut selection =
        native::selection::Selection::load(&disc, Default::default(), true).unwrap();
    selection.state.fade = 0;
    let mut equipment = native::equipment::Selection::load(
        &disc,
        0,
        2,
        progress::Stock {
            items: [Some(5); 21],
        },
    )
    .unwrap();
    equipment.state.fade = 0;
    let results = native::results::Results::load(
        &disc,
        progress::Results {
            record: Default::default(),
            summary: Default::default(),
            best: Default::default(),
            new_best: false,
            unlocked_item: None,
        },
    )
    .unwrap();
    let mut title = native::title::Title::load(&disc).unwrap();
    for _ in 0..75 {
        title.step(0, None);
    }
    let mut active_title = native::title::Title::load(&disc).unwrap();
    for _ in 0..75 {
        active_title.step(0, None);
    }
    active_title.step(0x2000, None);
    let mut attract_title = native::title::Title::load(&disc).unwrap();
    for _ in 0..75 {
        attract_title.step(0, None);
    }
    attract_title.step(0x2000, None);
    attract_title.state.idle = 1801;
    attract_title.step(0, None);
    let cases = [
        ("title", Content::Title(Box::new(title))),
        ("title-menu", Content::Title(Box::new(active_title))),
        ("title-attract", Content::Title(Box::new(attract_title))),
        ("selection", Content::MissionSelection(Box::new(selection))),
        ("equipment", Content::Equipment(Box::new(equipment))),
        ("results", Content::Results(Box::new(results))),
    ];
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let size = [1280, 720];
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer =
        pollster::block_on(Renderer::new(&adapter, format, size, &Default::default())).unwrap();
    let mut settings = Settings::default();
    settings.graphics.asset_pack = false;
    renderer.configure(&settings.graphics);
    for (name, content) in cases {
        let before = state(&content);
        let mut drawing = ContentDrawing::new(&content, &settings.graphics, &mut renderer).unwrap();
        let mut captures = Vec::new();
        for (step, circle_confirm) in [true, false, true].into_iter().enumerate() {
            settings.circle_confirm = circle_confirm;
            drawing
                .update(&content, &mut renderer, &Default::default(), &settings)
                .unwrap();
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
                    |ctx| {
                        content.controls(ctx, circle_confirm, true);
                    },
                );
                renderer.update_overlay_textures(
                    &mut overlay,
                    &mut Vec::new(),
                    frame.textures_delta,
                );
            }
            let path = output.join(format!("{name}-{step}-circle-{circle_confirm}.png"));
            crate::capture::render_to_png(
                &mut renderer,
                size,
                format,
                &path,
                |renderer, target| {
                    drawing.draw(&content, target, renderer)?;
                    let frame = ctx.run(
                        egui::RawInput {
                            screen_rect: Some(egui::Rect::from_min_size(
                                egui::Pos2::ZERO,
                                egui::vec2(size[0] as f32, size[1] as f32),
                            )),
                            ..Default::default()
                        },
                        |ctx| {
                            drawing.paint(&content, ctx);
                            content.controls(ctx, circle_confirm, true);
                        },
                    );
                    let jobs = ctx.tessellate(frame.shapes, frame.pixels_per_point);
                    renderer.update_overlay_textures(
                        &mut overlay,
                        &mut Vec::new(),
                        frame.textures_delta,
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
            captures.push(std::fs::read(path).unwrap());
            assert!(
                state(&content) == before,
                "{name}: preference changed original menu state"
            );
        }
        assert!(captures[0] != captures[1], "{name}: prompt did not change");
        assert!(
            captures[0] == captures[2],
            "{name}: original prompts did not restore"
        );
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU2_HUD_REVIEW and GPU; paused text toggles, disc calligraphy, numeric menus and original restoration"]
fn native_hd_menu_text_preserves_state_and_restores_original_artwork() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU2_HUD_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let stock = progress::Stock {
        items: std::array::from_fn(|i| Some([0, 9, 10, 99][i % 4])),
    };
    let mut equipment = native::equipment::Selection::load(&disc, 0, 2, stock).unwrap();
    equipment.state.fade = 0;
    equipment.state.description = 3;
    let mut summary = progress::Summary {
        time_tenths: 1234,
        detections: 17,
        effort: 987,
        ..Default::default()
    };
    summary.kills.stealth_kills = 12;
    summary.kills.combat_kills = 3;
    summary.kills.innocents = 2;
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
    let mut intro = native::film::Introduction::load(&disc, 2).unwrap();
    intro.state.frame = Frame {
        columns: vec![(0, -100, 128), (7, -40, 96), (15, 80, 128)],
        ..Default::default()
    };
    let mut prologue = native::film::Prologue::load(&disc).unwrap();
    prologue.state.frame = Frame {
        columns: vec![(0, -100, 128), (5, -40, 96), (15, 20, 64), (19, 80, 128)],
        ..Default::default()
    };
    let cases = [
        ("equipment", Content::Equipment(Box::new(equipment))),
        ("results", Content::Results(Box::new(results))),
        ("introduction", Content::Introduction(Box::new(intro))),
        ("prologue", Content::Prologue(Box::new(prologue))),
    ];
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let size = [1280, 720];
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        size,
        &Default::default(),
    ))
    .unwrap();
    let mut settings = Settings::default();
    settings.graphics.asset_pack = false;
    renderer.configure(&settings.graphics);
    let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Native HD menu text"),
        size: wgpu::Extent3d {
            width: size[0],
            height: size[1],
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format: renderer.format(),
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    let ctx = egui::Context::default();
    crate::fonts::install(&ctx);
    let mut overlay = egui_wgpu::Renderer::new(&renderer.device, renderer.format(), None, 1, false);
    crate::frontend::gpu::register(&mut overlay, renderer.format());
    let mut free = Vec::new();
    let view = target.create_view(&Default::default());
    for (name, content) in cases {
        let before = state(&content);
        let mut drawing = ContentDrawing::new(&content, &settings.graphics, &mut renderer).unwrap();
        let mut captures = Vec::new();
        for (step, sharp_text) in [false, true, false].into_iter().enumerate() {
            settings.sharp_text = sharp_text;
            drawing
                .update(&content, &mut renderer, &Default::default(), &settings)
                .unwrap();
            drawing.draw(&content, &view, &mut renderer).unwrap();
            let frame = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(size[0] as f32, size[1] as f32),
                    )),
                    ..Default::default()
                },
                |ctx| drawing.paint(&content, ctx),
            );
            let jobs = ctx.tessellate(frame.shapes, frame.pixels_per_point);
            renderer.update_overlay_textures(&mut overlay, &mut free, frame.textures_delta);
            renderer.draw_overlay(
                &view,
                &mut overlay,
                &jobs,
                &egui_wgpu::ScreenDescriptor {
                    size_in_pixels: size,
                    pixels_per_point: 1.0,
                },
            );
            let path = output.join(format!("{name}-{step}-hd-{sharp_text}.png"));
            crate::capture::read_texture(&renderer, &target, &path).unwrap();
            captures.push(std::fs::read(path).unwrap());
            if sharp_text {
                let screenshot = output.join(format!("{name}-screenshot.png"));
                let format = renderer.format();
                crate::capture::render_to_png(
                    &mut renderer,
                    size,
                    format,
                    &screenshot,
                    |renderer, view| {
                        drawing.draw(&content, view, renderer)?;
                        renderer.draw_overlay(
                            view,
                            &mut overlay,
                            &jobs,
                            &egui_wgpu::ScreenDescriptor {
                                size_in_pixels: size,
                                pixels_per_point: frame.pixels_per_point,
                            },
                        );
                        Ok(())
                    },
                )
                .unwrap();
                assert_eq!(
                    std::fs::read(screenshot).unwrap(),
                    *captures.last().unwrap(),
                    "{name}: screenshot lost the native menu/HD text"
                );
            }
            assert_eq!(state(&content), before, "{name}");
        }
        assert_ne!(
            captures[0], captures[1],
            "{name}: HD made no visible change"
        );
        assert_eq!(captures[0], captures[2], "{name}: Original did not restore");
    }
}

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU2_HUD_REVIEW and GPU; screenshots preserve native cameras/HUD while fog and haze restore exactly"]
fn native_screenshots_preserve_the_frame_and_fog_controls() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU2_HUD_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = native::Session::load(&disc, 9, 1).unwrap();
    for _ in 0..60 {
        session.step(0).unwrap();
    }
    let before = postcard::to_allocvec(&session.world).unwrap();
    let vram = session.assets.sprites.vram.clone();
    let content = Content::Gameplay(Box::new(session));
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let size = [800, 450];
    for format in [
        wgpu::TextureFormat::Rgba8Unorm,
        wgpu::TextureFormat::Bgra8Unorm,
    ] {
        let mut renderer =
            pollster::block_on(Renderer::new(&adapter, format, size, &vram)).unwrap();
        let mut settings = Settings {
            sharp_text: true,
            ..Default::default()
        };
        settings.graphics.asset_pack = false;
        settings.graphics.post.enabled = true;
        settings.graphics.fog_distance = 50;
        let mut drawing = ContentDrawing::new(&content, &settings.graphics, &mut renderer).unwrap();
        let ctx = egui::Context::default();
        crate::fonts::install(&ctx);
        let mut overlay = egui_wgpu::Renderer::new(&renderer.device, format, None, 1, false);
        let mut free = Vec::new();
        let mut images = Vec::new();
        for mode in [
            "base",
            "tint",
            "haze",
            "photo",
            "camera",
            "restored",
            "original",
            "tint",
            "original-restored",
        ] {
            settings.graphics.post.enabled = !mode.starts_with("original");
            settings.graphics.post.fog_tint = u16::from(mode == "tint");
            settings.graphics.post.haze = if mode == "haze" { 80 } else { 0 };
            let mut debug = crate::debug::Debug::default();
            debug.photo_mode = mode == "photo";
            if mode == "camera" {
                let Content::Gameplay(session) = &content else {
                    unreachable!()
                };
                let mut camera =
                    crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
                camera.position.x += 1200.0;
                debug.camera = Some(camera);
            }
            renderer.configure(&native::preferences::graphics(&settings));
            drawing
                .update(&content, &mut renderer, &debug, &settings)
                .unwrap();
            let frame = ctx.run(
                egui::RawInput {
                    screen_rect: Some(egui::Rect::from_min_size(
                        egui::Pos2::ZERO,
                        egui::vec2(size[0] as f32, size[1] as f32),
                    )),
                    ..Default::default()
                },
                |ctx| drawing.paint(&content, ctx),
            );
            let jobs = ctx.tessellate(frame.shapes, frame.pixels_per_point);
            renderer.update_overlay_textures(&mut overlay, &mut free, frame.textures_delta);
            let path = output.join(format!("native-{format:?}-{mode}.png"));
            crate::capture::render_to_png(
                &mut renderer,
                size,
                format,
                &path,
                |renderer, target| {
                    drawing.draw(&content, target, renderer)?;
                    renderer.draw_overlay(
                        target,
                        &mut overlay,
                        &jobs,
                        &egui_wgpu::ScreenDescriptor {
                            size_in_pixels: size,
                            pixels_per_point: frame.pixels_per_point,
                        },
                    );
                    Ok(())
                },
            )
            .unwrap();
            assert!(renderer.linear_fog.is_none());
            images.push(crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap());
            let Content::Gameplay(session) = &content else {
                unreachable!()
            };
            assert!(postcard::to_allocvec(&session.world).unwrap() == before);
        }
        for index in 1..5 {
            assert!(
                images[0].rgba != images[index].rgba,
                "{format:?} mode {index} made no visible change"
            );
        }
        assert!(
            images[0].rgba == images[5].rgba,
            "{format:?}: visual controls did not restore"
        );
        assert!(
            images[6].rgba == images[8].rgba,
            "{format:?}: Original did not restore"
        );
    }
}
