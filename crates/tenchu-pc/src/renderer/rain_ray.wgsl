const RAIN_VOLUME_FAR: f32 = 250000.0;

fn rain_view_coordinate(depth: f32) -> f32 {
    return log2(1.0 + depth / 1000.0) / log2(1.0 + RAIN_VOLUME_FAR / 1000.0);
}

fn rain_view_depth(coordinate: f32) -> f32 {
    return 1000.0 * (exp2(coordinate * log2(1.0 + RAIN_VOLUME_FAR / 1000.0)) - 1.0);
}

fn rain_view_exposure(start0: vec3<f32>, start1: vec3<f32>, delta0: vec3<f32>, delta1: vec3<f32>, radius: f32, span0: vec2<f32>, span1: vec2<f32>) -> f32 {
    var exposed = 0.0;
    for (var step = 0; step < 6; step++) {
        let spread = vec2(radius, select(-radius, radius, step % 2 == 0));
        let t = (f32(step) + 0.5) / 6.0;
        exposed += rain_air_exposure(start0 + delta0 * t, start1 + delta1 * t, spread, span0, span1);
    }
    return exposed / 6.0;
}

fn rain_air_exposure(p0: vec3<f32>, p1: vec3<f32>, spread: vec2<f32>, span0: vec2<f32>, span1: vec2<f32>) -> f32 {
    var exposure = 0.0;
    var remaining = 1.0;
    for (var layer = 0; layer < 2; layer++) {
        if remaining < 0.001 { break; }
        let p = select(p0, p1, layer == 1);
        let uv = p.xy * vec2(0.5, -0.5) + vec2(0.5);
        if any(uv < vec2(0.0)) || any(uv > vec2(1.0)) { continue; }
        let span = select(span0, span1, layer == 1);
        let offset = spread / span;
        let exposed = (textureSampleCompareLevel(weather_map, shadow_sampler, uv - offset, layer, p.z)
            + textureSampleCompareLevel(weather_map, shadow_sampler, uv + offset, layer, p.z)) * 0.5;
        let weight = select(1.0 - smoothstep(0.75, 0.95, max(abs(p.x), abs(p.y))), remaining, layer == 1);
        exposure += exposed * weight;
        remaining -= weight;
    }
    return exposure;
}

