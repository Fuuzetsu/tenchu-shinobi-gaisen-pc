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
    if let Some(attacks) = attacks {
        game.set_player_unarmed(&attacks)?;
    }
    Ok(())
}
