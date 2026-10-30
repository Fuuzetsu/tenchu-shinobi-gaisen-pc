use super::Game;
use crate::{actor::State, camera, ids::*, math::Position};

#[derive(Clone, Copy, Debug)]
pub struct Viewpoint {
    pub position: Position,
    pub target: Position,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct Navigation {
    pub heading: Option<i16>,
    pub camera: Option<Viewpoint>,
}

impl Game {
    pub fn player_can_steer(&self) -> bool {
        let player = self.player();
        !self.cinematic.active
            && !player.scripted
            && player.health > 0
            && player.attribute & (ATTR_HIT | ATTR_SUSPEND) == 0
            && matches!(
                player.motion.action,
                MOT_NORMAL
                    | MOT_NORMAL_TURN_L
                    | MOT_NORMAL_TURN_R
                    | MOT_MOVE
                    | MOT_MOVE_BACK
                    | MOT_ENGAGE_STANCE
                    | MOT_ENGAGE_TURN_L
                    | MOT_ENGAGE_TURN_R
                    | MOT_CHASE
                    | MOT_CHASE_BACK
                    | MOT_SQUAT
                    | MOT_SQUAT_WALK_F
                    | MOT_SQUAT_WALK_B
                    | MOT_SQUAT_WALK_L
                    | MOT_SQUAT_WALK_R
                    | MOT_SWIM
                    | MOT_SWIM_STROKE
            )
    }

    pub fn player_can_orbit(&self) -> bool {
        !self.cinematic.active
            && !self.player().scripted
            && !matches!(self.player().status, State::AimShuriken | State::Grapple)
            && self.camera.mode != camera::Mode::Critical
    }

    pub(super) fn steer_player(&mut self, heading: i16) {
        if !self.player_can_steer() {
            return;
        }
        let player = self.roster.get_mut(self.player_id);
        let delta = heading.wrapping_sub(player.rotation[1]);
        if delta == 0 {
            return;
        }
        let [sin, cos] = self.math.pair(delta);
        let [x, _, z] = player.velocity.map(i32::from);
        if (x != 0 || z != 0)
            && matches!(
                player.motion.action,
                MOT_MOVE | MOT_CHASE | MOT_SQUAT_WALK_F
            )
            && let Some(clip) = player.motion.clip.map(|id| &self.mission.clips[id])
        {
            let velocity = self.math.velocity(
                heading,
                i16::from(clip.forward_speed),
                i16::from(clip.side_speed),
            );
            player.velocity[0] = velocity[0];
            player.velocity[2] = velocity[2];
        } else {
            player.velocity[0] = ((x * cos + z * sin) >> 12) as i16;
            player.velocity[2] = ((z * cos - x * sin) >> 12) as i16;
        }
        player.rotation[1] = heading;
        self.scene
            .update(player.root_model, player.root(&self.math));
    }
}
