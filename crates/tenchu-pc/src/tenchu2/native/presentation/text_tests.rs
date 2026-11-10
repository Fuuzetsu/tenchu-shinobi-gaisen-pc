use super::{Content, ContentDrawing};
use crate::{renderer::Renderer, settings::Settings, tenchu2::native};
use tenchu_core::tenchu2::{introduction::Frame, progress};

fn state(content: &Content) -> Vec<u8> {
    match content {
        Content::Equipment(value) => postcard::to_allocvec(&value.state),
        Content::Results(value) => postcard::to_allocvec(&value.values),
        Content::Introduction(value) => postcard::to_allocvec(&value.state),
        Content::Prologue(value) => postcard::to_allocvec(&value.state),
        _ => unreachable!(),
    }
    .unwrap()
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
            assert_eq!(state(&content), before, "{name}");
        }
        assert_ne!(
            captures[0], captures[1],
            "{name}: HD made no visible change"
        );
        assert_eq!(captures[0], captures[2], "{name}: Original did not restore");
    }
}
