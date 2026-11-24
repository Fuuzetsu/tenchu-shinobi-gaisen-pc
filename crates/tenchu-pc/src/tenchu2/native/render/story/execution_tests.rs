use super::*;

#[test]
#[ignore = "requires JP/EU discs, isolated TENCHU_T2_CONFIG, TENCHU_EXECUTION_REVIEW and GPU; matched authored Toda execution shots"]
fn european_execution_renders_source_head_fire_and_throw() {
    std::env::var_os("TENCHU_T2_CONFIG").expect("Use isolated disc preferences");
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let europe = std::path::PathBuf::from(std::env::var_os("TENCHU2_EU_DISC").unwrap());
    crate::tenchu2::menu::remember(&europe).unwrap();
    let output = std::path::PathBuf::from(std::env::var_os("TENCHU_EXECUTION_REVIEW").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    let campaign = tenchu_assets::tenchu2::campaign::Campaign::load(&disc).unwrap();
    let sequence = campaign
        .sequences
        .iter()
        .position(|row| {
            row.kind == tenchu_assets::tenchu2::campaign::Kind::Event && row.resource == 113
        })
        .unwrap();
    for enabled in [false, true] {
        let mut story = Story::load_optional_execution(&disc, sequence, 113, 17, enabled).unwrap();
        for (shot, frame) in if enabled {
            [("cut", 420), ("head", 550), ("fire", 1000), ("throw", 1120)]
        } else {
            [("cut", 580), ("head", 760), ("fire", 1365), ("throw", 1510)]
        } {
            while story.state.frame < frame {
                story.step(true).unwrap();
            }
            let state = story.capture_state().unwrap();
            for modern in [false, true] {
                let mut graphics = Graphics {
                    asset_pack: false,
                    ..Default::default()
                };
                if modern {
                    graphics.set_preset(crate::settings::presets::MODERN_HIGH);
                    graphics.asset_pack = false;
                }
                let path = output.join(format!(
                    "toda-{shot}-enabled-{enabled}-modern-{modern}-frame-{frame}.png"
                ));
                capture_frame(&story.sprites.vram, &graphics, &path, |renderer| {
                    let mut drawing = Drawing::new(&story, &graphics, renderer)?;
                    drawing.update(&story, renderer, None)?;
                    Ok((drawing.scene, story.state.view, drawing.texts))
                })
                .unwrap();
                assert_eq!(story.capture_state().unwrap(), state);
            }
            eprintln!(
                "E113 enabled={enabled}, {shot}, source frame {frame}, seconds {:.3}, actors {}",
                story.seconds(),
                story.state.actors.len()
            );
        }
    }
}
