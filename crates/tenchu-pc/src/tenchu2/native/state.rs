use super::*;
use serde::{Deserialize, Serialize};
use tenchu_core::tenchu2::world::gaisen::Director;

const SIGNATURE: &str = "native-tenchu2-jp-session-2";

mod reuse;

#[derive(Serialize, Deserialize)]
pub(super) struct State<C, D> {
    signature: String,
    pub(super) core: C,
    feedback: tenchu_core::tenchu2::feedback::Feedback,
    contact: contact::History,
    bank: Arc<tenchu_assets::audio::bank::Bank>,
    actor_voices: BTreeMap<usize, Option<Arc<VoiceSet>>>,
    pending: BTreeSet<String>,
    interpolation: Option<crate::visuals::interpolation::History>,
    navigation: Option<navigation::Input>,
    circle_action: tenchu_core::input::CircleAction,
    models: models::Override,
    gaisen: Option<gaisen::State<D>>,
}

pub(super) type Owned = State<(World, Assets, Frame), Director>;
pub(super) type Borrowed<'a> = State<(&'a World, &'a Assets, &'a Frame), &'a Director>;

impl Borrowed<'_> {
    pub(super) fn capture<C: tenchu_assets::checkpoint::Capture>(
        &self,
        out: &mut C,
    ) -> std::result::Result<(), C::Error> {
        let Self {
            signature,
            core: (world, assets, frame),
            feedback,
            contact,
            bank,
            actor_voices,
            pending,
            interpolation,
            navigation,
            circle_action,
            models,
            gaisen,
        } = self;
        out.value(signature)?;
        world.capture(out)?;
        assets.capture(out)?;
        out.value(frame)?;
        out.value(feedback)?;
        out.value(contact)?;
        out.shared_arc(bank, true)?;
        out.value(&actor_voices.len())?;
        for (actor, voice) in actor_voices {
            out.value(actor)?;
            out.value(&voice.is_some())?;
            if let Some(voice) = voice {
                out.shared_arc(voice, true)?;
            }
        }
        out.value(pending)?;
        out.value(interpolation)?;
        out.value(navigation)?;
        out.value(circle_action)?;
        models.capture(out)?;
        out.value(&gaisen.is_some())?;
        if let Some(gaisen) = gaisen {
            gaisen.capture(out)?;
        }
        Ok(())
    }
}

impl Session {
    pub(super) fn snapshot(&self) -> Borrowed<'_> {
        let Self {
            world,
            assets,
            frame,
            feedback,
            contact,
            bank,
            actor_voices,
            pending,
            interpolation,
            navigation,
            circle_action,
            gaisen,
            models,
        } = self;
        State {
            signature: SIGNATURE.into(),
            core: (world, assets, frame),
            feedback: *feedback,
            contact: contact.clone(),
            bank: bank.clone(),
            actor_voices: actor_voices.clone(),
            pending: pending.clone(),
            interpolation: interpolation.clone(),
            navigation: *navigation,
            circle_action: *circle_action,
            models: models.clone(),
            gaisen: gaisen.as_ref().map(gaisen::Scene::snapshot),
        }
    }

    #[cfg(test)]
    pub fn capture_state(&self) -> Result<Vec<u8>> {
        crate::savestate::encode(&self.snapshot())
    }

    #[cfg(test)]
    pub fn restore_state(&mut self, bytes: &[u8]) -> Result<()> {
        let mut saved: Owned = crate::savestate::decode(bytes)?;
        saved.validate()?;
        if let Some(gaisen) = &saved.gaisen {
            gaisen.validate(self.gaisen.as_ref())?;
        }
        let gaisen = saved.gaisen.take().map(|saved| {
            let mut scene = self.gaisen.take().expect("validated imported stage");
            saved.restore(&mut scene);
            scene
        });
        *self = saved.build(gaisen);
        Ok(())
    }
}

impl Owned {
    pub(super) fn restore(mut self, source: Option<&gaisen::Source>) -> Result<Session> {
        self.validate()?;
        if self.gaisen.is_some() != source.is_some() {
            return Err(
                "Open the matching native campaign or imported T1 stage before loading this state"
                    .into(),
            );
        }
        let gaisen = self
            .gaisen
            .take()
            .map(|saved| saved.create(source.unwrap()))
            .transpose()?;
        Ok(self.build(gaisen))
    }

    fn build(self, gaisen: Option<gaisen::Scene>) -> Session {
        let State {
            core: (world, assets, frame),
            feedback,
            contact,
            bank,
            actor_voices,
            pending,
            interpolation,
            navigation,
            circle_action,
            models,
            ..
        } = self;
        Session {
            world,
            assets,
            frame,
            feedback,
            contact,
            bank,
            actor_voices,
            pending,
            interpolation,
            navigation,
            circle_action,
            models,
            gaisen,
        }
    }
}

impl Owned {
    fn validate(&self) -> Result<()> {
        let (world, assets, _) = &self.core;
        if self.signature != SIGNATURE
            || assets.resources.registry.edition != tenchu_assets::tenchu2::Edition::Japan
            || assets.hero >= 3
            || world.player >= world.actors.len()
            || world.actors.len() != world.instances.len()
            || !(1..=3).contains(&world.difficulty)
            || world.map != assets.level.id
            || usize::from(world.sequence) != assets.sequence
            || assets.sequence >= assets.campaign.sequences.len()
            || assets.level.vram.words.len() != 1024 * 512
            || assets.weapon_vram.words.len() != 1024 * 512
            || assets.sprites.vram.words.len() != 1024 * 512
            || assets
                .models
                .values()
                .any(|model| model.vram.words.len() != 1024 * 512)
            || self.actor_voices.keys().any(|&id| id >= world.actors.len())
            || self.contact.gunshots.len() > contact::GUNSHOT_LIMIT
            || self.contact.gunshots.iter().any(|(at, _)| {
                u64::from(world.tick)
                    .checked_sub(*at)
                    .is_none_or(|age| age >= contact::GUNSHOT_TICKS)
            })
            || self.navigation.is_some_and(|input| {
                !input.yaw.is_finite() || input.sticks.iter().any(|v| !v.is_finite())
            })
            || world.controls.walking.is_some_and(|walking| {
                !assets
                    .resources
                    .clips
                    .get(&walking.clip)
                    .is_some_and(|clip| {
                        clip.duration > 0 && clip.forward_speed > 0 && !clip.bones.is_empty()
                    })
            })
        {
            return Err("Invalid native T2 session identity or player".into());
        }
        world
            .collision
            .validate_references(world.scene.models.len(), world.actors.len())?;
        for model in &world.scene.models {
            let mut parent = model.parent;
            for depth in 0..=world.scene.models.len() {
                let Some(id) = parent else { break };
                if id >= world.scene.models.len() || depth == world.scene.models.len() {
                    return Err("Invalid native T2 scene hierarchy".into());
                }
                parent = world.scene.models[id].parent;
            }
        }
        for (agent, instance) in world.actors.iter().zip(&world.instances) {
            let actor = &agent.actor;
            if let Some(state) = actor.sheathing {
                let mut settings = state.settings;
                settings.validate();
                if !settings.enabled || settings != state.settings {
                    return Err("Invalid native T2 sheathing settings".into());
                }
            }
            let definition = assets
                .resources
                .registry
                .actor(actor.definition_kind())
                .ok_or("Missing native T2 actor definition")?;
            let model = assets
                .models
                .get(&definition.model)
                .ok_or("Missing native T2 actor model")?;
            if let Some(unarmed) = &actor.unarmed {
                let moves = &unarmed.moves;
                if !moves.motions.contains_key(&0x900)
                    || moves.motions.len() > 32
                    || moves.attacks.len() > 32
                    || moves.motions.iter().any(|(&action, &clip)| {
                        !matches!(action, 0x900..=0x910 | 0x913..=0x915)
                            || !assets.resources.clips.contains_key(&clip)
                            || !moves.attacks.contains_key(&action)
                            || assets.resources.weapons.attack(clip).is_none()
                    })
                    || moves.attacks.values().any(|attack| {
                        attack.contacts[0].is_none()
                            || attack.contacts.iter().flatten().any(|c| {
                                c.part >= instance.rig.joints.len() || c.hitbox.radius <= 0
                            })
                            || attack
                                .combo
                                .iter()
                                .flatten()
                                .any(|a| !moves.motions.contains_key(a))
                    })
                    || unarmed.active
                        && (actor.weapon_drawn()
                            || actor.state != tenchu_core::tenchu2::actor::State::Attack)
                {
                    return Err("Invalid native T2 unarmed moveset".into());
                }
            }
            if instance.rig.root >= world.scene.models.len()
                || instance.rig.joints.len() < model.parts.len()
                || instance
                    .rig
                    .joints
                    .iter()
                    .any(|&id| id >= world.scene.models.len())
                || instance
                    .rig
                    .weapons
                    .iter()
                    .flatten()
                    .any(|&id| id >= world.scene.models.len())
                || actor
                    .motion
                    .clip
                    .is_some_and(|id| !assets.resources.clips.contains_key(&id))
                || actor.target.is_some_and(|id| id >= world.actors.len())
            {
                return Err("Invalid native T2 actor rig or motion".into());
            }
            if let Some(death) = &actor.dismemberment {
                if death.pending.is_some() && death.part.is_some() {
                    return Err("Invalid native T2 severing decision".into());
                }
                if let Some(part) = &death.part {
                    let parent = crate::visuals::appearance::parents(&model.parts)
                        .get(part.index)
                        .copied()
                        .flatten()
                        .and_then(|p| instance.rig.joints.get(p).copied());
                    if !matches!(part.index, 2 | 4 | 10)
                        || actor.health != 0
                        || parent.is_none()
                        || part.parent != parent
                        || instance
                            .rig
                            .joints
                            .get(part.index)
                            .is_none_or(|&joint| world.scene.models[joint].parent != Some(0))
                    {
                        return Err("Invalid native T2 detached part".into());
                    }
                }
            }
        }
        let mut dismemberment = world.controls.dismemberment;
        dismemberment.validate();
        if dismemberment != world.controls.dismemberment {
            return Err("Invalid native T2 dismemberment settings".into());
        }
        self.models.validate_state(world, assets)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU2_DISC; damaged and mismatched snapshots reject transactionally"]
    fn native_session_snapshot_rejects_invalid_state_without_mutation() {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let mut session = Session::load(&disc, 9, 17).unwrap();
        session.step(0).unwrap();
        session
            .configure_tatsumaru(
                &disc,
                tenchu_core::game::sheathing::Settings {
                    enabled: true,
                    unarmed: true,
                    unarmed_style: tenchu_core::game::sheathing::UnarmedStyle::Tatsumaru,
                    ..Default::default()
                },
            )
            .unwrap();
        let original = session.capture_state().unwrap();
        let rejected = |session: &mut Session, bytes: &[u8]| {
            assert!(session.restore_state(bytes).is_err());
            assert!(session.capture_state().unwrap() == original);
        };
        rejected(&mut session, &original[..original.len() - 1]);
        rejected(
            &mut session,
            &crate::savestate::encode(&"another engine").unwrap(),
        );
        for change in [
            (|s: &mut Owned| s.signature.clear()) as fn(&mut Owned),
            |s| s.core.0.player = usize::MAX,
            |s| s.core.1.hero = 3,
            |s| {
                s.core.0.instances.pop();
            },
            |s| s.core.0.scene.models[0].parent = Some(0),
            |s| s.core.0.instances[0].rig.root = usize::MAX,
            |s| s.core.0.instances[0].rig.weapons[0] = Some(usize::MAX),
            |s| s.core.0.actors[0].actor.motion.clip = Some(i16::MAX),
            |s| {
                Arc::make_mut(&mut s.core.0.actors[0].actor.unarmed.as_mut().unwrap().moves)
                    .motions
                    .clear();
            },
            |s| {
                s.core.0.actors[0].actor.unarmed.as_mut().unwrap().active = true;
            },
            |s| s.core.1.level.vram.words.clear(),
            |s| {
                s.navigation = Some(navigation::Input {
                    yaw: f32::NAN,
                    pitch: 0,
                    sticks: [glam::Vec2::ZERO; 2],
                    deadzone: 20,
                    align_aim: false,
                })
            },
            |s| {
                s.actor_voices.insert(usize::MAX, None);
            },
        ] {
            let mut saved: Owned = crate::savestate::decode(&original).unwrap();
            change(&mut saved);
            rejected(&mut session, &crate::savestate::encode(&saved).unwrap());
        }
    }
}
