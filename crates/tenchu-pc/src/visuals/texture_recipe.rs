// Resampling/compositing compatibility: see data/image-compat/README.md.
use super::pixels::Image;
use serde::{Deserialize, Serialize};
use tenchu_assets::Result;

mod resize;

#[derive(Clone, Copy, Default, Deserialize, Serialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Filter {
    #[default]
    Nearest,
    Bilinear,
    Bicubic,
    Lanczos,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Source {
    #[default]
    Original,
    Current,
    File(String),
}

#[derive(Clone, Copy, Default, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Channel {
    #[default]
    Alpha,
    Red,
    Luma,
}

#[derive(Default, Deserialize, Serialize)]
#[serde(default, deny_unknown_fields)]
pub struct Mask {
    pub source: Source,
    pub channel: Channel,
    pub filter: Filter,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intermediate_size: Option<[u32; 2]>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub before: Vec<u8>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub after: Vec<u8>,
}

#[derive(Deserialize, Serialize)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
pub enum Step {
    Opaque,
    ClearAlpha {
        rect: [u32; 4],
    },
    Resize {
        size: [u32; 2],
        filter: Filter,
    },
    Grain {
        image: String,
        smooth: String,
        gain: f64,
    },
    Overlay {
        image: String,
    },
    Blend {
        image: String,
        weight: f32,
    },
    MaskBlend {
        image: String,
        mask: Mask,
    },
    Alpha {
        mask: Mask,
    },
    Matte {
        color: [u8; 3],
    },
}

#[derive(Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Recipe {
    #[serde(default)]
    pub base: Source,
    pub steps: Vec<Step>,
}

impl Recipe {
    pub fn validate(&self) -> Result<()> {
        if self.steps.is_empty() || self.steps.len() > 16 {
            return Err("A texture recipe needs 1–16 operations".into());
        }
        if matches!(self.base, Source::Current) {
            return Err("A texture recipe cannot start from current".into());
        }
        for step in &self.steps {
            match step {
                Step::ClearAlpha { rect: [x, y, w, h] }
                    if *w == 0
                        || *h == 0
                        || x.saturating_add(*w) > 1024
                        || y.saturating_add(*h) > 1024 =>
                {
                    return Err("Texture alpha rectangle must fit within 1024 pixels".into());
                }
                Step::Resize { size, .. } if size.iter().any(|&n| n == 0 || n > 1024) => {
                    return Err("Texture recipe sizes must be 1–1024 pixels".into());
                }
                Step::Grain { gain, .. } if !gain.is_finite() || !(0.0..=2.0).contains(gain) => {
                    return Err("Texture grain gain must be 0–2".into());
                }
                Step::Blend { weight, .. }
                    if !weight.is_finite() || !(0.0..=1.0).contains(weight) =>
                {
                    return Err("Texture blend weight must be 0–1".into());
                }
                Step::Alpha { mask } | Step::MaskBlend { mask, .. } => {
                    if mask
                        .intermediate_size
                        .is_some_and(|size| size.iter().any(|&n| n == 0 || n > 1024))
                    {
                        return Err("Texture mask sizes must be 1–1024 pixels".into());
                    }
                    for table in [&mask.before, &mask.after] {
                        if !table.is_empty() && table.len() != 256 {
                            return Err("Texture mask tables must contain 256 values".into());
                        }
                    }
                }
                _ => {}
            }
        }
        for file in self.files() {
            super::pack::validate_path(file)?;
        }
        Ok(())
    }

    pub fn files(&self) -> Vec<&str> {
        let mut files = Vec::new();
        if let Source::File(path) = &self.base {
            files.push(path.as_str());
        }
        for step in &self.steps {
            match step {
                Step::Grain { image, smooth, .. } => {
                    files.extend([image.as_str(), smooth.as_str()])
                }
                Step::Overlay { image }
                | Step::Blend { image, .. }
                | Step::MaskBlend { image, .. } => {
                    files.push(image.as_str());
                }
                _ => {}
            }
            if let Step::Alpha { mask } | Step::MaskBlend { mask, .. } = step
                && let Source::File(path) = &mask.source
            {
                files.push(path.as_str());
            }
        }
        files
    }

    pub fn apply(
        &self,
        original: &Image,
        mut read: impl FnMut(&str) -> Result<Image>,
    ) -> Result<Image> {
        self.validate()?;
        let mut image = match &self.base {
            Source::Original => original.clone(),
            Source::File(path) => read(path)?,
            Source::Current => unreachable!(),
        };
        for step in &self.steps {
            match step {
                Step::Opaque => image.rgba.chunks_exact_mut(4).for_each(|p| p[3] = 255),
                Step::ClearAlpha { rect: [x, y, w, h] } => {
                    if x + w > image.size[0] || y + h > image.size[1] {
                        return Err("Texture alpha rectangle leaves the image".into());
                    }
                    for row in *y..y + h {
                        for column in *x..x + w {
                            image.rgba[((row * image.size[0] + column) * 4 + 3) as usize] = 0;
                        }
                    }
                }
                Step::Resize { size, filter } => image = resize::resize(&image, *size, *filter),
                Step::Grain {
                    image: path,
                    smooth,
                    gain,
                } => {
                    let detail = read(path)?;
                    let smooth = read(smooth)?;
                    same_size(&image, &detail)?;
                    same_size(&image, &smooth)?;
                    for ((pixel, detail), smooth) in image
                        .rgba
                        .chunks_exact_mut(4)
                        .zip(detail.rgba.chunks_exact(4))
                        .zip(smooth.rgba.chunks_exact(4))
                    {
                        for i in 0..3 {
                            pixel[i] = (f64::from(pixel[i])
                                + gain * (f64::from(detail[i]) - f64::from(smooth[i])))
                            .round_ties_even()
                            .clamp(0.0, 255.0) as u8;
                        }
                    }
                }
                Step::Overlay { image: path } => {
                    let layer = read(path)?;
                    same_size(&image, &layer)?;
                    for (pixel, layer) in image
                        .rgba
                        .chunks_exact_mut(4)
                        .zip(layer.rgba.chunks_exact(4))
                    {
                        composite(pixel, layer);
                    }
                }
                Step::Blend {
                    image: path,
                    weight,
                } => {
                    let layer = read(path)?;
                    same_size(&image, &layer)?;
                    for (pixel, layer) in image
                        .rgba
                        .chunks_exact_mut(4)
                        .zip(layer.rgba.chunks_exact(4))
                    {
                        for i in 0..3 {
                            pixel[i] = (f32::from(pixel[i])
                                + weight * (f32::from(layer[i]) - f32::from(pixel[i])))
                                as u8;
                        }
                    }
                }
                Step::MaskBlend { image: path, mask } => {
                    let layer = read(path)?;
                    same_size(&image, &layer)?;
                    let mask = mask.pixels(&image, original, &mut read)?;
                    for ((pixel, layer), mask) in image
                        .rgba
                        .chunks_exact_mut(4)
                        .zip(layer.rgba.chunks_exact(4))
                        .zip(mask)
                    {
                        for i in 0..3 {
                            pixel[i] = ((u32::from(pixel[i]) * (255 - u32::from(mask))
                                + u32::from(layer[i]) * u32::from(mask)
                                + 127)
                                / 255) as u8;
                        }
                    }
                }
                Step::Alpha { mask } => {
                    let mask = mask.pixels(&image, original, &mut read)?;
                    for (pixel, alpha) in image.rgba.chunks_exact_mut(4).zip(mask) {
                        pixel[3] = alpha;
                    }
                }
                Step::Matte { color } => {
                    for pixel in image.rgba.chunks_exact_mut(4) {
                        let layer: [u8; 4] = pixel.try_into().unwrap();
                        let mut base = [color[0], color[1], color[2], 255];
                        composite(&mut base, &layer);
                        pixel[..3].copy_from_slice(&base[..3]);
                    }
                }
            }
        }
        Ok(image)
    }
}

impl Mask {
    fn pixels(
        &self,
        current: &Image,
        original: &Image,
        read: &mut impl FnMut(&str) -> Result<Image>,
    ) -> Result<Vec<u8>> {
        let file;
        let source = match &self.source {
            Source::Current => current,
            Source::Original => original,
            Source::File(path) => {
                file = read(path)?;
                &file
            }
        };
        let rgba = source
            .rgba
            .chunks_exact(4)
            .flat_map(|pixel| {
                let mut value = match self.channel {
                    Channel::Alpha => pixel[3],
                    Channel::Red => pixel[0],
                    Channel::Luma => {
                        ((u32::from(pixel[0]) * 19595
                            + u32::from(pixel[1]) * 38470
                            + u32::from(pixel[2]) * 7471
                            + 32768)
                            >> 16) as u8
                    }
                };
                if !self.before.is_empty() {
                    value = self.before[usize::from(value)];
                }
                [value, value, value, 255]
            })
            .collect();
        let mut mask = Image {
            size: source.size,
            rgba,
        };
        if let Some(size) = self.intermediate_size {
            mask = resize::resize(&mask, size, self.filter);
        }
        let mask = resize::resize(&mask, current.size, self.filter);
        Ok(mask
            .rgba
            .chunks_exact(4)
            .map(|pixel| {
                if self.after.is_empty() {
                    pixel[0]
                } else {
                    self.after[usize::from(pixel[0])]
                }
            })
            .collect())
    }
}

fn same_size(a: &Image, b: &Image) -> Result<()> {
    if a.size == b.size {
        Ok(())
    } else {
        Err(format!(
            "Texture recipe layer size {:?} does not match {:?}",
            b.size, a.size
        ))
    }
}

fn composite(pixel: &mut [u8], layer: &[u8]) {
    if layer[3] == 0 {
        return;
    }
    let alpha = u32::from(layer[3]) * 255 + u32::from(pixel[3]) * (255 - u32::from(layer[3]));
    let foreground = u32::from(layer[3]) * 255 * 255 * 128 / alpha;
    let background = 255 * 128 - foreground;
    let div255 = |n: u32| ((n >> 8) + n) >> 8;
    for i in 0..3 {
        pixel[i] = (div255(
            u32::from(layer[i]) * foreground + u32::from(pixel[i]) * background + 128 * 128,
        ) >> 7) as u8;
    }
    pixel[3] = div255(alpha + 128) as u8;
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::visuals::catalog;

    #[test]
    fn resampling_matches_authoring_rounding_and_transparency() {
        #[derive(Deserialize)]
        struct Case {
            input: [u32; 2],
            opaque: bool,
            filter: Filter,
            size: [u32; 2],
            hash: String,
        }
        let cases: Vec<Case> =
            serde_json::from_str(include_str!("../../data/image-compat/resize-golden.json"))
                .unwrap();
        for case in cases {
            let original = Image {
                size: case.input,
                rgba: (0..(case.input[0] * case.input[1]) as usize)
                    .flat_map(|i| {
                        [
                            (i * 37 + 11) as u8,
                            (i * 73 + 29) as u8,
                            (i * 19 + 203) as u8,
                            if case.opaque {
                                255
                            } else {
                                [0, 64, 128, 255][i % 4]
                            },
                        ]
                    })
                    .collect(),
            };
            let result = resize::resize(&original, case.size, case.filter);
            assert_eq!(
                catalog::image_id(case.size.map(|n| n as u16), &result.rgba),
                case.hash,
                "filter {} size {:?}, opaque {}",
                serde_json::to_string(&case.filter).unwrap(),
                case.size,
                case.opaque
            );
        }
    }

    #[test]
    fn composed_recipes_match_authoring_and_reject_invalid_operations() {
        let values: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../../data/image-compat/recipe-golden.json"))
                .unwrap();
        let synthetic = |size: [u32; 2]| Image {
            size,
            rgba: (0..(size[0] * size[1]) as usize)
                .flat_map(|i| {
                    [
                        (i * 37 + 11) as u8,
                        (i * 73 + 29) as u8,
                        (i * 19 + 203) as u8,
                        [0, 64, 128, 255][i % 4],
                    ]
                })
                .collect(),
        };
        let original = synthetic([7, 5]);
        for mut value in values {
            let hash = value.as_object_mut().unwrap().remove("hash").unwrap();
            let recipe: Recipe = serde_json::from_value(value).unwrap();
            let image = recipe
                .apply(&original, |path| match path {
                    "paint.png" => Ok(synthetic([13, 9])),
                    "smooth.png" => Ok(Image {
                        size: [13, 9],
                        rgba: [111, 100, 90, 255].repeat(13 * 9),
                    }),
                    _ => Err(path.into()),
                })
                .unwrap();
            assert_eq!(
                catalog::image_id(image.size.map(|n| n as u16), &image.rgba),
                hash.as_str().unwrap()
            );
        }
        for value in [
            serde_json::json!({"steps": []}),
            serde_json::json!({"steps": [{"op":"resize", "size":[0, 1], "filter":"nearest"}]}),
            serde_json::json!({"steps": [{"op":"overlay", "image":"../outside.png"}]}),
            serde_json::json!({"steps": [{"op":"alpha", "mask":{"before":[255]}}]}),
        ] {
            let recipe: Recipe = serde_json::from_value(value).unwrap();
            assert!(
                recipe
                    .apply(&original, |_| panic!(
                        "Invalid recipe must fail before reads"
                    ))
                    .is_err()
            );
        }
    }

    #[test]
    #[ignore = "requires TENCHU_RECIPE_FIXTURE with private original/reference PNG paths"]
    fn reconstructed_pack_matches_accepted_artwork() {
        use std::{fs, path::Path};
        #[derive(Deserialize)]
        struct Fixture {
            pack: String,
            originals: std::collections::BTreeMap<String, String>,
            expected: String,
        }
        let path = std::env::var("TENCHU_RECIPE_FIXTURE").expect("TENCHU_RECIPE_FIXTURE");
        let fixture: Fixture = serde_json::from_slice(&fs::read(path).unwrap()).unwrap();
        let root = Path::new(&fixture.pack);
        let manifest: super::super::pack::Manifest =
            serde_json::from_slice(&fs::read(root.join("pack.json")).unwrap()).unwrap();
        let expected: super::super::pack::Manifest = serde_json::from_slice(
            &fs::read(Path::new(&fixture.expected).join("pack.json")).unwrap(),
        )
        .unwrap();
        let decode =
            |path: &Path| super::super::pack::decode_png(&fs::read(path).unwrap()).unwrap();
        let started = std::time::Instant::now();
        let mut failures = vec![];
        for (id, recipe) in &manifest.recipes {
            let original = decode(Path::new(&fixture.originals[id]));
            let result = recipe
                .apply(&original, |file| Ok(decode(&root.join(file))))
                .unwrap();
            let reference = decode(&Path::new(&fixture.expected).join(&expected.textures[id]));
            if result != reference {
                let different = result
                    .rgba
                    .iter()
                    .zip(&reference.rgba)
                    .filter(|(a, b)| a != b)
                    .count();
                failures.push(format!(
                    "{id}: {:?} vs {:?}; {different} bytes",
                    result.size, reference.size
                ));
            }
        }
        println!(
            "{} recipes: {:?}",
            manifest.recipes.len(),
            started.elapsed()
        );
        assert!(failures.is_empty(), "{}", failures.join("\n"));
    }
}
