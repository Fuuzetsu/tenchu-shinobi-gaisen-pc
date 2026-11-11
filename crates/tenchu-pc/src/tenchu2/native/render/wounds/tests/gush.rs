use super::*;
use crate::{render_mesh, settings::BloodStyle, visuals::surfaces::Surfaces};

#[test]
#[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; native deaths use source joints, shared finite gushes/styles and surface contacts"]
fn native_death_gushes_follow_source_blood_and_stop() {
    let out = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
    std::fs::create_dir_all(&out).unwrap();
    for hero in 0..3 {
        for player_victim in [false, true] {
            let (mut session, player, guard) = combat_fixture(hero);
            let (attacker, victim) = if player_victim {
                (guard, player)
            } else {
                (player, guard)
            };
            for (id, agent) in session.world.actors.iter_mut().enumerate() {
                if id != victim {
                    agent.actor.invulnerable = true;
                }
                if id != player && id != guard {
                    agent.actor.flags |= 8;
                }
            }
            session.world.actors[victim].actor.health = 1;
            let graphics = Graphics {
                fog: false,
                asset_pack: false,
                blood: crate::settings::Blood {
                    hit: BloodStyle::Off,
                    drips: BloodStyle::Off,
                    death: BloodStyle::Original,
                    ground: BloodStyle::Modern,
                    gush_style: BloodStyle::Modern,
                    death_gush: 100,
                    gush_seconds: 2,
                    player_death: true,
                    ..Default::default()
                },
                ..Default::default()
            };
            let vram = session.assets.sprites.vram.clone();
            super::super::super::capture_frame(&vram, &graphics,
                &out.join(format!("hero-{hero}-player-{player_victim}.png")), |renderer| {
                let mut current = graphics.clone();
                let mut drawing = Drawing::new(&session, &graphics, renderer)?;
                let mut floor = crate::render_effects::quad(
                    [[-6000.0, 0.0, -6000.0], [6000.0, 0.0, -6000.0],
                     [-6000.0, 0.0, 6000.0], [6000.0, 0.0, 6000.0]],
                    [[0.25; 3]; 4], None, 0);
                for vertex in &mut floor { vertex.flags = render_mesh::DOUBLE_SIDED; }
                drawing.scene.environment.as_mut().unwrap().surfaces = Surfaces::new(&floor);
                weapon_hit(&mut session, attacker, victim)?;
                assert_eq!(session.world.actors[victim].actor.health, 0);
                let mut source = None;
                let mut started = None;
                let mut stained = false;
                let mut impacted = false;
                let clear = hero == 1 && !player_victim;
                let mut cleared = false;
                let mut captured = false;
                let mut styles_checked = false;
                let mut camera = crate::debug::camera::Camera::from_view(session.world.camera.view, 300.0);
                for elapsed in 0..210 {
                    let contact = session.contact.wounds.iter().find(|w| w.owner == victim);
                    if contact.is_some() { source.get_or_insert(elapsed); }
                    let state = postcard::to_allocvec(&session.world).unwrap();
                    drawing.tick(&session, &current)?;
                    assert!(state == postcard::to_allocvec(&session.world).unwrap());
                    let blood = &drawing.scene.environment.as_ref().unwrap().blood;
                    if let Some(key) = blood.gush_parts().next() {
                        if started.is_none() {
                            let wound = contact.expect("gush preceded source blood");
                            assert!(matches!(key, PartId::Actor {root, model, ..}
                                if root == session.world.instances[victim].rig.root && model == wound.joint));
                            assert_eq!(source, Some(elapsed));
                            started = Some(elapsed);
                        }
                        assert!(elapsed - started.unwrap() < 60, "gush outlived configured duration");
                        assert!(!cleared, "Clear blood restarted a gush");
                    }
                    if blood.has_drops() && !styles_checked {
                        styles_checked = true;
                        for style in [BloodStyle::Original, BloodStyle::OriginalHd, BloodStyle::Modern] {
                            let mut copy = blood.clone();
                            let mut options = graphics.blood;
                            options.gush_style = style;
                            copy.configure(&options);
                            let mut vertices = Vec::new();
                            copy.append_drops(session.world.camera.view, 1.0,
                                session.assets.sprites.effects[1], &mut vertices);
                            assert!(!vertices.is_empty());
                            assert_eq!(vertices[0].flags & render_mesh::SOFT != 0, style == BloodStyle::Modern);
                            assert_eq!(vertices[0].flags & render_mesh::BLOOD_HD != 0, style == BloodStyle::OriginalHd);
                        }
                    }
                    stained |= blood.pages().any(|page| !page.is_empty());
                    if started.is_some_and(|first| elapsed == first + 2) {
                        let stored = postcard::to_allocvec(blood).unwrap();
                        drawing.tick(&session, &current)?;
                        assert!(stored == postcard::to_allocvec(&drawing.scene.environment.as_ref().unwrap().blood).unwrap(),
                            "redraw advanced gushes");
                        let center = point(session.world.actors[victim].actor.position) - Vec3::Y * 650.0;
                        camera.position = center + Vec3::new(1800.0, -900.0, 1600.0);
                        let forward = (center - camera.position).normalize();
                        camera.yaw = forward.x.atan2(forward.z);
                        camera.pitch = (-forward.y).asin();
                        drawing.camera = Some(camera);
                        drawing.show_hud = false;
                        drawing.update(&session, renderer, None)?;
                        drawing.scene.terrain = renderer.upload(&floor);
                        drawing.scene.props = renderer.upload(&[]);
                        drawing.scene.backgrounds.clear();
                        captured = true;
                    }
                    let sound_state = postcard::to_allocvec(&session.world).unwrap();
                    let sounds = drawing.blood_audio(&session)?;
                    impacted |= !sounds.is_empty();
                    assert!(sounds.iter().all(|sound| sound.sound.id == 29));
                    assert!(sound_state == postcard::to_allocvec(&session.world).unwrap());
                    if clear && started.is_some_and(|first| elapsed == first + 8) {
                        drawing.clear_blood();
                        cleared = true;
                    }
                    if hero == 2 && player_victim && started.is_some_and(|first| elapsed == first + 8) {
                        current.blood.player_death = false;
                        cleared = true;
                    }
                    session.step_world(0, true)?;
                }
                assert!(captured && started.is_some(), "hero {hero}, player {player_victim}: no gush");
                let blood = &drawing.scene.environment.as_ref().unwrap().blood;
                assert!(blood.gush_parts().next().is_none() && !blood.has_drops());
                if !clear { assert!(stained && impacted, "gush droplets never reached ground"); }
                for off in [true, false] {
                    let mut settings = graphics.clone();
                    if off { settings.blood.gush_style = BloodStyle::Off; }
                    drawing.tick(&session, &settings)?;
                }
                for _ in 0..15 {
                    session.step_world(0, true)?;
                    drawing.tick(&session, &current)?;
                }
                assert!(drawing.scene.environment.as_ref().unwrap().blood.gush_parts().next().is_none(),
                    "reenabling replayed a past death");
                Ok((drawing.scene, camera.source_view(), Vec::new()))
            }).unwrap();
        }
    }
}
