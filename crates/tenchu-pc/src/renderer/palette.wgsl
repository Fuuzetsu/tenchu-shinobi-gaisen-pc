fn word(x: u32, y: u32) -> u32 {
    return textureLoad(vram, vec2<i32>(i32(x & 1023u), i32(y & 511u)), 0).r;
}

fn texel(coordinate: vec2<i32>, page: u32, palette: u32) -> u32 {
    let uv = vec2<u32>(coordinate & vec2(255));
        let depth = (page >> 7u) & 3u;
        let x = (page & 15u) * 64u;
        let y = ((page >> 4u) & 1u) * 256u;
        var pixel: u32;
        if depth == 0u {
            let packed = word(x + uv.x / 4u, y + uv.y);
            let index = (packed >> ((uv.x & 3u) * 4u)) & 15u;
            pixel = word((palette & 63u) * 16u + index, palette >> 6u);
        } else if depth == 1u {
            let packed = word(x + uv.x / 2u, y + uv.y);
            let index = (packed >> ((uv.x & 1u) * 8u)) & 255u;
            pixel = word((palette & 63u) * 16u + index, palette >> 6u);
        } else {
            pixel = word(x + uv.x, y + uv.y);
        }
    return pixel;
}

fn rgb(pixel: u32) -> vec3<f32> {
    return vec3<f32>(f32(pixel & 31u), f32((pixel >> 5u) & 31u), f32((pixel >> 10u) & 31u)) / 31.0;
}

fn filtered(uv: vec2<f32>, page: u32, palette: u32, nearest: u32) -> vec3<f32> {
    let base = vec2<i32>(floor(uv));
    let f = fract(uv);
    var color = vec3(0.0);
    var weight = 0.0;
    for (var y = 0; y < 2; y++) {
        for (var x = 0; x < 2; x++) {
            let pixel = texel(base + vec2(x, y), page, palette);
            let w = select(1.0 - f.x, f.x, x == 1) * select(1.0 - f.y, f.y, y == 1);
            if pixel != 0u && (pixel & 32768u) == (nearest & 32768u) {
                color += rgb(pixel) * w;
                weight += w;
            }
        }
    }
    return select(rgb(nearest), color / max(weight, 0.0001), weight > 0.0001);
}

