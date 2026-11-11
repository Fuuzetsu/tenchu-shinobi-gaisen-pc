use super::*;
use serde::{Deserialize, Serialize};
use tenchu_core::tenchu2::world::gaisen::Director;

const SIGNATURE: &str = "native-tenchu2-jp-session-1";

#[derive(Serialize, Deserialize)]
struct State<C, D> {
    signature: String,
    core: C,
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

type Owned = State<(World, Assets, Frame), Director>;

impl Session {
    fn snapshot(&self) -> State<(&World, &Assets, &Frame), &Director> {
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

    pub fn capture_state(&self) -> Result<Vec<u8>> {
        crate::savestate::encode(&self.snapshot())
    }

    pub fn restore_state(&mut self, bytes: &[u8]) -> Result<()> {
        let saved: Owned = crate::savestate::decode(bytes)?;
        saved.validate(self)?;
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
            gaisen,
            ..
        } = saved;
        let gaisen = gaisen.map(|saved| {
            let mut scene = self.gaisen.take().expect("validated imported stage");
            saved.restore(&mut scene);
            scene
        });
        *self = Self {
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
        };
        Ok(())
    }
}

impl Owned {
    fn validate(&self, current: &Session) -> Result<()> {
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
            || self.navigation.is_some_and(|input| {
                !input.yaw.is_finite() || input.sticks.iter().any(|v| !v.is_finite())
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
            let definition = assets
                .resources
                .registry
                .actor(actor.definition_kind())
                .ok_or("Missing native T2 actor definition")?;
            let model = assets
                .models
                .get(&definition.model)
                .ok_or("Missing native T2 actor model")?;
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
        }
        self.models.validate_state(world, assets)?;
        if let Some(saved) = &self.gaisen {
            saved.validate(current.gaisen.as_ref())?;
        }
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
