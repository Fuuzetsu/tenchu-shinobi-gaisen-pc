use super::{
    Context, Event,
    flight::{Contact, Flight, Motion},
};
use crate::{
    Result,
    math::{Matrix, Position, Vector3},
    scene::ModelId,
    tenchu2::{
        actor::Actor,
        collision::Collision,
        terrain::{self, Query},
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rice {
    pub owner: usize,
    pub target: Option<usize>,
    pub flight: Flight,
    pub transform: Matrix,
    pub rotation: [i16; 3],
    pub floor_initialized: bool,
}

impl Rice {
    pub fn launch(
        owner: usize,
        hand: Position,
        actor: &Actor,
        camera: Position,
        context: &mut Context<'_>,
    ) -> Result<Self> {
        let wall = matches!(actor.motion.action, 0x310 | 0x311);
        let (mut position, direction) = if wall {
            let (position, direction) =
                super::wall_throw(hand, actor.position, camera, actor.motion.action);
            (position, context.math.normalize_vector(direction))
        } else {
            let [sin, cos] = context.math.pair(actor.rotation.y);
            (hand, Position::new(-sin, 0, -cos))
        };
        let probe = position.wrapping_sub(Position::new(0, if wall { 0 } else { 500 }, 0));
        let floor = terrain::level(context.map, context.cursor, probe, Query::Surface)?;
        if let Some(floor) = floor {
            if !wall {
                position.y = floor;
            }
        } else {
            position = actor.position;
        }
        let rotation = [0, actor.rotation.y, 0];
        Ok(Self {
            owner,
            target: None,
            flight: Flight {
                position,
                velocity: direction,
                speed: 200,
                floor: if wall { 0 } else { floor.unwrap_or(0) },
                ..Default::default()
            },
            transform: Matrix {
                translation: position,
                rotation: context.math.rotate_yxz(rotation),
            },
            rotation,
            floor_initialized: !wall && floor.is_some(),
        })
    }

    pub fn register(&self, model: ModelId, collision: &mut Collision) -> Result<()> {
        let slot = collision.insert(model)?;
        slot.size = Vector3::new(400, 400, 400);
        slot.owner = Some(self.owner);
        slot.flags = 0x500b;
        Ok(())
    }

    pub fn advance(
        &mut self,
        model: ModelId,
        context: &mut Context<'_>,
        mut emit: impl FnMut(Event) -> Result<()>,
    ) -> Result<bool> {
        self.flight.position.y = self.flight.position.y.wrapping_sub(150);
        let (contact, splash) =
            self.flight
                .advance(Motion::Directed, context.map, context.cursor, context.math)?;
        if let Some(position) = splash {
            emit(Event::Splash(position))?;
        }
        if contact == Contact::Outside {
            return Ok(false);
        }
        self.flight.position.y = self.flight.position.y.wrapping_add(150);
        if self.flight.speed > 0 {
            self.rotation[0] = self.rotation[0].wrapping_add(0x155);
            self.transform.rotation = context.math.rotate_yxz(self.rotation);
        }
        if self.flight.position.y < self.flight.floor {
            self.flight.gravity = self.flight.gravity.wrapping_add(16);
        } else {
            self.flight.speed = self
                .flight
                .speed
                .wrapping_sub(self.flight.position.y.wrapping_sub(self.flight.floor) >> 5);
            self.flight.position.y = self.flight.floor;
            if self.flight.speed > 0 {
                self.flight.speed = self.flight.speed.wrapping_sub(4);
            }
            if self.flight.speed <= 0 {
                self.flight.speed = 0;
                context
                    .collision
                    .get_mut(model)
                    .ok_or("Missing Tenchu 2 rice collider")?
                    .owner = None;
            }
        }
        self.transform.translation = self.flight.position;
        context.scene.update(model, self.transform);
        Ok(true)
    }
}

pub fn nearest(
    owner: usize,
    position: Position,
    actors: &[super::super::ai::runtime::Agent],
    map: &tenchu_assets::tenchu2::collision::Collision,
    cursor: &mut terrain::Cursor,
    math: &crate::math::Math,
) -> Result<Option<usize>> {
    let mut nearest = None;
    let mut best = i32::MAX;
    for (index, agent) in actors.iter().enumerate() {
        let actor = &agent.actor;
        if index == owner
            || actor.flags & 8 != 0
            || matches!(
                actor.state,
                super::super::actor::State::Hurt | super::super::actor::State::Dead
            )
        {
            continue;
        }
        let delta = actor.position.wrapping_sub(position);
        if delta.y.wrapping_abs() > 4000
            || delta.x.wrapping_abs() > 8000
            || delta.z.wrapping_abs() > 8000
        {
            continue;
        }
        let length = delta.into_iter().fold(0i32, |sum, v| {
            let v = i32::from(v as i16);
            sum.wrapping_add(v * v)
        });
        let direction = math.normalize_vector(delta).map(|v| v as i16);
        if terrain::traverse(map, cursor, position, direction, 0, math)?.is_none() && length < best
        {
            best = length;
            nearest = Some(index);
        }
    }
    Ok(nearest)
}
