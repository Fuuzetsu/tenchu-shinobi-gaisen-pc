use super::*;

#[test]
fn numeric_runs_keep_last_digit_center_and_font_size() {
    let ctx = egui::Context::default();
    install(&ctx);
    let _ = ctx.run(Default::default(), |ctx| {
        for advance in [5.0, 6.0, 8.0, 10.0] {
            for number in ["0", "9", "10", "67", "100", "987", "-110", "+240"] {
                for reverse in [false, true] {
                    let mut digits: Vec<_> = number
                        .chars()
                        .enumerate()
                        .map(|(i, digit)| Text {
                            position: Pos2::new(
                                100.0 - (number.len() - i - 1) as f32 * advance,
                                20.0,
                            ),
                            text: digit.to_string(),
                            color: Color32::WHITE,
                            size: 6.75,
                            width: 4.5,
                            align: Align::Center,
                        })
                        .collect();
                    if reverse {
                        digits.reverse();
                    }
                    let texts = compact_numbers(digits, advance);
                    assert_eq!(texts.len(), 1);
                    let text = &texts[0];
                    assert_eq!(text.text, number);
                    assert_eq!(text.position, Pos2::new(100.0, 20.0));
                    assert_eq!(text.align, Align::LastDigit);
                    for scale in [1.5, 3.0] {
                        let galley = ctx.fonts(|fonts| {
                            fonts.layout_no_wrap(
                                text.text.clone(),
                                id(text.size * scale),
                                text.color,
                            )
                        });
                        assert!(
                            galley.size().x <= text.width * scale,
                            "{number}: shrank the font"
                        );
                        if number.len() > 1 {
                            assert!(
                                galley.size().x
                                    < (advance * (number.len() - 1) as f32 + 4.5) * scale
                            );
                        }
                    }
                }
            }
        }
    });
}

#[test]
fn numeric_paint_keeps_the_last_glyph_center_with_padded_source_cells() {
    let ctx = egui::Context::default();
    install(&ctx);
    for (size, width, advance) in [(6.75, 4.5, 6.0), (12.8, 16.0, 8.0)] {
        for number in ["2", "03", "100", "-110"] {
            for [w, h] in [[640.0, 360.0], [1280.0, 720.0]] {
                let digit = |character: char, x| Text {
                    position: Pos2::new(x, 0.0),
                    text: character.to_string(),
                    color: Color32::WHITE,
                    size,
                    width,
                    align: Align::Center,
                };
                let mut text = vec![digit(number.chars().next_back().unwrap(), 100.0)];
                text.extend(compact_numbers(
                    number
                        .chars()
                        .enumerate()
                        .map(|(i, ch)| digit(ch, 100.0 - (number.len() - i - 1) as f32 * advance))
                        .collect(),
                    advance,
                ));
                let rect = Rect::from_min_size(Pos2::ZERO, Vec2::new(w, h));
                let output = ctx.run(
                    egui::RawInput {
                        screen_rect: Some(rect),
                        ..Default::default()
                    },
                    |ctx| {
                        paint(ctx, rect, &text);
                    },
                );
                let centers: Vec<_> = output
                    .shapes
                    .iter()
                    .filter_map(|shape| {
                        let egui::Shape::Text(text) = &shape.shape else {
                            return None;
                        };
                        if text.override_text_color.is_some() {
                            return None;
                        }
                        let last = text.galley.rows.last()?.glyphs.last()?;
                        Some(text.pos.x + last.pos.x + last.advance_width / 2.0)
                    })
                    .collect();
                assert_eq!(centers.len(), 2);
                assert!(
                    (centers[0] - centers[1]).abs() <= 0.5,
                    "{number}, cell{width}, viewport{w}: {centers:?}"
                );
            }
        }
    }
}

#[test]
fn numeric_runs_preserve_animation_style_and_separate_readouts() {
    let digit = |x| Text {
        position: Pos2::new(x, 20.0),
        text: "1".into(),
        color: Color32::WHITE,
        size: 6.75,
        width: 4.5,
        align: Align::Center,
    };
    for boundary in 0..7 {
        let mut second = digit(5.0);
        match boundary {
            0 => second.position.x += 1.0,
            1 => second.position.y += 1.0,
            2 => second.color = Color32::RED,
            3 => second.size *= 1.25,
            4 => second.width *= 1.25,
            5 => second.align = Align::Left,
            _ => second.text = ":".into(),
        }
        assert_eq!(compact_numbers(vec![digit(0.0), second], 5.0).len(), 2);
    }
    let reversed = compact_numbers(vec![digit(0.0), digit(5.0), digit(0.0)], 5.0);
    assert_eq!(
        reversed.len(),
        2,
        "a separate overlapping counter joined the run"
    );
}

#[test]
#[ignore = "requires TENCHU_DISC; isolated health/Ki/item glyphs retain source sprites and fixed HD anchors for both heroes"]
fn hud_numbers_group_source_digits_without_changing_the_game() {
    let assets = tenchu_assets::game::GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    for hero in 0..2 {
        let mut game = crate::session::Options {
            character: hero,
            ..Default::default()
        }
        .load(&assets)
        .unwrap();
        game.step(0).unwrap();
        for value in [1, 10, 100] {
            game.hud.sprites.clear();
            for [x, y] in [[-148, 105], [-100, 100], [140, 100]] {
                for (i, digit) in value.to_string().bytes().rev().enumerate() {
                    let mut image = game.mission.images[51];
                    image.size[0] = 4;
                    image.uv[0] = image.uv[0].wrapping_add((digit - b'0') * 4);
                    game.hud
                        .sprites
                        .push(Sprite::new(image, [x - i as i16 * 6, y]));
                }
            }
            let before = postcard::to_allocvec(&game).unwrap();
            let texts = game_text(&game, true, true, true, None);
            assert_eq!(texts.len(), 3);
            for (text, x) in texts.iter().zip([-148.0, -100.0, 140.0]) {
                assert_eq!(text.text, value.to_string());
                assert_eq!(text.position.x, x);
                assert_eq!(text.align, Align::LastDigit);
            }
            assert!(game_text(&game, true, true, false, None).is_empty());
            assert!(game_text(&game, false, true, true, None).is_empty());
            assert_eq!(postcard::to_allocvec(&game).unwrap(), before);
        }
    }
}

#[test]
#[ignore = "requires TENCHU_DISC; compares independent subtitles with four live localized scenes and preserves the source scene"]
fn independent_scene_subtitles_follow_localized_cues_without_changing_the_scene() {
    use std::collections::BTreeMap;
    use tenchu_assets::{game::GameAssets, script::Language};

    let disc = std::env::var_os("TENCHU_DISC").unwrap();
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let languages = [
        Language::English,
        Language::French,
        Language::Italian,
        Language::Japanese,
    ];
    let load = |language| {
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
        game
    };
    let mut references: [BTreeMap<i16, String>; 4] = Default::default();
    for language in languages {
        let mut game = load(language);
        for tick in 0..2000 {
            game.step(0).unwrap();
            if game.cinematic.frame {
                let expected = game_text(&game, false, true, true, None)
                    .into_iter()
                    .map(|text| text.text)
                    .collect::<Vec<_>>()
                    .join("\n");
                assert_eq!(
                    game.mission.scripts.subtitles[language as usize].text(
                        game.cinematic.sequence,
                        game.cinematic.elapsed.saturating_sub(1)
                    ),
                    expected,
                    "{language:?} tick {tick}, elapsed {}",
                    game.cinematic.elapsed
                );
                references[language as usize].insert(game.cinematic.elapsed, expected);
            }
            if tick > 0 && !game.cinematic.active {
                break;
            }
            assert!(tick < 1999, "unfinished {language:?} scene");
        }
    }
    assert_ne!(
        references[Language::Japanese as usize][&231],
        references[Language::Japanese as usize][&246]
    );
    assert_eq!(
        references[Language::English as usize][&231],
        references[Language::English as usize][&246]
    );

    let mut game = load(Language::Japanese);
    while game.tick < 2000 {
        game.step(0).unwrap();
        if game.cinematic.frame {
            for subtitle in languages {
                if let Some(expected) = references[subtitle as usize].get(&game.cinematic.elapsed) {
                    let text = game_text(&game, false, true, true, Some(subtitle))
                        .into_iter()
                        .map(|text| text.text)
                        .collect::<Vec<_>>()
                        .join("\n");
                    assert_eq!(
                        &text, expected,
                        "{subtitle:?} at {}",
                        game.cinematic.elapsed
                    );
                }
            }
        }
        if game.cinematic.elapsed == 340 && game.cinematic.tail == 0 {
            let state = crate::session::tests::state(&game);
            let geometry = |subtitle_language| {
                crate::render_effects::build_presented(
                    &game,
                    true,
                    None,
                    true,
                    None,
                    None,
                    crate::render_effects::Style {
                        subtitle_language,
                        ..Default::default()
                    },
                )
            };
            let original = geometry(None);
            let translated = geometry(Some(Language::English));
            let world = |geometry: &crate::render_effects::Geometry| {
                let vertices: Vec<_> = geometry
                    .vertices
                    .iter()
                    .copied()
                    .filter(|vertex| vertex.flags & crate::render_mesh::UI == 0)
                    .collect();
                bytemuck::cast_slice::<_, u8>(&vertices).to_vec()
            };
            assert_eq!(world(&original), world(&translated));
            assert!(translated.vertices.len() < original.vertices.len());
            assert!(game_text(&game, true, true, false, None).is_empty());
            assert!(!game_text(&game, true, true, false, Some(Language::English)).is_empty());
            if let Some(directory) = std::env::var_os("TENCHU_FONT_CAPTURE_DIR") {
                std::fs::create_dir_all(&directory).unwrap();
                for (name, subtitle_language) in
                    [("linked", None), ("english", Some(Language::English))]
                {
                    crate::capture::frame_with_settings(
                        &game,
                        &std::path::Path::new(&directory).join(format!("jp-scene-{name}.png")),
                        &assets,
                        &crate::settings::Settings {
                            subtitle_language,
                            ..Default::default()
                        },
                    )
                    .unwrap();
                }
            }
            assert_eq!(state, crate::session::tests::state(&game));
        }
        if game.tick > 1 && !game.cinematic.active {
            break;
        }
    }
    game.step(0).unwrap();
    assert!(game_text(&game, false, true, true, Some(Language::English)).is_empty());
}

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
                let wave_dash = tenchu_assets::script::Language::Japanese.decode(&[0x81, 0x60]);
                let samples = [
                    "MISSION EQUIPMENT 0123456789",
                    "任務開始　忍術指南　各種設定",
                    "Difficulté, français : À É È Ç Œ œ",
                    "Missione completata, città, più, è",
                    &wave_dash,
                ];
                ctx.fonts(|fonts| {
                    for sample in samples {
                        for family in [family(), FontFamily::Proportional, FontFamily::Monospace] {
                            assert!(
                                fonts.has_glyphs(&FontId::new(12.0, family.clone()), sample),
                                "Missing glyph in {sample} ({family:?})"
                            );
                        }
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
                        align: Align::Left,
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
#[ignore = "requires TENCHU_DISC; all Japanese disc captions must be covered by the embedded outline font"]
fn disc_captions_have_complete_outline_font_coverage() {
    use tenchu_assets::{
        game::GameAssets,
        script::{Command, Cutscenes, Language},
    };
    let disc = std::env::var_os("TENCHU_DISC").expect("TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
    let mut codepoints = std::collections::BTreeSet::new();
    let mut captions = 0;
    for path in assets
        .archive
        .files
        .keys()
        .filter(|p| p.contains("/ANIM/") && p.ends_with(".CAD"))
    {
        let relative = path.split_once("/ANIM/").unwrap().1;
        if relative.contains('/') {
            continue;
        }
        let language = Language::Japanese;
        let script = Cutscenes::parse(assets.asset(&format!("ANIM/{relative}")).unwrap()).unwrap();
        for command in &script.commands {
            if let Command::Caption {
                offset: Some(offset),
            } = *command
            {
                captions += 1;
                codepoints.extend(
                    language
                        .decode(script.text(offset).unwrap())
                        .chars()
                        .filter(|c| !c.is_control()),
                );
            }
        }
    }
    assert!(captions > 100);
    let ctx = egui::Context::default();
    install(&ctx);
    let _ = ctx.run(Default::default(), |ctx| {
        ctx.fonts(|fonts| {
            for character in &codepoints {
                // egui's has_glyph reports false for its own '?' replacement glyph.
                if *character == '?' {
                    continue;
                }
                assert!(
                    fonts.has_glyph(&id(14.0), *character),
                    "Missing caption character {character:?} (U+{:04X})",
                    *character as u32
                );
            }
        })
    });
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
        let original = crate::render_effects::build_presented(
            &game,
            true,
            None,
            true,
            None,
            None,
            crate::render_effects::Style::default(),
        );
        let sharp = crate::render_effects::build_presented(
            &game,
            true,
            None,
            true,
            None,
            None,
            crate::render_effects::Style {
                sharp_text: true,
                ..Default::default()
            },
        );
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
        let mut text = game_text(&game, false, true, true, None);
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
        let panel = crate::ui::build(&campaign, false, false.into(), None, Some(&mut labels));
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
                crate::renderer::SceneMeshes::new(&[], &[]),
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
