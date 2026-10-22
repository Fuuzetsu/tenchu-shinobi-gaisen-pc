struct Light { position: vec4<f32>, color: vec4<f32> }
struct Lighting {
    lights: array<Light, MAX_LIGHTS>,
    matrices: array<mat4x4<f32>, SHADOW_FACES>,
    options: vec4<f32>,
    lamps: vec4<f32>,
    sun_inverse: mat4x4<f32>,
    sun_direction: vec4<f32>,
    sun_color: vec4<f32>,
}
@group(1) @binding(0) var<uniform> lighting: Lighting;
@group(1) @binding(1) var shadow_maps: texture_depth_2d_array;
@group(1) @binding(2) var shadow_sampler: sampler_comparison;
@group(1) @binding(3) var sun_map: texture_depth_2d;

fn shadow_face(direction: vec3<f32>) -> u32 {
    let a = abs(direction);
    if a.x >= a.y && a.x >= a.z { return select(1u, 0u, direction.x >= 0.0); }
    if a.y >= a.z { return select(3u, 2u, direction.y >= 0.0); }
    return select(5u, 4u, direction.z >= 0.0);
}

fn illumination(position: vec3<f32>, normal: vec3<f32>, surface_normal: vec3<f32>) -> vec3<f32> {
    if !LIGHTING_ENABLED || lighting.options.w == 0.0 { return vec3(1.0); }
    var result = lighting.options.x * mix(vec3(1.0), vec3(0.6, 0.75, 1.0), lighting.lamps.z);
    let sun_diffuse = max(0.0, dot(normal, lighting.sun_direction.xyz));
    if lighting.sun_direction.w > 0.0 && sun_diffuse > 0.0 {
        result += lighting.sun_color.rgb * lighting.sun_direction.w
            * sun_diffuse * sun_visibility(position, surface_normal);
    }
    for (var i = 0u; i < u32(lighting.options.z); i++) {
        let light = lighting.lights[i];
        let delta = position - light.position.xyz;
        let distance = length(delta);
        if distance >= light.position.w || distance < 1.0 { continue; }
        let diffuse = max(0.0, dot(normal, -delta / distance));
        if diffuse == 0.0 { continue; }
        var visibility = 1.0;
        if light.color.w != 0.0 {
            let layer = u32(light.color.w - 1.0) * 6u + shadow_face(delta);
            let matrix = lighting.matrices[layer];
            let receiver = position - delta / distance * 2.0;
            let p = matrix * vec4(receiver, 1.0);
            let uv = p.xy / p.w * vec2(0.5, -0.5) + vec2(0.5);
            let right = vec3(matrix[0].x, matrix[1].x, matrix[2].x);
            let up = vec3(matrix[0].y, matrix[1].y, matrix[2].y);
            let plane = dot(surface_normal, receiver - light.position.xyz);
            var gradient = vec2(0.0);
            if abs(plane) > 0.001 {
                // Receiver-plane depth gradients for the 90-degree point-light projection.
                let projection = SHADOW_NEAR * light.position.w / (light.position.w - SHADOW_NEAR);
                gradient = 2.0 * projection / plane
                    * vec2(-dot(surface_normal, right), dot(surface_normal, up));
            }
            visibility = shadow_visibility(uv, layer, p.z / p.w, gradient);
        }
        let falloff = 1.0 - smoothstep(0.0, 1.0, distance / light.position.w);
        result += light.color.rgb * lighting.options.y * falloff * falloff
            * diffuse * visibility;
    }
    return result;
}

fn sun_visibility(position: vec3<f32>, normal: vec3<f32>) -> f32 {
    if lighting.sun_color.w == 0.0 { return 1.0; }
    let receiver = position + lighting.sun_direction.xyz * 2.0;
    let p = lighting.matrices[SHADOW_FACES - 1u] * vec4(receiver, 1.0);
    let uv = p.xy * vec2(0.5, -0.5) + vec2(0.5);
    if any(uv < vec2(0.0)) || any(uv > vec2(1.0)) || p.z < 0.0 || p.z > 1.0 { return 1.0; }
    let plane = transpose(lighting.sun_inverse) * vec4(normal, -dot(normal, receiver));
    var gradient = vec2(0.0);
    if abs(plane.z) > 0.00001 { gradient = vec2(-2.0, 2.0) * plane.xy / plane.z; }
    let size = vec2<f32>(textureDimensions(sun_map));
    let reference = p.z - dot(abs(gradient), vec2(0.5) / size);
    var visibility = 0.0;
    if lighting.lamps.y == 0.0 {
        visibility = textureSampleCompareLevel(sun_map, shadow_sampler, uv, reference);
    } else {
        for (var y = -1; y <= 1; y++) {
            for (var x = -1; x <= 1; x++) {
                let offset = vec2(f32(x), f32(y)) * lighting.lamps.y / size;
                visibility += textureSampleCompareLevel(sun_map, shadow_sampler, uv + offset, reference + dot(gradient, offset)) / 9.0;
            }
        }
    }
    let edge = max(abs(uv.x - 0.5), abs(uv.y - 0.5)) * 2.0;
    return mix(visibility, 1.0, smoothstep(0.85, 1.0, edge));
}

fn shadow_visibility(uv: vec2<f32>, layer: u32, depth: f32, gradient: vec2<f32>) -> f32 {
    let size = vec2<f32>(textureDimensions(shadow_maps));
    let reference = depth - dot(abs(gradient), vec2(0.5) / size);
    if lighting.lamps.y == 0.0 {
        return textureSampleCompareLevel(shadow_maps, shadow_sampler, uv, i32(layer), reference);
    }
    let step = lighting.lamps.y / size;
    var visibility = 0.0;
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let point = clamp(uv + vec2(f32(x), f32(y)) * step, vec2(0.5) / size, vec2(1.0) - vec2(0.5) / size);
            visibility += textureSampleCompareLevel(shadow_maps, shadow_sampler,
                point, i32(layer), reference + dot(gradient, point - uv));
        }
    }
    return visibility / 9.0;
}
