use super::{Builder, render_mesh};
use tenchu_assets::tenchu2::effects::Sprites;
use tenchu_core::tenchu2::hud::{Image, Primitive};

pub(super) fn append(output: &mut Builder, sprites: &Sprites, frame: &[Primitive]) {
    for primitive in frame.iter().rev() {
        match primitive {
            Primitive::Sprite(sprite) => {
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
    use crate::settings::Graphics;
    use tenchu_core::tenchu2::{ai::status::Ki, inventory::Inventory};

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
        for post in [false, true] {
            let mut graphics = Graphics {
                asset_pack: false,
                ..Default::default()
            };
            graphics.post.enabled = post;
            session.frame.transition_fade = None;
            super::super::capture(
                &session,
                &graphics,
                &output.join(format!("hud-post-{post}.png")),
            )
            .unwrap();
            session.frame.transition_fade = Some(255);
            let faded = output.join(format!("hud-faded-post-{post}.png"));
            super::super::capture(&session, &graphics, &faded).unwrap();
            let mut reader =
                png::Decoder::new(std::io::BufReader::new(std::fs::File::open(faded).unwrap()))
                    .read_info()
                    .unwrap();
            let mut pixels = vec![0; reader.output_buffer_size()];
            let info = reader.next_frame(&mut pixels).unwrap();
            assert!(
                pixels[..info.buffer_size()]
                    .chunks_exact(4)
                    .all(|p| p[..3] == [0; 3]),
                "the final fade must cover the scene, HUD and widescreen edges (post={post})"
            );
        }
    }
}
