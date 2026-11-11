use super::{Builder, render_mesh};
use tenchu_assets::tenchu2::effects::Sprites;
use tenchu_core::tenchu2::hud::{Image, Primitive, Sprite};

fn digit(sprite: &Sprite) -> Option<u8> {
    match sprite.image {
        Image::Digit(value) if sprite.rotation == 0 && sprite.scale > 0 => Some(value),
        _ => None,
    }
}

pub(super) fn bleeding_text(session: &super::super::Session) -> Vec<crate::fonts::Text> {
    let world = &session.world;
    let actor = &world.actors[world.player].actor;
    let Some(state) = &actor.bleeding else {
        return Vec::new();
    };
    if actor.health <= 0
        || !world.mission.input_allowed()
        || session.frame.transition_fade == Some(255)
    {
        return Vec::new();
    }
    #[cfg(not(target_arch = "wasm32"))]
    if session.cinematic() {
        return Vec::new();
    }
    let status = if !state.active {
        "No bleeding".into()
    } else if state.settings.seconds == 0 {
        "Bleeding until treated".into()
    } else {
        format!("Bleeding · {} s", state.remaining.div_ceil(30))
    };
    let fade = f32::from(session.frame.transition_fade.unwrap_or(0)) / 255.0;
    let color = if state.active {
        [1.0, 0.4, 0.35]
    } else {
        [0.8; 3]
    };
    let mut texts = vec![crate::fonts::Text {
        position: egui::pos2(-145.0, -65.0),
        text: format!("{status} · {} bandages", state.bandages),
        color: crate::fonts::color(color.map(|c| c - fade)),
        size: 8.0,
        width: 290.0,
        centered: false,
    }];
    if world.controls.hud.bandage_selected()
        && let Some(sprite) = world.controls.hud.frame.iter().find_map(|p| match p {
            Primitive::Sprite(s) if s.image == Image::Bandage => Some(s),
            _ => None,
        })
    {
        texts.push(crate::fonts::Text {
            position: egui::pos2(f32::from(sprite.position[0]).min(120.0), 62.0),
            text: "Bandage".into(),
            color: crate::fonts::color([1.0 - fade; 3]),
            size: 8.0,
            width: 70.0,
            centered: true,
        });
    }
    texts
}

pub(super) fn text(frame: &[Primitive], fade: Option<u8>) -> Vec<crate::fonts::Text> {
    frame
        .iter()
        .rev()
        .filter_map(|primitive| {
            let Primitive::Sprite(sprite) = primitive else {
                return None;
            };
            let value = digit(sprite)?;
            if sprite.color == [0; 3] || fade == Some(255) {
                return None;
            }
            let scale = f32::from(sprite.scale) / 4096.0;
            Some(crate::fonts::Text {
                position: egui::pos2(
                    f32::from(sprite.position[0]),
                    f32::from(sprite.position[1]) - 5.0 * scale,
                ),
                text: char::from(b'0' + value).to_string(),
                color: crate::fonts::color(
                    sprite
                        .color
                        .map(|c| f32::from(c) / 128.0 - f32::from(fade.unwrap_or(0)) / 255.0),
                ),
                size: 6.75 * scale,
                width: 4.5 * scale,
                centered: true,
            })
        })
        .collect()
}

pub(super) fn append(
    output: &mut Builder,
    sprites: &Sprites,
    frame: &[Primitive],
    sharp_text: bool,
) {
    for primitive in frame.iter().rev() {
        if let Primitive::Sprite(sprite) = primitive
            && sprite.image == Image::Bandage
        {
            let start = output.surfaces.len();
            crate::render_effects::bandage::draw_at(
                &mut output.surfaces,
                sprite.position.map(f32::from),
                f32::from(sprite.scale) / 4096.0 * 0.75,
                sprite.color.map(|c| f32::from(c) / 128.0),
            );
            for surface in &mut output.surfaces[start..] {
                surface.depth = 16.0;
                for vertex in &mut surface.vertices {
                    vertex.flags |= render_mesh::UI;
                }
            }
            continue;
        }
        match primitive {
            Primitive::Sprite(sprite) => {
                if sharp_text && digit(sprite).is_some() {
                    continue;
                }
                let mut image = match sprite.image {
                    Image::Item(id) => sprites.items[42 + id],
                    Image::Selection => sprites.items[63],
                    Image::PlayerFrame => sprites.items[34],
                    Image::PlayerFill => sprites.items[40],
                    Image::EnemyFrame => sprites.items[35],
                    Image::EnemyFill => sprites.items[41],
                    Image::Ki(id) => sprites.items[36 + id],
                    Image::Digit(_) => sprites.effects[27],
                    Image::Aim => sprites.effects[30],
                    Image::Bandage => unreachable!("handled above"),
                };
                let mut center = sprite.position.map(f32::from);
                let scale = f32::from(sprite.scale) / 4096.0;
                let mut pivot = image.size.map(|n| f32::from(n / 2));
                if let Image::Digit(digit) = sprite.image {
                    image.uv[0] = image.uv[0].wrapping_add(digit * 4);
                    image.size = [4, 5];
                    pivot = [2.0, 3.0];
                }
                if let Some(width) = sprite.width {
                    image.size[0] = width;
                }
                let size = image.size.map(f32::from);
                let (sin, cos) = (sprite.rotation as f32 / 4096.0).to_radians().sin_cos();
                let offset = std::array::from_fn::<_, 2, _>(|i| (size[i] / 2.0 - pivot[i]) * scale);
                center[0] += offset[0] * cos - offset[1] * sin;
                center[1] += offset[0] * sin + offset[1] * cos;
                let blend = match sprite.image {
                    Image::Selection | Image::Ki(_) => 2,
                    Image::Aim => 1,
                    _ => 0,
                };
                output.screen_sprite(
                    [center[0], center[1], 16.0],
                    size,
                    [scale; 2],
                    sprite.rotation,
                    sprite.color.map(|c| f32::from(c) / 128.0),
                    Some(image),
                    blend,
                );
            }
            Primitive::Breath(width) => {
                output.screen_sprite(
                    [-113.0 + f32::from(*width) / 2.0, 84.0, 16.0],
                    [f32::from(*width), 2.0],
                    [1.0; 2],
                    0,
                    [32.0 / 255.0, 64.0 / 255.0, 128.0 / 255.0],
                    None,
                    0,
                );
            }
        }
        for vertex in &mut output.surfaces.last_mut().unwrap().vertices {
            vertex.flags |= render_mesh::UI;
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::settings::{Graphics, Settings};
    use tenchu_core::tenchu2::{ai::status::Ki, inventory::Inventory};

    #[test]
    #[ignore = "requires TENCHU2_DISC, TENCHU2_HUD_REVIEW and GPU; optional bandages render with original/HD text, hide and fade without advancing gameplay"]
    fn native_bleeding_hud_respects_visibility_and_does_not_advance_time() {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let output = std::path::PathBuf::from(std::env::var_os("TENCHU2_HUD_REVIEW").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        let mut session = super::super::super::Session::load(&disc, 9, 17).unwrap();
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        let mut settings = Settings::default();
        settings.graphics.asset_pack = false;
        session
            .world
            .set_bleeding(tenchu_core::game::bleeding::Settings {
                enabled: true,
                ..Default::default()
            });
        let actor = &mut session.world.actors[0].actor;
        actor.inventory = Inventory::from_amounts([0; 21]);
        let bleeding = actor.bleeding.as_mut().unwrap();
        bleeding.active = true;
        bleeding.remaining = 300;
        session.step(0).unwrap();
        assert!(session.world.controls.hud.bandage_selected());
        let before = postcard::to_allocvec(&session.world).unwrap();
        for hd in [false, true] {
            settings.sharp_text = hd;
            assert!(
                super::bleeding_text(&session)
                    .iter()
                    .any(|t| t.text == "Bandage")
            );
            let path = output.join(format!("bleeding-hd-{hd}.png"));
            super::super::capture(&session, &settings, &path).unwrap();
            session.frame.transition_fade = Some(255);
            assert!(super::bleeding_text(&session).is_empty());
            let path = output.join(format!("bleeding-faded-hd-{hd}.png"));
            super::super::capture(&session, &settings, &path).unwrap();
            let image = crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap();
            assert!(image.rgba.chunks_exact(4).all(|p| p[..3] == [0; 3]));
            session.frame.transition_fade = None;
        }
        settings.show_hud = false;
        super::super::capture(&session, &settings, &output.join("bleeding-hidden.png")).unwrap();
        assert!(postcard::to_allocvec(&session.world).unwrap() == before);
        let expected = {
            session.step(0).unwrap();
            postcard::to_allocvec(&session.world).unwrap()
        };
        session.world = postcard::from_bytes(&before).unwrap();
        session.step(0).unwrap();
        assert!(postcard::to_allocvec(&session.world).unwrap() == expected);
    }

    #[test]
    #[ignore = "requires TENCHU2_DISC, TENCHU2_HUD_REVIEW and a GPU; native HUD composition and full-screen fade, with/without post-processing"]
    fn native_hud_and_scene_fade_share_the_overlay_pass() {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let output = std::path::PathBuf::from(std::env::var_os("TENCHU2_HUD_REVIEW").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        let mut session = super::super::super::Session::load(&disc, 9, 1).unwrap();
        session.step(0).unwrap();
        for i in 1..=5 {
            session
                .world
                .controls
                .hud
                .show_health(i, session.world.actors[i].actor.kind);
            session.world.actors[i].actor.health = i as i16 * 7;
        }
        session.world.actors[2].actor.flags |= 0x100;
        let player = &session.world.actors[session.world.player].actor;
        let inventory = Inventory::from_amounts([
            255, 0, 0, 0, 12, 1, 2, 0, 0, 0, 5, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1,
        ]);
        session
            .world
            .controls
            .hud
            .step(player.into(), 50, &inventory, Ki::Nearby(75), |i| {
                session.world.actors.get(i).map(|a| (&a.actor).into())
            })
            .unwrap();
        let world = postcard::to_allocvec(&session.world).unwrap();
        let read = |path: &std::path::Path| {
            crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap()
        };
        for post in [false, true] {
            let mut graphics = Graphics {
                asset_pack: false,
                ..Default::default()
            };
            graphics.post.enabled = post;
            let mut settings = Settings {
                graphics,
                ..Default::default()
            };
            let mut images = Vec::new();
            for sharp_text in [false, true] {
                settings.sharp_text = sharp_text;
                session.frame.transition_fade = None;
                let path = output.join(format!("hud-post-{post}-hd-{sharp_text}.png"));
                super::super::capture(&session, &settings, &path).unwrap();
                images.push(read(&path));
                session.frame.transition_fade = Some(255);
                let faded = output.join(format!("hud-faded-post-{post}-hd-{sharp_text}.png"));
                super::super::capture(&session, &settings, &faded).unwrap();
                assert!(
                    read(&faded).rgba.chunks_exact(4).all(|p| p[..3] == [0; 3]),
                    "fade left visible pixels (post={post}, HD={sharp_text})"
                );
                assert_eq!(postcard::to_allocvec(&session.world).unwrap(), world);
            }
            let [original, sharp] = images.as_slice() else {
                unreachable!()
            };
            assert_eq!(original.size, sharp.size);
            let width = original.size[0] as usize;
            assert_eq!(
                original.rgba[200 * width * 4..500 * width * 4],
                sharp.rgba[200 * width * 4..500 * width * 4],
                "HD text changed the world view (post={post})"
            );
            assert!(
                original
                    .rgba
                    .iter()
                    .zip(&sharp.rgba)
                    .filter(|(a, b)| a != b)
                    .count()
                    > 100,
                "HD text did not change the readouts (post={post})"
            );
        }

        use super::super::super::{campaign::Content, presentation::ContentDrawing};
        session.frame.transition_fade = None;
        let content = Content::Gameplay(Box::new(session));
        let Content::Gameplay(session) = &content else {
            unreachable!()
        };
        let mut settings = Settings {
            sharp_text: true,
            ..Default::default()
        };
        let mut debug = crate::debug::Debug::default();
        // The hosts update presentation even when the simulation is paused.
        super::super::capture_frame(
            &session.assets.sprites.vram,
            &settings.graphics.clone(),
            &output.join("hud-paused-hidden.png"),
            |renderer| {
                let mut drawing = ContentDrawing::new(&content, &settings.graphics, renderer)?;
                for (visible, photo, sharp, expected) in [
                    (true, false, true, true),
                    (true, false, false, false),
                    (true, false, true, true),
                    (false, false, true, false),
                    (true, true, true, false),
                ] {
                    settings.show_hud = visible;
                    settings.sharp_text = sharp;
                    debug.photo_mode = photo;
                    drawing.update(&content, renderer, &debug, &settings)?;
                    let ContentDrawing::Gameplay(drawing) = &drawing else {
                        unreachable!()
                    };
                    assert_eq!(!drawing.texts.is_empty(), expected);
                }
                let ContentDrawing::Gameplay(drawing) = drawing else {
                    unreachable!()
                };
                assert_eq!(postcard::to_allocvec(&session.world).unwrap(), world);
                Ok((drawing.scene, session.world.camera.view, drawing.texts))
            },
        )
        .unwrap();
    }
}
