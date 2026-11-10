use super::*;
use tenchu_core::tenchu2::actor::Velocity;

#[test]
#[ignore = "requires TENCHU_DISC, Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; native/imported blades share coating, flow, controls and clean restoration"]
fn native_weapon_blood_follows_hits_blades_and_shared_controls() {
    let gaisen = crate::startup::open(std::env::var_os("TENCHU_DISC").unwrap().as_ref()).unwrap();
    let mut library = crate::visuals::appearance::library::Library::load(&gaisen).unwrap();
    let disc =
        crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
    library.load_tenchu2(&disc).unwrap();
    let mut donor = crate::session::Options::default().load(&gaisen).unwrap();
    for _ in 0..30 {
        donor.step(0).unwrap();
    }
    let out = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    for (hero, imported, blades) in [
        (0, "", 1),
        (1, "", 2),
        (2, "", 0),
        (0, "disc:0", 1),
        (0, "disc:1", 2),
        (0, "tenchu2-jp:ayame", 2),
    ] {
        let (mut session, player, guard) = combat_fixture(hero);
        if !imported.is_empty() {
            session.world.actors[player]
                .actor
                .change_motion(0, Velocity::Stop, &session.assets.resources)
                .unwrap();
            let settings = crate::settings::Settings {
                player_model: imported.into(),
                model_moveset: true,
                ..Default::default()
            };
            assert!(
                session
                    .models
                    .sync(
                        &mut session.world,
                        &mut session.assets,
                        &mut donor,
                        &library,
                        &std::sync::Arc::default(),
                        &settings
                    )
                    .unwrap()
            );
        }
        let graphics = Graphics {
            fog: false,
            asset_pack: false,
            blood: crate::settings::Blood {
                hit: crate::settings::BloodStyle::Off,
                weapons: true,
                weapon_coverage: 140,
                weapon_flow: 100,
                ..Default::default()
            },
            ..Default::default()
        };
        let label = format!("hero-{hero}-{}", imported.replace(':', "-"));
        let vram = session.assets.sprites.vram.clone();
        crate::tenchu2::native::render::capture_frame(&vram, &graphics,
            &out.join(format!("{label}.png")), |renderer| {
            let mut drawing = Drawing::new(&session, &graphics, renderer)?;
            weapon_hit(&mut session, guard, player)?;
            assert!(session.contact.melee_hits.contains(&guard));
            let state = postcard::to_allocvec(&session.world).unwrap();
            drawing.tick(&session, &graphics)?;
            assert!(state == postcard::to_allocvec(&session.world).unwrap());
            let blood = &drawing.scene.environment.as_ref().unwrap().blood;
            assert_eq!(blood.weapon_parts().any(|key| matches!(key, PartId::Actor {root, ..} if root == session.world.instances[guard].rig.root)), hero != 2, "{label}: armed / unarmed opponent");
            assert!(!blood.weapon_parts().any(|key| matches!(key, PartId::Actor {root, ..} if root == session.world.instances[player].rig.root)), "victim weapon stained");
            weapon_hit(&mut session, player, guard)?;
            assert!(session.contact.melee_hits.contains(&player));
            drawing.tick(&session, &graphics)?;
            let root = session.world.instances[player].rig.root;
            let blood = &drawing.scene.environment.as_ref().unwrap().blood;
            let parts: BTreeSet<_> = blood.weapon_parts().filter(|key| matches!(key, PartId::Actor {root: r, ..} if *r == root)).collect();
            assert_eq!(parts.len(), blades, "{label}: wrong blade/handle selection");
            let stored = postcard::to_allocvec(blood).unwrap();
            drawing.tick(&session, &graphics)?;
            assert!(stored == postcard::to_allocvec(&drawing.scene.environment.as_ref().unwrap().blood).unwrap(), "redraw aged coating");
            let mesh = |session: &crate::tenchu2::native::Session, drawing: &Drawing| geometry(
                &drawing.scene.environment.as_ref().unwrap().blood, session, &drawing.models, None,
                &Default::default(), View { alpha: 1.0, ..Default::default() });
            let before = mesh(&session, &drawing)?;
            assert_eq!(!before.vertices.is_empty(), hero != 2);
            assert!(before.vertices.iter().all(|v| Vec3::from(v.position).is_finite()));
            for id in [player, guard] {
                session.world.actors[id].actor.position.x += 700;
                session.world.publish(id, &session.assets);
            }
            let translated = mesh(&session, &drawing)?;
            assert_eq!(before.vertices.len(), translated.vertices.len());
            for (a,b) in before.vertices.iter().zip(&translated.vertices) {
                assert!((Vec3::from(b.position) - Vec3::from(a.position) - Vec3::X * 700.0).length() < 0.5);
            }
            // Hidden blades retain their coating, but never transfer it to the sheath.
            session.world.actors[player].actor.set_weapon(&mut session.world.instances[player].rig,
                player, &mut session.world.collision, &session.assets.resources, WeaponMode::Sheathe)?;
            let sheathed = mesh(&session, &drawing)?;
            if blades != 0 { assert!(sheathed.vertices.len() < translated.vertices.len()); }
            session.world.actors[player].actor.set_weapon(&mut session.world.instances[player].rig,
                player, &mut session.world.collision, &session.assets.resources, WeaponMode::Draw)?;
            assert!(bytemuck::cast_slice::<_, u8>(&mesh(&session, &drawing)?.vertices) == bytemuck::cast_slice::<_, u8>(&translated.vertices));
            let coating = drawing.scene.environment.as_ref().unwrap().blood.clone();
            for zero_amount in [false, true] {
                let mut disabled = graphics.clone();
                if zero_amount { disabled.blood.amount = 0; } else { disabled.blood.weapons = false; }
                drawing.tick(&session, &disabled)?;
                assert!(drawing.scene.environment.as_ref().unwrap().blood.weapon_parts().next().is_none());
                drawing.tick(&session, &graphics)?;
                assert!(drawing.scene.environment.as_ref().unwrap().blood.weapon_parts().next().is_none(), "toggle replayed old hit");
                drawing.scene.environment.as_mut().unwrap().blood = coating.clone();
            }
            let center = point(session.world.actors[player].actor.position) - Vec3::Y * 650.0;
            let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
            camera.position = center + Vec3::new(1600.0, -600.0, 1700.0);
            let forward = (center - camera.position).normalize();
            camera.yaw = forward.x.atan2(forward.z);
            camera.pitch = (-forward.y).asin();
            drawing.camera = Some(camera);
            drawing.show_hud = false;
            drawing.update(&session, renderer, None)?;
            drawing.scene.effects = renderer.upload_effects(mesh(&session, &drawing)?);
            drawing.scene.backgrounds.clear();
            drawing.scene.terrain = renderer.upload(&[]);
            drawing.scene.props = renderer.upload(&[]);
            for actor in &mut session.world.actors { actor.actor.invulnerable = true; }
            let mut dripped = false;
            for _ in 0..65 {
                session.step_world(0, true)?;
                let state = postcard::to_allocvec(&session.world).unwrap();
                drawing.tick(&session, &graphics)?;
                assert!(state == postcard::to_allocvec(&session.world).unwrap());
                dripped |= drawing.scene.environment.as_ref().unwrap().blood.has_drops();
            }
            assert_eq!(dripped, hero != 2, "{label}: weapon dripping");
            drawing.clear_blood();
            assert!(drawing.scene.environment.as_ref().unwrap().blood.weapon_parts().next().is_none());
            Ok((drawing.scene, camera.source_view(), Vec::new()))
        }).unwrap();
    }
}

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; paired kills stain blades only at authored blood events"]
fn native_stealth_weapon_blood_waits_for_source_emission() {
    let out = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    for hero in 0..2 {
        let (mut session, player, guard) = combat_fixture(hero);
        for id in 0..session.world.actors.len() {
            if id != player && id != guard {
                session.world.actors[id].actor.flags |= 8;
                for &joint in &session.world.instances[id].rig.joints {
                    session.world.collision.remove(joint);
                }
                continue;
            }
            let actor = &mut session.world.actors[id].actor;
            actor
                .set_weapon(
                    &mut session.world.instances[id].rig,
                    id,
                    &mut session.world.collision,
                    &session.assets.resources,
                    WeaponMode::Sheathe,
                )
                .unwrap();
            actor.flags &= !(3 | 0x2f000);
            actor.position = Position::new(0, 0, if id == player { 0 } else { -1000 });
            actor.previous = actor.position;
            actor.rotation = Default::default();
            actor.velocity = Default::default();
            actor.terrain.height = 0;
            actor.terrain.floor = Some(0);
            actor.target = (id == guard).then_some(player);
            actor
                .change_motion(0, Velocity::Stop, &session.assets.resources)
                .unwrap();
            session.world.publish(id, &session.assets);
        }
        session.contact = Default::default();
        let graphics = Graphics {
            blood: crate::settings::Blood {
                weapons: true,
                ..Default::default()
            },
            ..Default::default()
        };
        let vram = session.assets.sprites.vram.clone();
        crate::tenchu2::native::render::capture_frame(
            &vram,
            &graphics,
            &out.join(format!("stealth-{hero}.png")),
            |renderer| {
                let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                session.world.actors[player]
                    .actor
                    .pad
                    .update(tenchu_core::tenchu2::input::ATTACK);
                session
                    .world
                    .control_actor(player, &mut session.assets, &mut Hits::default())?;
                assert_eq!(session.world.statistics.stealth_kills, 1);
                assert_eq!(session.world.actors[guard].actor.health, 0);
                let mut bled = false;
                let mut coated = false;
                for _ in 0..240 {
                    bled |= session.contact.wounds.iter().any(|&(id, _)| id == guard);
                    let state = postcard::to_allocvec(&session.world).unwrap();
                    drawing.tick(&session, &graphics)?;
                    assert!(state == postcard::to_allocvec(&session.world).unwrap());
                    coated |= drawing
                        .scene
                        .environment
                        .as_ref()
                        .unwrap()
                        .blood
                        .weapon_parts()
                        .any(|key| {
                            matches!(key, PartId::Actor { root, .. }
                            if root == session.world.instances[player].rig.root)
                        });
                    assert!(
                        !coated || bled,
                        "hero {hero}: blade stained before source blood"
                    );
                    let mut events = Hits::default();
                    for id in [player, guard] {
                        session.world.move_actor(id, &session.assets, &mut events)?;
                        session.world.actors[id].actor.pad.update(0);
                        session
                            .world
                            .control_actor(id, &mut session.assets, &mut events)?;
                        session.world.animate_actor(id, &session.assets)?;
                    }
                    session.world.update_items(&session.assets, &mut events)?;
                    session.frame.effects = session.world.update_effects(&session.assets)?;
                    session.world.finish_tick(&session.assets, 6)?;
                    session.contact.wounds = events.wounds;
                    session.contact.melee_hits = events.melee;
                }
                assert!(
                    bled && coated,
                    "hero {hero}: source blood {bled}, blade coating {coated}"
                );
                drawing.update(&session, renderer, None)?;
                Ok((drawing.scene, session.world.camera.view, Vec::new()))
            },
        )
        .unwrap();
    }
}
