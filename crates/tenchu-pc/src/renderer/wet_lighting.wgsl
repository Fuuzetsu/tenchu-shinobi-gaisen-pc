@group(3) @binding(0) var surface_diffuse: texture_2d<f32>;
@group(3) @binding(1) var surface_geometry: texture_2d<u32>;
@group(3) @binding(2) var surface_emission: texture_2d<f32>;

// Geometry stores three octahedral normals and full-precision linear depth.
// Zero depth is empty; FORWARD_SURFACE stores an already shaded color.
// The sign of the reflection exponent in diffuse.w carries the fog flag.
const FORWARD_SURFACE: u32 = 0xffffffffu;

struct PreparedSurface {
    @location(0) diffuse: vec4<f32>,
    @location(1) geometry: vec4<u32>,
    @location(2) emission: vec4<f32>,
}

fn oct_pack(n: vec3<f32>) -> u32 {
    let v = n / (abs(n.x) + abs(n.y) + abs(n.z));
    var p = v.xy;
    if v.z < 0.0 { p = (vec2(1.0) - abs(v.yx)) * select(vec2(-1.0), vec2(1.0), v.xy >= vec2(0.0)); }
    return pack2x16snorm(p);
}

fn oct_unpack(bits: u32) -> vec3<f32> {
    let p = unpack2x16snorm(bits);
    var v = vec3(p, 1.0 - abs(p.x) - abs(p.y));
    let t = clamp(-v.z, 0.0, 1.0);
    v = vec3(v.xy + select(vec2(t), vec2(-t), v.xy >= vec2(0.0)), v.z);
    return normalize(v);
}

@vertex fn deferred_vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    return vec4(vec2(f32((index << 1u) & 2u), f32(index & 2u)) * 2.0 - vec2(1.0), 0.0, 1.0);
}

@fragment fn deferred_light(@builtin(position) pixel: vec4<f32>) -> @location(0) vec4<f32> {
    let p = vec2<i32>(pixel.xy);
    let data = textureLoad(surface_geometry, p, 0);
    if data.w == 0u { discard; }
    let base = textureLoad(surface_diffuse, p, 0);
    if data.w == FORWARD_SURFACE { return vec4(base.rgb, 1.0); }
    let unlit_color = textureLoad(surface_emission, p, 0);
    let depth = bitcast<f32>(data.w);
    let ndc = pixel.xy / vec2<f32>(textureDimensions(surface_diffuse)) * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    let position = camera.rays[3].xyz + (camera.rays[0].xyz + camera.rays[1].xyz * ndc.x + camera.rays[2].xyz * ndc.y) * depth;
    let normal = oct_unpack(data.x);
    let wet_normal = oct_unpack(data.y);
    let surface = oct_unpack(data.z);
    var light = vec3(1.0);
    var reflected = vec3(0.0);
    if LIGHTING_ENABLED && lighting.options.w != 0.0 {
        light = lighting.options.x * mix(vec3(1.0), vec3(0.6, 0.75, 1.0), lighting.lamps.z);
        let sun_diffuse = max(0.0, dot(normal, lighting.sun_direction.xyz));
        var sun = 1.0;
        let view = normalize(camera.eye.xyz - position);
        let power = abs(base.w);
        let specular = unlit_color.w;
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
            if specular > 0.0 && dot(wet_normal, -delta) > 0.0 {
                let halfway = normalize(view + -delta / distance);
                glint = pow(max(0.0, dot(wet_normal, halfway)), power);
            }
            if diffuse == 0.0 && glint < 0.001 { continue; }
            let visibility = point_visibility(source, position, surface);
            let falloff = 1.0 - smoothstep(0.0, 1.0, distance / source.position.w);
            if diffuse > 0.0 { light += source.color.rgb * lighting.options.y * falloff * falloff * diffuse * visibility * local; }
            if glint >= 0.001 { reflected += source.color.rgb * lighting.options.y * falloff * falloff * glint * specular * visibility * local; }
        }
    }
    var color = base.rgb * light + unlit_color.rgb + reflected;
    if base.w < 0.0 {
        let fog = clamp(camera.fog.x / max(depth, 1.0) + camera.fog.y, 0.0, 1.0);
        let haze = select(0.0, 1.0 - exp(-max(depth, 0.0) * camera.fog_color.w / 50000.0), HDR_ENABLED);
        color = mix(color, camera.fog_color.rgb, 1.0 - (1.0 - fog) * (1.0 - haze));
    }
    color = rain_atmosphere(color, position, true, true, true) * camera.options.x;
    return vec4(max(color, vec3(0.0)), 1.0);
}

@fragment fn deferred_fallback(input: Fragment, @builtin(front_facing) front_facing: bool) -> PreparedSurface {
    var v = input;
    v.flags = (v.flags & (TEXTURED | TRANSLUCENT | DOUBLE_SIDED | FOG | EMITTER | WATER | WATERFALL | FOLIAGE | UNLIT)) | SCENERY;
    var result: PreparedSurface;
    result.diffuse = vec4(shade(v, front_facing, false, false, true, true, false).rgb, 0.0);
    result.geometry.w = FORWARD_SURFACE;
    return result;
}

@fragment fn deferred_material(input: Fragment, @builtin(front_facing) front_facing: bool) -> PreparedSurface {
    var v = input;
    v.flags = (v.flags & (TEXTURED | TRANSLUCENT | DOUBLE_SIDED | FOG | EMITTER | WATERFALL | FOLIAGE)) | SCENERY;
    let material = prepare_material(v, front_facing, false, false);
    let color = material.color;
    let emission = material.emission;
    let normal = material.normal;
    let surface_normal = material.surface_normal;
    var result: PreparedSurface;
    var diffuse = color * material.cavity * (1.0 - emission.a);
    var unlit_color = emission.rgb * emission.a;
    var wet_normal = normal;
    var power = 1.0;
    var specular = 0.0;
    if WEATHER_ENABLED && (v.flags & (SCENERY | TRANSLUCENT | WATER)) == SCENERY {
        if lighting.weather.x + lighting.weather.y + lighting.weather.w > 0.0 {
            let incidence = weather_incidence(surface_normal, true);
            let shelter = weather_exposure(surface_normal, v.world, true, false);
            if incidence > 0.0 && shelter > 0.0 {
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
    unlit_color += diffuse * storm_light(v.world, normal, surface_normal, true);
    result.diffuse = vec4(diffuse, select(power, -power, (v.flags & FOG) != 0u));
    result.geometry = vec4(oct_pack(normal), oct_pack(wet_normal), oct_pack(surface_normal), bitcast<u32>(v.depth));
    result.emission = vec4(unlit_color, specular);
    return result;
}

@fragment fn deferred_sky(input: Fragment) -> PreparedSurface {
    var result: PreparedSurface;
    result.diffuse = vec4(sky_color(input.normal).rgb, 0.0);
    result.geometry.w = FORWARD_SURFACE;
    return result;
}
