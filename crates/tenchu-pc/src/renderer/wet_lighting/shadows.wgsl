@group(3) @binding(3) var coarse_shadow_a: texture_2d<f32>;
@group(3) @binding(4) var coarse_shadow_b: texture_2d<f32>;

struct ScreenShadows {
    @location(0) a: vec4<f32>,
    @location(1) b: vec4<f32>,
}

struct InterpolatedShadows {
    a: vec4<f32>,
    b: vec4<f32>,
    valid: bool,
}

fn surface_position(pixel: vec2<f32>, depth: f32) -> vec3<f32> {
    let ndc = pixel / vec2<f32>(textureDimensions(surface_diffuse)) * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    return camera.rays[3].xyz + (camera.rays[0].xyz + camera.rays[1].xyz * ndc.x + camera.rays[2].xyz * ndc.y) * depth;
}

@fragment fn prepare_screen_shadows(@builtin(position) pixel: vec4<f32>) -> ScreenShadows {
    var result = ScreenShadows(vec4(1.0), vec4(1.0));
    let point = vec2<i32>(pixel.xy) * SCREEN_SHADOW_STEP;
    let data = textureLoad(surface_geometry, point, 0);
    if data.w == 0u || data.w == FORWARD_SURFACE { return result; }
    let position = surface_position(vec2<f32>(point) + vec2(0.5), bitcast<f32>(data.w));
    let surface = oct_unpack(data.z);
    var nearby = local_lights(position);
    loop {
        let index = next_light(&nearby);
        if index == 0u { break; }
        let light = lighting.lights[index - 1u];
        if light.color.w == 0.0 || distance(position, light.position.xyz) >= light.position.w { continue; }
        let slot = u32(floor(light.color.w) - 1.0);
        let visibility = point_visibility(light, position, surface);
        if slot < 4u { result.a[slot] = visibility; }
        else if slot < 8u { result.b[slot - 4u] = visibility; }
    }
    return result;
}

fn interpolate_shadows(pixel: vec2<f32>, position: vec3<f32>, surface: vec3<f32>) -> InterpolatedShadows {
    var result = InterpolatedShadows(vec4(0.0), vec4(0.0), false);
    if !SCREEN_SHADOWS { return result; }
    let coordinate = (pixel - vec2(0.5)) / f32(SCREEN_SHADOW_STEP);
    let low = vec2<i32>(floor(coordinate));
    let fraction = fract(coordinate);
    let limit = vec2<i32>(textureDimensions(coarse_shadow_a)) - vec2(1);
    var total = 0.0;
    for (var y = 0; y < 2; y++) {
        for (var x = 0; x < 2; x++) {
            let point = clamp(low + vec2(x, y), vec2(0), limit);
            let weights = mix(vec2(1.0) - fraction, fraction, vec2(f32(x), f32(y)));
            let weight = weights.x * weights.y;
            if weight == 0.0 { continue; }
            let data = textureLoad(surface_geometry, point * SCREEN_SHADOW_STEP, 0);
            if data.w == 0u || data.w == FORWARD_SURFACE { continue; }
            // Reject other receiver planes instead of spreading shadows across their edges.
            if dot(oct_unpack(data.z), surface) < 0.9995 { continue; }
            let depth = bitcast<f32>(data.w);
            let neighbor = surface_position(vec2<f32>(point * SCREEN_SHADOW_STEP) + vec2(0.5), depth);
            if abs(dot(neighbor - position, surface)) > max(0.25, depth * 0.00002) { continue; }
            result.a += textureLoad(coarse_shadow_a, point, 0) * weight;
            result.b += textureLoad(coarse_shadow_b, point, 0) * weight;
            total += weight;
        }
    }
    if total > 0.1 {
        result.a /= total;
        result.b /= total;
        result.valid = true;
    }
    return result;
}

fn interpolated_visibility(values: InterpolatedShadows, light: Light, position: vec3<f32>, surface: vec3<f32>) -> f32 {
    if !SCREEN_SHADOWS || !values.valid { return point_visibility(light, position, surface); }
    if light.color.w == 0.0 { return 1.0; }
    let slot = u32(floor(light.color.w) - 1.0);
    if slot < 4u { return values.a[slot]; }
    if slot < 8u { return values.b[slot - 4u]; }
    return point_visibility(light, position, surface);
}
