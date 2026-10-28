use super::{Filter, Image};

const PRECISION: u32 = 22;

struct Weights {
    start: usize,
    values: Vec<i32>,
}

fn kernel(filter: Filter, x: f64) -> f64 {
    let x = x.abs();
    match filter {
        Filter::Nearest => unreachable!(),
        Filter::Bilinear => (1.0 - x).max(0.0),
        Filter::Bicubic if x < 1.0 => (1.5 * x - 2.5) * x * x + 1.0,
        Filter::Bicubic if x < 2.0 => (((x - 5.0) * x + 8.0) * x - 4.0) * -0.5,
        Filter::Lanczos if x == 0.0 => 1.0,
        Filter::Lanczos if x < 3.0 => {
            let x = x * std::f64::consts::PI;
            (x.sin() / x) * ((x / 3.0).sin() / (x / 3.0))
        }
        _ => 0.0,
    }
}

fn weights(input: usize, output: usize, filter: Filter) -> Vec<Weights> {
    let scale = input as f64 / output as f64;
    let stretch = scale.max(1.0);
    let support = stretch
        * match filter {
            Filter::Bilinear => 1.0,
            Filter::Bicubic => 2.0,
            Filter::Lanczos => 3.0,
            Filter::Nearest => unreachable!(),
        };
    (0..output)
        .map(|i| {
            let center = (i as f64 + 0.5) * scale;
            let start = (center - support + 0.5).max(0.0) as usize;
            let end = ((center + support + 0.5) as usize).min(input);
            let mut values: Vec<_> = (start..end)
                .map(|j| kernel(filter, (j as f64 - center + 0.5) * (1.0 / stretch)))
                .collect();
            let sum: f64 = values.iter().sum();
            for value in &mut values {
                *value /= sum;
            }
            Weights {
                start,
                values: values
                    .into_iter()
                    .map(|w| (w * f64::from(1 << PRECISION)).round() as i32)
                    .collect(),
            }
        })
        .collect()
}

pub(super) fn resize(image: &Image, size: [u32; 2], filter: Filter) -> Image {
    if image.size == size {
        return image.clone();
    }
    let [width, height] = image.size.map(|v| v as usize);
    let [out_width, out_height] = size.map(|v| v as usize);
    if filter == Filter::Nearest {
        let mut rgba = Vec::with_capacity(out_width * out_height * 4);
        let indices = |input: usize, output: usize| {
            let scale = input as f64 / output as f64;
            let mut position = scale * 0.5;
            (0..output)
                .map(|_| {
                    let index = (position as usize).min(input - 1);
                    position += scale;
                    index
                })
                .collect::<Vec<_>>()
        };
        let columns = indices(width, out_width);
        for sy in indices(height, out_height) {
            for &sx in &columns {
                let at = (sy * width + sx) * 4;
                rgba.extend_from_slice(&image.rgba[at..at + 4]);
            }
        }
        return Image { size, rgba };
    }
    let mut pixels = image.rgba.clone();
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        for color in &mut pixel[..3] {
            *color = ((u32::from(*color) * alpha + 127) / 255) as u8;
        }
    }
    if width != out_width {
        let coefficients = weights(width, out_width, filter);
        let mut horizontal = Vec::with_capacity(out_width * height * 4);
        for row in pixels.chunks_exact(width * 4) {
            for weights in &coefficients {
                let mut channels = [1 << (PRECISION - 1); 4];
                for (pixel, &weight) in row[weights.start * 4..]
                    .chunks_exact(4)
                    .zip(&weights.values)
                {
                    for (sum, &value) in channels.iter_mut().zip(pixel) {
                        *sum += i32::from(value) * weight;
                    }
                }
                horizontal.extend(channels.map(|n| (n >> PRECISION).clamp(0, 255) as u8));
            }
        }
        pixels = horizontal;
    }
    if height != out_height {
        let coefficients = weights(height, out_height, filter);
        let mut vertical = Vec::with_capacity(out_width * out_height * 4);
        for weights in &coefficients {
            for x in 0..out_width {
                let mut channels = [1 << (PRECISION - 1); 4];
                for (offset, &weight) in weights.values.iter().enumerate() {
                    let at = ((weights.start + offset) * out_width + x) * 4;
                    for (sum, &value) in channels.iter_mut().zip(&pixels[at..at + 4]) {
                        *sum += i32::from(value) * weight;
                    }
                }
                vertical.extend(channels.map(|n| (n >> PRECISION).clamp(0, 255) as u8));
            }
        }
        pixels = vertical;
    }
    for pixel in pixels.chunks_exact_mut(4) {
        let alpha = u32::from(pixel[3]);
        if alpha != 0 && alpha != 255 {
            for color in &mut pixel[..3] {
                *color = (u32::from(*color) * 255 / alpha).min(255) as u8;
            }
        }
    }
    Image { size, rgba: pixels }
}
