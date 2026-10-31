@group(0) @binding(0) var depth_map: texture_multisampled_2d<f32>;

@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let uv = vec2(f32((index << 1u) & 2u), f32(index & 2u));
    return vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0);
}

@fragment fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) f32 {
    var depth = 1.0;
    for (var i = 0; i < SAMPLE_COUNT; i++) {
        depth = min(depth, textureLoad(depth_map, vec2<i32>(position.xy), i).r);
    }
    return depth;
}
