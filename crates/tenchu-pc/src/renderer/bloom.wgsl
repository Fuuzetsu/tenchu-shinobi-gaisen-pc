@group(0) @binding(0) var source: texture_2d<f32>;
@group(0) @binding(1) var linear_sampler: sampler;
@group(0) @binding(2) var<uniform> options: vec4<f32>;

struct Fragment { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
@vertex fn vertex(@builtin(vertex_index) index: u32) -> Fragment {
    let uv = vec2(f32((index << 1u) & 2u), f32(index & 2u));
    return Fragment(vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0), uv);
}

fn blur(uv: vec2<f32>) -> vec3<f32> {
    let texel = 1.0 / vec2<f32>(textureDimensions(source));
    var sum = vec3(0.0);
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let weight = f32((2 - abs(x)) * (2 - abs(y))) / 16.0;
            sum += textureSampleLevel(source, linear_sampler, uv + vec2(f32(x), f32(y)) * texel, 0.0).rgb * weight;
        }
    }
    return sum;
}

@fragment fn extract(v: Fragment) -> @location(0) vec4<f32> {
    let color = blur(v.uv);
    let brightness = max(color.r, max(color.g, color.b));
    let knee = options.x * 0.5;
    let soft = clamp(brightness - options.x + knee, 0.0, 2.0 * knee);
    let contribution = max(brightness - options.x, soft * soft / (4.0 * knee + 0.0001));
    return vec4(color * contribution / max(brightness, 0.0001), 1.0);
}

@fragment fn downsample(v: Fragment) -> @location(0) vec4<f32> {
    return vec4(blur(v.uv), 1.0);
}
