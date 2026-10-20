struct Light { position: vec4<f32>, color: vec4<f32> }
struct Lighting {
    lights: array<Light, 2>,
    matrices: array<mat4x4<f32>, 12>,
    options: vec4<f32>,
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

fn illumination(position: vec3<f32>, normal: vec3<f32>) -> vec3<f32> {
    if lighting.options.w == 0.0 { return vec3(1.0); }
    var result = vec3(lighting.options.x);
    for (var i = 0u; i < u32(lighting.options.z); i++) {
        let light = lighting.lights[i];
        let delta = position - light.position.xyz;
        let distance = length(delta);
        if distance >= light.position.w || distance < 1.0 { continue; }
        var visibility = 1.0;
        if lighting.options.w == 2.0 {
            let layer = i * 6u + shadow_face(delta);
            let p = lighting.matrices[layer] * vec4(position, 1.0);
            let uv = p.xy / p.w * vec2(0.5, -0.5) + vec2(0.5);
            visibility = textureSampleCompareLevel(shadow_maps, shadow_sampler, uv, i32(layer), p.z / p.w);
        }
        let falloff = max(0.0, 1.0 - distance / light.position.w);
        result += light.color.rgb * lighting.options.y * falloff * falloff
            * abs(dot(normal, delta / distance)) * visibility;
    }
    return result;
}
