use super::*;
use crate::frontend::{
    draw,
    raster::{Canvas, Sprite},
};
use tenchu_assets::{frontend::Image, texture::Vram};

struct Capture {
    gpu: crate::renderer::Renderer,
    overlay: egui_wgpu::Renderer,
    target: wgpu::Texture,
}

impl Capture {
    fn new(size: [u32; 2]) -> Self {
        let instance = wgpu::Instance::new(&wgpu::InstanceDescriptor::from_env_or_default());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let gpu = pollster::block_on(crate::renderer::Renderer::new(
            &adapter,
            wgpu::TextureFormat::Rgba8Unorm,
            size,
            &Vram::default(),
        ))
        .unwrap();
        let mut overlay =
            egui_wgpu::Renderer::new(&gpu.device, wgpu::TextureFormat::Rgba8Unorm, None, 1, false);
        register(&mut overlay, wgpu::TextureFormat::Rgba8Unorm);
        let target = gpu.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Frontend review"),
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
        Self {
            gpu,
            overlay,
            target,
        }
    }

    fn draw(&mut self, frame: Frame) -> Vec<u8> {
        let frame = Arc::new(frame);
        let size = [self.target.width(), self.target.height()];
        let rect =
            egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(size[0] as f32, size[1] as f32));
        let ctx = egui::Context::default();
        let output = ctx.run(
            egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ctx| {
                let painter = ctx.layer_painter(egui::LayerId::background());
                painter.rect_filled(rect, 0.0, egui::Color32::BLACK);
                painter.add(egui_wgpu::Callback::new_paint_callback(
                    rect,
                    Paint::Frontend(frame.clone()),
                ));
            },
        );
        for (id, delta) in &output.textures_delta.set {
            self.overlay
                .update_texture(&self.gpu.device, &self.gpu.queue, *id, delta);
        }
        let jobs = ctx.tessellate(output.shapes, output.pixels_per_point);
        self.gpu.draw_overlay(
            &self.target.create_view(&Default::default()),
            &mut self.overlay,
            &jobs,
            &egui_wgpu::ScreenDescriptor {
                size_in_pixels: size,
                pixels_per_point: output.pixels_per_point,
            },
        );
        let mut pixels = Vec::new();
        crate::capture::Readback::new(&self.gpu, &self.target)
            .read(&self.gpu, &self.target, |bytes, stride| {
                for row in bytes.chunks_exact(stride) {
                    pixels.extend_from_slice(&row[..size[0] as usize * 4]);
                }
                Ok(())
            })
            .unwrap();
        pixels
    }

    fn review(
        &mut self,
        frontend: &mut crate::frontend::Frontend,
        campaign: &crate::campaign::Campaign,
        name: &str,
    ) {
        frontend.canvas.sharp_text = true;
        frontend.canvas.frame = Some(Frame::new(frontend.canvas.size, frontend.artwork.clone()));
        frontend.paint(campaign);
        self.draw(frontend.canvas.frame.take().unwrap());
        crate::fonts::capture(&self.gpu, &self.target, &frontend.canvas.text);
        if let Some(directory) = std::env::var_os("TENCHU_FRONTEND_CAPTURE") {
            let directory = std::path::Path::new(&directory);
            std::fs::create_dir_all(directory).unwrap();
            crate::capture::read_texture(
                &self.gpu,
                &self.target,
                &directory.join(format!("{name}.png")),
            )
            .unwrap();
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC, the complete HD pack and a GPU; verifies localized mission-screen coverage and layout"]
fn hd_mission_screens_cover_all_languages_and_briefings() {
    use crate::{
        campaign::{Action, Campaign, Screen},
        frontend::{Frontend, Page},
    };
    use tenchu_assets::{game::GameAssets, script::Language};
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, Default::default(), None).unwrap();
    let mut frontend =
        Frontend::new(&assets, disc.as_ref(), &campaign, Default::default()).unwrap();
    let state = crate::session::tests::state(&campaign.game);
    let mut capture = Capture::new([1280, 960]);
    let covered = |frontend: &Frontend, image: &Image| {
        assert!(
            frontend
                .artwork
                .contains_key(&draw::identity(&draw::pixels(image))),
            "Missing mission artwork"
        );
    };
    for language in [
        Language::English,
        Language::Japanese,
        Language::French,
        Language::Italian,
    ] {
        campaign.options.language = Some(language);
        campaign.options.mission = 1;
        campaign.progress.stock[0][1..9].fill(20);
        campaign.inventory();
        frontend.prepare(&campaign, &assets, false).unwrap();
        frontend.ticks = 120;
        for item in [1, 1, 3] {
            campaign.cursor = campaign
                .shop
                .order
                .iter()
                .position(|&id| id == item)
                .unwrap();
            frontend
                .action(Action::Confirm, &mut campaign, &assets)
                .unwrap();
        }
        covered(&frontend, &frontend.mission_art.shop);
        for image in &frontend.mission_art.help {
            covered(&frontend, image);
        }
        capture.review(&mut frontend, &campaign, &format!("{language:?}-shop"));
        for mission in 1..=10 {
            campaign.options.mission = mission;
            frontend.prepare(&campaign, &assets, true).unwrap();
            let briefing = frontend.briefing.as_ref().unwrap();
            covered(&frontend, &briefing.background);
            covered(&frontend, &briefing.text);
            for _ in 0..60 {
                frontend.advance_briefing();
            }
            frontend.narration_ready(None);
            let briefing = frontend.briefing.as_ref().unwrap();
            let duration = briefing.assets.narration.end - briefing.assets.narration.start;
            for _ in 0..duration * 60 / 300 {
                frontend.advance_briefing();
            }
            assert_eq!(frontend.page, Page::Briefing);
            capture.review(
                &mut frontend,
                &campaign,
                &format!("{language:?}-briefing-{mission:02}"),
            );
        }
        let record = tenchu_core::game::score::Record {
            bosses: 1,
            enemies: 10,
            criticals: 9,
            murders: 1,
            clock: 4890,
            ..Default::default()
        };
        let score = record.score(campaign.game.mission.stage.id);
        campaign.screen = Screen::Results(tenchu_core::progress::Results {
            record,
            score,
            best: record,
            new_best: true,
            training_place: Some(0),
        });
        for mission in [0, 1, 10] {
            campaign.options.mission = mission;
            frontend.enter_results(&assets, &campaign).unwrap();
            frontend.ticks = 120;
            for image in [
                &frontend.mission_art.results,
                &frontend.mission_art.training,
                &frontend.result_background,
            ] {
                covered(&frontend, image);
            }
            capture.review(
                &mut frontend,
                &campaign,
                &format!("{language:?}-results-{mission:02}"),
            );
        }
    }
    assert_eq!(state, crate::session::tests::state(&campaign.game));
}

fn image(words: &[u16], size: [u16; 2]) -> Image {
    let mut vram = Vram::default();
    for (y, row) in words.chunks_exact(usize::from(size[0])).enumerate() {
        vram.words[y * 1024..y * 1024 + row.len()].copy_from_slice(row);
    }
    Image::from_vram(
        &vram,
        tenchu_assets::texture::Image {
            page: 0x100,
            palette: 0,
            uv: [0; 2],
            size,
        },
    )
}

#[test]
#[ignore = "requires a GPU; compares every PS1 sprite blend and fade against the existing rasterizer"]
fn gpu_preserves_sprite_blends_and_cutouts() {
    let background = image(&[0x56b5], [1, 1]);
    let sprite = image(
        &[0, 0x7fff, 0x821f, 0x83e0, 0x7c00, 0x801f, 0x03ff, 0xfc00],
        [4, 2],
    );
    let mut original = Canvas::new(320);
    let mut modern = Canvas::new(320);
    modern.frame = Some(Frame::new(modern.size, Arc::default()));
    for canvas in [&mut original, &mut modern] {
        canvas.clear();
        canvas.sprite(Sprite::new(&background, 0.0, 0.0).scale(320.0, 240.0));
        for (i, blend) in [
            Blend::Opaque,
            Blend::Average,
            Blend::Add,
            Blend::Subtract,
            Blend::Quarter,
        ]
        .into_iter()
        .enumerate()
        {
            canvas.sprite(
                Sprite::new(&sprite, -112.0 + i as f32 * 56.0, 0.0)
                    .scale(if i % 2 == 0 { 10.0 } else { -10.0 }, 20.0)
                    .blend(blend),
            );
        }
        canvas.fade(11);
    }
    let mut capture = Capture::new([320, 240]);
    let rendered = capture.draw(modern.frame.take().unwrap());
    let error = rendered
        .iter()
        .zip(&original.pixels)
        .map(|(&a, &b)| a.abs_diff(b))
        .max()
        .unwrap();
    assert!(error <= 2, "GPU blend differs by {error}");
}

#[test]
#[ignore = "requires a GPU; verifies sub-texel replacement detail survives 4K output and original fallback"]
fn replacement_detail_is_not_reduced_to_the_240_line_canvas() {
    let original = image(&[0x7fff], [1, 1]);
    let replacement = crate::visuals::pixels::Image {
        size: [256, 256],
        rgba: (0..256 * 256)
            .flat_map(|i| {
                if i % 256 % 4 < 2 {
                    [240, 20, 30, 255]
                } else {
                    [20, 30, 240, 255]
                }
            })
            .collect(),
    };
    let id = draw::identity(&draw::pixels(&original));
    let mut capture = Capture::new([3840, 2160]);
    for enabled in [true, false] {
        let artwork = if enabled {
            [(id.clone(), Arc::new(replacement.clone()))].into()
        } else {
            BTreeMap::new()
        };
        let mut canvas = Canvas::new(320);
        canvas.frame = Some(Frame::new(canvas.size, Arc::new(artwork)));
        canvas.sprite(Sprite::new(&original, 0.0, 0.0).scale(16.0, 16.0));
        let pixels = capture.draw(canvas.frame.take().unwrap());
        let line = &pixels[(1080 * 3840 + 1830) * 4..(1080 * 3840 + 2010) * 4];
        if enabled {
            let changes = line
                .chunks_exact(4)
                .map(|p| p[0] > p[2])
                .collect::<Vec<_>>()
                .windows(2)
                .filter(|w| w[0] != w[1])
                .count();
            assert!(changes > 90, "Only {changes} stripe changes survived");
        } else {
            assert!(line.chunks_exact(4).all(|p| p == [255; 4]));
        }
    }
    let original = image(&[0x7fff; 4], [2, 2]);
    let id = draw::identity(&draw::pixels(&original));
    let atlas = crate::visuals::pixels::Image {
        size: [64, 64],
        rgba: (0..64 * 64)
            .flat_map(|i| {
                if i / 64 < 32 {
                    [240, 20, 0, 255]
                } else {
                    [0, 20, 240, 255]
                }
            })
            .collect(),
    };
    let mut canvas = Canvas::new(320);
    canvas.frame = Some(Frame::new(
        canvas.size,
        Arc::new([(id, Arc::new(atlas))].into()),
    ));
    canvas.sprite(
        Sprite::new(&original, 0.0, 0.0)
            .row(0, 2)
            .scale(100.0, 100.0),
    );
    let pixels = capture.draw(canvas.frame.take().unwrap());
    let at = (1529 * 3840 + 1920) * 4;
    assert_eq!(
        &pixels[at..at + 4],
        &[240, 20, 0, 255],
        "Adjacent atlas row leaked into a filtered sprite"
    );
}

#[test]
#[ignore = "requires TENCHU_DISC and a GPU; reviews the original frontend through both presentation paths"]
fn original_pages_keep_their_layout_and_gameplay_state_with_gpu_presentation() {
    use crate::{
        campaign::{Campaign, Screen},
        frontend::{Frontend, Page, Preferences},
    };
    use tenchu_assets::{game::GameAssets, script::Language};
    let disc = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, Default::default(), None).unwrap();
    let record = tenchu_core::game::score::Record {
        bosses: 1,
        enemies: 18,
        criticals: 8,
        murders: 12,
        found: 3,
        friends: 2,
        clock: 107_970,
    };
    campaign.progress.records[0].fill([record; 3]);
    campaign.screen = Screen::Results(tenchu_core::progress::Results {
        record,
        score: record.score(campaign.game.mission.stage.id),
        best: record,
        new_best: false,
        training_place: None,
    });
    let progress = campaign.progress.clone();
    let mut frontend =
        Frontend::new(&assets, disc.as_ref(), &campaign, Preferences::default()).unwrap();
    let state = crate::session::tests::state(&campaign.game);
    let mut captures = [Capture::new([320, 240]), Capture::new([640, 240])];
    let directory = std::env::var_os("TENCHU_FRONTEND_CAPTURE");
    let mut large = directory.as_ref().map(|_| Capture::new([960, 720]));
    for language in [
        Language::English,
        Language::Japanese,
        Language::French,
        Language::Italian,
    ] {
        frontend.language(&assets, language).unwrap();
        if crate::visuals::pack::configured_root().is_some() {
            frontend.reload_artwork().unwrap();
        }
        for (page, mission) in [
            Page::Title,
            Page::Character,
            Page::Missions,
            Page::Options,
            Page::Controls,
            Page::Volume,
            Page::SoundTest,
            Page::Records,
            Page::Results,
            Page::Card,
            Page::Choice(1),
            Page::Choice(2),
            Page::Choice(6),
            Page::Choice(8),
            Page::Choice(9),
        ]
        .into_iter()
        .flat_map(|page| {
            (1..=if page == Page::Missions { 10 } else { 1 }).map(move |mission| (page, mission))
        }) {
            frontend.title.activate();
            frontend.set_page(page);
            frontend.mission = mission;
            frontend.cursor = frontend.tables.cursor_targets[mission - 1].map(f32::from);
            let name = if page == Page::Missions {
                format!("{page:?}-{mission:02}")
            } else {
                format!("{page:?}")
            };
            frontend.ticks = 120;
            frontend.canvas.sharp_text = false;
            frontend.canvas.frame = None;
            frontend.paint(&campaign);
            let original = frontend.canvas.pixels.clone();
            frontend.canvas.frame = Some(Frame::new(frontend.canvas.size, Arc::default()));
            frontend.paint(&campaign);
            let capture = &mut captures[usize::from(page == Page::Missions)];
            let pixels = capture.draw(frontend.canvas.frame.take().unwrap());
            let error = pixels
                .iter()
                .zip(&original)
                .map(|(&a, &b)| f64::from(a.abs_diff(b)))
                .sum::<f64>()
                / pixels.len() as f64;
            assert!(error < 2.0, "{language:?} {page:?} mean error {error}");
            if let (Some(directory), Some(large)) = (&directory, &mut large) {
                let directory = std::path::Path::new(directory);
                std::fs::create_dir_all(directory).unwrap();
                frontend.canvas.frame = Some(Frame::new(frontend.canvas.size, Arc::default()));
                frontend.paint(&campaign);
                large.draw(frontend.canvas.frame.take().unwrap());
                crate::capture::read_texture(
                    &large.gpu,
                    &large.target,
                    &directory.join(format!("{language:?}-{name}-gpu.png")),
                )
                .unwrap();
                frontend.canvas.sharp_text = true;
                frontend.canvas.frame = Some(Frame::new(frontend.canvas.size, Arc::default()));
                frontend.paint(&campaign);
                large.draw(frontend.canvas.frame.take().unwrap());
                crate::fonts::capture(&large.gpu, &large.target, &frontend.canvas.text);
                crate::capture::read_texture(
                    &large.gpu,
                    &large.target,
                    &directory.join(format!("{language:?}-{name}-sharp.png")),
                )
                .unwrap();
                if !frontend.artwork.is_empty() {
                    frontend.canvas.frame =
                        Some(Frame::new(frontend.canvas.size, frontend.artwork.clone()));
                    frontend.paint(&campaign);
                    large.draw(frontend.canvas.frame.take().unwrap());
                    crate::fonts::capture(&large.gpu, &large.target, &frontend.canvas.text);
                    crate::capture::read_texture(
                        &large.gpu,
                        &large.target,
                        &directory.join(format!("{language:?}-{name}-art.png")),
                    )
                    .unwrap();
                }
            }
        }
    }
    assert_eq!(state, crate::session::tests::state(&campaign.game));
    assert_eq!(progress, campaign.progress);
}
