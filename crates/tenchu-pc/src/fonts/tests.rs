use super::*;

#[test]
fn outline_font_contains_localized_letters_and_keeps_layout_within_the_view() {
    let ctx = egui::Context::default();
    install(&ctx);
    for size in [[1280.0, 720.0], [3840.0, 2160.0]] {
        let rect = Rect::from_min_size(Pos2::ZERO, Vec2::from(size));
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ctx| {
                let samples = [
                    "MISSION EQUIPMENT 0123456789",
                    "任務開始　忍術指南　各種設定",
                    "Difficulté, français : À É È Ç Œ œ",
                    "Missione completata, città, più, è",
                ];
                ctx.fonts(|fonts| {
                    for sample in samples {
                        assert!(
                            fonts.has_glyphs(&FontId::new(12.0, family()), sample),
                            "Missing glyph in {sample}"
                        );
                    }
                });
                let text: Vec<_> = samples
                    .iter()
                    .enumerate()
                    .map(|(i, sample)| Text {
                        position: egui::pos2(-140.0, -80.0 + i as f32 * 40.0),
                        text: sample.to_string(),
                        color: Color32::WHITE,
                        size: 14.0,
                        width: 280.0,
                        centered: false,
                    })
                    .collect();
                paint(ctx, rect, &text);
            },
        );
        assert!(!output.shapes.is_empty());
        for shape in output.shapes {
            assert!(rect.contains_rect(shape.shape.visual_bounding_rect()));
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC and a GPU; compares localized text replacement with original world geometry"]
fn localized_captions_change_only_presentation_and_render_at_720p_and_4k() {
    use tenchu_assets::{game::GameAssets, script::Language};
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let mut samples = Vec::new();
    for (row, language) in [
        Language::English,
        Language::Japanese,
        Language::French,
        Language::Italian,
    ]
    .into_iter()
    .enumerate()
    {
        let mut game = crate::session::Options {
            language: Some(language),
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        game.step(0).unwrap();
        let player = game.roster.get_mut(game.player_id);
        player.position = tenchu_core::math::Position::new(1000, 0, 5000);
        game.scene
            .update(player.root_model, player.root(&game.math));
        for _ in 0..3000 {
            game.step(0).unwrap();
            if !game.cinematic.displayed_caption.is_empty() && caption_sprites(&game) != 0 {
                break;
            }
        }
        assert!(!game.cinematic.displayed_caption.is_empty());
        let state = crate::session::tests::state(&game);
        let original =
            crate::render_effects::build_presented(&game, true, None, true, None, None, false);
        let sharp =
            crate::render_effects::build_presented(&game, true, None, true, None, None, true);
        let world = |geometry: &crate::render_effects::Geometry| -> Vec<u8> {
            let vertices: Vec<_> = geometry
                .vertices
                .iter()
                .copied()
                .filter(|v| v.flags & crate::render_mesh::UI == 0)
                .collect();
            bytemuck::cast_slice(&vertices).to_vec()
        };
        assert_eq!(world(&original), world(&sharp));
        assert!(sharp.vertices.len() < original.vertices.len());
        let mut text = game_text(&game, false);
        assert!(!text.is_empty());
        for line in &mut text {
            line.position.y = -80.0 + row as f32 * 45.0;
        }
        samples.extend(text);
        assert_eq!(state, crate::session::tests::state(&game));
    }
    let Some(directory) = std::env::var_os("TENCHU_FONT_CAPTURE_DIR") else {
        return;
    };
    std::fs::create_dir_all(&directory).unwrap();
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
    let campaign = crate::campaign::Campaign::new(&assets, Default::default(), None).unwrap();
    for size in [[1280, 720], [3840, 2160]] {
        let mut renderer = pollster::block_on(crate::renderer::Renderer::new(
            &adapter,
            wgpu::TextureFormat::Rgba8Unorm,
            size,
            &Default::default(),
        ))
        .unwrap();
        let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Localized outline text"),
            size: wgpu::Extent3d {
                width: size[0],
                height: size[1],
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba8Unorm,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        capture(&renderer, &target, &samples);
        crate::capture::read_texture(
            &renderer,
            &target,
            &std::path::Path::new(&directory).join(format!("captions-{}p.png", size[1])),
        )
        .unwrap();
        let mut labels = Vec::new();
        let panel = crate::ui::build(&campaign, false, None, Some(&mut labels));
        let ctx = egui::Context::default();
        install(&ctx);
        let rect = Rect::from_min_size(Pos2::ZERO, egui::vec2(size[0] as f32, size[1] as f32));
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ctx| paint(ctx, rect, &labels),
        );
        for shape in output.shapes {
            assert!(rect.contains_rect(shape.shape.visual_bounding_rect()));
        }
        let panel = renderer.upload_effects(panel);
        renderer
            .draw_lit(
                &target.create_view(&Default::default()),
                glam::Mat4::IDENTITY,
                &[],
                Some(&panel),
                &[],
                &[],
            )
            .unwrap();
        capture(&renderer, &target, &labels);
        crate::capture::read_texture(
            &renderer,
            &target,
            &std::path::Path::new(&directory).join(format!("launcher-{}p.png", size[1])),
        )
        .unwrap();
    }
}
