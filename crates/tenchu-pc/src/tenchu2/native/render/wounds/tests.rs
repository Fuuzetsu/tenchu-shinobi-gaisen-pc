use super::*;
use tenchu_core::tenchu2::rig::WeaponMode;

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; source weapon contacts must stain native bodies without changing source state"]
fn native_model_stains_follow_hits_pose_and_shared_controls() {
    check_model_stains(None);
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW, TENCHU_MODEL_DIR and GPU; wounds must use displayed custom bodies"]
fn native_custom_model_stains_follow_hits_and_attachments() {
    let pack = Pack::bodies(std::env::var("TENCHU_MODEL_DIR").unwrap().as_str()).unwrap();
    check_model_stains(Some(&pack));
}

fn check_model_stains(custom: Option<&Pack>) {
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    let out = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    for hero in 0..3 {
        let mut session =
            crate::tenchu2::native::Session::load(&disc, [9, 119, 191][hero], 17).unwrap();
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        let player = session.world.player;
        let guard = session
            .world
            .actors
            .iter()
            .enumerate()
            .find(|(id, a)| {
                *id != player
                    && a.actor.health > 0
                    && session
                        .assets
                        .resources
                        .registry
                        .actor(a.actor.definition_kind())
                        .is_some_and(|definition| {
                            session
                                .assets
                                .resources
                                .weapons
                                .parts(definition.weapon)
                                .any(|part| part.radius != 0)
                        })
            })
            .unwrap()
            .0;
        // A flat combat fixture avoids authored terrain interrupting the hit rays.
        session.world.terrain = tenchu_assets::tenchu2::collision::Collision {
            origin: [-3000; 2],
            dimensions: [1; 2],
            cell_size: 6000,
            cells: vec![tenchu_assets::tenchu2::collision::Cell {
                minimum: i16::MIN,
                maximum: i16::MAX,
                starts: [0; 4],
                nodes: vec![tenchu_assets::terrain::Node {
                    y: 0,
                    dy: i16::MAX,
                    bounds: [-3000, -3000, 3000, 3000],
                    attributes: 0,
                    division: u16::MAX,
                }],
            }],
        };
        session.world.actors[player].actor.position = Position::ZERO;
        session.world.actors[player].actor.previous = Position::ZERO;
        session.world.actors[player].actor.rotation.y = 0;
        session.world.actors[player].actor.terrain = Default::default();
        session.world.cursor = Default::default();
        let actor = &session.world.actors[player].actor;
        let position = actor.position.wrapping_add(
            session
                .assets
                .resources
                .math
                .velocity(actor.rotation.y, 1000, 0)
                .map(i32::from)
                .into(),
        );
        let yaw = actor.rotation.y.wrapping_add(2048);
        session.world.actors[guard].behavior.has_suspicious_program = true;
        let enemy = &mut session.world.actors[guard].actor;
        enemy.flags &= !(8 | 0x10);
        enemy.health = 9999;
        enemy.max_health = 9999;
        enemy.position = position;
        enemy.previous = position;
        enemy.terrain = Default::default();
        enemy.rotation.y = yaw;
        enemy.target = Some(player);
        enemy
            .set_weapon(
                &mut session.world.instances[guard].rig,
                guard,
                &mut session.world.collision,
                &session.assets.resources,
                WeaponMode::Draw,
            )
            .unwrap();
        enemy
            .set_body_collision(
                session.world.instances[guard].rig.joints[0],
                guard,
                &mut session.world.collision,
                true,
            )
            .unwrap();
        session.world.publish(guard, &session.assets);
        session.world.actors[player].actor.rotation.y = yaw;
        session.world.publish(player, &session.assets);
        session.world.update_camera(&session.assets, true).unwrap();
        let root = session.world.instances[player].rig.root;
        session.world.actors[player].actor.health = 9999;
        session.world.actors[player].actor.max_health = 9999;
        session.interpolate(true).unwrap();
        let mut graphics = Graphics {
            fog: false,
            asset_pack: false,
            ..Default::default()
        };
        graphics.blood.model_stains = true;
        graphics.motion.hair = true;
        graphics.motion.cloth = true;
        graphics.motion.equipment = true;
        let vram = session.assets.sprites.vram.clone();
        let mut saved = None;
        let mut camera = None;
        super::super::capture_frame(
            &vram,
            &graphics,
            &out.join(format!("hero-{hero}-stained.png")),
            |renderer| {
                let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                if let Some(pack) = custom {
                    drawing
                        .models
                        .custom("0000", pack, &session, &graphics, renderer)?;
                }
                for (attacker, victim) in [(guard, player), (player, guard)] {
                    weapon_hit(&mut session, attacker, victim)?;
                    assert!(
                        session.contact.wounds.iter().any(|(id, _)| *id == victim),
                        "damage controller emitted no blood for hero {hero}, actor {victim}"
                    );
                    let state = postcard::to_allocvec(&session.world).unwrap();
                    drawing.tick(&session, &graphics)?;
                    assert!(
                        state == postcard::to_allocvec(&session.world).unwrap(),
                        "stains changed native world"
                    );
                }
                for _ in 0..60 {
                    session.step(0)?;
                    let state = postcard::to_allocvec(&session.world).unwrap();
                    drawing.tick(&session, &graphics)?;
                    assert!(
                        state == postcard::to_allocvec(&session.world).unwrap(),
                        "stains changed native world"
                    );
                }
                assert!(session.world.actors[player].actor.health < 9999);
                let blood = &drawing.scene.environment.as_ref().unwrap().blood;
                let parts: BTreeSet<_> = blood.attached_parts().collect();
                assert!(
                    !parts.is_empty(),
                    "hero {hero}: real hits produced no body stains"
                );
                assert!(
                    parts
                        .iter()
                        .any(|key| matches!(key, PartId::Actor { root: r, .. } if *r == root)),
                    "player was not stained"
                );
                assert!(
                    parts.iter().all(|key| {
                        let PartId::Actor { root, model, .. } = key else {
                            return false;
                        };
                        [player, guard].iter().any(|&id| {
                            let rig = &session.world.instances[id].rig;
                            *root == rig.root && rig.joints.contains(model)
                        })
                    }),
                    "body wounds reached unrelated actors or weapons"
                );
                drawing.update(&session, renderer, None)?;
                let stored =
                    postcard::to_allocvec(&drawing.scene.environment.as_ref().unwrap().blood)
                        .unwrap();
                for alpha in [0.0, 0.5, 1.0] {
                    drawing.alpha = alpha;
                    drawing.update(&session, renderer, None)?;
                }
                assert!(
                    stored
                        == postcard::to_allocvec(
                            &drawing.scene.environment.as_ref().unwrap().blood
                        )
                        .unwrap(),
                    "redraw changed wound state"
                );
                let center = point(session.world.actors[player].actor.position) - Vec3::Y * 1000.0;
                let mut view =
                    crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
                view.position = center + Vec3::new(1500.0, -300.0, 2200.0);
                let forward = (center - view.position).normalize();
                view.yaw = forward.x.atan2(forward.z);
                view.pitch = (-forward.y).asin();
                camera = Some(view);
                drawing.camera = Some(view);
                drawing.show_hud = false;
                drawing.update(&session, renderer, None)?;
                let blood = &drawing.scene.environment.as_ref().unwrap().blood;
                let mut attached = geometry(
                    blood,
                    &session,
                    &drawing.models,
                    None,
                    &Default::default(),
                    view.matrix(1280.0 / 720.0),
                )?;
                assert!(!attached.vertices.is_empty());
                assert!(
                    attached
                        .vertices
                        .iter()
                        .all(|v| Vec3::from(v.position).is_finite())
                );
                let before = attached.vertices.clone();
                let state = session.world.clone();
                for id in [player, guard] {
                    session.world.actors[id].actor.position.x += 700;
                    session.world.publish(id, &session.assets);
                }
                attached = geometry(
                    blood,
                    &session,
                    &drawing.models,
                    None,
                    &Default::default(),
                    view.matrix(1280.0 / 720.0),
                )?;
                assert_eq!(before.len(), attached.vertices.len());
                for (a, b) in before.iter().zip(&attached.vertices) {
                    assert!(
                        (Vec3::from(b.position) - Vec3::from(a.position) - Vec3::X * 700.0)
                            .length()
                            < 0.5,
                        "wound remained at old world position"
                    );
                }
                session.world = state;
                saved = Some(drawing.scene.environment.as_ref().unwrap().blood.clone());
                Ok((drawing.scene, view.source_view(), Vec::new()))
            },
        )
        .unwrap();
        super::super::capture_frame(
            &vram,
            &graphics,
            &out.join(format!("hero-{hero}-clean.png")),
            |renderer| {
                let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                if let Some(pack) = custom {
                    drawing
                        .models
                        .custom("0000", pack, &session, &graphics, renderer)?;
                }
                drawing.scene.environment.as_mut().unwrap().blood = saved.take().unwrap();
                drawing.camera = camera;
                drawing.show_hud = false;
                let mut disabled = graphics.clone();
                disabled.blood.model_stains = false;
                renderer.configure(&disabled);
                drawing.update(&session, renderer, None)?;
                assert!(
                    drawing
                        .scene
                        .environment
                        .as_ref()
                        .unwrap()
                        .blood
                        .attached_parts()
                        .next()
                        .is_none(),
                    "paused disable retained model stains"
                );
                renderer.configure(&graphics);
                drawing.update(&session, renderer, None)?;
                assert!(
                    drawing
                        .scene
                        .environment
                        .as_ref()
                        .unwrap()
                        .blood
                        .attached_parts()
                        .next()
                        .is_none(),
                    "toggle replayed an old hit"
                );
                Ok((drawing.scene, camera.unwrap().source_view(), Vec::new()))
            },
        )
        .unwrap();
    }
}

fn weapon_hit(
    session: &mut crate::tenchu2::native::Session,
    attacker: usize,
    victim: usize,
) -> Result<()> {
    use tenchu_core::{
        random::Random,
        tenchu2::{
            environment,
            world::controls::{Event, EventSource, Events},
        },
    };
    #[derive(Default)]
    struct Hits(Vec<(usize, Position)>);
    impl environment::Events for Hits {
        fn emit(&mut self, _: environment::Event, _: &mut Random) -> Result<()> {
            Ok(())
        }
    }
    impl Events for Hits {
        fn emit(&mut self, _: EventSource<'_>, _: Event, _: &mut Random) -> Result<()> {
            Ok(())
        }
        fn blood(&mut self, actor: usize, position: Position) {
            self.0.push((actor, position));
        }
    }
    use tenchu_core::tenchu2::{actor::Velocity, collision::WEAPON, physics::HIT};
    // Supply a contact from an actual equipped weapon, then let the original
    // damage/hurt controllers choose the health change, joint, sound and spurt.
    let world = &mut session.world;
    let assets = &mut session.assets;
    for (id, mode) in [(attacker, WeaponMode::Draw), (victim, WeaponMode::Sheathe)] {
        let actor = &mut world.actors[id].actor;
        actor.change_motion(
            if id == attacker { 0x900 } else { 0 },
            Velocity::Stop,
            &assets.resources,
        )?;
        actor.set_weapon(
            &mut world.instances[id].rig,
            id,
            &mut world.collision,
            &assets.resources,
            mode,
        )?;
        actor.attack.index = assets
            .resources
            .weapons
            .attacks
            .iter()
            .position(|a| Some(a.clip) == actor.motion.clip);
        actor.hit = None;
        actor.flags &= !HIT;
        world.publish(id, assets);
    }
    world.collision.compute(|id| world.scene.world(id))?;
    let weapon = world
        .collision
        .slots()
        .iter()
        .position(|s| s.owner == Some(attacker) && s.flags & WEAPON != 0)
        .unwrap();
    let target = &mut world.actors[victim].actor;
    target.hit = Some(weapon);
    target.flags |= HIT;
    let before = target.health;
    let mut hits = Hits::default();
    world.control_actor(victim, assets, &mut hits)?;
    assert!(
        world.actors[victim].actor.health < before,
        "source weapon contact did not injure {victim}"
    );
    world.animate_actor(victim, assets)?;
    session.frame = world.step([0; 2], assets, &mut hits)?;
    session
        .contact
        .record(u64::from(world.tick), &session.frame.effects);
    session.contact.wounds = hits.0;
    if let Some(history) = &mut session.interpolation {
        history.record(crate::tenchu2::native::interpolation::gameplay(
            &session.world,
            &session.assets,
        )?);
    }
    Ok(())
}
