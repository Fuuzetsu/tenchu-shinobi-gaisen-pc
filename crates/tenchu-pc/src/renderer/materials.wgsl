@group(2) @binding(0) var material_images: texture_2d_array<f32>;
@group(2) @binding(1) var material_sampler: sampler;

fn material_emission(index: u32, color: vec3<f32>) -> vec4<f32> {
    if index == 0u || index >= material_count() { return vec4(0.0); }
    let emission = material_emissive(index);
    if all(emission.rgb == vec3(0.0)) { return vec4(0.0); }
    let brightness = dot(color, vec3(0.2126, 0.7152, 0.0722));
    return vec4(color * emission.rgb, smoothstep(emission.a - 0.05, emission.a + 0.05, brightness));
}

fn material_replaced(index: u32) -> bool {
    return index != 0u && index < material_count() && (material_info(index).w & 1u) != 0u;
}

fn material_coordinate(index: u32, world: vec3<f32>, uv: vec2<f32>) -> vec2<f32> {
    if index != 0u && index < material_count() && (material_info(index).w & 32u) != 0u {
        let tile = material_relief(index).zw;
        return world.xz / tile - vec2(0.5);
    }
    return uv;
}

fn material_transmits(index: u32, color: vec3<f32>) -> bool {
    if index == 0u || index >= material_count() { return false; }
    let emission = material_emissive(index);
    return any(emission.rgb > vec3(0.0))
        && dot(color, vec3(0.2126, 0.7152, 0.0722)) >= emission.a;
}

fn material_uv(index: u32, uv: vec2<f32>) -> vec2<f32> {
    if (material_info(index).w & 16u) != 0u { return fract(uv); }
    return uv;
}

fn material_point(index: u32, uv: vec2<f32>, size: vec2<f32>) -> vec2<f32> {
    if (material_info(index).w & 16u) != 0u { return fract(uv) * size; }
    return clamp(uv * size, vec2(0.5), size - vec2(0.5));
}

fn material_texel(index: u32, coordinate: vec2<f32>) -> vec4<f32> {
    let source = material_source(index);
    let info = material_info(index);
    let uv = material_uv(index, (coordinate - source.xy + vec2(0.5)) / source.zw);
    let rect = material_rect(index, 0u);
    let side = textureDimensions(material_images).x;
    let page = u32(rect.x) / side;
    let origin = vec2<i32>(i32(u32(rect.x) % side), i32(rect.y));
    let point = origin + vec2<i32>(clamp(floor(uv * rect.zw), vec2(0.0), rect.zw - vec2(1.0)));
    var opaque = vec4(0.0);
    var semi = vec4(0.0);
    if (info.w & 2u) != 0u { opaque = textureLoad(material_images, point, i32(page * select(2u, 3u, (info.w & 8u) != 0u)), 0); }
    if (info.w & 4u) != 0u { semi = textureLoad(material_images, point, i32(page * select(2u, 3u, (info.w & 8u) != 0u) + 1u), 0); }
    return vec4(opaque.rgb + semi.rgb, opaque.a + semi.a * 0.5);
}

fn material_level(index: u32, uv: vec2<f32>, level: u32, semi: bool, blended: bool) -> vec4<f32> {
    let info = material_info(index);
    let rect = material_rect(index, level);
    let side = f32(textureDimensions(material_images).x);
    let page = u32(rect.x) / u32(side);
    let origin = vec2(rect.x - f32(page) * side, rect.y);
    let coordinate = (origin + material_point(index, uv, rect.zw)) / side;
    if blended {
        return textureSampleLevel(material_images, material_sampler, coordinate, i32(page * select(2u, 3u, (info.w & 8u) != 0u) + select(0u, 1u, semi)), 0.0);
    }
    var color = vec4(0.0);
    if (info.w & 2u) != 0u { color += textureSampleLevel(material_images, material_sampler, coordinate, i32(page * select(2u, 3u, (info.w & 8u) != 0u)), 0.0); }
    if (info.w & 4u) != 0u { color += textureSampleLevel(material_images, material_sampler, coordinate, i32(page * select(2u, 3u, (info.w & 8u) != 0u) + 1u), 0.0); }
    return color;
}

fn cubic_weights(t: f32) -> vec4<f32> {
    let t2 = t * t;
    let t3 = t2 * t;
    return vec4(-0.5 * t + t2 - 0.5 * t3, 1.0 - 2.5 * t2 + 1.5 * t3,
                0.5 * t + 2.0 * t2 - 1.5 * t3, -0.5 * t2 + 0.5 * t3);
}

fn material_closeup(index: u32, uv: vec2<f32>, footprint: vec2<f32>, mode: f32, semi: bool, blended: bool) -> vec4<f32> {
    let size = material_rect(index, 0u).zw;
    let pixel = uv * size - vec2(0.5);
    let base = floor(pixel);
    let fraction = fract(pixel);
    if mode == 1.0 {
        let transition = clamp((fraction - vec2(0.5)) / max(footprint, vec2(0.0001)) + vec2(0.5), vec2(0.0), vec2(1.0));
        return material_level(index, (base + transition + vec2(0.5)) / size, 0u, semi, blended);
    }
    if mode == 3.0 {
        return material_level(index, (floor(uv * size) + vec2(0.5)) / size, 0u, semi, blended);
    }
    let wx = cubic_weights(fraction.x);
    let wy = cubic_weights(fraction.y);
    var color = vec4(0.0);
    var low = vec4(1.0);
    var high = vec4(0.0);
    var low_rgb = vec3(1.0);
    var high_rgb = vec3(0.0);
    for (var y = 0u; y < 4u; y++) {
        for (var x = 0u; x < 4u; x++) {
            let sample = material_level(index, (base + vec2(f32(x), f32(y)) - vec2(0.5)) / size, 0u, semi, blended);
            color += sample * wx[x] * wy[y];
            if (x == 1u || x == 2u) && (y == 1u || y == 2u) {
                low = min(low, sample);
                high = max(high, sample);
                if sample.a > 0.00001 {
                    let rgb = sample.rgb / sample.a;
                    low_rgb = min(low_rgb, rgb);
                    high_rgb = max(high_rgb, rgb);
                }
            }
        }
    }
    // Bound straight color as well as coverage: independent RGBA bounds can brighten cutout edges.
    color = clamp(color, low, high);
    return vec4(clamp(color.rgb, low_rgb * color.a, high_rgb * color.a), color.a);
}

fn material_color(index: u32, coordinate: vec2<f32>, dx: vec2<f32>, dy: vec2<f32>, anisotropy: f32, filtering: f32, magnification: f32, semi: bool, blended: bool, fallback: vec3<f32>) -> vec3<f32> {
    if index == 0u || index >= material_count() { return fallback; }
    let source = material_source(index);
    let info = material_info(index);
    let uv = (coordinate - source.xy + vec2(0.5)) / source.zw;
    let gx = dx / source.zw;
    let gy = dy / source.zw;
    let size = vec2<f32>(info.yz);
    let tx = gx * size;
    let ty = gy * size;
    let a = tx.x * tx.x + ty.x * ty.x;
    let b = tx.x * tx.y + ty.x * ty.y;
    let c = tx.y * tx.y + ty.y * ty.y;
    let spread = sqrt(max(0.0, (a - c) * (a - c) + 4.0 * b * b));
    let eigenvalue = max(0.0, (a + c + spread) * 0.5);
    let major = sqrt(eigenvalue);
    if magnification > 0.0 && major < 1.0 {
        let closeup = material_closeup(index, uv, min(abs(tx) + abs(ty), vec2(1.0)), magnification, semi, blended);
        let color = mix(closeup, material_level(index, uv, 0u, semi, blended), smoothstep(0.75, 1.0, major));
        return select(fallback, color.rgb / max(color.a, 0.00001), color.a > 0.00001);
    }
    if filtering < 2.0 {
        let color = material_level(index, uv, 0u, semi, blended);
        return select(fallback, color.rgb / max(color.a, 0.00001), color.a > 0.00001);
    }
    let minor = sqrt(max(1.0, (a + c - spread) * 0.5));
    var direction = vec2(1.0, 0.0);
    let candidate = select(vec2(b, eigenvalue - a), vec2(eigenvalue - c, b), a >= c);
    if dot(candidate, candidate) > 0.000001 { direction = normalize(candidate); }
    let axis = direction * major / size;
    let count = u32(clamp(ceil(major / minor), 1.0, anisotropy));
    let level = clamp(log2(max(major / f32(count), minor)), 0.0, f32(info.x - 1u));
    let low = u32(floor(level));
    let high = min(low + 1u, info.x - 1u);
    var sum = vec4(0.0);
    for (var i = 0u; i < count; i++) {
        let point = uv + axis * ((f32(i) + 0.5) / f32(count) - 0.5);
        sum += mix(material_level(index, point, low, semi, blended),
                   material_level(index, point, high, semi, blended), fract(level));
    }
    return select(fallback, sum.rgb / max(sum.a, 0.00001), sum.a > 0.00001);
}
