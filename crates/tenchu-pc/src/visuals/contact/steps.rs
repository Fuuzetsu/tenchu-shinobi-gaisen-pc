use super::{Hit, Surfaces, Vec3, point};
use std::collections::BTreeMap;
use tenchu_core::game::Game;
struct Previous {
    position: Vec3,
    distance: f32,
    airborne: bool,
    fall: f32,
    side: bool,
}
pub struct Contact {
    pub hit: Hit,
    pub strength: f32,
}
#[derive(Default)]
pub struct Tracker {
    actors: BTreeMap<usize, Previous>,
}
impl Tracker {
    pub fn update(&mut self, game: &Game, surfaces: &Surfaces, eye: Vec3) -> Vec<Contact> {
        let mut contacts = Vec::new();
        self.actors.retain(|id, _| game.roster.order.contains(id));
        for &id in &game.roster.order {
            let actor = game.roster.get(id);
            let position = point(actor.position);
            if actor.health <= 0 || position.distance_squared(eye) > 30000.0_f32.powi(2) {
                continue;
            }
            let airborne = actor.map.height.abs() > 160;
            let previous = self.actors.entry(id).or_insert(Previous {
                position,
                distance: 0.0,
                airborne,
                fall: 0.0,
                side: false,
            });
            let delta = position - previous.position;
            previous.position = position;
            if delta.length_squared() > 3000.0_f32.powi(2) {
                previous.distance = 0.0;
                previous.airborne = airborne;
                continue;
            }
            let speed = Vec3::new(delta.x, 0.0, delta.z).length();
            let landing = !airborne && previous.airborne;
            let fall = previous.fall;
            previous.airborne = airborne;
            if airborne {
                previous.fall = delta.y.max(previous.fall);
                continue;
            }
            previous.fall = 0.0;
            previous.distance += speed;
            if !landing && previous.distance < 500.0 {
                continue;
            }
            previous.distance = 0.0;
            previous.side = !previous.side;
            let side = Vec3::new(delta.z, 0.0, -delta.x).normalize_or_zero()
                * if previous.side { 100.0 } else { -100.0 };
            let start = position + side - Vec3::Y * 300.0;
            let Some(hit) = surfaces.cast_excluding(
                start,
                start + Vec3::Y * 1000.0,
                crate::render_mesh::FOLIAGE | crate::render_mesh::WATER,
            ) else {
                continue;
            };
            if hit.normal.y > -0.5 {
                continue;
            }
            let strength = if landing {
                (1.5 + fall / 100.0).clamp(1.5, 4.0)
            } else {
                (0.45 + speed / 100.0).clamp(0.5, 2.4)
            };
            contacts.push(Contact { hit, strength });
        }
        contacts
    }
}
