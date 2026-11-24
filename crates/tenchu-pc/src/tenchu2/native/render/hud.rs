use super::{Builder, render_mesh};
use tenchu_assets::tenchu2::effects::Sprites;
use tenchu_core::tenchu2::hud::{Image, Primitive, Sprite};

#[derive(Default)]
pub(super) struct Artwork(std::collections::BTreeMap<tenchu_assets::texture::Image, u32>);

impl Artwork {
    pub fn new(
        sprites: &Sprites,
        graphics: &crate::settings::Graphics,
        renderer: &mut crate::renderer::Renderer,
    ) -> tenchu_assets::Result<Self> {
        use crate::visuals::{catalog, models::Material, pack::Pack, pixels};
        if !graphics.asset_pack {
            return Ok(Self::default());
        }
        let images: Vec<_> = sprites.items[34..64]
            .iter()
            .copied()
            .chain([sprites.effects[27], sprites.effects[30]])
            .map(|sprite| {
                let region = catalog::Region::sprite(&sprites.vram, sprite);
                let image = pixels::Image {
                    size: [region.rect[2].into(), region.rect[3].into()],
                    rgba: region.pixels(&sprites.vram),
                };
                let id = catalog::image_id(image.size.map(|v| v as u16), &image.rgba);
                (sprite, id, image)
            })
            .collect();
        let pack = Pack::tenchu2_interface(
            images
                .iter()
                .map(|(_, id, image)| (id.clone(), image.clone()))
                .collect(),
        )?;
        Ok(Self(
            images
                .into_iter()
                .filter_map(|(sprite, id, _)| {
                    let image = pack.textures.get(&id)?.clone();
                    let handle = renderer.bind_material(&Material {
                        id: format!("{id}:interface"),
                        image,
                        region: None,
                        animation: None,
                    });
                    Some((sprite, handle))
                })
                .collect(),
        ))
    }

    fn bind(
        &self,
        surface: &mut crate::render_effects::Surface,
        image: tenchu_assets::texture::Image,
    ) {
        let Some(&material) = self.0.get(&image) else {
            return;
        };
        for vertex in &mut surface.vertices {
            vertex.material = material;
            vertex.uv = std::array::from_fn(|axis| {
                (vertex.uv[axis] - f32::from(image.uv[axis])) / f32::from(image.size[axis]) - 0.5
            });
        }
    }
}

fn digit(sprite: &Sprite) -> Option<u8> {
    match sprite.image {
        Image::Digit(value) if sprite.rotation == 0 && sprite.scale > 0 => Some(value),
        _ => None,
    }
}

pub(super) fn bleeding_state(
    session: &super::super::Session,
) -> Option<&tenchu_core::game::bleeding::State> {
    let world = &session.world;
    let actor = &world.actors[world.player].actor;
    if actor.health <= 0
        || !world.mission.input_allowed()
        || session.frame.transition_fade == Some(255)
    {
        return None;
    }
    #[cfg(not(target_arch = "wasm32"))]
    if session.cinematic() {
        return None;
    }
    actor.bleeding.as_ref()
}

pub(super) fn bleeding_text(
    session: &super::super::Session,
    details: bool,
) -> Vec<crate::fonts::Text> {
    let Some(state) = bleeding_state(session) else {
        return Vec::new();
    };
    let fade = f32::from(session.frame.transition_fade.unwrap_or(0)) / 255.0;
    let mut texts = Vec::new();
    if details {
        let status = if !state.active {
            "No bleeding".into()
        } else if state.settings.seconds == 0 {
            "Bleeding until treated".into()
        } else {
            format!("Bleeding · {} s", state.remaining.div_ceil(30))
        };
        let color = if state.active {
            [1.0, 0.4, 0.35]
        } else {
            [0.8; 3]
        };
        texts.push(crate::fonts::Text {
            position: egui::pos2(-145.0, -65.0),
            text: format!("{status} · {} bandages", state.bandages),
            color: crate::fonts::color(color.map(|c| c - fade)),
            size: 8.0,
            width: 290.0,
            align: crate::fonts::Align::Left,
        });
    }
    let world = &session.world;
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
            align: crate::fonts::Align::Center,
        });
    }
    texts
}

pub(super) fn text(frame: &[Primitive], fade: Option<u8>) -> Vec<crate::fonts::Text> {
    let digits = frame
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
                align: crate::fonts::Align::Center,
            })
        })
        .collect();
    crate::fonts::compact_numbers(digits, 5.0)
}

pub(super) fn append(
    output: &mut Builder,
    sprites: &Sprites,
    frame: &[Primitive],
    sharp_text: bool,
    artwork: &Artwork,
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
                let original_image = image;
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
                artwork.bind(output.surfaces.last_mut().unwrap(), original_image);
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
    fn native_numbers_keep_source_anchors_fades_and_rotation() {
        use tenchu_core::tenchu2::hud::{Health, Hud};
        for value in [1, 10, 100] {
            let mut hud = Hud::default();
            let mut items = [0; 21];
            items[0] = 255;
            items[1] = value;
            hud.step(
                Health {
                    current: i16::from(value),
                    maximum: 100,
                    poisoned: false,
                },
                0,
                &Inventory::from_amounts(items),
                Ki::Nearby(value),
                |_| None,
            )
            .unwrap();
            let before = postcard::to_allocvec(&hud).unwrap();
            let text = super::text(&hud.frame, None);
            for x in [-64.0, -110.0] {
                let number = text.iter().find(|text| text.position.x == x).unwrap();
                assert_eq!(number.text, value.to_string());
                assert_eq!(number.align, crate::fonts::Align::LastDigit);
            }
            assert!(
                text.iter()
                    .any(|t| t.position.x > 0.0 && t.text == value.to_string())
            );
            let faded = super::text(&hud.frame, Some(128));
            assert_eq!(faded.len(), text.len());
            for (before, after) in text.iter().zip(faded) {
                assert_eq!(before.text, after.text);
                assert_eq!(before.position, after.position);
                assert!(after.color.r() < before.color.r());
            }
            assert!(super::text(&hud.frame, Some(255)).is_empty());
            assert_eq!(postcard::to_allocvec(&hud).unwrap(), before);
            let sprite = hud
                .frame
                .iter_mut()
                .find_map(|p| match p {
                    super::Primitive::Sprite(s) if matches!(s.image, super::Image::Digit(_)) => {
                        Some(s)
                    }
                    _ => None,
                })
                .unwrap();
            sprite.rotation = 1;
            assert!(
                super::digit(sprite).is_none(),
                "animated digit must retain the source sprite"
            );
        }
    }

    #[test]
    #[ignore = "requires TENCHU2_DISC, TENCHU2_ASSET_PACK, TENCHU2_UI_REVIEW and GPU; all item icons, clipped health fills, Ki blend classes, digit slices and exact original restoration"]
    fn hd_hud_keeps_layout_blending_and_game_state() {
        use super::super::{Builder, Models, Presentation, SceneDrawing, capture_frame};
        use tenchu_core::tenchu2::hud::{Image, Primitive, Sprite};
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let output = std::path::PathBuf::from(std::env::var_os("TENCHU2_UI_REVIEW").unwrap())
            .join("screenshots");
        std::fs::create_dir_all(&output).unwrap();
        let mut session = super::super::super::Session::load(&disc, 9, 17).unwrap();
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        session.world.controls.hud.frame.clear();
        let mut add = |image, position, width| {
            session
                .world
                .controls
                .hud
                .frame
                .push(Primitive::Sprite(Sprite {
                    image,
                    position,
                    width,
                    scale: 4096,
                    rotation: 0,
                    color: [128; 3],
                }))
        };
        for i in 0..21 {
            add(
                Image::Item(i),
                [-120 + (i % 7) as i16 * 40, -50 + (i / 7) as i16 * 40],
                None,
            );
        }
        for i in 0..4 {
            add(Image::Ki(i), [-130 + i as i16 * 32, 85], None);
        }
        for (i, width) in [0, 1, 27, 54].into_iter().enumerate() {
            add(Image::PlayerFill, [30 + i as i16 * 38, 85], Some(width));
        }
        add(Image::PlayerFrame, [-112, -90], None);
        add(Image::EnemyFrame, [72, -90], None);
        add(Image::EnemyFill, [72, -90], Some(23));
        add(Image::Selection, [0, 100], None);
        add(Image::Aim, [130, 110], None);
        for i in 0..10 {
            add(Image::Digit(i), [-20 + i as i16 * 8, -90], None);
        }
        let before = postcard::to_allocvec(&session.world).unwrap();
        let graphics = Graphics {
            asset_pack: false,
            ..Default::default()
        };
        let mut captures = Vec::new();
        for (i, enabled) in [false, true, false].into_iter().enumerate() {
            let path = output.join(format!("hud-icons-{i}-hd-{enabled}.png"));
            capture_frame(&session.assets.sprites.vram, &graphics, &path, |renderer| {
                let mut models = Models::new(&session.assets, &session.world, &graphics, renderer)?;
                let mut hd = graphics.clone();
                hd.asset_pack = enabled;
                models.hud = super::Artwork::new(&session.assets.sprites, &hd, renderer)?;
                if enabled {
                    for image in &session.assets.sprites.items[34..64] {
                        assert!(
                            models.hud.0.contains_key(image),
                            "Missing HUD artwork: {image:?}"
                        );
                    }
                    let mut original =
                        Builder::new(session.world.camera.view, &session.assets.level, &graphics);
                    let mut replaced =
                        Builder::new(session.world.camera.view, &session.assets.level, &graphics);
                    super::append(
                        &mut original,
                        &session.assets.sprites,
                        &session.world.controls.hud.frame,
                        false,
                        &Default::default(),
                    );
                    super::append(
                        &mut replaced,
                        &session.assets.sprites,
                        &session.world.controls.hud.frame,
                        false,
                        &models.hud,
                    );
                    assert_eq!(original.surfaces.len(), replaced.surfaces.len());
                    for (a, b) in original.surfaces.iter().zip(&replaced.surfaces) {
                        assert_eq!(a.blend, b.blend);
                        for (a, b) in a.vertices.iter().zip(&b.vertices) {
                            assert_eq!(a.position, b.position);
                            assert_eq!(a.color, b.color);
                            assert_eq!(a.flags, b.flags);
                            assert_ne!(b.material, 0);
                        }
                    }
                }
                let geometry = models.geometry(
                    &session.world,
                    &session.assets,
                    &session.frame,
                    &graphics,
                    Presentation {
                        show_hud: true,
                        ..Default::default()
                    },
                )?;
                Ok((
                    SceneDrawing::new(geometry, renderer, &session.assets.level),
                    session.world.camera.view,
                    Vec::new(),
                ))
            })
            .unwrap();
            captures.push(std::fs::read(path).unwrap());
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), before);
        }
        assert!(captures[0] != captures[1]);
        assert!(captures[0] == captures[2]);
        let mut settings = Settings::default();
        settings.graphics.asset_pack = true;
        session.frame.transition_fade = Some(255);
        let path = output.join("hud-icons-faded.png");
        super::super::capture(&session, &settings, &path).unwrap();
        let faded = crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap();
        assert!(faded.rgba.chunks_exact(4).all(|p| p[..3] == [0; 3]));
        session.frame.transition_fade = None;
        settings.show_hud = false;
        let path = output.join("hud-icons-hidden.png");
        super::super::capture(&session, &settings, &path).unwrap();
        let hidden = std::fs::read(&path).unwrap();
        session.world.controls.hud.frame.clear();
        settings.show_hud = true;
        super::super::capture(&session, &settings, &path).unwrap();
        assert!(std::fs::read(path).unwrap() == hidden);
    }

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
        for (preset, name) in [
            (0, "original"),
            (crate::settings::presets::MODERN_HIGH, "high"),
            (crate::settings::presets::MODERN_ULTRA, "ultra"),
        ] {
            settings.set_visual_preset(preset);
            assert!(
                super::bleeding_text(&session, false)
                    .iter()
                    .any(|t| t.text == "Bandage")
            );
            assert!(
                !super::bleeding_text(&session, false)
                    .iter()
                    .any(|t| t.text.contains("bleeding") || t.text.contains("Bleeding"))
            );
            for details in [true, false] {
                settings.bleeding_details = details;
                assert_eq!(
                    super::bleeding_text(&session, details)
                        .iter()
                        .any(|t| t.text.starts_with("Bleeding ·")),
                    details
                );
                let path = output.join(format!("native-{name}-details-{details}.png"));
                super::super::capture(&session, &settings, &path).unwrap();
            }
            session.frame.transition_fade = Some(255);
            assert!(super::bleeding_state(&session).is_none());
            assert!(super::bleeding_text(&session, true).is_empty());
            let path = output.join(format!("native-{name}-faded.png"));
            super::super::capture(&session, &settings, &path).unwrap();
            let image = crate::visuals::pack::decode_png(&std::fs::read(path).unwrap()).unwrap();
            assert!(image.rgba.chunks_exact(4).all(|p| p[..3] == [0; 3]));
            session.frame.transition_fade = None;
        }
        session.world.actors[0].actor.health = 0;
        assert!(super::bleeding_state(&session).is_none());
        assert!(super::bleeding_text(&session, true).is_empty());
        session.world = postcard::from_bytes(&before).unwrap();
        settings.show_hud = false;
        let hidden = output.join("native-hidden.png");
        super::super::capture(&session, &settings, &hidden).unwrap();
        assert!(postcard::to_allocvec(&session.world).unwrap() == before);
        session.world.actors[0]
            .actor
            .bleeding
            .as_mut()
            .unwrap()
            .clear();
        assert!(!super::bleeding_state(&session).unwrap().active);
        assert_eq!(super::bleeding_text(&session, false).len(), 1);
        let treated_hidden = output.join("native-treated-hidden.png");
        super::super::capture(&session, &settings, &treated_hidden).unwrap();
        assert_eq!(
            std::fs::read(&hidden).unwrap(),
            std::fs::read(&treated_hidden).unwrap()
        );
        settings.show_hud = true;
        super::super::capture(&session, &settings, &output.join("native-treated.png")).unwrap();
        session.world = postcard::from_bytes(&before).unwrap();
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

        session.frame.transition_fade = None;
        let instance = wgpu::Instance::new(&Default::default());
        let adapter = pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
        let graphics = Graphics {
            asset_pack: false,
            ..Default::default()
        };
        let format = wgpu::TextureFormat::Rgba8Unorm;
        let mut renderer = pollster::block_on(crate::renderer::Renderer::new(
            &adapter,
            format,
            [640, 360],
            &session.assets.sprites.vram,
        ))
        .unwrap();
        renderer.configure(&graphics);
        let mut drawing = super::super::Drawing::new(&session, &graphics, &mut renderer).unwrap();
        let target = renderer.device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Small native HUD"),
            size: wgpu::Extent3d {
                width: 640,
                height: 360,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
            view_formats: &[],
        });
        let mut captures = Vec::new();
        for (index, sharp) in [false, true, false].into_iter().enumerate() {
            drawing.sharp_text = sharp;
            drawing.update(&session, &mut renderer, None).unwrap();
            drawing
                .draw(&target.create_view(&Default::default()), &mut renderer)
                .unwrap();
            crate::fonts::capture(&renderer, &target, &drawing.texts);
            let path = output.join(format!("hud-small-{index}-hd-{sharp}.png"));
            crate::capture::read_texture(&renderer, &target, &path).unwrap();
            captures.push(std::fs::read(path).unwrap());
            assert_eq!(postcard::to_allocvec(&session.world).unwrap(), world);
        }
        assert_ne!(captures[0], captures[1]);
        assert_eq!(
            captures[0], captures[2],
            "small Original HUD did not restore"
        );

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
