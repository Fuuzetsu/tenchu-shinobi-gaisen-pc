use super::interpolation::{Frame, Part};
use std::collections::BTreeMap;
use tenchu_assets::{ReadAt, Result, executable::Executable, game::GameAssets, motion::Clip};
use tenchu_core::{actor::State, game::Game, ids::*, math::Matrix, motion::Motion};

pub mod blood;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct Deaths {
    clips: Vec<Clip>,
    death: Option<Death>,
    pub frame: Option<Frame>,
}

#[derive(serde::Serialize, serde::Deserialize, Clone)]
struct Death {
    started: u64,
    tick: u64,
    root: usize,
    clip: usize,
    motion: Option<Motion>,
    settled: Option<Matrix>,
    pose: Matrix,
    blood: Option<blood::Blood>,
}

impl Deaths {
    pub fn load(assets: &GameAssets<impl ReadAt>) -> Result<Self> {
        let characters = Executable::new(&assets.executable)?.characters()?;
        let mut clips = tenchu_assets::motion::parse(&assets.asset("HUMAN/MOTION/STAGE2.AMD")?)?;
        clips.extend(tenchu_assets::motion::parse(
            &assets.asset("HUMAN/MOTION/RIKIMARU.AMD")?,
        )?);
        let mut deaths = Vec::<Clip>::new();
        for character in characters
            .iter()
            .filter(|c| matches!(c.kind, 0 | 1 | 0x20..=0x22))
        {
            for registration in character
                .motions
                .iter()
                .filter(|r| matches!(r.action, MOT_DEAD | MOT_DEAD_ALT))
            {
                if deaths.iter().any(|c| c.id == registration.clip) {
                    continue;
                }
                deaths.push(
                    clips
                        .iter()
                        .find(|c| c.id == registration.clip)
                        .ok_or("Missing player/ninja death motion")?
                        .clone(),
                );
            }
        }
        Ok(Self {
            clips: deaths,
            death: None,
            frame: None,
        })
    }

    pub fn reset(&mut self) {
        self.death = None;
        self.frame = None;
    }

    pub fn tick(&mut self, game: &Game, chance: u16) -> Result<()> {
        let actor = game.player();
        if actor.health != 0 {
            self.reset();
            return Ok(());
        }
        let Some(clip) = actor.motion.clip else {
            self.reset();
            return Ok(());
        };
        if self
            .death
            .as_ref()
            .is_some_and(|death| death.root != actor.root_model || death.tick > game.tick)
        {
            self.reset();
        }
        if self.death.is_none() {
            let eligible = !game.cinematic.active
                && !actor.scripted
                && game.moveset.kind.is_none()
                && matches!(actor.kind, 0 | 1)
                && actor.status == State::Dead
                && matches!(actor.motion.action, MOT_DEAD | MOT_DEAD_ALT)
                && actor.map.height <= 0
                && actor.map.attribute & 0x10c == 0;
            let random = roll(game.random.seed, game.tick);
            let original = &game.mission.clips[clip];
            let variants = || {
                self.clips
                    .iter()
                    .enumerate()
                    .filter(|(_, c)| c.id != original.id)
            };
            let count = variants().count();
            let selected = eligible && count != 0 && random % 100 < u64::from(chance.min(100));
            let motion = selected.then(|| {
                let mut motion = actor.motion.clone();
                motion.clip = variants()
                    .nth((random / 100 % count as u64) as usize)
                    .map(|(i, _)| i);
                motion.mask = u32::MAX;
                motion
            });
            let age = if actor.motion.loops != 0 {
                motion.as_ref().map_or(0, |motion| {
                    let clip = &self.clips[motion.clip.unwrap()];
                    (clip.duration - i16::from(clip.sweep)) as u64
                })
            } else {
                (actor.motion.count - i16::from(original.sweep) - 1).max(0) as u64
            };
            self.death = Some(Death {
                started: game.tick.saturating_sub(age),
                tick: game.tick,
                root: actor.root_model,
                clip,
                motion,
                settled: None,
                pose: actor.root(&game.math),
                blood: selected.then(|| blood::Blood::new(game)),
            });
        }
        let death = self.death.as_mut().unwrap();
        if death.clip != clip || actor.scripted || game.cinematic.active {
            death.motion = None;
            self.frame = None;
        }
        if death.tick == game.tick && self.frame.is_some() {
            return Ok(());
        }
        let advance = death.tick != game.tick;
        death.tick = game.tick;
        let Some(motion) = &mut death.motion else {
            return Ok(());
        };
        let alternate = &self.clips[motion.clip.unwrap()];
        // Play at the authored speed, then hold the corpse through the original
        // death countdown. No simulation motion, effect timing or RNG is changed.
        let age = (game.tick - death.started).min(i16::MAX as u64) as i16;
        motion.count = age
            .saturating_add(i16::from(alternate.sweep))
            .min(alternate.duration);
        if advance && death.settled.is_none() {
            let velocity = game.math.velocity(
                actor.rotation[1],
                i16::from(alternate.forward_speed),
                i16::from(alternate.side_speed),
            );
            let mut position = death.pose.translation;
            position.x += i32::from(velocity[0]);
            position.z += i32::from(velocity[2]);
            let ground = tenchu_core::terrain::vector(
                &game.mission.terrain,
                &mut tenchu_core::terrain::Cursor::new(&game.mission.terrain),
                position,
                i32::from(game.mission.characters[actor.definition].width),
                tenchu_core::terrain::STEP_DOWN,
            )?;
            if ground.vector == 0
                && ground.level != tenchu_core::terrain::NONE
                && ground.height.abs() <= 500
                && ground.attribute & 12 == 0
            {
                position.y = ground.level;
                death.pose.translation = position;
            }
        }
        let emit = motion.count < alternate.duration && motion.count & 4 != 0;
        if death.settled.is_none() && (actor.map.height > 0 || actor.velocity[1] != 0) {
            death.pose.translation.y = actor.position.y;
        }
        if motion.count == alternate.duration && actor.map.height <= 0 && actor.velocity[1] == 0 {
            death.settled.get_or_insert(death.pose);
        }
        let root = death.pose;
        motion.loops = 0;
        motion.step(&self.clips, &game.math.tables.hermite)?;
        let mut matrices = motion.pose.matrices(&game.math, root);
        if let Some(severed) = &actor.severed {
            let mut detached = vec![false; actor.parts.len()];
            for (index, &model) in actor.parts.iter().enumerate() {
                if index == severed.index {
                    detached[index] = true;
                    matrices[index] = game.scene.models[model].local;
                } else if let Some(parent) = game.scene.models[model]
                    .parent
                    .and_then(|id| actor.parts.iter().position(|&part| part == id))
                    .filter(|&parent| parent < index && detached[parent])
                {
                    detached[index] = true;
                    matrices[index] = matrices[parent].compose(&game.scene.models[model].local);
                }
            }
        }
        let mut parts = BTreeMap::new();
        for inspection in [false, true] {
            let kind = if inspection {
                actor.mesh_kind
            } else {
                actor.rendered_mesh_kind
            };
            for (&model, &matrix) in actor.parts.iter().zip(&matrices) {
                parts.insert(
                    Part::Actor {
                        root: actor.root_model,
                        model,
                        kind,
                        inspection,
                    },
                    matrix,
                );
            }
            for weapon in actor.weapons.iter().flatten() {
                let mut id = weapon.model;
                let mut local = game.scene.models[id].local;
                while let Some(parent) = game.scene.models[id].parent {
                    if let Some(index) = actor.parts.iter().position(|&part| part == parent) {
                        parts.insert(
                            Part::Actor {
                                root: actor.root_model,
                                model: weapon.model,
                                kind: weapon.kind,
                                inspection,
                            },
                            matrices[index].compose(&local),
                        );
                        break;
                    }
                    local = game.scene.models[parent].local.compose(&local);
                    id = parent;
                }
            }
        }
        let mut frame = Frame::new(parts, game.rendered_view);
        if let Some(blood) = &mut death.blood {
            frame.death_blood =
                Some(blood.tick(game, matrices[1], emit, root, death.settled.is_some())?);
        }
        self.frame = Some(frame);
        Ok(())
    }
}

fn roll(seed: u32, tick: u64) -> u64 {
    let mut value = u64::from(seed) ^ tick;
    value = (value ^ (value >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    value = (value ^ (value >> 27)).wrapping_mul(0x94d049bb133111eb);
    value ^ (value >> 31)
}

#[cfg(test)]
pub(super) mod tests;
