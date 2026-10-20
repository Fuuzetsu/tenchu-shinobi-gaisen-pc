#[derive(Clone)]
pub struct Image {
    pub size: [u32; 2],
    pub rgba: Vec<u8>,
}

pub struct Mip {
    pub size: [u32; 2],
    pub classes: [Vec<u8>; 2],
}

impl Image {
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
