fn weather_noise(p: vec3<f32>) -> f32 {
    let cell = floor(p);
    let f = smoothstep(vec3(0.0), vec3(1.0), fract(p));
    var value = 0.0;
    for (var z = 0; z <= 1; z++) {
        for (var y = 0; y <= 1; y++) {
            for (var x = 0; x <= 1; x++) {
                let corner = vec3(f32(x), f32(y), f32(z));
                let weight = mix(vec3(1.0) - f, f, corner);
                var h = fract((cell + corner) * 0.1031);
                h += dot(h, h.yzx + vec3(33.33));
                let hash = fract((h.x + h.y) * h.z);
                value += hash * weight.x * weight.y * weight.z;
            }
        }
    }
    return value;
}

fn weather_direction() -> vec3<f32> {
    return normalize((lighting.weather_inverse[0] * vec4(0.0, 0.0, 1.0, 0.0)).xyz);
}

fn weather_incidence(normal: vec3<f32>) -> f32 {
    let direction = weather_direction();
    // Rainfall amount measures vertical flux, so open level ground keeps the
    // same accumulation while wind changes which slopes and walls get hit.
    return clamp(-dot(normal, direction) / max(direction.y, 0.001), 0.0, 1.0);
}

fn weather_compare(uv: vec2<f32>, depth: f32, gradient: vec2<f32>, layer: i32) -> f32 {
    let size = vec2<f32>(textureDimensions(weather_map));
    let pixel = uv * size - vec2(0.5);
    let blend = fract(pixel);
    var exposed = 0.0;
    for (var y = 0; y < 2; y++) {
        for (var x = 0; x < 2; x++) {
            let corner = vec2(f32(x), f32(y));
            let sample_uv = (clamp(floor(pixel) + corner, vec2(0.0), size - vec2(1.0)) + vec2(0.5)) / size;
            let reference = depth + dot(gradient, sample_uv - uv);
            let weight = mix(vec2(1.0) - blend, blend, corner);
            exposed += weight.x * weight.y
                * textureSampleCompareLevel(weather_map, shadow_sampler, sample_uv, layer, reference);
        }
    }
    return exposed;
}

fn weather_exposure(normal: vec3<f32>, position: vec3<f32>) -> f32 {
    let direction = weather_direction();
    var shelter = 0.0;
    var remaining = 1.0;
    for (var layer = 0; layer < 2; layer++) {
        if remaining < 0.001 { break; }
        let p = lighting.weather_matrix[layer] * vec4(position - direction * 35.0, 1.0);
        let uv = p.xy * vec2(0.5, -0.5) + vec2(0.5);
        if any(uv < vec2(0.0)) || any(uv > vec2(1.0)) { continue; }
        let inverse = lighting.weather_inverse[layer];
        let plane = transpose(inverse) * vec4(normal, -dot(normal, position));
        var gradient = vec2(0.0);
        if abs(plane.z) > 0.00001 { gradient = vec2(-2.0, 2.0) * plane.xy / plane.z; }
        var exposed = weather_compare(uv, p.z, gradient, layer);
        if lighting.weather.y > 0.0 && lighting.rain.w > 0.0 {
            let size = vec2<f32>(textureDimensions(weather_map));
            let pixel = vec2<i32>(clamp(uv * size, vec2(0.0), size - vec2(1.0)));
            let blocker = (inverse * vec4(p.xy, textureLoad(weather_map, pixel, layer, 0), 1.0)).xyz;
            let gap = max(0.0, dot(position - blocker, direction));
            let radius = (65.0 + min(gap * 0.08, 250.0)) * lighting.rain.w;
            let span = vec2(length(inverse[0].xyz), length(inverse[1].xyz)) * 2.0;
            let turn = mat2x2(vec2(0.92388, 0.382683), vec2(-0.382683, 0.92388));
            exposed *= 4.0;
            for (var y = -1; y <= 1; y++) {
                for (var x = -1; x <= 1; x++) {
                    if x == 0 && y == 0 { continue; }
                    let offset = turn * vec2(f32(x), f32(y)) * radius / span;
                    let weight = select(1.0, 2.0, x == 0 || y == 0);
                    exposed += weight * weather_compare(uv + offset, p.z + dot(gradient, offset), gradient, layer);
                }
            }
            exposed /= 16.0;
        }
        let weight = select(1.0 - smoothstep(0.75, 0.95, max(abs(p.x), abs(p.y))), remaining, layer == 1);
        shelter += exposed * weight;
        remaining -= weight;
    }
    return shelter;
}

fn snow_variation(position: vec3<f32>) -> f32 {
    let p = vec3(position.x * 0.8 + position.z * 0.6, position.y, position.z * 0.8 - position.x * 0.6) / 450.0;
    return weather_noise(p) * 0.7 + weather_noise(p * 2.1 + vec3(17.0, 4.0, 29.0)) * 0.3;
}

fn snow_coverage(normal: vec3<f32>, exposure: f32, variation: f32) -> f32 {
    let retention = 0.35 * (1.0 - smoothstep(0.0, 0.3, normal.y))
        + 0.65 * smoothstep(0.3, 0.85, -normal.y);
    let coverage = lighting.weather.y * weather_incidence(normal);
    return retention * exposure * smoothstep(0.04 + variation * 0.2, 0.5 + variation * 0.2, coverage);
}

fn weather_snow(normal: vec3<f32>, position: vec3<f32>) -> f32 {
    return snow_coverage(normal, weather_exposure(normal, position), snow_variation(position));
}

fn rain_color() -> vec3<f32> {
    return mix(vec3(0.025, 0.04, 0.065), vec3(0.22, 0.28, 0.34), camera.detail.w);
}

fn rain_ripples(position: vec3<f32>, footprint: f32) -> vec3<f32> {
    let fade = 1.0 - smoothstep(12.0, 80.0, footprint);
    if fade == 0.0 || lighting.rain.z == 0.0 { return vec3(0.0); }
    let p = position.xz / 380.0;
    let cell = floor(p);
    var gradient = vec2(0.0);
    for (var y = -1; y <= 1; y++) {
        for (var x = -1; x <= 1; x++) {
            let tile = cell + vec2(f32(x), f32(y));
            var h = fract(vec3(tile.x, tile.y, tile.x) * 0.1031);
            h += dot(h, h.yzx + vec3(33.33));
            let seed = fract((h.xx + h.yz) * h.zy);
            let age = fract(lighting.weather.z * 0.85 + seed.x);
            let delta = p - tile - seed;
            let radius = length(delta);
            let ring = radius - age * 0.62;
            let wave = sin(ring * 60.0) * exp(-ring * ring * 110.0);
            gradient += delta / max(radius, 0.001) * wave * (1.0 - age) * smoothstep(0.0, 0.12, age);
        }
    }
    return vec3(gradient.x, 0.0, gradient.y) * fade * lighting.rain.z * 0.075;
}

fn wet_reflection(position: vec3<f32>, normal: vec3<f32>, surface: vec3<f32>, puddle: f32) -> vec3<f32> {
    let view = normalize(camera.eye.xyz - position);
    let fresnel = 0.025 + 0.975 * pow(1.0 - max(0.0, dot(normal, view)), 5.0);
    let sky = rain_color();
    var reflection = sky * mix(0.02 + 0.22 * fresnel, 0.06 + 0.7 * fresnel, puddle)
        * (0.3 + 0.7 * max(0.0, -surface.y));
    let power = mix(48.0, 180.0, puddle);
    let specular = 0.22 + fresnel * 1.8;
    if LIGHTING_ENABLED && lighting.options.w != 0.0 {
        if lighting.sun_direction.w > 0.0 {
            let halfway = normalize(view + lighting.sun_direction.xyz);
            reflection += lighting.sun_color.rgb * lighting.sun_direction.w * specular
                * pow(max(0.0, dot(normal, halfway)), power) * sun_visibility(position, surface);
        }
        for (var i = 0u; i < u32(lighting.options.z); i++) {
            let light = lighting.lights[i];
            let delta = light.position.xyz - position;
            let distance = length(delta);
            if distance < 1.0 || distance >= light.position.w || dot(normal, delta) <= 0.0 { continue; }
            let halfway = normalize(view + delta / distance);
            let glint = pow(max(0.0, dot(normal, halfway)), power);
            if glint < 0.001 { continue; }
            let falloff = 1.0 - smoothstep(0.0, 1.0, distance / light.position.w);
            reflection += light.color.rgb * lighting.options.y * falloff * falloff
                * glint * specular * point_visibility(light, position, surface);
        }
    }
    return reflection * lighting.rain.x;
}

fn weather_surface(input: vec3<f32>, normal: vec3<f32>, surface: vec3<f32>, position: vec3<f32>, footprint: f32, linear: bool) -> vec3<f32> {
    if lighting.weather.x + lighting.weather.y + lighting.weather.w <= 0.0 { return input; }
    let slope = clamp(-surface.y, 0.0, 1.0);
    let shelter = weather_exposure(surface, position);
    let incidence = weather_incidence(surface);
    let variation = weather_noise(position / 450.0);
    let basin = weather_noise(position / 1400.0) * 0.8 + variation * 0.2;
    let level = lighting.weather.w * 0.6;
    let puddle = (1.0 - smoothstep(level - 0.07, level + 0.07, basin)) * smoothstep(0.0, 0.12, lighting.weather.w)
        * smoothstep(0.96, 0.998, slope) * shelter * incidence;
    let wet = max(lighting.weather.x * shelter * incidence * (0.75 + variation * 0.25), puddle);
    var color = input * (1.0 - wet * 0.32 - puddle * 0.08);
    if wet > 0.0 && lighting.rain.x > 0.0 {
        let smooth_normal = normalize(mix(normal, surface, puddle * 0.9)
            + rain_ripples(position, footprint) * puddle);
        var reflection = wet_reflection(position, smooth_normal, surface, puddle) * wet;
        if !linear { reflection = pow(max(reflection, vec3(0.0)), vec3(1.0 / 2.2)) * 0.5; }
        color += reflection;
    }
    if lighting.weather.y == 0.0 { return color; }
    let snow = snow_coverage(surface, shelter, snow_variation(position));
    let grain = mix(weather_noise(position / 18.0), 0.5, smoothstep(8.0, 30.0, footprint));
    var white = vec3(0.68, 0.74, 0.8) * (0.9 + grain * 0.1);
    if linear { white = pow(white, vec3(2.2)); }
    return mix(color, white * illumination(position, normal, surface), snow);
}

fn rain_atmosphere(input: vec3<f32>, position: vec3<f32>, linear: bool) -> vec3<f32> {
    if !WEATHER_ENABLED || lighting.rain.y == 0.0 { return input; }
    let delta = position - camera.eye.xyz;
    let distance = length(delta);
    let shelter = (weather_exposure(vec3(0.0, -1.0, 0.0), camera.eye.xyz + delta * 0.25)
        + weather_exposure(vec3(0.0, -1.0, 0.0), camera.eye.xyz + delta * 0.75)) * 0.5;
    let haze = min(0.94, 1.0 - exp(-distance * lighting.rain.y * shelter / 18000.0));
    var color = rain_color();
    if !linear { color = pow(color, vec3(1.0 / 2.2)); }
    return mix(input, color, haze);
}

