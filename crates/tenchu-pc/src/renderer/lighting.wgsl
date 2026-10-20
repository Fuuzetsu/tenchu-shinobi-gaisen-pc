struct Light { position: vec4<f32>, color: vec4<f32> }
struct Lighting {
    lights: array<Light, MAX_LIGHTS>,
    matrices: array<mat4x4<f32>, SHADOW_FACES>,
    options: vec4<f32>,
    lamps: vec4<f32>,
}
@group(1) @binding(0) var<uniform> lighting: Lighting;
@group(1) @binding(1) var shadow_maps: texture_depth_2d_array;
@group(1) @binding(2) var shadow_sampler: sampler_comparison;

fn shadow_face(direction: vec3<f32>) -> u32 {
    let a = abs(direction);
    if a.x >= a.y && a.x >= a.z { return select(1u, 0u, direction.x >= 0.0); }
    if a.y >= a.z { return select(3u, 2u, direction.y >= 0.0); }
    return select(5u, 4u, direction.z >= 0.0);
}

fn illumination(position: vec3<f32>, normal: vec3<f32>, surface_normal: vec3<f32>) -> vec3<f32> {
    if lighting.options.w == 0.0 { return vec3(1.0); }
    var result = vec3(lighting.options.x);
    for (var i = 0u; i < u32(lighting.options.z); i++) {
        let light = lighting.lights[i];
        let delta = position - light.position.xyz;
        let distance = length(delta);
        if distance >= light.position.w || distance < 1.0 { continue; }
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
        let falloff = max(0.0, 1.0 - distance / light.position.w);
        result += light.color.rgb * lighting.options.y * falloff * falloff
            * abs(dot(normal, delta / distance)) * visibility;
    }
    return result;
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
