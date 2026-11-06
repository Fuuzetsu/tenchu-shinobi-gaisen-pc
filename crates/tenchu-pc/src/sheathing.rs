use tenchu_assets::{ReadAt, Result, executable::Executable, game::GameAssets};
use tenchu_core::{
    game::{
        Game,
        sheathing::{Motions, Settings},
    },
    ids::*,
};

pub fn configure(
    game: &mut Game,
    assets: &GameAssets<impl ReadAt>,
    settings: Settings,
) -> Result<()> {
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
    let attacks = if settings.enabled
        && settings.unarmed
        && game
            .player()
            .sheathing
            .as_ref()
            .is_none_or(|s| s.needs_unarmed_motions())
    {
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
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

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
                }
                configure(&mut game, &assets, Settings::default()).unwrap();
                configure(&mut game, &assets, settings).unwrap();
                assert_eq!(game.mission.imported_voices.len(), voices);
                assert!(game.player().voice_program.is_none());
            }
        }
    }
}
