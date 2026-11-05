use super::Game;
use crate::{Result, ids::MOT_CHASE, motion::Motion};
use tenchu_assets::motion::Clip;

#[derive(Clone, Copy, Debug, serde::Serialize, serde::Deserialize)]
pub struct State {
    pub clip: usize,
    pub held: bool,
}

impl Game {
    pub fn set_player_walk(&mut self, clip: Option<&Clip>) -> Result<()> {
        if clip.is_some_and(|c| c.forward_speed <= 0 || c.duration <= 0 || c.bones.is_empty()) {
            return Err("Walking requires a moving, animated character clip".into());
        }
        if let Some(walk) = self.walking
            && self.player().motion.action == MOT_CHASE
            && self.player().motion.clip == Some(walk.clip)
        {
            let player = self.player();
            let index = Motion::clip_index(
                MOT_CHASE,
                &self.mission.characters[player.definition].motions,
                &self.mission.common_motions,
                &self.mission.clips,
            )
            .ok_or("Player running motion is missing")?;
            let player = self.roster.get_mut(self.player_id);
            player.motion.select_clip(index, &self.mission.clips);
            let source = &self.mission.clips[index];
            let v = self.math.velocity(
                player.rotation[1],
                i16::from(source.forward_speed),
                i16::from(source.side_speed),
            );
            player.velocity[0] = v[0];
            player.velocity[2] = v[2];
        }
        self.walking = clip.map(|source| {
            let clip = self
                .mission
                .clips
                .iter()
                .position(|c| c == source)
                .unwrap_or_else(|| {
                    let clips = std::sync::Arc::make_mut(&mut self.mission.clips);
                    clips.push(source.clone());
                    clips.len() - 1
                });
            State { clip, held: false }
        });
        Ok(())
    }
}
