struct Detail { coordinate: vec2<f32>, normal: vec3<f32>, cavity: f32 }

fn height_level(index: u32, uv: vec2<f32>, level: u32) -> vec3<f32> {
    let rect = materials[index].mips[level];
    let side = f32(textureDimensions(material_images).x);
    let page = u32(rect.x) / u32(side);
    let origin = vec2(rect.x - f32(page) * side, rect.y);
    let coordinate = (origin + clamp(uv * rect.zw, vec2(0.5), rect.zw - vec2(0.5))) / side;
    return textureSampleLevel(material_images, material_sampler, coordinate, i32(page * 3u + 2u), 0.0).rgb;
}

fn height_sample(index: u32, uv: vec2<f32>, lod: f32) -> vec3<f32> {
    let low = u32(lod);
    return mix(height_level(index, uv, low), height_level(index, uv, min(low + 1u, materials[index].info.x - 1u)), fract(lod));
}

fn surface_detail(v: Fragment, normal: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>, du: vec2<f32>, dv: vec2<f32>) -> Detail {
    var result = Detail(v.uv, normal, 1.0);
    if camera.detail.x == 0.0 || camera.detail.y == 0.0 || camera.options.y == 0.0
        || (v.flags & (SCREEN | DEBUG | EMITTER | TRANSLUCENT | TEXTURED)) != TEXTURED
        || v.material == 0u || v.material >= arrayLength(&materials) { return result; }
    let index = v.material;
    let source = materials[index].source;
    let info = materials[index].info;
    let relief = materials[index].relief;
    if (info.w & 8u) == 0u || relief.x == 0.0
        || ((v.flags & SCENERY) == 0u && relief.y == 0.0) { return result; }
    let gx = du / source.zw;
    let gy = dv / source.zw;
    let determinant = gx.x * gy.y - gx.y * gy.x;
    if abs(determinant) < 0.0000000001 { return result; }
    let tangent = (dx * gy.y - dy * gx.y) / determinant;
    let bitangent = (dy * gx.x - dx * gy.x) / determinant;
    let plane = dot(tangent, cross(bitangent, normal));
    if abs(plane) < 0.000001 { return result; }
    let u_gradient = cross(bitangent, normal) / plane;
    let v_gradient = cross(normal, tangent) / plane;
    let size = vec2<f32>(info.yz);
    let footprint = max(length(gx * size), length(gy * size));
    let fade = 1.0 - smoothstep(1.0, 8.0, footprint);
    if fade == 0.0 { return result; }
    let lod = clamp(log2(max(footprint, 1.0)), 0.0, f32(materials[index].info.x - 1u));
    var uv = (v.uv - source.xy + vec2(0.5)) / source.zw;
    let edge = min(min(uv.x, uv.y), min(1.0 - uv.x, 1.0 - uv.y));
    let strength = camera.detail.y * relief.x * fade * smoothstep(0.0, 0.06, edge);
    let height = min(length(tangent), length(bitangent)) * 0.06 * strength;
    let eye = normalize(camera.eye.xyz - v.world);
    let facing = dot(eye, normal);
    if camera.detail.x == 2.0 && facing > 0.1 && footprint < 2.0 {
        // Bounded ray/height intersection. Fade grazing rays and region edges to avoid atlas seams.
        let ray = clamp(vec2(dot(eye, u_gradient), dot(eye, v_gradient)) * height
            / max(facing, 0.25), vec2(-0.06), vec2(0.06)) * smoothstep(0.1, 0.35, facing) * (1.0 - smoothstep(0.75, 2.0, footprint));
        let start = uv + ray * 0.5;
        var previous = start;
        var previous_gap = 1.0 - height_level(index, start, u32(lod)).b;
        for (var step = 1u; step <= 8u; step++) {
            let depth = f32(step) / 8.0;
            let point = start - ray * depth;
            let gap = 1.0 - depth - height_level(index, point, u32(lod)).b;
            if gap <= 0.0 {
                uv = mix(previous, point, previous_gap / max(previous_gap - gap, 0.00001));
                break;
            }
            previous = point;
            previous_gap = gap;
        }
    }
    let detail = height_sample(index, uv, lod);
    let gradient = (detail.rg * 255.0 - vec2(128.0)) / 127.0 * RELIEF_GRADIENT_RANGE;
    result.normal = normalize(normal - height * (u_gradient * gradient.x + v_gradient * gradient.y));
    result.coordinate = uv * source.zw + source.xy - vec2(0.5);
    result.cavity = 1.0 - max(0.0, 0.5 - detail.b) * 0.2 * strength;
    return result;
}
