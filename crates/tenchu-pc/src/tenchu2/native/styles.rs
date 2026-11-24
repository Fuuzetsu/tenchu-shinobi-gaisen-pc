use super::Session;
use crate::settings::{Settings, styles};
use crate::visuals::{appearance::library::Library, pack::Pack};
use std::sync::Arc;
use tenchu_assets::Result;
use tenchu_core::tenchu2::actor::State;

impl Session {
    pub(crate) fn switch_combat_style(
        &mut self,
        settings: &mut Settings,
        donor: &mut tenchu_core::game::Game,
        library: &Library,
        bodies: &Arc<Pack>,
    ) -> Result<String> {
        let actor = &self.world.actors[self.world.player].actor;
        if self.cinematic()
            || self.world.script != 0
            || !self.world.player_can_steer()
            || !matches!(actor.state, State::Normal | State::Ready)
            || actor.terrain.height > i32::from(actor.height)
        {
            return Err("Combat style unchanged: stand or move on the ground after the current action finishes.".into());
        }
        let (id, name) = styles::next(settings, library, super::models::compatible_moveset)?;
        let owner = self.world.player;
        let anchor = self.world.instances[owner]
            .rig
            .root_position(&self.world.actors[owner].actor, &mut self.world.scene)?;
        let previous = settings.clone();
        styles::select(settings, id);
        if let Err(error) = self.models.sync(
            &mut self.world,
            &mut self.assets,
            donor,
            library,
            bodies,
            settings,
        ) {
            *settings = previous;
            let _ = self.models.sync(
                &mut self.world,
                &mut self.assets,
                donor,
                library,
                bodies,
                settings,
            );
            return Err(error);
        }
        let root = self.world.instances[owner]
            .rig
            .root_position(&self.world.actors[owner].actor, &mut self.world.scene)?;
        // Replacing the idle pose must not accumulate its bind-root offset on every press.
        let position = &mut self.world.actors[owner].actor.position;
        position.x = position.x.wrapping_add(anchor.x.wrapping_sub(root.x));
        position.z = position.z.wrapping_add(anchor.z.wrapping_sub(root.z));
        self.world.publish(owner, &self.assets);
        self.world
            .collision
            .compute(|model| self.world.scene.world(model))?;
        self.world.scene.finish_tick();
        self.reset_interpolation()?;
        Ok(format!("Combat style: {name}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tenchu_core::{
        game::sheathing::{Settings as Sheathing, UnarmedStyle},
        tenchu2::input::ATTACK,
    };

    #[test]
    #[ignore = "requires both discs; native style cycling preserves hero/body/equipment/unarmed state and rejects attacks/air/scenes"]
    fn native_combat_style_cycle_preserves_identity_and_safe_transitions() {
        let gaisen =
            crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let mut library = Library::load(&gaisen).unwrap();
        library.load_tenchu2(&disc).unwrap();
        let mut donor = super::super::models::donor(&gaisen).unwrap();
        for sequence in [9, 119, 191] {
            for drawn in [false, true] {
                let mut session = super::super::sheathing_tests::ready(sequence);
                let mut settings = Settings {
                    player_model: "disc:0".into(),
                    weapon_model: "tenchu2-jp:kagami".into(),
                    voice_model: "tenchu2-jp:ayame".into(),
                    combat_styles: [
                        Some(String::new()),
                        Some("disc:1".into()),
                        Some("tenchu2-jp:rikimaru".into()),
                    ],
                    sheathing: Sheathing {
                        enabled: true,
                        unarmed: true,
                        unarmed_style: UnarmedStyle::Tatsumaru,
                        ..Default::default()
                    },
                    ..Default::default()
                };
                session
                    .configure_t2_unarmed(&disc, settings.sheathing)
                    .unwrap();
                session
                    .world
                    .set_player_sheathing(settings.sheathing, &session.assets)
                    .unwrap();
                session
                    .models
                    .sync(
                        &mut session.world,
                        &mut session.assets,
                        &mut donor,
                        &library,
                        &Arc::default(),
                        &settings,
                    )
                    .unwrap();
                super::super::sheathing_tests::weapon(&mut session, drawn);
                let hero = session.assets.hero;
                let preferences = settings.clone();
                for index in 0..6 {
                    if index == 3 {
                        for _ in 0..4 {
                            session.step(tenchu_core::tenchu2::input::UP).unwrap();
                        }
                    }
                    let world = &mut session.world;
                    let actor = &world.actors[world.player].actor;
                    let point = world.instances[world.player]
                        .rig
                        .root_position(actor, &mut world.scene)
                        .unwrap();
                    let identity = (
                        actor.kind,
                        actor.mission_kind(),
                        actor.position,
                        actor.health,
                        actor.inventory.clone(),
                    );
                    let unarmed = postcard::to_allocvec(&actor.unarmed).unwrap();
                    let sheathing = postcard::to_allocvec(&actor.sheathing).unwrap();
                    let random = world.random;
                    let npcs: Vec<_> = world
                        .actors
                        .iter()
                        .enumerate()
                        .filter(|(id, _)| *id != world.player)
                        .map(|(_, actor)| postcard::to_allocvec(actor).unwrap())
                        .collect();
                    assert!(
                        session
                            .switch_combat_style(
                                &mut settings,
                                &mut donor,
                                &library,
                                &Arc::default()
                            )
                            .unwrap()
                            .starts_with("Combat style: ")
                    );
                    let after = session.world.instances[session.world.player]
                        .rig
                        .root_position(
                            &session.world.actors[session.world.player].actor,
                            &mut session.world.scene,
                        )
                        .unwrap();
                    assert_eq!(
                        (after.x, after.z),
                        (point.x, point.z),
                        "root before {point:?}, after {after:?}"
                    );
                    let world = &session.world;
                    let actor = &world.actors[world.player].actor;
                    assert_eq!(
                        (
                            actor.kind,
                            actor.mission_kind(),
                            actor.position.y,
                            actor.health
                        ),
                        (identity.0, identity.1, identity.2.y, identity.3)
                    );
                    assert_eq!(
                        postcard::to_allocvec(&actor.inventory).unwrap(),
                        postcard::to_allocvec(&identity.4).unwrap()
                    );
                    assert_eq!(world.random, random);
                    assert_eq!(actor.weapon_drawn(), drawn);
                    assert_eq!(postcard::to_allocvec(&actor.unarmed).unwrap(), unarmed);
                    assert_eq!(postcard::to_allocvec(&actor.sheathing).unwrap(), sheathing);
                    assert_eq!(
                        world
                            .actors
                            .iter()
                            .enumerate()
                            .filter(|(id, _)| *id != world.player)
                            .map(|(_, actor)| postcard::to_allocvec(actor).unwrap())
                            .collect::<Vec<_>>(),
                        npcs
                    );
                    assert_eq!(session.assets.hero, hero);
                    assert!(session.models.current(&settings));
                    assert_eq!(settings.player_model, preferences.player_model);
                    assert_eq!(settings.weapon_model, preferences.weapon_model);
                    assert_eq!(settings.voice_model, preferences.voice_model);
                    assert_eq!(settings.sheathing, preferences.sheathing);
                    assert_eq!(
                        postcard::to_allocvec(&actor.attack).unwrap(),
                        postcard::to_allocvec(&tenchu_core::tenchu2::actor::Attack::default())
                            .unwrap()
                    );
                }
                session.step(ATTACK).unwrap();
                let before = postcard::to_allocvec(&session.world).unwrap();
                assert!(
                    session
                        .switch_combat_style(&mut settings, &mut donor, &library, &Arc::default())
                        .is_err()
                );
                assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
                for _ in 0..120 {
                    session.step(0).unwrap();
                    if session.world.player_can_steer() {
                        break;
                    }
                }
                session
                    .switch_combat_style(&mut settings, &mut donor, &library, &Arc::default())
                    .unwrap_or_else(|error| {
                        let actor = &session.world.actors[session.world.player].actor;
                        panic!("{error}: sequence {sequence}, drawn {drawn}, action {:x}, state {:?}, health {}, flags {:x}, terrain {}, height {}, script {}", actor.motion.action, actor.state, actor.health, actor.flags, actor.terrain.height, actor.height, session.world.script);
                    });
                if sequence == 9 && drawn {
                    let saved = session.capture_state().unwrap();
                    let mut recorder = crate::rewind::Recorder::default();
                    recorder.begin();
                    session.snapshot().capture(&mut recorder).unwrap();
                    let mut rewind: super::super::state::Owned =
                        recorder.finish().unwrap().decode().unwrap();
                    for _ in 0..10 {
                        session.step(0).unwrap();
                    }
                    let expected = session.capture_state().unwrap();
                    assert!(rewind.reusable(&session).unwrap());
                    rewind.swap(&mut session);
                    rewind.finish_reuse(&mut session);
                    assert_eq!(session.capture_state().unwrap(), saved);
                    for _ in 0..10 {
                        session.step(0).unwrap();
                    }
                    assert_eq!(session.capture_state().unwrap(), expected);
                    session.restore_state(&saved).unwrap();
                    for _ in 0..10 {
                        session.step(0).unwrap();
                    }
                    assert_eq!(session.capture_state().unwrap(), expected);
                }
                let ready = session.world.clone();
                for action in [0x110, 0x200, 0x600, 0x700, 0x800, 0x900, 0xa00, 0xb00] {
                    session.world = ready.clone();
                    session.world.actors[session.world.player]
                        .actor
                        .motion
                        .action = action;
                    session.world.actors[session.world.player].actor.state =
                        State::from_action(action).unwrap();
                    let before = postcard::to_allocvec(&session.world).unwrap();
                    assert!(
                        session
                            .switch_combat_style(
                                &mut settings,
                                &mut donor,
                                &library,
                                &Arc::default()
                            )
                            .is_err(),
                        "{action:x}"
                    );
                    assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
                }
                session.world = ready;
                session.world.script = 3;
                let before = postcard::to_allocvec(&session.world).unwrap();
                assert!(
                    session
                        .switch_combat_style(&mut settings, &mut donor, &library, &Arc::default())
                        .is_err()
                );
                assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
            }
        }
    }
}
