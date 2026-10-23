use super::review::{self, Review};
use crate::{render_effects, visuals::catalog};
use glam::Mat4;
use std::{collections::BTreeSet, fs, path::Path};
use tenchu_assets::{Result, stage::MissionAssets};

pub fn render(mission: &MissionAssets, output: &Path, indices: &[&str]) -> Result<()> {
    let indices = indices
        .iter()
        .map(|index| {
            index
                .parse::<usize>()
                .ok()
                .filter(|&i| i < mission.images.len())
                .ok_or_else(|| format!("Invalid sprite index {index}"))
        })
        .collect::<Result<BTreeSet<_>>>()?;
    review::prepare_directory(output)?;
    let mut review = Review::new(mission)?;
    let mut views = Vec::new();
    for index in indices {
        let image = mission.images[index];
        let region = catalog::Region::sprite(&mission.vram, image);
        let identity = catalog::image_id(
            [region.rect[2], region.rect[3]],
            &region.original_pixels(&mission.vram),
        );
        let mut geometry = render_effects::Geometry {
            vertices: Vec::new(),
            batches: Vec::new(),
        };
        let size = image.size.map(f32::from);
        let scale = 70.0 / size[0].max(size[1]).max(1.0);
        let [width, height] = size.map(|n| n * scale * 0.5);
        for blend in 0..=4 {
            let x = (f32::from(blend) - 2.0) * 82.0;
            let start = geometry.vertices.len() as u32;
            geometry.vertices.extend(render_effects::quad(
                [
                    [x - width, -height, 0.1],
                    [x + width, -height, 0.1],
                    [x - width, height, 0.1],
                    [x + width, height, 0.1],
                ],
                [[1.0; 3]; 4],
                Some(image),
                blend,
            ));
            geometry.batches.push(render_effects::Batch {
                blend,
                range: start..geometry.vertices.len() as u32,
            });
        }
        let effects = review.renderer.upload_effects(geometry);
        for (name, shade) in [("black", 0.0), ("grey", 0.4), ("white", 0.9)] {
            let background = review.backdrop(shade);
            for (enabled, suffix) in [(false, "original"), (true, "pack")] {
                review.capture(
                    Mat4::IDENTITY,
                    &[&background],
                    Some(&effects),
                    enabled,
                    &output.join(format!("{identity}-{name}-{suffix}.png")),
                )?;
            }
            views.push(
                serde_json::json!({"texture":identity,"sprite":index,"view":name,
                "blends_left_to_right":["opaque","average","add","subtract","quarter_add"]}),
            );
        }
    }
    fs::write(
        output.join("views.json"),
        serde_json::to_vec_pretty(&views).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())
}
