use super::*;

#[test]
#[ignore = "requires TENCHU_DISC; English HD menu labels follow language/quality changes while source text, controls and game state remain intact"]
fn english_menu_text_follows_language_and_restores_source_presentation() {
    let path = std::env::var_os("TENCHU_DISC").expect("Set TENCHU_DISC");
    let assets = GameAssets::open(crate::source::open_disc(path.as_ref()).unwrap()).unwrap();
    let mut campaign = Campaign::new(&assets, Default::default(), None).unwrap();
    campaign.progress.records[0].fill(
        [tenchu_core::game::score::Record {
            bosses: 1,
            enemies: 18,
            criticals: 8,
            murders: 12,
            found: 3,
            friends: 2,
            clock: 107_970,
        }; 3],
    );
    let mut frontend =
        Frontend::new(&assets, path.as_ref(), &campaign, Preferences::default()).unwrap();
    let context = egui::Context::default();
    crate::fonts::install(&context);
    let mut settings = crate::settings::Settings::default();
    settings.graphics.asset_pack = false;
    let state = crate::session::tests::state(&campaign.game);
    let progress = campaign.progress.clone();
    let preferences = frontend.preferences.clone();
    for language in [
        Language::English,
        Language::Japanese,
        Language::French,
        Language::Italian,
        Language::English,
    ] {
        frontend.language(&assets, language).unwrap();
        for page in [
            Page::Title,
            Page::Options,
            Page::Records,
            Page::Controls,
            Page::Volume,
            Page::SoundTest,
            Page::Card,
            Page::Choice(1),
            Page::Choice(2),
            Page::Choice(6),
            Page::Choice(8),
            Page::Choice(9),
        ] {
            frontend.title.activate();
            frontend.set_page(page);
            frontend.ticks = 120;
            for convention in crate::settings::MenuButtons::ALL {
                settings.set_menu_buttons(convention);
                let mut original = None;
                for sharp in [false, true, false] {
                    settings.sharp_text = sharp;
                    let _ = context.run(Default::default(), |ctx| {
                        frontend.show(ctx, &campaign, false, true, &settings);
                    });
                    let text: Vec<_> = frontend
                        .canvas
                        .text
                        .iter()
                        .map(|t| t.text.clone())
                        .collect();
                    let japanese = text.iter().any(|text| {
                        text.chars().any(|c| {
                            ('\u{3040}'..='\u{30ff}').contains(&c)
                                || ('\u{4e00}'..='\u{9fff}').contains(&c)
                        })
                    });
                    if sharp {
                        assert!(!text.is_empty(), "{language:?}/{page:?}");
                        assert_eq!(
                            japanese,
                            language != Language::English,
                            "{language:?}/{page:?}"
                        );
                        if page != Page::Title
                            && (page != Page::Controls || language == Language::English)
                        {
                            let (confirm, back) = convention.symbols();
                            assert!(text.iter().any(|s| s.contains(confirm) && s.contains(back)));
                        }
                    } else {
                        assert!(
                            text.is_empty() || japanese,
                            "{language:?}/{page:?}: source legend changed"
                        );
                        let current = (frontend.canvas.pixels.clone(), text);
                        if let Some(original) = &original {
                            assert_eq!(&current, original, "{language:?}/{page:?}/{convention:?}");
                        } else {
                            original = Some(current);
                        }
                    }
                    assert_eq!(frontend.page, page);
                    assert_eq!(frontend.preferences, preferences);
                    assert_eq!(campaign.progress, progress);
                    assert_eq!(crate::session::tests::state(&campaign.game), state);
                }
            }
        }
    }

    frontend.language(&assets, Language::English).unwrap();
    frontend.set_page(Page::Records);
    frontend.ticks = 120;
    frontend.canvas.sharp_text = true;
    frontend.canvas.frame = None;
    frontend.paint(&campaign);
    for size in [[640.0, 480.0], [960.0, 720.0]] {
        let rect = egui::Rect::from_min_size(egui::Pos2::ZERO, size.into());
        let output = context.run(
            egui::RawInput {
                screen_rect: Some(rect),
                ..Default::default()
            },
            |ctx| crate::fonts::paint(ctx, rect, &frontend.canvas.text),
        );
        let mut ranks: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|shape| {
                let egui::Shape::Text(text) = &shape.shape else {
                    return None;
                };
                (text.override_text_color.is_none() && text.galley.job.text == "Grand Master")
                    .then(|| egui::Rect::from_min_size(text.pos, text.galley.size()))
            })
            .collect();
        assert!(ranks.len() >= 3);
        ranks.sort_by(|a, b| {
            a.min
                .y
                .total_cmp(&b.min.y)
                .then(a.min.x.total_cmp(&b.min.x))
        });
        for pair in ranks.windows(2) {
            if pair[0].min.y == pair[1].min.y {
                assert!(
                    pair[1].left() - pair[0].right() >= 4.0,
                    "Rank columns touch at {size:?}"
                );
            }
        }
        assert!(ranks.iter().all(|r| r.height() >= 14.0));
    }

    frontend.set_page(Page::Controls);
    frontend.ticks = 120;
    frontend.paint(&campaign);
    let english = frontend.canvas.pixels.clone();
    frontend.language(&assets, Language::Japanese).unwrap();
    frontend.paint(&campaign);
    let mut changed = 0;
    for (index, (a, b)) in english
        .chunks_exact(4)
        .zip(frontend.canvas.pixels.chunks_exact(4))
        .enumerate()
    {
        if a != b {
            let (x, y) = (index % 320, index / 320);
            assert!(
                (98..226).contains(&x) && (203..215).contains(&y),
                "Controller panel changed at {x},{y}"
            );
            changed += 1;
        }
    }
    assert!(changed > 0);
    assert_eq!(crate::session::tests::state(&campaign.game), state);
}
