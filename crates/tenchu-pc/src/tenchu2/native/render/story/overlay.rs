use super::Builder;
use crate::{fonts, render_effects, render_mesh, renderer::Renderer, visuals};
use std::sync::Arc;
use tenchu_assets::tenchu2::story::{Command, Picture, Resources};
use tenchu_core::tenchu2::story::PresentationFrame;

pub(super) fn append(
    output: &mut Builder,
    resources: &Resources,
    frame: &PresentationFrame,
    renderer: &mut Renderer,
    event: u16,
) -> Vec<fonts::Text> {
    let start = output.surfaces.len();
    if let Some((index, intensity)) = frame.picture
        && let Some(picture) = resources.pictures.get(index)
    {
        rectangle(
            output,
            [0.0, 0.0],
            [320.0, 240.0],
            48.0,
            [f32::from(intensity * 2).clamp(0.0, 255.0) / 255.0; 3],
            3,
            true,
        );
        image(
            output,
            renderer,
            picture,
            format!("tenchu2/event/{event}/picture/{index}"),
            [0.0, 0.0],
            1.25,
            intensity,
            32.0,
        );
    }
    for overlay in &frame.overlays {
        rectangle(
            output,
            [0.0, 0.0],
            [320.0, 240.0],
            f32::from(overlay.layer) * 16.0,
            overlay.rgb.map(|v| f32::from(v.clamp(0, 255)) / 255.0),
            if overlay.subtract { 3 } else { 2 },
            true,
        );
    }
    for &(index, portrait, intensity) in &frame.portraits {
        let Some(picture) = resources.portraits.get(index) else {
            continue;
        };
        let position = portrait.position.map(f32::from);
        rectangle(
            output,
            position,
            [f32::from(picture.size[0]), 32.0],
            64.0,
            [f32::from(intensity) / 255.0; 3],
            3,
            false,
        );
        image(
            output,
            renderer,
            picture,
            format!("tenchu2/event/{event}/portrait/{index}"),
            position,
            1.0,
            i16::from(intensity),
            48.0,
        );
    }
    for y in [-104.0, 104.0] {
        rectangle(output, [0.0, y], [320.0, 32.0], 0.0, [0.0; 3], 0, true);
    }
    // The source ordering table prepends primitives within each layer.
    output.surfaces[start..].reverse();
    let mut text = Vec::new();
    if let Some(index) = frame.caption
        && let Command::Caption { text: caption, .. } = &resources.script.commands[index].command
    {
        text.push(fonts::Text {
            position: egui::pos2(0.0, 90.0),
            text: caption.clone(),
            color: egui::Color32::WHITE,
            size: 14.0,
            width: 300.0,
            centered: true,
        });
    }
    text
}

#[allow(clippy::too_many_arguments)]
fn rectangle(
    output: &mut Builder,
    center: [f32; 2],
    size: [f32; 2],
    depth: f32,
    color: [f32; 3],
    blend: u8,
    wide: bool,
) {
    let points = std::array::from_fn(|i| {
        [
            center[0] + ((i & 1) as f32 - 0.5) * size[0],
            center[1] + ((i >> 1) as f32 - 0.5) * size[1],
            depth * 0.000001,
        ]
    });
    let mut vertices = render_effects::quad(points, [color; 4], None, blend);
    for vertex in &mut vertices {
        vertex.flags |= render_mesh::UI | if wide { render_mesh::SCREEN_WIDE } else { 0 };
    }
    output.surfaces.push(render_effects::Surface {
        depth,
        blend,
        vertices,
    });
}

#[allow(clippy::too_many_arguments)]
fn image(
    output: &mut Builder,
    renderer: &mut Renderer,
    picture: &Picture,
    id: String,
    center: [f32; 2],
    scale: f32,
    intensity: i16,
    depth: f32,
) {
    let handle = renderer.bind_material(&visuals::models::Material {
        id,
        region: None,
        image: Arc::new(visuals::pixels::Image {
            size: picture.size.map(u32::from),
            rgba: picture.rgba.clone(),
        }),
        animation: None,
    });
    rectangle(
        output,
        center,
        picture.size.map(|v| f32::from(v) * scale),
        depth,
        [f32::from(intensity.clamp(0, 255)) / 128.0; 3],
        0,
        false,
    );
    for vertex in &mut output.surfaces.last_mut().unwrap().vertices {
        vertex.flags |= render_mesh::TEXTURED;
        vertex.material = handle;
        vertex.uv = std::array::from_fn(|i| {
            (vertex.position[i] - center[i]) / (f32::from(picture.size[i]) * scale)
        });
    }
}
