use super::pixels::Image;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

// UV-space slopes keep their precision when the color texture is upscaled.
pub const GRADIENT_RANGE: f32 = 32.0;

#[derive(Clone, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Rule {
    pub height: Option<String>,
    pub strength: f32,
    pub world_tile: Option<[f32; 2]>,
}

impl Default for Rule {
    fn default() -> Self {
        Self {
            height: None,
            strength: 1.0,
            world_tile: None,
        }
    }
}

#[derive(Clone)]
pub struct Surface {
    pub height: Option<Arc<Image>>,
    pub strength: f32,
    pub world_tile: Option<[f32; 2]>,
}

/// Remove broad painted lighting; retain softened grain, ridges and seams.
fn inferred(image: &Image) -> Vec<f32> {
    let luma: Vec<_> = image
        .rgba
        .chunks_exact(4)
        .map(|p| {
            (0.2126 * f32::from(p[0]) + 0.7152 * f32::from(p[1]) + 0.0722 * f32::from(p[2])) / 255.0
        })
        .collect();
    let small = blur(&luma, image.size, 1);
    let broad = blur(&small, image.size, 5);
    small
        .iter()
        .zip(broad)
        .map(|(a, b)| {
            let difference = a - b;
            if difference.abs() < 0.000001 {
                0.5
            } else {
                (0.5 + difference * 1.5).clamp(0.0, 1.0)
            }
        })
        .collect()
}

fn blur(input: &[f32], size: [u32; 2], radius: i32) -> Vec<f32> {
    let mut values = input.to_vec();
    for axis in 0..2 {
        let mut out = vec![0.0; values.len()];
        for y in 0..size[1] as i32 {
            for x in 0..size[0] as i32 {
                let mut sum = 0.0;
                for offset in -radius..=radius {
                    let sx = (x + if axis == 0 { offset } else { 0 }).clamp(0, size[0] as i32 - 1);
                    let sy = (y + if axis == 1 { offset } else { 0 }).clamp(0, size[1] as i32 - 1);
                    sum += values[sy as usize * size[0] as usize + sx as usize];
                }
                out[y as usize * size[0] as usize + x as usize] = sum / (radius * 2 + 1) as f32;
            }
        }
        values = out;
    }
    values
}

fn resize(input: &[f32], from: [u32; 2], to: [u32; 2]) -> Vec<f32> {
    let sample = |x: i32, y: i32| {
        input[(y.clamp(0, from[1] as i32 - 1) as u32 * from[0]
            + x.clamp(0, from[0] as i32 - 1) as u32) as usize]
    };
    let mut out = Vec::with_capacity((to[0] * to[1]) as usize);
    let scale = [from[0] as f32 / to[0] as f32, from[1] as f32 / to[1] as f32];
    for y in 0..to[1] {
        for x in 0..to[0] {
            let taps = scale.map(|n| n.ceil().max(1.0) as u32);
            let mut sum = 0.0;
            for j in 0..taps[1] {
                for i in 0..taps[0] {
                    let px = (x as f32 + (i as f32 + 0.5) / taps[0] as f32) * scale[0] - 0.5;
                    let py = (y as f32 + (j as f32 + 0.5) / taps[1] as f32) * scale[1] - 0.5;
                    let [ix, iy] = [px.floor() as i32, py.floor() as i32];
                    let [fx, fy] = [px - px.floor(), py - py.floor()];
                    sum += (sample(ix, iy) * (1.0 - fx) + sample(ix + 1, iy) * fx) * (1.0 - fy)
                        + (sample(ix, iy + 1) * (1.0 - fx) + sample(ix + 1, iy + 1) * fx) * fy;
                }
            }
            out.push(sum / (taps[0] * taps[1]) as f32);
        }
    }
    out
}

pub fn mips(image: &Image, explicit: Option<&Image>, sizes: &[[u32; 2]]) -> Vec<Vec<u8>> {
    if image.size.into_iter().any(|n| n < 4) || image.rgba.chunks_exact(4).any(|p| p[3] == 0) {
        return Vec::new();
    }
    let (size, heights) = if let Some(map) = explicit {
        (
            map.size,
            map.rgba
                .chunks_exact(4)
                .map(|p| f32::from(p[0]) / 255.0)
                .collect(),
        )
    } else {
        (image.size, inferred(image))
    };
    sizes
        .iter()
        .map(|&level| {
            let heights = resize(&heights, size, level);
            let at = |x: i32, y: i32| {
                heights[(y.clamp(0, level[1] as i32 - 1) as u32 * level[0]
                    + x.clamp(0, level[0] as i32 - 1) as u32) as usize]
            };
            let mut pixels = Vec::with_capacity(heights.len() * 4);
            for y in 0..level[1] as i32 {
                for x in 0..level[0] as i32 {
                    let dx = (at(x + 1, y) - at(x - 1, y)) * 0.5 * level[0] as f32 / GRADIENT_RANGE;
                    let dy = (at(x, y + 1) - at(x, y - 1)) * 0.5 * level[1] as f32 / GRADIENT_RANGE;
                    pixels.extend([
                        (128.0 + dx * 127.0).round() as u8,
                        (128.0 + dy * 127.0).round() as u8,
                        (at(x, y) * 255.0).round() as u8,
                        255,
                    ]);
                }
            }
            pixels
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_and_cutout_art_stay_flat_while_height_gradients_filter_with_distance() {
        let mut image = Image {
            size: [8, 8],
            rgba: [120, 90, 60, 255].repeat(64),
        };
        let sizes = [[8, 8], [4, 4], [2, 2], [1, 1]];
        assert!(
            mips(&image, None, &sizes)
                .iter()
                .flat_map(|m| m.chunks_exact(4))
                .all(|p| p == [128, 128, 128, 255])
        );
        let map = Image {
            size: [8, 8],
            rgba: (0..64)
                .flat_map(|i| [((i % 8) * 32) as u8, 0, 0, 255])
                .collect(),
        };
        let levels = mips(&image, Some(&map), &sizes);
        assert!(levels[0].chunks_exact(4).all(|p| p[0] > 128 && p[1] == 128));
        assert_eq!(&levels[3][..2], &[128, 128]);
        let upscaled = mips(&image, Some(&map), &[[128, 128]]);
        let original = levels[0][(3 * 8 + 3) * 4];
        let larger = upscaled[0][(56 * 128 + 56) * 4];
        assert!(original.abs_diff(larger) <= 1);
        image.rgba[3] = 0;
        assert!(mips(&image, Some(&map), &sizes).is_empty());
    }
}
