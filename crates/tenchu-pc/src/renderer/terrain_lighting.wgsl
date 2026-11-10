struct TerrainSurface {
    diffuse: vec3<f32>,
    emission: vec3<f32>,
    normal: vec3<f32>,
    wet_normal: vec3<f32>,
    surface: vec3<f32>,
    power: f32,
    specular: f32,
}

struct InterpolatedShadows {
    a: vec4<f32>,
    b: vec4<f32>,
    valid: bool,
}

fn prepare_terrain(v: Fragment, material: SurfaceMaterial) -> TerrainSurface {
    let color = material.color;
    let emission = material.emission;
    let normal = material.normal;
    let surface_normal = material.surface_normal;
    var diffuse = color * material.cavity * (1.0 - emission.a);
    var unlit_color = emission.rgb * emission.a;
    var wet_normal = normal;
    var power = 1.0;
    var specular = 0.0;
    var shelter = -1.0;
    if WATER_ENABLED && (v.flags & WATER) != 0u {
        let water = prepare_water(normalize(camera.eye.xyz - v.world), normal);
        let reflection = water_reflection(v);
        diffuse = diffuse * water.transmission + vec3(water_foam(v));
        unlit_color = unlit_color * water.transmission + water.sky * reflection;
        power = water.power;
        specular = water.specular * reflection;
    }
    if WEATHER_ENABLED && (v.flags & (SCENERY | TRANSLUCENT | WATER)) == SCENERY {
        if lighting.weather.x + lighting.weather.y + lighting.weather.w > 0.0 {
            let incidence = weather_incidence(surface_normal, true);
            if incidence > 0.0 { shelter = weather_exposure(surface_normal, v.world, true, false); }
            if shelter > 0.0 {
                if lighting.weather.x + lighting.weather.w > 0.0 {
                    let wet_surface = prepare_wet_surface(normal, surface_normal, v.world, material.footprint, shelter, incidence, true);
                    let puddle = wet_surface.puddle;
                    let wet = wet_surface.wet;
                    let dark = 1.0 - wet * 0.32 - puddle * 0.08;
                    diffuse *= dark;
                    unlit_color *= dark;
                    if wet > 0.0 && lighting.rain.x > 0.0 {
                        wet_normal = wet_surface.normal;
                        let view = normalize(camera.eye.xyz - v.world);
                        let fresnel = 0.025 + 0.975 * pow(1.0 - max(0.0, dot(wet_normal, view)), 5.0);
                        let sky = rain_color() * mix(0.02 + 0.22 * fresnel, 0.06 + 0.7 * fresnel, puddle)
                            * (0.3 + 0.7 * max(0.0, -surface_normal.y));
                        unlit_color += sky * lighting.rain.x * wet;
                        power = mix(48.0, 180.0, puddle);
                        specular = (0.22 + fresnel * 1.8) * lighting.rain.x * wet;
                    }
                }
                if lighting.weather.y > 0.0 {
                    let snow = snow_coverage(surface_normal, shelter, snow_variation(v.world), true);
                    let grain_fade = smoothstep(8.0, 30.0, material.footprint);
                    var grain = 0.5;
                    if grain_fade < 1.0 { grain = mix(weather_noise(v.world / 18.0), 0.5, grain_fade); }
                    let white = pow(vec3(0.68, 0.74, 0.8) * (0.9 + grain * 0.1), vec3(2.2));
                    diffuse = mix(diffuse, white, snow);
                    unlit_color *= 1.0 - snow;
                    specular *= 1.0 - snow;
                }
            }
        }
    }
    if WEATHER_ENABLED && lighting.source.z != 0.0 {
        if shelter < 0.0 { shelter = weather_exposure(surface_normal, v.world, true, false); }
        unlit_color += diffuse * storm_illumination(normal, shelter);
    }
    return TerrainSurface(diffuse, unlit_color, normal, wet_normal, surface_normal, power, specular);
}

fn light_terrain(data: TerrainSurface, position: vec3<f32>, filtered: InterpolatedShadows) -> vec3<f32> {
    let normal = data.normal;
    let wet_normal = data.wet_normal;
    let surface = data.surface;
    let power = data.power;
    let specular = abs(data.specular);
    var light = vec3(1.0);
    var reflected = vec3(0.0);
    if LIGHTING_ENABLED && lighting.options.w != 0.0 {
        light = lighting.options.x * mix(vec3(1.0), vec3(0.6, 0.75, 1.0), lighting.lamps.z);
        let sun_diffuse = max(0.0, dot(normal, lighting.sun_direction.xyz));
        var sun = 1.0;
        let view = normalize(camera.eye.xyz - position);
        if lighting.sun_direction.w > 0.0 {
            if sun_diffuse > 0.0 || lighting.source.y != 0.0 || specular > 0.0 { sun = sun_visibility(position, surface); }
            if sun_diffuse > 0.0 { light += lighting.sun_color.rgb * lighting.sun_direction.w * sun_diffuse * sun; }
            if specular > 0.0 {
                let halfway = normalize(view + lighting.sun_direction.xyz);
                reflected += lighting.sun_color.rgb * lighting.sun_direction.w * specular
                    * pow(max(0.0, dot(wet_normal, halfway)), power) * sun;
            }
        }
        let local = daylight_exposure(sun);
        var nearby = local_lights(position);
        loop {
            let index = next_light(&nearby);
            if index == 0u { break; }
            let source = lighting.lights[index - 1u];
            let delta = position - source.position.xyz;
            let distance = length(delta);
            if distance >= source.position.w || distance < 1.0 { continue; }
            let diffuse = max(0.0, dot(normal, -delta / distance));
            var glint = 0.0;
            if data.specular > 0.0 && dot(wet_normal, -delta) > 0.0 {
                let halfway = normalize(view + -delta / distance);
                glint = pow(max(0.0, dot(wet_normal, halfway)), power);
            }
            if diffuse == 0.0 && glint < 0.001 { continue; }
            let visibility = interpolated_visibility(filtered, source, position, surface);
            let falloff = 1.0 - smoothstep(0.0, 1.0, distance / source.position.w);
            if diffuse > 0.0 { light += source.color.rgb * lighting.options.y * falloff * falloff * diffuse * visibility * local; }
            if glint >= 0.001 { reflected += source.color.rgb * lighting.options.y * falloff * falloff * glint * specular * visibility * local; }
        }
    }
    return data.diffuse * light + data.emission + reflected;
}

fn interpolated_visibility(values: InterpolatedShadows, light: Light, position: vec3<f32>, surface: vec3<f32>) -> f32 {
    if !values.valid { return point_visibility(light, position, surface); }
    if light.color.w == 0.0 { return 1.0; }
    let slot = u32(floor(light.color.w) - 1.0);
    if slot < 4u { return values.a[slot]; }
    if slot < 8u { return values.b[slot - 4u]; }
    return point_visibility(light, position, surface);
}

fn forward_weather(v: Fragment, surface: SurfaceMaterial) -> vec4<f32> {
    let material = prepare_terrain(v, surface);
    var shaded = light_terrain(material, v.world, InterpolatedShadows(vec4(0.0), vec4(0.0), false));
    if (v.flags & FOG) != 0u {
        shaded = fog_color(shaded, v.depth, baked_fog(v.flags), true);
    }
    return vec4(max(rain_atmosphere(shaded, v.world, true, true, true) * camera.options.x, vec3(0.0)), 1.0);
}

@fragment fn scenery_weather(input: Fragment, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    var v = input;
    v.flags = (v.flags & (TEXTURED | TRANSLUCENT | DOUBLE_SIDED | FOG | BAKED_FOG | EMITTER | FOLIAGE)) | SCENERY;
    return forward_weather(v, prepare_material(v, front_facing, false, false));
}
