use tenchu_assets::{ReadAt, Result, executable::Executable, game::GameAssets};
use tenchu_core::{game::Game, ids::MOT_MOVE};

pub fn configure(game: &mut Game, assets: &GameAssets<impl ReadAt>, enabled: bool) -> Result<()> {
    if game.walking.is_some() == enabled {
        return Ok(());
    }
    if !enabled {
        return game.set_player_walk(None);
    }
    // Gaisen's NINJAA/KUNOITI registrations; STAGE2.AMD contains both walk cycles.
    let kind = if game.player().kind == 0 { 0x20 } else { 0x22 };
    let characters = Executable::new(&assets.executable)?.characters()?;
    let id = characters
        .iter()
        .find(|c| c.kind == kind)
        .and_then(|c| c.motions.iter().find(|r| r.action == MOT_MOVE))
        .ok_or("Missing ninja walking registration")?
        .clip;
    let clips = tenchu_assets::motion::parse(&assets.asset("HUMAN/MOTION/STAGE2.AMD")?)?;
    let clip = clips
        .iter()
        .find(|clip| clip.id == id)
        .ok_or("Missing ninja walking animation")?;
    game.set_player_walk(Some(clip))
}

pub fn requested(buttons: u16, magnitude: f32, threshold: u16, previous: bool) -> bool {
    if buttons & crate::controls::WALK != 0 {
        return true;
    }
    let limit = f32::from(threshold.clamp(25, 95)) / 100.0 + if previous { 0.03 } else { -0.03 };
    magnitude.is_finite() && magnitude > 0.0 && magnitude < limit
}
