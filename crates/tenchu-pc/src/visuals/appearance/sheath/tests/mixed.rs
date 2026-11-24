use super::*;
use std::sync::Arc;
use tenchu_core::ids::ATTR_WEAPON_DRAWN;

#[test]
#[ignore = "requires both discs; mixed Rikimaru sword grips, scabbards and transition samples"]
fn mixed_rikimaru_swords() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&assets).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let rikimaru = format!(
        "{}rikimaru",
        tenchu_assets::tenchu2::Edition::detect(&disc)
            .unwrap()
            .model_prefix()
    );
    let rikimaru = rikimaru.as_str();
    for hero in 0..2 {
        for (language, code) in [
            (tenchu_assets::script::Language::Japanese, "ja"),
            (tenchu_assets::script::Language::English, "en"),
        ] {
            let source = tenchu_assets::audio::voice::VoiceSet::extract(
                &tenchu_assets::audio::bank::Bank::load(&assets, 0, hero, language).unwrap(),
                5,
                tenchu_assets::audio::voice::Dialect::Gaisen,
            )
            .unwrap();
            let voice = library
                .voice(&format!("voice:{code}:disc:{hero}"))
                .unwrap()
                .unwrap();
            assert_eq!(
                postcard::to_allocvec(&voice.bank.programs[0]).unwrap(),
                postcard::to_allocvec(&source.bank.programs[0]).unwrap()
            );
            assert_eq!(
                postcard::to_allocvec(&voice.bank.samples[..source.bank.samples.len()]).unwrap(),
                postcard::to_allocvec(&source.bank.samples).unwrap()
            );
            for cue in 0..2 {
                let mut mixer = tenchu_audio::Mixer::new(48000);
                mixer.play_from(
                    &voice.bank,
                    voice.cue(tenchu_assets::audio::voice::Dialect::Gaisen, cue, false),
                    127,
                    64,
                );
                let peak = (0..24000)
                    .flat_map(|_| mixer.frame())
                    .map(f32::abs)
                    .fold(0.0, f32::max);
                assert!(peak > 0.01, "silent {code} hero {hero} weapon cue {cue}");
            }
        }
    }
    let pack = Pack::default();
    for (moves, selected) in [
        ("", ""),
        ("disc:0", ""),
        (rikimaru, ""),
        (rikimaru, "disc:0"),
        ("", rikimaru),
    ] {
        let mut game = crate::session::Options::default().load(&assets).unwrap();
        game.events = Default::default();
        game.intelligence.ignore_player = true;
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        let mut settings = Settings {
            model_moveset: !moves.is_empty(),
            moveset_model: moves.into(),
            weapon_model: selected.into(),
            ..Default::default()
        };
        settings.sheathing.enabled = true;
        crate::sheathing::configure(&mut game, &assets, settings.sheathing).unwrap();
        library.apply_player(&mut game, &settings).unwrap();
        game = crate::savestate::decode(&crate::savestate::encode(&game).unwrap()).unwrap();
        let bank = Arc::new(crate::audio::game_bank(&assets, &game).unwrap());
        let grip = &game.mission.weapons.grips[&tenchu_assets::weapon::kind::NINJA];
        let handle_start = grip
            .iter()
            .flat_map(|g| {
                g.faces
                    .iter()
                    .filter(|f| f.texture.is_some())
                    .flat_map(|f| f.corners.iter().map(|c| g.vertices[c.vertex as usize][2]))
            })
            .min()
            .unwrap();
        let guard_end = grip
            .iter()
            .flat_map(|g| {
                g.faces
                    .iter()
                    .filter(|f| {
                        f.texture.is_none()
                            && f.corners
                                .iter()
                                .all(|c| g.vertices[c.vertex as usize][2] < 0)
                    })
                    .flat_map(|f| f.corners.iter().map(|c| g.vertices[c.vertex as usize][2]))
            })
            .max()
            .unwrap();
        assert!(
            handle_start <= guard_end,
            "stowed grip is disconnected from its guard"
        );
        for drawing in [false, true] {
            game.roster
                .get_mut(game.player_id)
                .sheathing
                .as_mut()
                .unwrap()
                .input(true);
            let mut heard = Vec::new();
            for _ in 0..150 {
                game.step(0).unwrap();
                game.roster
                    .get_mut(game.player_id)
                    .sheathing
                    .as_mut()
                    .unwrap()
                    .input(false);
                let event = game
                    .intelligence
                    .voices
                    .iter()
                    .any(|v| v.actor == game.player_id && v.id == i16::from(!drawing));
                if event {
                    for sound in game.audio_with_burning([], None).unwrap() {
                        let mut mixer = tenchu_audio::Mixer::new(48000);
                        mixer.set_bank(bank.clone());
                        if let Some(source) = sound.bank {
                            mixer.play_from(
                                &source,
                                sound.sound.id,
                                sound.sound.volume,
                                sound.sound.pan,
                            );
                        } else {
                            mixer.play(sound.sound.id, sound.sound.volume, sound.sound.pan);
                        }
                        let peak = (0..24000)
                            .flat_map(|_| mixer.frame())
                            .map(f32::abs)
                            .fold(0.0, f32::max);
                        heard.push((sound.sound.id, peak));
                    }
                }
            }
            eprintln!("moves={moves:?} weapon={selected:?} draw={drawing} heard={heard:?}");
            assert_eq!(heard.len(), 1);
            assert!(
                heard[0].1 > 0.01,
                "silent transition: moves={moves:?}, weapon={selected:?}, draw={drawing}"
            );
            assert_eq!(game.player().attribute & ATTR_WEAPON_DRAWN != 0, drawing);
            for enabled in [false, true] {
                let state = crate::savestate::encode(&game).unwrap();
                settings.graphics.models.rikimaru_sheath = enabled;
                let mut models = Models::build(&game.mission, &pack, &settings.graphics)
                    .unwrap()
                    .with_player(&game, "", &pack, &pack, &settings.graphics, Some(&library))
                    .unwrap();
                if enabled || moves == rikimaru && selected != "disc:0" || selected == rikimaru {
                    assert_eq!(
                        models.player.as_ref().unwrap().parts[1].geometry[0]
                            .faces
                            .len(),
                        52
                    );
                }
                capture(
                    &game,
                    &mut models,
                    &settings.graphics,
                    &format!(
                        "mixed-moves-{}-weapon-{}-draw-{drawing}-optional-{enabled}",
                        moves.replace(':', "-"),
                        selected.replace(':', "-")
                    ),
                );
                assert_eq!(crate::savestate::encode(&game).unwrap(), state);
            }
        }
    }
}

#[test]
#[ignore = "requires both discs and artwork packs; mixed sword body/preset/pack choices preserve source state and explicit geometry"]
fn mixed_rikimaru_presentation_choices() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let mut library = Library::load(&assets).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let rikimaru = format!(
        "{}rikimaru",
        tenchu_assets::tenchu2::Edition::detect(&disc)
            .unwrap()
            .model_prefix()
    );
    let rikimaru = rikimaru.as_str();
    let mut game = crate::session::Options::default().load(&assets).unwrap();
    game.events = Default::default();
    game.intelligence.ignore_player = true;
    for _ in 0..30 {
        game.step(0).unwrap();
    }
    let mut settings = Settings {
        model_moveset: true,
        moveset_model: rikimaru.into(),
        ..Default::default()
    };
    settings.sheathing.enabled = true;
    crate::sheathing::configure(&mut game, &assets, settings.sheathing).unwrap();
    library.apply_player(&mut game, &settings).unwrap();
    game.roster
        .get_mut(game.player_id)
        .sheathing
        .as_mut()
        .unwrap()
        .input(true);
    for _ in 0..150 {
        game.step(0).unwrap();
        game.roster
            .get_mut(game.player_id)
            .sheathing
            .as_mut()
            .unwrap()
            .input(false);
    }
    let pack = Pack::from_env(&game.mission).unwrap();
    assert!(!pack.textures.is_empty(), "configure the real artwork pack");
    let empty = Pack::default();
    let state = crate::savestate::encode(&game).unwrap();
    for body in ["", "disc:0", "resource:HUMAN/RIKIMAUA.MAD", rikimaru] {
        for preset in [0, 8, 9] {
            for enabled in [false, true] {
                for artwork in [false, true] {
                    let mut graphics = Graphics::default();
                    graphics.set_preset(preset);
                    graphics.asset_pack = artwork;
                    graphics.models.rikimaru_sheath = enabled;
                    let mut models = Models::build(&game.mission, &pack, &graphics)
                        .unwrap()
                        .with_player(&game, body, &pack, &empty, &graphics, Some(&library))
                        .unwrap();
                    let appearance = models.player.as_ref().unwrap();
                    if body == rikimaru {
                        assert_eq!(appearance.parts, library.get(body).unwrap().parts);
                        assert!(appearance.sheath.is_none());
                    } else {
                        assert_eq!(
                            appearance.parts[1].geometry[0].faces.len(),
                            if body.contains("RIKIMAUA") { 30 } else { 52 }
                        );
                        assert!(appearance.sheath.is_some());
                    }
                    let vertices =
                        render_mesh::characters_presented(&game, Some(&models), true, None);
                    assert!(!vertices.is_empty());
                    if body.is_empty() && artwork && !enabled {
                        capture(
                            &game,
                            &mut models,
                            &graphics,
                            &format!("mixed-packed-preset-{preset}"),
                        );
                    }
                    assert_eq!(crate::savestate::encode(&game).unwrap(), state);
                }
            }
        }
    }
    let mut custom = Pack::default();
    custom.models.insert("actors/0000/1".into(), Vec::new());
    let graphics = Graphics {
        asset_pack: true,
        ..Default::default()
    };
    assert!(
        appearance::Appearance::build(&game, "", &custom, &empty, &graphics, Some(&library))
            .unwrap()
            .is_none()
    );
}
