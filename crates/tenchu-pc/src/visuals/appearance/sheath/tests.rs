use super::*;
use crate::{
    render_mesh,
    settings::{Graphics, Settings},
    visuals::{
        appearance,
        interpolation::{History, Part},
        models::Models,
        pack::Pack,
    },
};

#[test]
#[ignore = "requires TENCHU_DISC and TENCHU2_DISC; optional sheath geometry, outfits, equipment, interpolation and original fallback"]
fn rikimaru_sheath_preserves_source_state_and_equipment() {
    let assets = tenchu_assets::game::GameAssets::open(
        crate::source::open_disc(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap(),
    )
    .unwrap();
    let mut library = Library::load(&assets).unwrap();
    library
        .load_tenchu2(
            &crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap(),
        )
        .unwrap();
    let pack = Pack::default();
    for (character, outfit) in [(0, 0), (0, 1), (1, 0)] {
        let mut inventory = [0; 20];
        inventory[19] = outfit;
        let mut game = crate::session::Options {
            character,
            ..Default::default()
        }
        .load_equipped(&assets, 1, inventory)
        .unwrap();
        game.events = Default::default();
        game.intelligence.ignore_player = true;
        for _ in 0..30 {
            game.step(0).unwrap();
        }
        crate::sheathing::configure(
            &mut game,
            &assets,
            tenchu_core::game::sheathing::Settings {
                enabled: true,
                ..Default::default()
            },
        )
        .unwrap();
        for stowed in [false, true] {
            if stowed {
                game.roster
                    .get_mut(game.player_id)
                    .sheathing
                    .as_mut()
                    .unwrap()
                    .input(true);
                game.step(0).unwrap();
                game.roster
                    .get_mut(game.player_id)
                    .sheathing
                    .as_mut()
                    .unwrap()
                    .input(false);
                for _ in 0..40 {
                    game.step(0).unwrap();
                }
                assert_eq!(
                    game.player().attribute & tenchu_core::ids::ATTR_WEAPON_DRAWN,
                    0
                );
            }
            for detail in [0, 9] {
                let mut graphics = Graphics::default();
                graphics.set_preset(detail);
                graphics.asset_pack = false;
                let mut history = History::default();
                history.reset(&game);
                game.step(0).unwrap();
                history.tick(&game);
                let frame = history.sample(game.tick, 0.5);
                assert!(frame.is_some());
                let before = crate::savestate::encode(&game).unwrap();
                let original = Models::build(&game.mission, &pack, &graphics).unwrap();
                let original_vertices =
                    render_mesh::characters_presented(&game, Some(&original), true, None);
                for enabled in [false, true] {
                    graphics.models.rikimaru_sheath = enabled;
                    let mut models = Models::build(&game.mission, &pack, &graphics)
                        .unwrap()
                        .with_player(&game, "", &pack, &pack, &graphics, Some(&library))
                        .unwrap();
                    let active = enabled && character == 0;
                    assert_eq!(
                        models.player.as_ref().is_some_and(|p| p.sheath.is_some()),
                        active
                    );
                    if active {
                        let torso = &models.player.as_ref().unwrap().parts[1].geometry[0];
                        assert_eq!(torso.faces.len(), if outfit == 0 { 52 } else { 30 });
                        for i in (0..15).filter(|&i| i != 1) {
                            assert_eq!(
                                models.player.as_ref().unwrap().parts[i],
                                game.mission.models[&0][i]
                            );
                        }
                    }
                    let mut sheaths = 0;
                    appearance::visit_parts(
                        &game,
                        true,
                        Some(&models),
                        frame.as_ref(),
                        |key, geometry, matrix| {
                            if matches!(key, Part::Equipment { .. }) {
                                sheaths += 1;
                                assert!(models.visit(geometry, matrix, false, |_| {}));
                            }
                        },
                    );
                    assert_eq!(sheaths, usize::from(active));
                    appearance::visit_parts_filtered(
                        &game,
                        true,
                        Some(&models),
                        frame.as_ref(),
                        |key| !matches!(key, Part::Actor { root, model, .. } if root == game.player().root_model && model == game.player().parts[1]),
                        |key, _, _| assert!(!matches!(key, Part::Equipment { .. })),
                    );
                    let vertices =
                        render_mesh::characters_presented(&game, Some(&models), true, None);
                    if !active {
                        assert_eq!(
                            bytemuck::cast_slice::<_, u8>(&vertices),
                            bytemuck::cast_slice::<_, u8>(&original_vertices)
                        );
                    }
                    if character == 0 && detail == 0 {
                        capture(
                            &game,
                            &mut models,
                            &graphics,
                            &format!("outfit-{outfit}-stowed-{stowed}-enabled-{enabled}"),
                        );
                    }
                    assert_eq!(crate::savestate::encode(&game).unwrap(), before);
                }
                let fallback = Models::build(&game.mission, &pack, &graphics)
                    .unwrap()
                    .with_player(&game, "", &pack, &pack, &graphics, None)
                    .unwrap();
                assert!(fallback.player.is_none());
                assert_eq!(
                    bytemuck::cast_slice::<_, u8>(&render_mesh::characters_presented(
                        &game,
                        Some(&fallback),
                        true,
                        None
                    )),
                    bytemuck::cast_slice::<_, u8>(&original_vertices)
                );
            }
        }
        if character == 0 {
            let mut settings = Settings::default();
            settings.graphics.models.rikimaru_sheath = true;
            assert!(
                serde_json::from_slice::<Settings>(&serde_json::to_vec(&settings).unwrap())
                    .unwrap()
                    .graphics
                    .models
                    .rikimaru_sheath
            );
            assert!(replacement(&game, "disc:1", &library).is_none());
            assert!(replacement(&game, "pack:custom", &library).is_none());
            let donor = replacement(&game, "", &library).unwrap();
            settings.weapon_model = donor
                .model
                .id
                .split(":weapons/")
                .next()
                .unwrap()
                .to_string()
                + ":rikimaru";
            library.apply_player(&mut game, &settings).unwrap();
            let before = crate::savestate::encode(&game).unwrap();
            let models = Models::build(&game.mission, &pack, &settings.graphics)
                .unwrap()
                .with_player(&game, "", &pack, &pack, &settings.graphics, Some(&library))
                .unwrap();
            let mut extra = 0;
            appearance::visit_parts(&game, true, Some(&models), None, |key, _, _| {
                extra += usize::from(matches!(key, Part::Equipment { .. }));
            });
            assert_eq!(extra, 0, "existing imported equipment supplies the sheath");
            assert_eq!(crate::savestate::encode(&game).unwrap(), before);
        }
    }
}

fn capture(game: &Game, models: &mut Models, graphics: &Graphics, name: &str) {
    let Some(directory) = std::env::var_os("TENCHU_SHEATH_REVIEW") else {
        return;
    };
    let directory = std::path::Path::new(&directory);
    std::fs::create_dir_all(directory).unwrap();
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    review.configure(graphics.clone());
    review.renderer.bind_models(models);
    let actors = review.renderer.upload(&render_mesh::characters_presented(
        game,
        Some(models),
        true,
        None,
    ));
    let center = glam::Vec3::from(game.player().position.to_array().map(|v| v as f32))
        - glam::Vec3::Y * 1000.0;
    for turn in [0, 1, 2] {
        let offset = game.math.velocity(
            game.player().rotation[1].wrapping_add(turn * 1024),
            -2300,
            0,
        );
        let eye = center + glam::Vec3::from(offset.map(f32::from)) - glam::Vec3::Y * 100.0;
        let camera = glam::Mat4::perspective_rh(50f32.to_radians(), 1280.0 / 720.0, 10.0, 60000.0)
            * glam::Mat4::look_at_rh(eye, center, -glam::Vec3::Y);
        review
            .capture(
                camera,
                &[&actors],
                None,
                false,
                &directory.join(format!("{name}-side-{turn}.png")),
            )
            .unwrap();
    }
}
