@group(0) @binding(0) var current: texture_2d<f32>;
@group(0) @binding(1) var previous: texture_2d<f32>;
@group(0) @binding(2) var source_sampler: sampler;
@group(0) @binding(3) var<uniform> weights: vec4<f32>;

struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
};

@vertex fn vertex(@builtin(vertex_index) index: u32) -> Vertex {
    let uv = vec2<f32>(f32((index << 1u) & 2u), f32(index & 2u));
    return Vertex(vec4<f32>(uv * vec2<f32>(2.0, -2.0) + vec2<f32>(-1.0, 1.0), 0.0, 1.0), uv);
}

@fragment fn combine(input: Vertex) -> @location(0) vec4<f32> {
    let a = textureSample(current, source_sampler, input.uv).rgb;
    let b = textureSample(previous, source_sampler, input.uv).rgb;
    return vec4<f32>(clamp(a * weights.x + b * weights.y, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}

@fragment fn present(input: Vertex) -> @location(0) vec4<f32> {
    return textureSample(current, source_sampler, input.uv);
}
