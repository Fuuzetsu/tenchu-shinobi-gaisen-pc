use super::{
    Context, Event,
    flight::{Contact, Flight, Motion},
};
use crate::{
    Result,
    math::{Math, Matrix, Position, Vector3},
    random::Random,
    scene::ModelId,
    tenchu2::{
        actor::Actor,
        collision::{BODY, Collision},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Caltrop {
    pub owner: Option<usize>,
    pub flight: Flight,
    pub settled: bool,
}

impl Caltrop {
    pub fn scatter(
        owner: usize,
        hand: Position,
        actor: &Actor,
        camera_direction: Position,
        math: &Math,
        random: &mut Random,
    ) -> [Self; 4] {
        std::array::from_fn(|index| {
            let mut position = hand;
            let direction = if matches!(actor.motion.action, 0x310 | 0x311) {
                let sign = if actor.motion.action == 0x310 { -1 } else { 1 };
                let dx = (camera_direction.z >> 2).wrapping_mul(index as i32);
                let dz = (camera_direction.x >> 2).wrapping_mul(index as i32);
                let direction = Position::new(
                    camera_direction.x.wrapping_add(dx.wrapping_mul(sign)),
                    0,
                    camera_direction.z.wrapping_sub(dz.wrapping_mul(sign)),
                );
                position.x = hand.x.wrapping_add(hand.x.wrapping_sub(actor.position.x));
                position.z = hand.z.wrapping_add(hand.z.wrapping_sub(actor.position.z));
                math.normalize_vector(direction)
            } else {
                let yaw = actor.rotation.y.wrapping_add(index as i16 * 256 - 512);
                let [sin, cos] = math.pair(yaw);
                Position::new(sin, 0, cos)
            };
            let speed = (random.next15() & 255) + 52;
            Self {
                owner: Some(owner),
                flight: Flight {
                    position,
                    velocity: direction.map(|v| v.wrapping_mul(speed) >> 12),
                    speed,
                    gravity: 16,
                    ..Default::default()
                },
                settled: false,
            }
        })
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.size = Vector3::new(400, 400, 400);
        slot.owner = self.owner;
        slot.flags = 0x5005;
        Ok(())
    }

    pub fn advance(
        &mut self,
        model: ModelId,
        context: &mut Context<'_>,
        mut emit: impl FnMut(Event) -> Result<()>,
    ) -> Result<bool> {
        if !self.settled {
            let (contact, splash) = self.flight.advance(
                Motion::Ballistic,
                context.map,
                context.cursor,
                context.math,
            )?;
            if let Some(position) = splash {
                emit(Event::Splash(position))?;
            }
            match contact {
                Contact::Outside => return Ok(false),
                Contact::Ground => {
                    emit(Event::Sound {
                        position: self.flight.position,
                        id: 53,
                    })?;
                    self.owner = None;
                    context
                        .collision
                        .get_mut(model)
                        .ok_or("Missing Tenchu 2 caltrop collider")?
                        .owner = None;
                    self.settled = true;
                }
                Contact::Air => {}
            }
        }
        context.scene.update(
            model,
            Matrix {
                translation: self.flight.position,
                ..Default::default()
            },
        );
        for contact in context.collision.contacts(model) {
            let flags = context
                .collision
                .slot(contact.index)
                .ok_or("Missing Tenchu 2 caltrop contact")?
                .flags;
            if flags & BODY != 0 {
                return Ok(false);
            }
            if flags & 0x200 == 0 {
                self.flight.speed = 0;
            }
        }
        Ok(true)
    }
}
