use super::*;
use tenchu_assets::texture::Image;

pub fn timer(game: &Game, image: Image) -> Option<char> {
    let digits = game.mission.trial.as_ref()?.digits;
    let height = digits.size[1] / 11;
    if height == 0
        || image.size != [digits.size[0], height]
        || image.page != digits.page
        || image.palette != digits.palette
        || image.uv[0] != digits.uv[0]
    {
        return None;
    }
    let offset = u16::from(image.uv[1].wrapping_sub(digits.uv[1]));
    if !offset.is_multiple_of(height) || offset / height > 10 {
        return None;
    }
    Some(if offset / height == 10 {
        ':'
    } else {
        char::from(b'0' + (offset / height) as u8)
    })
}

pub fn text(sprite: &Sprite, character: char) -> Text {
    let [sx, sy] = sprite.scale.map(|n| f32::from(n) / 4096.0);
    let [width, height] = sprite.image.size.map(f32::from);
    Text {
        position: Pos2::new(
            f32::from(sprite.xy[0]) + (width * 0.5 - f32::from(sprite.origin[0])) * sx,
            f32::from(sprite.xy[1]) - (f32::from(sprite.origin[1]) + height * 0.42) * sy,
        ),
        text: character.into(),
        color: color(sprite.color.map(|n| f32::from(n) / 128.0)),
        size: height * sy * 1.10,
        width: width * sx,
        centered: true,
    }
}

pub fn candle(game: &Game, sprite: &Sprite) -> Option<usize> {
    game.mission
        .trial
        .as_ref()?
        .candles
        .iter()
        .position(|&image| image == sprite.image)
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC and GPU; optional TENCHU_HUD_CAPTURE captures source and HD trial HUD"]
    fn trial_hud_keeps_source_glyphs_counts_and_game_state() {
        use tenchu_assets::{
            game::GameAssets,
            trial::{Mission, Objective, Point, Tables, Theme, editing},
        };
        let assets = GameAssets::open(
            crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
        )
        .unwrap();
        let tables = Tables::load(&assets).unwrap();
        let theme = Theme::load(&assets, 0).unwrap();
        let mut mission =
            Mission::new(0, Objective::EliminateEnemies, tables.appearance[0][2]).unwrap();
        for x in [10, 22] {
            editing::add_actor(
                &mut mission,
                &theme,
                Point { x, z: 12 },
                tables.appearance[0][2],
            )
            .unwrap();
        }
        let mut random = tenchu_core::random::Random::new(1);
        let prepared = tenchu_assets::stage::MissionAssets::load_trial(
            &assets,
            &tables,
            &theme,
            &mission,
            0,
            || random.next15() as u16,
        )
        .unwrap();
        let mut game = Game::with_random(
            prepared,
            &tenchu_assets::executable::Executable::new(&assets.executable).unwrap(),
            0,
            random,
        )
        .unwrap();
        game.clock = 22348;
        game.trial.as_mut().unwrap().elapsed = game.clock;
        for _ in 0..12 {
            game.step(0).unwrap();
        }
        let before = postcard::to_allocvec(&game).unwrap();
        let glyphs: String = game
            .hud
            .sprites
            .iter()
            .filter_map(|s| timer(&game, s.image))
            .collect();
        assert_eq!(glyphs, "12:25");
        let candles = game
            .hud
            .sprites
            .iter()
            .filter(|s| candle(&game, s).is_some())
            .count();
        assert_eq!(candles, 3);
        for (show_timer, sharp) in [(true, true), (false, true), (true, false)] {
            let texts = game_text(&game, true, show_timer, sharp, None);
            let timer_text: String = texts
                .iter()
                .filter(|t| (-110.0..-60.0).contains(&t.position.y))
                .map(|t| t.text.as_str())
                .collect();
            assert_eq!(
                timer_text,
                if show_timer && sharp {
                    glyphs.as_str()
                } else {
                    ""
                }
            );
        }
        let candle_boxes: Vec<_> = game
            .hud
            .sprites
            .iter()
            .filter(|s| candle(&game, s).is_some())
            .map(|s| {
                let min = egui::pos2(
                    f32::from(s.xy[0] - s.origin[0]),
                    f32::from(s.xy[1] - s.origin[1]),
                );
                egui::Rect::from_min_size(
                    min,
                    egui::vec2(f32::from(s.image.size[0]), f32::from(s.image.size[1])),
                )
            })
            .collect();
        for hd in [false, true] {
            let geometry = crate::render_effects::build_presented(
                &game,
                true,
                None,
                true,
                None,
                None,
                crate::render_effects::Style {
                    hd_hud: hd,
                    sharp_text: hd,
                    ..Default::default()
                },
            );
            let textured_candles = geometry
                .vertices
                .iter()
                .filter(|v| {
                    v.flags & crate::render_mesh::UI != 0
                        && v.flags & crate::render_mesh::TEXTURED != 0
                        && candle_boxes
                            .iter()
                            .any(|r| r.contains(egui::pos2(v.position[0], v.position[1])))
                })
                .count();
            assert_eq!(textured_candles == 0, hd);
        }
        if let Some(output) = std::env::var_os("TENCHU_HUD_CAPTURE") {
            let output = std::path::PathBuf::from(output);
            std::fs::create_dir_all(&output).unwrap();
            let setup = game.mission.trial.as_ref().unwrap();
            std::fs::write(output.join("source-sizes.json"), serde_json::to_vec_pretty(&serde_json::json!({"timer": setup.digits.size, "candles": setup.candles.iter().map(|i| i.size).collect::<Vec<_>>()})).unwrap()).unwrap();
            for sharp in [false, true] {
                let mut settings = crate::settings::Settings {
                    sharp_text: sharp,
                    ..Default::default()
                };
                settings.graphics.asset_pack = sharp;
                crate::capture::frame_with_settings(
                    &game,
                    &output.join(if sharp {
                        "hud-hd.png"
                    } else {
                        "hud-original.png"
                    }),
                    &assets,
                    &settings,
                )
                .unwrap();
            }
        }
        assert_eq!(postcard::to_allocvec(&game).unwrap(), before);
        for (clock, expected) in [
            (0, "0:00"),
            (18000, "10:00"),
            (107970, "59:59"),
            (107985, ""),
        ] {
            game.clock = clock;
            game.trial.as_mut().unwrap().elapsed = clock;
            game.step(0).unwrap();
            let source: String = game
                .hud
                .sprites
                .iter()
                .filter_map(|s| timer(&game, s.image))
                .collect();
            assert_eq!(source, expected);
            let sharp: String = game_text(&game, true, true, true, None)
                .into_iter()
                .filter(|t| (-110.0..-60.0).contains(&t.position.y))
                .map(|t| t.text)
                .collect();
            assert_eq!(sharp, source);
        }
    }
}
