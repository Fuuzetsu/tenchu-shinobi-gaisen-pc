use super::{FALLING, HIT, LANDED, TOUCHING, TOUCHING_PLAYER};
use crate::{
    Result,
    math::{Position, Vector3},
    random::Random,
    scene::ModelId,
    tenchu2::{
        actor::{Actor, Resources, State, Velocity},
        ai::direction,
        collision::{BODY, Collision, OBJECT, WEAPON},
    },
};

pub struct ContactActor {
    pub position: Position,
    pub action: i16,
    pub is_player: bool,
}

impl Actor {
    pub fn resolve_contacts(
        &mut self,
        collision: &Collision,
        body: ModelId,
        resources: &Resources,
        random: &mut Random,
        mut actor: impl FnMut(usize) -> Result<ContactActor>,
    ) -> Result<Vec<u8>> {
        let mut voices = Vec::new();
        if self.health < 0 {
            return Ok(voices);
        }
        let mut push_from_props = false;
        for contact in collision.contacts(body) {
            let slot = collision
                .slot(contact.index)
                .ok_or("Missing Tenchu 2 contact")?;
            if slot.flags & WEAPON != 0 {
                if self.flags & HIT == 0 || slot.flags & OBJECT == 0 {
                    self.hit = Some(contact.index);
                    self.flags |= HIT;
                }
                continue;
            }
            if slot.flags & BODY != 0 {
                if self.motion.action == 0xb09 {
                    continue;
                }
                self.push_away(contact.distance, resources)?;
                let other = actor(slot.owner.ok_or("Missing Tenchu 2 contact owner")?)?;
                if matches!(self.state, State::Normal | State::Ready)
                    && matches!(other.action, 0xa21 | 0xa22)
                {
                    voices.push((random.next15() & 1) as u8 + 13);
                    self.motion.action = -1;
                    let delta = other.position.wrapping_sub(self.position);
                    let angle = direction(&resources.math, delta.x, delta.z, self.rotation.y)?;
                    // JP shares this flag's register with the knockback angle.
                    push_from_props = angle == 0;
                    let front = i32::from(angle).abs() < 1024;
                    self.change_motion(
                        if front { 0xa21 } else { 0xa22 },
                        Velocity::Keep,
                        resources,
                    )?;
                    self.rotation.y = self.rotation.y.wrapping_add(angle);
                    if !front {
                        self.rotation.y = self.rotation.y.wrapping_add(2048);
                    }
                    let [x, _, z] =
                        resources
                            .math
                            .velocity(self.rotation.y, if front { -100 } else { 100 }, 0);
                    self.velocity.x = x;
                    self.velocity.z = z;
                    if !self.invulnerable {
                        self.health = self.health.wrapping_sub(5).max(0);
                    }
                }
                if self.velocity.x != 0 || self.velocity.z != 0 {
                    self.update_velocity(Velocity::Motion, resources)?;
                }
                if other.is_player {
                    self.flags |= TOUCHING_PLAYER;
                }
                continue;
            }
            if slot.flags & 0x200 != 0 {
                continue;
            }
            let delta = contact.distance.map(i32::from);
            if delta.y > 0
                && delta.x.abs() < i32::from(slot.size.x)
                && delta.z.abs() < i32::from(slot.size.z)
            {
                if (self.state != State::Jumping && self.motion.action != 0x804)
                    || (self.motion.frame > 5 && delta.y <= i32::from(self.height) / 2)
                {
                    self.flags = self.flags & !FALLING | LANDED;
                    let floor = slot
                        .position
                        .y
                        .wrapping_sub(i32::from(slot.size.y))
                        .wrapping_add(200);
                    self.terrain.floor = (floor != i32::MIN).then_some(floor);
                    self.position.y = floor;
                    self.terrain.height = i32::from(self.height);
                    self.velocity.y = 0;
                    push_from_props = true;
                }
            } else if push_from_props {
                self.push_away(contact.distance, resources)?;
                if self.velocity.x != 0 || self.velocity.z != 0 {
                    self.update_velocity(Velocity::Motion, resources)?;
                }
            }
        }
        Ok(voices)
    }

    fn push_away(&mut self, distance: Vector3<i16>, resources: &Resources) -> Result<()> {
        self.flags |= TOUCHING;
        let push = |distance| {
            if distance < 0 {
                -i32::from(self.width) / 8
            } else {
                i32::from(self.width) / 8
            }
        };
        self.position.x = self.position.x.wrapping_sub(push(distance.x));
        self.position.z = self.position.z.wrapping_sub(push(distance.z));
        let angle = direction(
            &resources.math,
            i32::from(distance.x),
            i32::from(distance.z),
            self.rotation.y,
        )?;
        self.rotation.y = self.rotation.y.wrapping_add(if angle < 0 {
            self.turn
        } else {
            self.turn.wrapping_neg()
        });
        Ok(())
    }
}
