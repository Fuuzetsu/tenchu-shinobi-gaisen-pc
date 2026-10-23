use egui::{Color32, FontFamily, FontId, Pos2, Rect, Vec2};
use tenchu_core::{game::Game, hud::Sprite};

#[cfg(test)]
mod tests;

pub struct Text {
    pub position: Pos2,
    pub text: String,
    pub color: Color32,
    pub size: f32,
    pub width: f32,
    pub centered: bool,
}

pub fn install(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "game-text".into(),
        egui::FontData::from_static(include_bytes!("../data/hd-font/TenchuUISans-Regular.ttf"))
            .into(),
    );
    fonts.families.insert(family(), vec!["game-text".into()]);
    ctx.set_fonts(fonts);
}

fn family() -> FontFamily {
    FontFamily::Name("Game text".into())
}

pub fn id(size: f32) -> FontId {
    FontId::new(size, family())
}

pub fn color(rgb: [f32; 3]) -> Color32 {
    let [r, g, b] = rgb.map(|n| (n.clamp(0.0, 1.0) * 255.0).round() as u8);
    Color32::from_rgb(r, g, b)
}

pub fn paint(ctx: &egui::Context, rect: Rect, texts: &[Text]) {
    let painter = ctx
        .layer_painter(egui::LayerId::background())
        .with_clip_rect(rect);
    let scale = rect.height() / 240.0;
    for text in texts {
        let mut font = id(text.size * scale);
        let mut galley = painter.layout_no_wrap(text.text.clone(), font.clone(), text.color);
        let width = text.width * scale;
        if galley.size().x > width {
            font.size *= width / galley.size().x;
            galley = painter.layout_no_wrap(text.text.clone(), font, text.color);
        }
        let mut pos = rect.center() + text.position.to_vec2() * scale;
        if text.centered {
            pos.x -= galley.size().x / 2.0;
        }
        painter.galley_with_override_text_color(
            pos + Vec2::splat((scale * 0.45).max(1.0)),
            galley.clone(),
            Color32::BLACK,
        );
        painter.galley(pos, galley, text.color);
    }
}

pub fn digit(game: &Game, sprite: &Sprite) -> Option<char> {
    let original = game.mission.images[51];
    let offset = sprite.image.uv[0].wrapping_sub(original.uv[0]);
    (sprite.image.page == original.page
        && sprite.image.palette == original.palette
        && sprite.image.uv[1] == original.uv[1]
        && sprite.image.size == [4, original.size[1]]
        && sprite.rotation == 0
        && offset < 40
        && offset.is_multiple_of(4))
    .then(|| char::from(b'0' + offset / 4))
}

pub fn caption_sprites(game: &Game) -> usize {
    let poems = game
        .cinematic
        .tanka
        .iter()
        .zip(&game.mission.tanka)
        .filter(|(shown, _)| **shown)
        .count();
    game.cinematic.sprites.len().saturating_sub(poems)
}

pub fn game_text(game: &Game, show_hud: bool) -> Vec<Text> {
    let mut text = Vec::new();
    if show_hud {
        for sprite in &game.hud.sprites {
            let Some(digit) = digit(game, sprite) else {
                continue;
            };
            let scale = sprite.scale.map(|n| f32::from(n) / 4096.0);
            text.push(Text {
                position: Pos2::new(
                    f32::from(sprite.xy[0]) - (f32::from(sprite.origin[0]) - 2.0) * scale[0],
                    f32::from(sprite.xy[1])
                        - (f32::from(sprite.origin[1]) + f32::from(sprite.image.size[1]) * 0.4)
                            * scale[1],
                ),
                text: digit.to_string(),
                color: color(sprite.color.map(|n| f32::from(n) / 128.0)),
                size: f32::from(sprite.image.size[1]) * scale[1] * 1.35,
                width: 4.5 * scale[0],
                centered: true,
            });
        }
    }
    if caption_sprites(game) != 0 {
        let bytes = &game.cinematic.displayed_caption;
        let language = game.mission.scripts.language;
        let bytes = if language == tenchu_assets::script::Language::Japanese {
            &bytes[..bytes.len().min(32)]
        } else {
            bytes
        };
        for (line, caption) in language.decode(bytes).lines().enumerate() {
            text.push(Text {
                position: Pos2::new(0.0, 90.0 + line as f32 * 16.0),
                text: caption.into(),
                color: Color32::WHITE,
                size: 14.0,
                width: 300.0,
                centered: true,
            });
        }
    }
    text
}

#[cfg(not(target_arch = "wasm32"))]
pub fn capture(gpu: &crate::renderer::Renderer, target: &wgpu::Texture, texts: &[Text]) {
    let ctx = egui::Context::default();
    install(&ctx);
    let rect = Rect::from_min_size(
        Pos2::ZERO,
        egui::vec2(target.width() as f32, target.height() as f32),
    );
    let output = ctx.run(
        egui::RawInput {
            screen_rect: Some(rect),
            ..Default::default()
        },
        |ctx| paint(ctx, rect, texts),
    );
    let jobs = ctx.tessellate(output.shapes, output.pixels_per_point);
    let mut renderer = egui_wgpu::Renderer::new(&gpu.device, target.format(), None, 1, false);
    for (id, delta) in output.textures_delta.set {
        renderer.update_texture(&gpu.device, &gpu.queue, id, &delta);
    }
    gpu.draw_overlay(
        &target.create_view(&Default::default()),
        &mut renderer,
        &jobs,
        &egui_wgpu::ScreenDescriptor {
            size_in_pixels: [target.width(), target.height()],
            pixels_per_point: output.pixels_per_point,
        },
    );
}
