use super::*;
use std::collections::BTreeSet;
use tenchu_core::{actor::State, effects::RumbleAction};

const COATING_LIMIT: usize = 128;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
struct Coating {
    vertices: Vec<Vertex>,
    tip: Vec3,
    previous: Vec3,
    flow: Vec3,
    previous_flow: Vec3,
    amount: f32,
    wet: f32,
    hit: u64,
}

#[derive(Clone, Default, serde::Serialize, serde::Deserialize)]
pub(super) struct Weapons {
    coatings: BTreeMap<Part, Coating>,
}

pub(super) fn contacts(game: &Game, blood: &[Contact]) -> BTreeSet<usize> {
    use tenchu_core::ids::{MOT_ATTACK_STEALTH_BACK, MOT_ATTACK_STEALTH_SIDE_AYAME};
    let mut hits: BTreeSet<_> = game
        .effects
        .rumble
        .iter()
        .filter(|r| matches!(r.action, RumbleAction::Hit | RumbleAction::TargetHit))
        .map(|r| r.actor)
        .collect();
    let player = game.player();
    if (MOT_ATTACK_STEALTH_BACK..=MOT_ATTACK_STEALTH_SIDE_AYAME).contains(&player.motion.action)
        && let Some(victim) = game
            .roster
            .actors
            .iter()
            .flatten()
            .find(|actor| Some(actor.root_model) == player.target)
        && blood.iter().any(|contact| {
            contact
                .source
                .is_some_and(|part| victim.parts.contains(&part))
        })
    {
        hits.insert(player.id);
    }
    hits
}

impl Weapons {
    pub fn parts(&self) -> impl Iterator<Item = Part> + '_ {
        self.coatings.keys().copied()
    }

    pub fn clear(&mut self) {
        self.coatings.clear();
    }

    pub fn tick(
        &mut self,
        game: &Game,
        models: &Models,
        transforms: &BTreeMap<Part, Matrix>,
        hits: &BTreeSet<usize>,
        drops: &mut Vec<Drop>,
        settings: &settings::Blood,
    ) {
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
                    let has_metal = vertices
                        .iter()
                        .any(|v| v.flags & render_mesh::WEAPON_METAL != 0);
                    let mut vertices: Vec<_> = vertices
                        .chunks_exact(3)
                        .filter(|t| {
                            t[0].flags & render_mesh::TRANSLUCENT == 0
                                && (!has_metal || t[0].flags & render_mesh::WEAPON_METAL != 0)
                        })
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
                        v.flags = (v.flags & render_mesh::WEAPON_METAL)
                            | render_mesh::WEAPON_COAT
                            | render_mesh::FOG
                            | render_mesh::STAIN
                            | render_mesh::ALPHA
                            | render_mesh::DOUBLE_SIDED;
                    }
                    entry.insert(Coating {
                        vertices,
                        tip,
                        previous: rotation(matrix) * tip + translation(matrix),
                        flow: Vec3::ZERO,
                        previous_flow: Vec3::ZERO,
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
            let r = rotation(matrix);
            let position = r * c.tip + translation(matrix);
            let velocity = position - c.previous;
            c.previous = position;
            c.wet = (c.wet - 1.0 / 1800.0).max(0.0);
            c.previous_flow = c.flow;
            let flow = f32::from(settings.weapon_flow.min(200)) / 100.0;
            if flow > 0.0 && c.wet > 0.0 && r.determinant().abs() >= 0.0001 {
                c.flow += r.inverse() * Vec3::Y * (8.0 * flow * c.wet);
            }
            if c.amount <= 0.08 || c.wet == 0.0 || drops.len() >= DROP_LIMIT {
                return true;
            }
            let swing = attacking && velocity.length() > 75.0 && velocity.length() < 1500.0;
            let drip_interval = (60.0 / (1.0 + flow)).round() as u64;
            if swing && game.tick.is_multiple_of(2) || game.tick.is_multiple_of(drip_interval) {
                let position = if flow > 0.0 && !swing {
                    c.vertices
                        .iter()
                        .map(|v| r * Vec3::from(v.position) + translation(matrix))
                        .max_by(|a, b| a.y.total_cmp(&b.y))
                        .unwrap_or(position)
                } else {
                    position
                };
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

    pub fn append(
        &self,
        out: &mut Vec<Vertex>,
        transforms: &BTreeMap<Part, Matrix>,
        settings: &settings::Blood,
        alpha: f32,
    ) {
        for (key, coating) in &self.coatings {
            let Some(&matrix) = transforms.get(key) else {
                continue;
            };
            let r = rotation(matrix);
            let offset = translation(matrix);
            let flow = coating
                .previous_flow
                .lerp(coating.flow, alpha.clamp(0.0, 1.0));
            let wet = if settings.weapon_flow == 0 {
                0.0
            } else {
                coating.wet
            };
            out.extend(coating.vertices.iter().map(|v| {
                let normal = Vec3::from(v.normal).normalize_or_zero();
                let tangent = flow - normal * flow.dot(normal);
                let flow = [
                    tangent.x * 0.008 + tangent.z * 0.003,
                    tangent.y * 0.006 + tangent.z * 0.007,
                ];
                Vertex {
                    position: (r * Vec3::from(v.position) + offset).to_array(),
                    normal: (r * normal).normalize_or_zero().to_array(),
                    opacity: (coating.amount * f32::from(settings.weapon_coverage) / 100.0)
                        .min(1.0),
                    color: Vec3::from(v.color)
                        .lerp(Vec3::new(0.43, 0.009, 0.018), wet)
                        .to_array(),
                    // Untextured WEAPON_COAT uses these slots for tangent flow and wetness.
                    page: flow[0].to_bits(),
                    palette: flow[1].to_bits(),
                    material: wet.to_bits(),
                    ..*v
                }
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
    #[ignore = "requires TENCHU_DISC; successful Kunoichi melee hits must coat both weapons with original and replacement artwork"]
    fn kunoichi_hits_stain_both_weapons() {
        use tenchu_assets::weapon::kind::{EN, JYUTE};
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for (mission, kind) in [(2, JYUTE), (9, EN)] {
            for artwork in [false, true] {
                let mut game = crate::session::Options {
                    mission,
                    ..Default::default()
                }
                .load(&assets)
                .unwrap();
                game.events = Default::default();
                let guard = *game
                    .roster
                    .order
                    .iter()
                    .find(|&&id| {
                        game.mission.characters[game.roster.get(id).definition].weapon == kind
                    })
                    .unwrap();
                for id in game.roster.order.clone() {
                    if id != guard && id != game.player_id {
                        let _ = game.remove_actor(id);
                    }
                }
                let enemy = game.roster.get(guard);
                let offset = game
                    .math
                    .velocity(enemy.rotation[1], 1300, 0)
                    .map(i32::from);
                game.teleport_player(enemy.position.wrapping_add(offset.into()), true)
                    .unwrap();
                crate::demo::sparring(&mut game, guard).unwrap();
                let hero = game.roster.get_mut(game.player_id);
                hero.invulnerable = false;
                hero.health = 30000;
                let graphics = settings::Graphics {
                    asset_pack: artwork,
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
                blood.tick(&game, &models, &surfaces, &graphics.blood, None);
                let mut hit = false;
                for _ in 0..900 {
                    game.step(0).unwrap();
                    blood.tick(&game, &models, &surfaces, &graphics.blood, None);
                    if game.player().health < 30000
                        && game
                            .effects
                            .rumble
                            .iter()
                            .any(|r| r.actor == guard && r.action == RumbleAction::Hit)
                    {
                        hit = true;
                        break;
                    }
                }
                assert!(hit, "M{mission}: no successful melee hit");
                let actor = game.roster.get(guard);
                let coated = blood
                    .weapons
                    .parts()
                    .filter(|part| matches!(part, Part::Actor { root, .. } if *root == actor.root_model))
                    .count();
                assert_eq!(coated, 2, "M{mission}, replacement artwork: {artwork}");
            }
        }
    }

    #[test]
    #[ignore = "requires TENCHU_DISC; six paired kills synchronize blade/body blood, including after rewind; the neck break stays bloodless"]
    fn stealth_blades_and_wounds_wait_for_the_victims_blood() {
        use tenchu_core::{ids::*, input};
        let disc = std::env::var_os("TENCHU_DISC").unwrap();
        let assets =
            tenchu_assets::game::GameAssets::open(crate::source::open_disc(disc.as_ref()).unwrap())
                .unwrap();
        for character in 0..2 {
            for direction in 0..3 {
                let mut game = crate::session::Options {
                    character,
                    ..Default::default()
                }
                .load(&assets)
                .unwrap();
                game.events = Default::default();
                let player = game.player_id;
                let victim = *game
                    .roster
                    .order
                    .iter()
                    .find(|&&id| (0x10..0x80).contains(&game.roster.get(id).kind))
                    .unwrap();
                for id in game.roster.order.clone() {
                    if id != player && id != victim {
                        let _ = game.remove_actor(id);
                    }
                }
                let hero = game.roster.get_mut(player);
                hero.rotation = [0; 3];
                hero.status = State::Engage;
                hero.motion.change(
                    MOT_ENGAGE_STANCE,
                    &game.mission.characters[hero.definition].motions,
                    &game.mission.common_motions,
                    &game.mission.clips,
                );
                game.scene.update(hero.root_model, hero.root(&game.math));
                let mut position = game.player().position;
                if direction == 2 {
                    position.x += 700;
                } else {
                    position.z -= 700;
                }
                game.move_actor(
                    victim,
                    position,
                    if direction == 1 { 2048 } else { 0 },
                    false,
                )
                .unwrap();
                let actor = game.roster.get_mut(victim);
                actor.attribute = 0;
                actor.health = actor.max_health;
                actor.inventory.fill(0);
                actor.status = State::Normal;
                actor.motion.change(
                    MOT_NORMAL,
                    &game.mission.characters[actor.definition].motions,
                    &game.mission.common_motions,
                    &game.mission.clips,
                );
                let variant = direction + character * 3;
                let expects_blood = game.mission.death_events[variant as usize]
                    .iter()
                    .any(|event| matches!(event.action, 3 | 4) && event.payload[0] != -1);
                let settings = settings::Blood::modern();
                let graphics = settings::Graphics {
                    blood: settings,
                    ..Default::default()
                };
                let models = Models::build(&game.mission, &Default::default(), &graphics).unwrap();
                let surfaces = Surfaces::new(&render_mesh::stage(
                    &game.mission,
                    &game.math,
                    Some(&models),
                ));
                let mut blood = Blood::default();
                blood.tick(&game, &models, &surfaces, &settings, None);
                let mut bled = false;
                let mut coated = false;
                let mut wounded = false;
                let mut stained = false;
                let mut dripped = false;
                let mut restored: Option<Blood> = None;
                for tick in 0..220 {
                    game.step(if tick == 0 { input::SQUARE } else { 0 })
                        .unwrap();
                    if tick == 0 {
                        assert_eq!(
                            game.roster.get(victim).motion.action,
                            MOT_DEAD_STEALTH_BACK + variant
                        );
                    }
                    bled |= game
                        .effects
                        .active_slots()
                        .iter()
                        .enumerate()
                        .any(|(i, effect)| {
                            matches!(effect, Some(Effect::Blood(b)) if b.gore)
                                && game.effects.gore_sources[i].is_some_and(|source| {
                                    game.roster.get(victim).parts.contains(&source)
                                })
                        });
                    let before = postcard::to_allocvec(&game).unwrap();
                    blood.tick(&game, &models, &surfaces, &settings, None);
                    if tick == 10 {
                        let mut recorder = crate::rewind::Recorder::default();
                        recorder.begin();
                        blood.capture(&mut recorder).unwrap();
                        restored = Some(recorder.finish().unwrap().decode().unwrap());
                    } else if let Some(restored) = &mut restored {
                        restored.tick(&game, &models, &surfaces, &settings, None);
                        assert_eq!(
                            postcard::to_allocvec(&blood).unwrap(),
                            postcard::to_allocvec(restored).unwrap()
                        );
                    }
                    assert_eq!(before, postcard::to_allocvec(&game).unwrap());
                    coated |= !blood.weapons.is_empty();
                    wounded |= !blood.wounds.is_empty();
                    stained |= blood
                        .marks
                        .attached
                        .iter()
                        .any(|(_, mark)| mark.1 == Kind::Body);
                    dripped |= blood.drops.iter().any(|drop| !drop.weapon);
                    assert!(
                        bled || blood.weapons.is_empty(),
                        "variant {variant}: blades bloodied before contact on tick {}",
                        game.tick
                    );
                    assert!(
                        bled || blood.drops.iter().all(|drop| !drop.weapon),
                        "variant {variant}: early weapon droplets"
                    );
                    assert!(
                        bled || !(wounded || stained || dripped),
                        "variant {variant}: early body blood"
                    );
                }
                assert_eq!(bled, expects_blood);
                assert_eq!(coated, expects_blood, "variant {variant}");
                assert_eq!(wounded, expects_blood, "variant {variant}: wound");
                assert_eq!(stained, expects_blood, "variant {variant}: body stain");
                assert_eq!(dripped, expects_blood, "variant {variant}: dripping");
                assert!(
                    blood.wounds.is_empty(),
                    "wounds must stop dripping after their configured duration"
                );
                assert!(blood.pending_wounds.is_empty());
            }
        }
    }

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
            let transforms =
                transforms(&game, &models, None, true, &blood.weapons.parts().collect());
            game.effects.rumble.clear();
            for actor in game.roster.actors.iter_mut().flatten() {
                actor.status = State::Attack;
            }
            let (mut idle_drops, mut moving_drops, mut restored_drops) =
                (Vec::new(), Vec::new(), Vec::new());
            for tick in 0..120 {
                game.tick += 1;
                idle.tick(
                    &game,
                    &models,
                    &transforms,
                    &BTreeSet::new(),
                    &mut idle_drops,
                    &graphics.blood,
                );
                let mut swung = transforms.clone();
                for matrix in swung.values_mut() {
                    matrix.translation.x += if tick % 2 == 0 { 140 } else { -140 };
                }
                moving.tick(
                    &game,
                    &models,
                    &swung,
                    &BTreeSet::new(),
                    &mut moving_drops,
                    &graphics.blood,
                );
                restored.tick(
                    &game,
                    &models,
                    &swung,
                    &BTreeSet::new(),
                    &mut restored_drops,
                    &graphics.blood,
                );
            }
            let amount = |w: &Weapons| w.coatings.values().map(|c| c.amount).sum::<f32>();
            assert!(amount(&moving) < amount(&idle) * 0.5);
            assert!(moving_drops.len() > idle_drops.len() * 3);
            assert_eq!(
                postcard::to_allocvec(&moving).unwrap(),
                postcard::to_allocvec(&restored).unwrap()
            );
            assert!(moving_drops.len() <= DROP_LIMIT);
            let mut flowing = idle.clone();
            let flow_settings = settings::Blood {
                weapon_flow: 100,
                weapon_coverage: 140,
                ..graphics.blood
            };
            let mut flow_drops = Vec::new();
            for _ in 0..120 {
                game.tick += 1;
                flowing.tick(
                    &game,
                    &models,
                    &transforms,
                    &BTreeSet::new(),
                    &mut flow_drops,
                    &flow_settings,
                );
            }
            assert!(!flow_drops.is_empty());
            for (part, coating) in &flowing.coatings {
                let matrix = transforms[part];
                let world_flow = rotation(matrix) * (coating.flow - idle.coatings[part].flow);
                assert!(world_flow.y > 0.0 && world_flow.x.abs() < 0.1 && world_flow.z.abs() < 0.1);
                let bottom = coating
                    .vertices
                    .iter()
                    .map(|v| rotation(matrix) * Vec3::from(v.position) + translation(matrix))
                    .max_by(|a, b| a.y.total_cmp(&b.y))
                    .unwrap();
                if idle.coatings[part].amount > 0.08 && idle.coatings[part].wet > 0.0 {
                    assert!(
                        flow_drops
                            .iter()
                            .any(|d| d.position.distance(bottom) < 0.01),
                        "wet coating must drip at the lowest edge"
                    );
                }
            }
            let mut wet_vertices = Vec::new();
            flowing.append(&mut wet_vertices, &transforms, &flow_settings, 0.5);
            assert!(
                wet_vertices
                    .iter()
                    .any(|v| f32::from_bits(v.material) > 0.0)
            );
            assert!(wet_vertices.iter().all(
                |v| f32::from_bits(v.page).is_finite() && f32::from_bits(v.palette).is_finite()
            ));
            for coating in flowing.coatings.values_mut() {
                coating.wet = 0.0;
            }
            let dry = flowing
                .coatings
                .iter()
                .map(|(&key, c)| (key, c.flow))
                .collect::<BTreeMap<_, _>>();
            flow_drops.clear();
            game.tick += 60;
            flowing.tick(
                &game,
                &models,
                &transforms,
                &BTreeSet::new(),
                &mut flow_drops,
                &flow_settings,
            );
            assert!(flow_drops.is_empty());
            assert!(flowing.coatings.iter().all(|(key, c)| dry[key] == c.flow));
            let mut vertices = Vec::new();
            moving.append(&mut vertices, &transforms, &graphics.blood, 0.5);
            assert!(!vertices.is_empty());
            assert!(
                vertices
                    .iter()
                    .all(|v| v.position.iter().chain(&v.normal).all(|n| n.is_finite()))
            );
        }
    }
}
