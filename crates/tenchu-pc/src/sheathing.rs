pub mod imports;

use tenchu_assets::{ReadAt, Result, executable::Executable, game::GameAssets};
use tenchu_core::{
    game::{
        Game,
        sheathing::{Motions, Settings, UnarmedStyle},
    },
    ids::*,
};

pub(crate) fn stowed_mount(
    actor: &tenchu_core::actor::Actor,
    slot: usize,
) -> Option<(usize, tenchu_core::math::Matrix)> {
    if actor.attribute & ATTR_WEAPON_DRAWN != 0 || !(2..4).contains(&slot) {
        return None;
    }
    let weapon = actor.weapons[slot].as_ref()?;
    if !matches!(
        weapon.kind,
        tenchu_assets::weapon::kind::NINJA | tenchu_assets::weapon::kind::KODATI
    ) {
        return None;
    }
    actor.sheathing.as_ref()?;
    // Bind to the mouths of the heroes' built-in scabbards, not the ninja
    // animation's hand positions (its swords are carried in different places).
    let (part, rotation, translation) = match (actor.kind, slot) {
        (0, 2) => (
            1,
            [[3439, 20, -2225], [-2225, 30, -3439], [0, 4096, 36]],
            [-198, -535, 134],
        ),
        (1, 2) => (
            0,
            [[-1167, 300, -3915], [-3926, -89, 1163], [0, 4084, 312]],
            [269, -134, 60],
        ),
        (1, 3) => (
            0,
            [[-977, -583, 3935], [3978, -143, 966], [0, 4052, 601]],
            [-269, -119, 40],
        ),
        _ => return None,
    };
    Some((
        part,
        tenchu_core::math::Matrix {
            rotation,
            translation: translation.into(),
        },
    ))
}

pub fn configure(
    game: &mut Game,
    assets: &GameAssets<impl ReadAt>,
    settings: Settings,
) -> Result<()> {
    let needs_attacks = settings.enabled
        && settings.unarmed
        && game.player().sheathing.as_ref().is_none_or(|s| {
            s.needs_unarmed_motions(settings.unarmed_style)
                || settings.unarmed_style == UnarmedStyle::Tatsumaru
                    && s.settings.forward_kicks != settings.forward_kicks
        });
    if needs_attacks
        && game.player().status == tenchu_core::actor::State::Attack
        && game.player().attribute & ATTR_WEAPON_DRAWN == 0
    {
        return Err("Finish the unarmed attack before changing its moveset".into());
    }
    let imported = (needs_attacks && settings.unarmed_style == UnarmedStyle::Tatsumaru)
        .then(imports::load)
        .transpose()?;
    let motions = if settings.enabled && game.player().sheathing.is_none() {
        // Gaisen's NINJAA/KUNOITI use the heroes' skeleton layout.
        let kind = if game.player().kind == 0 { 0x20 } else { 0x22 };
        let characters = Executable::new(&assets.executable)?.characters()?;
        let character = characters
            .iter()
            .find(|c| c.kind == kind)
            .ok_or("Missing ninja character")?;
        let clips = tenchu_assets::motion::parse(&assets.asset("HUMAN/MOTION/STAGE2.AMD")?)?;
        let find = |action| {
            character
                .motions
                .iter()
                .find(|r| r.action == action)
                .and_then(|r| clips.iter().find(|c| c.id == r.clip))
                .cloned()
                .ok_or_else(|| format!("Missing ninja sheathing motion {action:#x}"))
        };
        Some(Motions {
            idle: find(MOT_NORMAL)?,
            turn: find(MOT_NORMAL_TURN_L)?,
            draw: find(MOT_STATE_DRAW)?,
            sheathe: find(MOT_STATE_SHEATHE)?,
        })
    } else {
        None
    };
    let attacks = if needs_attacks && settings.unarmed_style == UnarmedStyle::Onikage {
        let characters = Executable::new(&assets.executable)?.characters()?;
        let character = characters
            .iter()
            .find(|c| c.kind == 0x87)
            .ok_or("Missing Onikage character")?;
        let clips = tenchu_assets::motion::parse(&assets.asset("HUMAN/MOTION/STAGE4.AMD")?)?;
        Some(character.motions.iter().filter(|m| matches!(m.action, MOT_ATTACK..=MOT_ATTACK_LUNGE | MOT_ATTACK_BACK..=MOT_ATTACK_LUNGE_BACK))
            .map(|m| clips.iter().find(|c| c.id == m.clip).cloned().map(|c| (m.action, c)).ok_or("Missing Onikage attack motion".to_string()))
            .collect::<Result<Vec<_>>>()?)
    } else {
        None
    };
    game.set_player_sheathing(settings, motions.as_ref())?;
    if game
        .player()
        .sheathing
        .as_ref()
        .is_some_and(|s| s.weapon_program.is_none())
    {
        let kind = if game.player().kind == 0 { 0x20 } else { 0x22 };
        let program = Executable::new(&assets.executable)?
            .appearance(1)?
            .iter()
            .position(|&k| k == kind)
            .ok_or("Missing ninja weapon sounds")?
            + 6;
        let source = tenchu_assets::stage::imports::Voice {
            stage: 1,
            character: game.player().kind,
            program,
        };
        let index = game
            .mission
            .imported_voices
            .iter()
            .position(|v| *v == source)
            .unwrap_or_else(|| {
                let index = game.mission.imported_voices.len();
                game.mission.imported_voices.push(source);
                index
            });
        game.roster
            .get_mut(game.player_id)
            .sheathing
            .as_mut()
            .unwrap()
            .weapon_program = Some((128 + index) as i16);
    }
    if let Some(attacks) = attacks {
        game.set_player_unarmed(&attacks)?;
    }
    if let Some(moves) = imported {
        game.set_player_tatsumaru(&moves)?;
    }
    Ok(())
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn capture_pose(game: &Game, path: &std::path::Path) {
        use glam::Vec3;
        let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
        let models = review.models(&game.mission).unwrap();
        let stage = review.renderer.upload(&crate::render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        let actors = review
            .renderer
            .upload(&crate::render_mesh::characters_presented(
                game,
                Some(&models),
                true,
                None,
            ));
        let center =
            Vec3::from(game.player().position.map(|v| v as f32).to_array()) - Vec3::Y * 900.0;
        let offset = game
            .math
            .velocity(game.player().rotation[1].wrapping_add(800), -3200, 0);
        let eye = center + Vec3::from(offset.map(f32::from)) - Vec3::Y * 300.0;
        let delta = center - eye;
        let camera = crate::debug::camera::Camera {
            position: eye,
            yaw: delta.x.atan2(delta.z),
            pitch: (-delta.y).atan2(delta.x.hypot(delta.z)),
            fov: 50.0,
        };
        review
            .capture(
                camera.matrix(1280.0 / 720.0),
                &[&stage, &actors],
                None,
                false,
                path,
            )
            .unwrap();
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; both hero draw/stow transitions must produce audible samples without replacing their voices"]
    fn sheathing_transitions_have_real_weapon_samples_in_other_missions() {
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets = GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap()).unwrap();
        for mission in [1, 6] {
            for character in 0..2 {
                let mut game = crate::session::Options {
                    mission,
                    character,
                    ..Default::default()
                }
                .load(&assets)
                .unwrap();
                game.events = Default::default();
                for _ in 0..30 {
                    game.step(0).unwrap();
                }
                let settings = Settings {
                    enabled: true,
                    ..Default::default()
                };
                configure(&mut game, &assets, settings).unwrap();
                let bank = std::sync::Arc::new(crate::audio::game_bank(&assets, &game).unwrap());
                let program = game
                    .player()
                    .sheathing
                    .as_ref()
                    .unwrap()
                    .weapon_program
                    .unwrap();
                let voices = game.mission.imported_voices.len();
                for drawing in [false, true] {
                    game.roster
                        .get_mut(game.player_id)
                        .sheathing
                        .as_mut()
                        .unwrap()
                        .input(true);
                    let mut heard = 0;
                    for _ in 0..100 {
                        game.step(0).unwrap();
                        game.roster
                            .get_mut(game.player_id)
                            .sheathing
                            .as_mut()
                            .unwrap()
                            .input(false);
                        for sound in game
                            .sounds()
                            .unwrap()
                            .into_iter()
                            .filter(|s| s.id >> 4 == program)
                        {
                            assert_eq!(sound.id & 15, if drawing { 0 } else { 1 });
                            let mut mixer = tenchu_audio::Mixer::new(48000);
                            mixer.set_bank(bank.clone());
                            mixer.play(sound.id, sound.volume, sound.pan);
                            let mut frames = vec![[0.0; 2]; 16000];
                            mixer.render(&mut frames);
                            assert!(
                                frames.iter().flatten().any(|v| v.abs() > 0.01),
                                "silent sheathing sample"
                            );
                            heard += 1;
                        }
                    }
                    assert_eq!(heard, 1);
                    assert_eq!(game.player().attribute & ATTR_WEAPON_DRAWN != 0, drawing);
                    if mission == 1
                        && !drawing
                        && let Some(path) = std::env::var_os("TENCHU_SHEATH_REVIEW")
                    {
                        capture_grips(&mut game, std::path::Path::new(&path));
                    }
                }
                configure(&mut game, &assets, Settings::default()).unwrap();
                configure(&mut game, &assets, settings).unwrap();
                assert_eq!(game.mission.imported_voices.len(), voices);
                assert!(game.player().voice_program.is_none());
            }
        }
    }

    fn capture_grips(game: &mut Game, directory: &std::path::Path) {
        use crate::{render_mesh, visuals::models::Models};
        use glam::Vec3;
        std::fs::create_dir_all(directory).unwrap();
        let models =
            Models::build(&game.mission, &Default::default(), &Default::default()).unwrap();
        let mut review = crate::capture::review::Review::new(&game.mission).unwrap();
        let center =
            Vec3::from(game.player().position.map(|v| v as f32).to_array()) - Vec3::Y * 1000.0;
        let stage = review.renderer.upload(&render_mesh::stage(
            &game.mission,
            &game.math,
            Some(&models),
        ));
        for before in [false, true] {
            let state = before.then(|| {
                game.roster
                    .get_mut(game.player_id)
                    .sheathing
                    .take()
                    .unwrap()
            });
            let actor = review.renderer.upload(&render_mesh::characters_presented(
                game,
                Some(&models),
                true,
                None,
            ));
            if let Some(state) = state {
                game.roster.get_mut(game.player_id).sheathing = Some(state);
            }
            for turn in 0..4 {
                let offset = game.math.velocity(
                    game.player().rotation[1].wrapping_add(turn * 1024),
                    -2400,
                    0,
                );
                let eye = center + Vec3::from(offset.map(f32::from)) - Vec3::Y * 300.0;
                let delta = center - eye;
                let camera = crate::debug::camera::Camera {
                    position: eye,
                    yaw: delta.x.atan2(delta.z),
                    pitch: (-delta.y).atan2(delta.x.hypot(delta.z)),
                    fov: 50.0,
                };
                review
                    .capture(
                        camera.matrix(1280.0 / 720.0),
                        &[&stage, &actor],
                        None,
                        false,
                        &directory.join(format!(
                            "hero-{}-side-{turn}-before-{before}.png",
                            game.player().kind
                        )),
                    )
                    .unwrap();
            }
        }
    }
}
