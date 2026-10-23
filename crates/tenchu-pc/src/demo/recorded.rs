use super::Player;
use crate::{recording::Start, session::Options, source::DiscSource};
use tenchu_assets::{Result, game::GameAssets};
use tenchu_core::game::Game;

impl Player {
    pub(super) fn load_recording(
        &mut self,
        assets: &GameAssets<DiscSource>,
    ) -> Result<(Game, Options)> {
        let clip = self.clip();
        let recording = clip.recording.as_ref().unwrap();
        let text = std::fs::read_to_string(&recording.path)
            .map_err(|e| format!("{}: {e}", recording.path.display()))?;
        let start = Start::parse(&text)?.ok_or("Demo recordings need a self-contained header")?;
        if start.options.mission != clip.mission || start.options.character != clip.character {
            return Err(format!(
                "Demo '{}' mission/character differ from its recording",
                clip.id
            ));
        }
        let runs = crate::session::parse_inputs(&text)?;
        let end = u64::from(recording.start_tick) + u64::from(clip.seconds) * 30;
        if runs.iter().map(|(count, _)| u64::from(*count)).sum::<u64>() < end {
            return Err(format!(
                "Demo '{}' needs recorded inputs through tick {end}",
                clip.id
            ));
        }
        let mut inputs = runs
            .into_iter()
            .flat_map(|(count, buttons)| std::iter::repeat_n(buttons, count as usize));
        let mut game = start.load(assets)?;
        for buttons in inputs.by_ref().take(recording.start_tick as usize) {
            game.step(buttons)?;
        }
        self.inputs = Some(inputs.take(usize::from(clip.seconds) * 30).collect());
        Ok((game, start.options))
    }
}
