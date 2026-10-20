#[derive(Clone, PartialEq)]
pub struct Image {
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
}

pub struct Mip {
    pub size: [u32; 2],
    pub classes: [Vec<u8>; 2],
}

impl Image {
    pub fn upscale(&self, scale: u16) -> Self {
        let mut image = self.clone();
        for _ in 0..scale.max(1).ilog2() {
            image = image.scale2x();
        }
        image
    }

    fn scale2x(&self) -> Self {
        let [width, height] = self.size;
        let pixel = |x: i32, y: i32| -> [u8; 4] {
            let index = (y.clamp(0, height as i32 - 1) as u32 * width
                + x.clamp(0, width as i32 - 1) as u32) as usize
                * 4;
            self.rgba[index..index + 4].try_into().unwrap()
        };
        let mut out = Self {
            size: [width * 2, height * 2],
            rgba: vec![0; (width * height * 16) as usize],
        };
        for y in 0..height as i32 {
            for x in 0..width as i32 {
                let center = pixel(x, y);
                let up = pixel(x, y - 1);
                let down = pixel(x, y + 1);
                let left = pixel(x - 1, y);
                let right = pixel(x + 1, y);
                let mut quad = [center; 4];
                if up != down && left != right {
                    for (at, a, b) in [
                        (0, left, up),
                        (1, up, right),
                        (2, left, down),
                        (3, down, right),
                    ] {
                        if a == b {
                            quad[at] = a;
                        }
                    }
                }
                for (i, color) in quad.into_iter().enumerate() {
                    let index = (((y as u32 * 2 + i as u32 / 2) * out.size[0]
                        + x as u32 * 2
                        + i as u32 % 2)
                        * 4) as usize;
                    out.rgba[index..index + 4].copy_from_slice(&color);
                }
            }
        }
        out
    }

    pub fn mips(&self) -> Vec<Mip> {
        let mut levels = vec![Mip {
            size: self.size,
            classes: std::array::from_fn(|class| {
                self.rgba
                    .chunks_exact(4)
                    .flat_map(|pixel| {
                        if pixel[3] > 0 && usize::from(pixel[3] < 192) == class {
                            [pixel[0], pixel[1], pixel[2], 255]
                        } else {
                            [0; 4]
                        }
                    })
                    .collect()
            }),
        }];
        while levels.last().unwrap().size != [1, 1] {
            let previous = levels.last().unwrap();
            let size = previous.size.map(|n| n.div_ceil(2));
            let classes = previous.classes.each_ref().map(|pixels| {
                let mut out = Vec::with_capacity((size[0] * size[1] * 4) as usize);
                for y in 0..size[1] {
                    for x in 0..size[0] {
                        let mut sum = [0u32; 4];
                        let mut count = 0;
                        for py in y * 2..(y * 2 + 2).min(previous.size[1]) {
                            for px in x * 2..(x * 2 + 2).min(previous.size[0]) {
                                let at = ((py * previous.size[0] + px) * 4) as usize;
                                for channel in 0..4 {
                                    sum[channel] += u32::from(pixels[at + channel]);
                                }
                                count += 1;
                            }
                        }
                        out.extend(sum.map(|n| ((n + count / 2) / count) as u8));
                    }
                }
                out
            });
            levels.push(Mip { size, classes });
        }
        levels
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mipmaps_keep_coverage_and_blend_classes_separate() {
        let image = Image {
            size: [2, 2],
            rgba: vec![255, 0, 0, 255, 0, 255, 0, 128, 0, 0, 255, 0, 0, 0, 0, 255],
        };
        let levels = image.mips();
        assert_eq!(levels[1].classes[0], [64, 0, 0, 128]);
        assert_eq!(levels[1].classes[1], [0, 64, 0, 64]);
        let odd = Image {
            size: [3, 1],
            rgba: [255, 0, 0, 255].repeat(3),
        };
        assert_eq!(odd.mips().last().unwrap().classes[0], [255, 0, 0, 255]);
    }
}
