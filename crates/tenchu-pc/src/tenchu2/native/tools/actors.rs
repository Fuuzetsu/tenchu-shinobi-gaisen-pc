use super::*;
use tenchu_core::math::Position;

pub(super) enum Edit {
    Move {
        id: usize,
        position: Position,
        yaw: i16,
        snap: bool,
    },
    Remove(usize),
}

impl Tools {
    pub fn edit_actor(&mut self, item: Item, yaw: &mut i16) -> Result<()> {
        let id = self.debug.selected_actor;
        let actor = self
            .scene
            .actor(id)
            .ok_or("Start a mission and select an actor first")?;
        match item {
            Item::ActorPosition => {
                self.debug.target = actor.position.to_array().map(|v| v.round() as i32).into();
                *yaw = (i32::from(actor.yaw).rem_euclid(4096) * 360 / 4096) as i16;
            }
            Item::PickActorPosition => {
                self.debug.inspection_camera(&self.scene)?;
                self.debug.pick_actor_position = true;
                self.debug.teleport_armed = false;
                self.debug.click_teleport = false;
                self.debug.click_kill = false;
                self.click = None;
            }
            Item::MoveActor => {
                self.actor_edit = Some(Edit::Move {
                    id,
                    position: self.debug.target,
                    yaw: (i32::from(*yaw) * 4096 / 360) as i16,
                    snap: self.debug.snap_to_floor,
                })
            }
            Item::RemoveActor => self.actor_edit = Some(Edit::Remove(id)),
            _ => return Err("Unsupported actor edit".into()),
        }
        Ok(())
    }

    pub(super) fn apply_actor_edit(&mut self, content: &mut Content) -> Result<()> {
        let Some(edit) = self.actor_edit.take() else {
            return Ok(());
        };
        let Content::Gameplay(session) = content else {
            return Err("Start a mission before editing actors".into());
        };
        if session.cinematic() {
            return Err("Wait for the cinematic before editing actors".into());
        }
        let id = match edit {
            Edit::Move { id, .. } | Edit::Remove(id) => id,
        };
        if let Some(gaisen) = &session.gaisen {
            gaisen.director.editable_actor(id)?;
        }
        let notice = match edit {
            Edit::Move {
                position,
                yaw,
                snap,
                ..
            } => {
                session
                    .world
                    .reposition_actor(id, position, yaw, snap, &session.assets)?;
                format!("Moved actor #{id}; its previous patrol route was cleared.")
            }
            Edit::Remove(_) => {
                session.world.remove_actor(id, &session.assets)?;
                self.debug.selected_actor = session.world.player;
                if self
                    .debug
                    .camera_actor
                    .is_some_and(|subject| subject.id == id)
                {
                    self.debug.camera_actor = None;
                    self.debug.follow_camera = None;
                }
                format!("Removed actor #{id}. Retry restores the original layout.")
            }
        };
        session.reset_interpolation()?;
        self.debug.actors_edited = true;
        self.debug.notice = Some((notice, web_time::Instant::now()));
        Ok(())
    }
}
