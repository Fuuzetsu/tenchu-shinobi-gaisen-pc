use crate::visuals::pixels::Image;

pub(super) fn refine(source: &Image) -> Image {
    let [width, height] = source.size;
    let scale = (512 / width.max(height).max(1)).clamp(1, 8);
    let size = source.size.map(|n| n * scale);
    let mut rgba = Vec::with_capacity((size[0] * size[1] * 4) as usize);
    for y in 0..size[1] {
        let py = (y as f32 + 0.5) / scale as f32 - 0.5;
        let wy = weights(py.fract().rem_euclid(1.0));
        for x in 0..size[0] {
            let px = (x as f32 + 0.5) / scale as f32 - 0.5;
            let wx = weights(px.fract().rem_euclid(1.0));
            let mut color = [0.0f32; 3];
            let mut coverage = 0.0;
            let mut semi = 0.0;
            for (dy, &vy) in wy.iter().enumerate() {
                for (dx, &vx) in wx.iter().enumerate() {
                    let sx = px.floor() as i32 + dx as i32 - 1;
                    let sy = py.floor() as i32 + dy as i32 - 1;
                    if sx < 0 || sy < 0 || sx >= width as i32 || sy >= height as i32 {
                        continue;
                    }
                    let at = (sy as u32 * width + sx as u32) as usize * 4;
                    let pixel = &source.rgba[at..at + 4];
                    if pixel[3] == 0 {
                        continue;
                    }
                    let weight = vx * vy;
                    coverage += weight;
                    semi += weight * f32::from(pixel[3] < 192);
                    for channel in 0..3 {
                        color[channel] += f32::from(pixel[channel]) * weight;
                    }
                }
            }
            // Reconstruct a crisp contour; do not turn the whole sprite into a blur.
            if coverage < 0.5 {
                rgba.extend([0; 4]);
            } else {
                rgba.extend(color.map(|c| (c / coverage).round().clamp(0.0, 255.0) as u8));
                rgba.push(if semi > coverage * 0.5 { 128 } else { 255 });
            }
        }
    }
    Image { size, rgba }
}

fn weights(t: f32) -> [f32; 4] {
    let t2 = t * t;
    let t3 = t2 * t;
    [
        -0.5 * t + t2 - 0.5 * t3,
        1.0 - 2.5 * t2 + 1.5 * t3,
        0.5 * t + 2.0 * t2 - 1.5 * t3,
        -0.5 * t2 + 0.5 * t3,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn thin_spatter_keeps_its_color_coverage_and_transparency_class() {
        for alpha in [128, 255] {
            let mut source = Image {
                size: [16, 16],
                rgba: vec![0; 16 * 16 * 4],
            };
            for x in 3..13 {
                source.rgba[(x * 16 + x) * 4..(x * 16 + x) * 4 + 4]
                    .copy_from_slice(&[160, 8, 16, alpha]);
            }
            source.rgba[(2 * 16 + 12) * 4..(2 * 16 + 12) * 4 + 4]
                .copy_from_slice(&[160, 8, 16, alpha]);
            let image = refine(&source);
            assert_eq!(image.size, [128, 128]);
            let occupied = image.rgba.chunks_exact(4).filter(|p| p[3] != 0).count();
            assert!(
                (560..850).contains(&occupied),
                "lost or inflated thin spatter: {occupied}"
            );
            assert!(
                image
                    .rgba
                    .chunks_exact(4)
                    .all(|p| p == [0; 4] || p == [160, 8, 16, alpha])
            );
            assert_eq!(
                image.rgba[(20 * 128 + 100) * 4 + 3],
                alpha,
                "isolated droplet survived"
            );
            assert!(image.rgba[..128 * 4].iter().all(|&n| n == 0));
        }
    }
}
