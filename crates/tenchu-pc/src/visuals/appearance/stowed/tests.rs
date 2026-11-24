use super::*;
use crate::visuals::{interpolation::History, models::Models, pack::Pack};

fn fixture(
    assets: &tenchu_assets::game::GameAssets<crate::source::DiscSource>,
    character: i16,
    variant: i16,
) -> (Game, usize) {
    let mut game = crate::session::Options {
        character,
        ..Default::default()
    }
    .load(assets)
    .unwrap();
    game.events = Default::default();
    game.intelligence.ignore_player = true;
    for _ in 0..30 {
        game.step(0).unwrap();
    }
    let victim = game
        .roster
        .order
        .iter()
        .copied()
        .find(|&id| {
            let a = game.roster.get(id);
            id != game.player_id
                && a.health > 0
                && (0x10..0x80).contains(&a.kind)
                && a.patrol.is_some()
                && game.editable_actor(id).is_ok()
                && a.attribute & ATTR_CUSTOMAI != 0
                && a.attribute & ATTR_WEAPON_DRAWN == 0
        })
        .unwrap();
    let position = game.player().position.wrapping_add(
        game.math
            .velocity(game.player().rotation[1], 2800, 0)
            .map(i32::from)
            .into(),
    );
    game.move_actor(victim, position, if variant == 0 { 0 } else { 2048 }, true)
        .unwrap();
    if variant == 2 {
        crate::demo::unarmed_kill(&mut game, victim).unwrap();
    } else {
        let position = game
            .roster
            .get(victim)
            .position
            .wrapping_add([0, 0, 700].into());
        game.teleport_player(position, true).unwrap();
        game.roster.get_mut(game.player_id).rotation[1] = 0;
        game.roster.get_mut(victim).setup_think(0).unwrap();
    }
    (game, victim)
}

fn parts(game: &Game, models: &Models, frame: Option<&Frame>) -> BTreeMap<Part, (usize, Matrix)> {
    let mut parts = BTreeMap::new();
    visit_parts(game, true, Some(models), frame, |key, geometry, matrix| {
        parts.insert(key, (geometry.as_ptr() as usize, matrix));
    });
    parts
}

#[test]
#[ignore = "requires TENCHU_DISC; all six paired kills preserve state, with interpolated stowed grips only for the third kills"]
fn third_kills_stow_only_the_players_blades() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let pack = Pack::default();
    for character in 0..2 {
        for variant in 0..3 {
            let (mut game, victim) = fixture(&assets, character, variant);
            let mut history = History::default();
            history.reset(&game);
            if variant == 2 {
                let clip = if character == 0 {
                    "unarmed-kill-rikimaru"
                } else {
                    "unarmed-kill-ayame"
                };
                let demo = crate::demo::Player::parse(&["--clips".into(), clip.into()]).unwrap();
                assert_eq!(demo.clip().character, character);
                for tick in 0..=30 {
                    game.step(demo.buttons(tick)).unwrap();
                    history.tick(&game);
                }
            } else {
                game.step(tenchu_core::input::SQUARE).unwrap();
            }
            assert_eq!(
                game.player().motion.action,
                MOT_ATTACK_STEALTH_BACK + character * 3 + variant
            );
            assert_eq!(unarmed_kill(game.player()), variant == 2);
            for _ in 0..70 {
                game.step(0).unwrap();
                history.tick(&game);
            }
            let before = crate::savestate::encode(&game).unwrap();
            let mut graphics = Graphics::default();
            let mut off = Models::build(&game.mission, &pack, &graphics).unwrap();
            graphics.models.stow_unarmed_kills = true;
            let mut on = Models::build(&game.mission, &pack, &graphics).unwrap();
            for alpha in [0.0, 0.25, 0.5, 0.75, 1.0] {
                let frame = history.sample(game.tick, alpha);
                let original = parts(&game, &off, frame.as_ref());
                let stowed = parts(&game, &on, frame.as_ref());
                let player = game.player();
                let mut changed = 0;
                assert_eq!(original.len(), stowed.len());
                for (key, old) in &original {
                    let new = stowed.get(key).expect("source part disappeared");
                    let is_blade = variant == 2
                        && matches!(key, Part::Actor {root, model, ..}
                        if *root == player.root_model && player.weapons[..2].iter().flatten().any(|w| w.model == *model));
                    if is_blade {
                        assert_ne!(new.0, old.0);
                        assert_ne!(new.1, old.1);
                        changed += 1;
                    } else {
                        assert_eq!(new, old);
                    }
                }
                assert_eq!(
                    changed,
                    if variant != 2 {
                        0
                    } else if character == 0 {
                        1
                    } else {
                        2
                    }
                );
            }
            if variant == 2 {
                for (enabled, models) in [(false, &mut off), (true, &mut on)] {
                    capture(&game, models, &graphics, character, "source", enabled);
                }
                graphics.set_preset(crate::settings::presets::MODERN_HIGH);
                graphics.asset_pack = false;
                for enabled in [false, true] {
                    graphics.models.stow_unarmed_kills = enabled;
                    let mut models = Models::build(&game.mission, &pack, &graphics).unwrap();
                    capture(&game, &mut models, &graphics, character, "high", enabled);
                }
            }
            assert_eq!(crate::savestate::encode(&game).unwrap(), before);
            game = crate::savestate::decode(&before).unwrap();
            assert_eq!(unarmed_kill(game.player()), variant == 2);
            for _ in 0..300 {
                game.step(0).unwrap();
            }
            assert!(!unarmed_kill(game.player()));
            assert_eq!(game.statistics.criticals, 1);
            assert_eq!(game.roster.get(victim).health, 0);
        }
    }
}

#[test]
#[ignore = "requires both discs; imported holsters and modified player bodies retain their own attachments during unarmed kills"]
fn third_kills_keep_imported_holsters_and_body_mounts() {
    let assets = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let mut library = library::Library::load(&assets).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let pack = Pack::default();
    for (hero, body) in [(0, "rikimaru"), (1, "ayame")] {
        let (mut game, _) = fixture(&assets, hero, 2);
        let settings = crate::settings::Settings {
            weapon_model: format!("tenchu2-jp:{body}"),
            ..Default::default()
        };
        library.apply_player(&mut game, &settings).unwrap();
        game.step(tenchu_core::input::SQUARE).unwrap();
        assert!(unarmed_kill(game.player()));
        for _ in 0..45 {
            game.step(0).unwrap();
        }
        let before = crate::savestate::encode(&game).unwrap();
        for appearance in ["".to_owned(), format!("tenchu2-jp:{body}")] {
            let mut graphics = Graphics::default();
            graphics.set_preset(crate::settings::presets::MODERN_HIGH);
            graphics.asset_pack = false;
            let build = |graphics: &Graphics| {
                Models::build(&game.mission, &pack, graphics)
                    .unwrap()
                    .with_player(&game, &appearance, &pack, &pack, graphics, Some(&library))
                    .unwrap()
            };
            let off = build(&graphics);
            graphics.models.stow_unarmed_kills = true;
            let on = build(&graphics);
            let original = parts(&game, &off, None);
            let stowed = parts(&game, &on, None);
            assert!(original != stowed);
            let player = game.player();
            for weapon in player.weapons[..2].iter().flatten() {
                assert!(!stowed.contains_key(&Part::Actor {
                    root: player.root_model,
                    model: weapon.model,
                    kind: weapon.kind,
                    inspection: true
                }));
            }
            for weapon in player.weapons[2..].iter().flatten() {
                let key = Part::Actor {
                    root: player.root_model,
                    model: weapon.model,
                    kind: weapon.kind,
                    inspection: true,
                };
                assert!(stowed.contains_key(&key), "imported holster disappeared");
                assert_eq!(
                    stowed[&key].0,
                    game.mission.weapons.models[&weapon.kind].as_ptr() as usize
                );
            }
        }
        assert_eq!(crate::savestate::encode(&game).unwrap(), before);
    }
}

fn capture(
    game: &Game,
    models: &mut Models,
    graphics: &Graphics,
    hero: i16,
    quality: &str,
    enabled: bool,
) {
    let Some(output) = std::env::var_os("TENCHU_MODEL_REVIEW") else {
        return;
    };
    let output = std::path::PathBuf::from(output);
    std::fs::create_dir_all(&output).unwrap();
    let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
    let mut graphics = graphics.clone();
    graphics.models.stow_unarmed_kills = enabled;
    graphics.asset_pack = false;
    review.configure(graphics);
    review.renderer.bind_models(models);
    let actors = review.renderer.upload(&render_mesh::characters_presented(
        game,
        Some(models),
        true,
        None,
    ));
    let mut low = Vec3::splat(f32::INFINITY);
    let mut high = Vec3::splat(f32::NEG_INFINITY);
    render_mesh::visit_character_parts(game, true, |key, _, matrix| {
        if matches!(key,Part::Actor{root,..} if root == game.player().root_model || Some(root) == game.player().target)
        {
            let point = Vec3::from(matrix.translation.to_array().map(|v| v as f32));
            low = low.min(point);
            high = high.max(point);
        }
    });
    let center = (low + high) * 0.5;
    let eye = center + Vec3::new(2700.0, -600.0, 3100.0);
    let camera = glam::Mat4::perspective_rh(45f32.to_radians(), 1280.0 / 720.0, 10.0, 60000.0)
        * glam::Mat4::look_at_rh(eye, center, -Vec3::Y);
    review
        .capture(
            camera,
            &[&actors],
            None,
            false,
            &output.join(format!("gaisen-third-{hero}-{quality}-{enabled}.png")),
        )
        .unwrap();
}
