fn depth_at(pixel: vec2<i32>) -> f32 {
    let point = clamp(pixel, vec2(0), vec2<i32>(textureDimensions(depth_map)) - vec2(1));
    var depth = 1.0;
    for (var i = 0; i < SAMPLE_COUNT; i++) { depth = min(depth, DEPTH_SAMPLE); }
    return depth;
}
