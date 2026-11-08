use super::*;
use crate::{
    renderer::Renderer,
    settings::Graphics,
    tenchu2::native::{Session, campaign::Content},
};
use tenchu_core::tenchu2::progress::{Record, Summary};

#[test]
#[ignore = "requires TENCHU2_DISC, TENCHU_MODEL_REVIEW and a GPU; disc results glyphs, rank rows and a frozen scene survive renderer replacement"]
fn results_keep_the_final_scene_and_disc_artwork() {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let mut session = Session::load(&disc, 9, 17).unwrap();
    for _ in 0..30 {
        session.step(0).unwrap();
    }
    let graphics = Graphics {
        asset_pack: false,
        fog: false,
        ..Default::default()
    };
    let content = Content::Gameplay(Box::new(session));
    let Content::Gameplay(session) = &content else {
        unreachable!()
    };
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        wgpu::TextureFormat::Rgba8Unorm,
        [960, 720],
        &session.assets.sprites.vram,
    ))
    .unwrap();
    renderer.configure(&graphics);
    let mut drawing =
        super::super::ContentDrawing::new(&content, &graphics, &mut renderer).unwrap();
    let mut summary = Summary {
        time_tenths: 1234,
        detections: 17,
        effort: 987,
        ..Default::default()
    };
    summary.kills.stealth_kills = 12;
    summary.kills.combat_kills = 3;
    summary.kills.innocents = 2;
    let record = summary.rank(13);
    let mut results = crate::tenchu2::native::results::Results::load(
        &disc,
        tenchu_core::tenchu2::progress::Results {
            summary,
            record,
            best: Record::default(),
            new_best: true,
            unlocked_item: None,
        },
    )
    .unwrap();
    assert!(results.resources.numbers.pixels.contains(&0x8000));
    assert!(
        results
            .resources
            .numbers
            .pixels
            .iter()
            .all(|&p| p == 0 || p & 0x8000 != 0)
    );
    let size = [960, 720];
    let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("JP results review"),
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
    let mut overlay = egui_wgpu::Renderer::new(&renderer.device, renderer.format(), None, 1, false);
    crate::frontend::gpu::register(&mut overlay, renderer.format());
    let mut free = Vec::new();
    let mut backdrop = None;
    for rank in 0..7 {
        results.values.record.rank = rank;
        let content = Content::Results(Box::new(results));
        drawing
            .replace(&content, &Default::default(), &graphics, &mut renderer)
            .unwrap();
        let super::super::ContentDrawing::Results(result) = &drawing else {
            unreachable!()
        };
        let image = result.backdrop.as_ref().unwrap();
        assert!(image.pixels.iter().any(|&p| p != 0));
        assert!(
            image
                .pixels
                .iter()
                .all(|&p| p & 31 == p >> 5 & 31 && p & 31 == p >> 10)
        );
        if let Some(previous) = &backdrop {
            assert!(Arc::ptr_eq(previous, image));
        }
        backdrop = Some(image.clone());
        let target_view = target.create_view(&Default::default());
        drawing.draw(&content, &target_view, &mut renderer).unwrap();
        let output_frame = ctx.run(
            egui::RawInput {
                screen_rect: Some(egui::Rect::from_min_size(
                    egui::Pos2::ZERO,
                    egui::vec2(960.0, 720.0),
                )),
                ..Default::default()
            },
            |ctx| drawing.paint(&content, ctx),
        );
        let jobs = ctx.tessellate(output_frame.shapes, output_frame.pixels_per_point);
        renderer.update_overlay_textures(&mut overlay, &mut free, output_frame.textures_delta);
        renderer.draw_overlay(
            &target_view,
            &mut overlay,
            &jobs,
            &egui_wgpu::ScreenDescriptor {
                size_in_pixels: size,
                pixels_per_point: 1.0,
            },
        );
        crate::capture::read_texture(
            &renderer,
            &target,
            &output.join(format!("jp-results-rank-{rank}.png")),
        )
        .unwrap();
        let Content::Results(value) = content else {
            unreachable!()
        };
        results = *value;
    }
}
