fn water_normal(v: Fragment, normal: vec3<f32>, dx: vec3<f32>, dy: vec3<f32>) -> vec3<f32> {
    if !WATER_ENABLED || (v.flags & WATER) == 0u { return normal; }
    let time = camera.eye.w * camera.water.z;
    let footprint = max(length(dx), length(dy));
    let fade = 1.0 - smoothstep(35.0, 160.0, footprint);
    var gradient = vec3(0.0);
    if (v.flags & WATERFALL) != 0u {
        let down = normalize(vec3(0.0, 1.0, 0.0) - normal * normal.y + vec3(0.00001));
        let across = normalize(cross(normal, down));
        let filaments = cos(dot(v.world, across) * 0.025 + v.world.y * 0.002 - time * 6.0);
        let pulses = sin(v.world.y * 0.014 - time * 14.0 + dot(v.world, across) * 0.008);
        gradient = across * filaments * 0.13 + down * pulses * 0.035;
    } else {
        let warp = sin(dot(v.world.xz, vec2(0.0017, 0.0023)) + time * 0.35) * 1.3
            + sin(dot(v.world.xz, vec2(-0.0031, 0.0011)) - time * 0.27) * 0.7;
        let a = dot(v.world.xz, vec2(0.007, 0.011)) + time * 2.1 + warp;
        let b = dot(v.world.xz, vec2(0.021, -0.014)) + time * 3.3 - warp * 0.6;
        let c = dot(v.world.xz, vec2(0.013, 0.027)) - time * 2.7 + sin(a * 0.43) * 0.8;
        gradient = vec3(cos(a) * 0.1 + cos(b) * 0.06 + cos(c) * 0.03,
            0.0, cos(a) * 0.15 - cos(b) * 0.04 + cos(c) * 0.05);
    }
    return normalize(normal + gradient * camera.water.x * fade);
}

struct WaterSurface {
    fresnel: f32,
    sky: vec3<f32>,
    transmission: f32,
    specular: f32,
    power: f32,
}

fn prepare_water(view: vec3<f32>, normal: vec3<f32>) -> WaterSurface {
    let fresnel = 0.025 + 0.975 * pow(1.0 - max(0.0, dot(normal, view)), 5.0);
    var sky = mix(vec3(0.02, 0.035, 0.055), vec3(0.16, 0.23, 0.3), camera.detail.w);
    if camera.motion.w != 0.0 {
        let ray = reflect(-view, normal);
        let height = clamp(-ray.y, 0.0, 1.0);
        let top = mix(vec3(0.002, 0.004, 0.012), vec3(0.018, 0.08, 0.23), camera.detail.w);
        let horizon = mix(sky, camera.fog_color.rgb, camera.detail.w * 0.5);
        sky = mix(horizon, top, sqrt(height));
        if WEATHER_ENABLED {
            sky = mix(sky, rain_color(), lighting.rain.y * 0.6);
            sky += vec3(0.18, 0.22, 0.3) * lighting.source.z;
        }
        return WaterSurface(fresnel, sky * fresnel, 1.0 - fresnel * camera.water.y, 0.3 + fresnel * 1.4, 80.0);
    }
    return WaterSurface(fresnel, sky * fresnel, 1.0 - fresnel * camera.water.y * 0.18, -0.8, 110.0);
}

fn water_foam(v: Fragment) -> f32 {
    if (v.flags & WATERFALL) == 0u { return 0.0; }
    let time = camera.eye.w * camera.water.z;
    let phase = dot(v.world.xz, vec2(0.018, 0.014)) + v.world.y * 0.004 - time * 5.0;
    return pow(0.5 + 0.5 * sin(phase), 8.0) * camera.water.w * 0.1;
}

fn water_reflection(v: Fragment) -> f32 {
    return camera.water.y * select(1.0, 0.2, (v.flags & WATERFALL) != 0u);
}

fn water_color(v: Fragment, input: vec3<f32>, normal: vec3<f32>, surface: vec3<f32>, light: vec3<f32>, linear: bool) -> vec3<f32> {
    if !WATER_ENABLED || (v.flags & WATER) == 0u { return input; }
    let view = normalize(camera.eye.xyz - v.world);
    let water = prepare_water(view, normal);
    var reflected = water.sky;
    if lighting.sun_direction.w > 0.0 && lighting.options.w != 0.0 {
        let halfway = normalize(view + lighting.sun_direction.xyz);
        reflected += lighting.sun_color.rgb * lighting.sun_direction.w * abs(water.specular)
            * pow(max(0.0, dot(normal, halfway)), water.power) * sun_visibility(v.world, surface);
    }
    if water.specular > 0.0 && LIGHTING_ENABLED && lighting.options.w != 0.0 {
        var nearby = local_lights(v.world);
        loop {
            let index = next_light(&nearby);
            if index == 0u { break; }
            let source = lighting.lights[index - 1u];
            let delta = source.position.xyz - v.world;
            let distance = length(delta);
            if distance < 1.0 || distance >= source.position.w || dot(normal, delta) <= 0.0 { continue; }
            let halfway = normalize(view + delta / distance);
            let glint = pow(max(0.0, dot(normal, halfway)), water.power);
            if glint < 0.001 { continue; }
            let falloff = 1.0 - smoothstep(0.0, 1.0, distance / source.position.w);
            reflected += source.color.rgb * lighting.options.y * falloff * falloff * glint * water.specular
                * point_visibility(source, v.world, surface) * local_exposure(v.world, surface);
        }
    }
    let foam = water_foam(v);
    reflected *= water_reflection(v);
    if !linear { reflected = pow(max(reflected, vec3(0.0)), vec3(1.0 / 2.2)) * 0.5; }
    return input * water.transmission + reflected + light * foam;
}
