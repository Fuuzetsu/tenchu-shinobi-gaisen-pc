use super::*;
use tenchu_core::{actor::State, effects::RumbleAction};

const COATING_LIMIT: usize = 128;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Coating {
    vertices: Vec<Vertex>,
    tip: Vec3,
    previous: Vec3,
    amount: f32,
    wet: f32,
    hit: u64,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Weapons {
    coatings: BTreeMap<Part, Coating>,
}

impl Weapons {
    pub fn needed(&self, game: &Game) -> bool {
        !self.is_empty()
            || game.effects.rumble.iter().any(|r| {
                matches!(
                    r.action,
                    RumbleAction::Hit | RumbleAction::TargetHit | RumbleAction::StealthKill
                )
            })
    }

    pub fn clear(&mut self) {
        self.coatings.clear();
    }

    pub fn tick(
        &mut self,
        game: &Game,
        models: &Models,
        transforms: &BTreeMap<Part, Matrix>,
        drops: &mut Vec<Drop>,
    ) {
        let hits: std::collections::BTreeSet<_> = game
            .effects
            .rumble
            .iter()
            .filter(|r| {
                matches!(
                    r.action,
                    RumbleAction::Hit | RumbleAction::TargetHit | RumbleAction::StealthKill
                )
            })
            .map(|r| r.actor)
            .collect();
        let mut live = BTreeMap::new();
        for &id in &game.roster.order {
            let actor = game.roster.get(id);
            for weapon in actor.weapons[..2].iter().flatten() {
                let key = Part::Actor {
                    root: actor.root_model,
                    model: weapon.model,
                    kind: weapon.kind,
                    inspection: false,
                };
                live.insert(key, actor.status == State::Attack);
                if !hits.contains(&id) {
                    continue;
                }
                let Some(&matrix) = transforms.get(&key) else {
                    continue;
                };
                if let std::collections::btree_map::Entry::Vacant(entry) = self.coatings.entry(key)
                {
                    let mut vertices = Vec::new();
                    render_mesh::append_geometry(
                        &mut vertices,
                        &game.mission.weapons.models[&weapon.kind],
                        Matrix::default(),
                        Some(models),
                    );
                    let mut vertices: Vec<_> = vertices
                        .chunks_exact(3)
                        .filter(|t| t[0].flags & render_mesh::WEAPON_METAL != 0)
                        .flat_map(|t| t.iter().copied())
                        .collect();
                    if vertices.is_empty() {
                        continue;
                    }
                    let tip = Vec3::from(
                        vertices
                            .iter()
                            .max_by(|a, b| {
                                Vec3::from(a.position)
                                    .length_squared()
                                    .total_cmp(&Vec3::from(b.position).length_squared())
                            })
                            .unwrap()
                            .position,
                    );
                    for triangle in vertices.chunks_exact_mut(3) {
                        let [a, b, c] = std::array::from_fn(|i| Vec3::from(triangle[i].position));
                        let normal = (b - a).cross(c - a).normalize_or_zero();
                        for v in triangle {
                            if Vec3::from(v.normal).length_squared() < 0.001 {
                                v.normal = normal.to_array();
                            }
                        }
                    }
                    for v in &mut vertices {
                        let p = Vec3::from(v.position);
                        v.position =
                            (p + Vec3::from(v.normal).normalize_or_zero() * 1.2).to_array();
                        v.uv = [p.x * 0.008 + p.z * 0.003, p.y * 0.006 + p.z * 0.007];
                        v.color = [0.31, 0.006, 0.012];
                        v.material = 0;
                        v.flags = render_mesh::WEAPON_COAT
                            | render_mesh::WEAPON_METAL
                            | render_mesh::FOG
                            | render_mesh::STAIN
                            | render_mesh::ALPHA
                            | render_mesh::DOUBLE_SIDED;
                    }
                    entry.insert(Coating {
                        vertices,
                        tip,
                        previous: rotation(matrix) * tip + translation(matrix),
                        amount: 0.0,
                        wet: 0.0,
                        hit: game.tick,
                    });
                }
                let coating = self.coatings.get_mut(&key).unwrap();
                coating.amount = (coating.amount + 0.7).min(1.0);
                coating.wet = 1.0;
                coating.hit = game.tick;
            }
        }
        self.coatings.retain(|key, c| {
            let Some(&attacking) = live.get(key) else {
                return false;
            };
            let Some(&matrix) = transforms.get(key) else {
                return true;
            };
            let position = rotation(matrix) * c.tip + translation(matrix);
            let velocity = position - c.previous;
            c.previous = position;
            c.wet = (c.wet - 1.0 / 1800.0).max(0.0);
            if c.amount <= 0.08 || c.wet == 0.0 || drops.len() >= DROP_LIMIT {
                return true;
            }
            let swing = attacking && velocity.length() > 75.0 && velocity.length() < 1500.0;
            if swing && game.tick.is_multiple_of(2) || game.tick.is_multiple_of(60) {
                let speed = if swing {
                    velocity.clamp_length_max(160.0) * 0.6
                } else {
                    Vec3::Y * 8.0
                };
                drops.push(Drop {
                    previous: position,
                    position,
                    velocity: speed,
                    life: 90,
                    weapon: true,
                });
                c.amount = (c.amount - if swing { 0.055 } else { 0.008 } * c.wet).max(0.08);
            }
            true
        });
        while self.coatings.len() > COATING_LIMIT {
            let oldest = *self.coatings.iter().min_by_key(|(_, c)| c.hit).unwrap().0;
            self.coatings.remove(&oldest);
        }
    }

    pub fn append(&self, out: &mut Vec<Vertex>, transforms: &BTreeMap<Part, Matrix>) {
        for (key, coating) in &self.coatings {
            let Some(&matrix) = transforms.get(key) else {
                continue;
            };
            let r = rotation(matrix);
            let offset = translation(matrix);
            out.extend(coating.vertices.iter().map(|v| Vertex {
                position: (r * Vec3::from(v.position) + offset).to_array(),
                normal: (r * Vec3::from(v.normal)).normalize_or_zero().to_array(),
                opacity: coating.amount,
                ..*v
            }));
        }
    }

    pub fn is_empty(&self) -> bool {
        self.coatings.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU_DISC; ordinary sword combat coats both heroes and guards, swing shedding exceeds idle drips, and state restoration preserves coating"]
    fn combat_stains_blades_and_swinging_sheds_more_than_idle() {
        use tenchu_core::input;
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for character in 0..2 {
            let mut game = crate::session::Options {
                mission: 2,
                character,
                ..Default::default()
            }
            .load(&assets)
            .unwrap();
            game.events = Default::default();
            for _ in 0..30 {
                game.step(0).unwrap();
            }
            let guard = game
                .roster
                .order
                .iter()
                .copied()
                .find(|&id| {
                    let a = game.roster.get(id);
                    id != game.player_id
                        && a.patrol.is_some()
                        && a.weapons.iter().any(Option::is_some)
                        && a.attribute & tenchu_core::ids::ATTR_CUSTOMAI != 0
                })
                .unwrap();
            let enemy = game.roster.get(guard);
            let offset = game
                .math
                .velocity(enemy.rotation[1], 1300, 0)
                .map(i32::from);
            game.teleport_player(enemy.position.wrapping_add(offset.into()), true)
                .unwrap();
            let delta = game
                .roster
                .get(guard)
                .position
                .wrapping_sub(game.player().position);
            game.roster.get_mut(game.player_id).rotation[1] =
                game.math.atan2(-delta.x, -delta.z).unwrap();
            crate::demo::sparring(&mut game, guard).unwrap();
            game.intelligence.difficulty = tenchu_core::ai::Difficulty::Easy;
            let graphics = settings::Graphics {
                blood: settings::Blood::modern(),
                ..Default::default()
            };
            let pack = crate::visuals::pack::Pack::from_env(&game.mission).unwrap();
            let models = Models::build(&game.mission, &pack, &graphics).unwrap();
            let surfaces = Surfaces::new(&render_mesh::stage(
                &game.mission,
                &game.math,
                Some(&models),
            ));
            let mut blood = Blood::default();
            let mut hits = std::collections::BTreeSet::new();
            for tick in 0u32..900 {
                game.step(if tick.is_multiple_of(45u32) {
                    input::SQUARE
                } else {
                    0
                })
                .unwrap();
                blood.tick(&game, &models, &surfaces, &graphics.blood, None);
                for key in blood.weapons.coatings.keys() {
                    if let Part::Actor { root, .. } = key {
                        hits.insert(*root);
                    }
                }
                if hits.contains(&game.player().root_model)
                    && hits.contains(&game.roster.get(guard).root_model)
                {
                    break;
                }
            }
            assert!(
                hits.contains(&game.player().root_model),
                "hero {character} never stained a blade"
            );
            assert!(
                hits.contains(&game.roster.get(guard).root_model),
                "guard never stained a weapon: {hits:?}"
            );
            let bytes = postcard::to_allocvec(&blood.weapons).unwrap();
            let mut idle: Weapons = postcard::from_bytes(&bytes).unwrap();
            let mut moving = idle.clone();
            let mut restored: Weapons = postcard::from_bytes(&bytes).unwrap();
            let transforms = transforms(&game, &models, None, true);
            game.effects.rumble.clear();
            for actor in game.roster.actors.iter_mut().flatten() {
                actor.status = State::Attack;
            }
            let (mut idle_drops, mut moving_drops, mut restored_drops) =
                (Vec::new(), Vec::new(), Vec::new());
            for tick in 0..120 {
                game.tick += 1;
                idle.tick(&game, &models, &transforms, &mut idle_drops);
                let mut swung = transforms.clone();
                for matrix in swung.values_mut() {
                    matrix.translation.x += if tick % 2 == 0 { 140 } else { -140 };
                }
                moving.tick(&game, &models, &swung, &mut moving_drops);
                restored.tick(&game, &models, &swung, &mut restored_drops);
            }
            let amount = |w: &Weapons| w.coatings.values().map(|c| c.amount).sum::<f32>();
            assert!(amount(&moving) < amount(&idle) * 0.5);
            assert!(moving_drops.len() > idle_drops.len() * 3);
            assert_eq!(
                postcard::to_allocvec(&moving).unwrap(),
                postcard::to_allocvec(&restored).unwrap()
            );
            assert!(moving_drops.len() <= DROP_LIMIT);
            let mut vertices = Vec::new();
            moving.append(&mut vertices, &transforms);
            assert!(!vertices.is_empty());
            assert!(
                vertices
                    .iter()
                    .all(|v| v.position.iter().chain(&v.normal).all(|n| n.is_finite()))
            );
        }
    }
}
