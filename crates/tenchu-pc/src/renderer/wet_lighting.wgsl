@group(3) @binding(0) var surface_diffuse: texture_2d<f32>;
@group(3) @binding(1) var surface_geometry: texture_2d<u32>;
@group(3) @binding(2) var surface_emission: texture_2d<f32>;

// Geometry stores three octahedral normals and full-precision linear depth.
// Zero depth is empty; FORWARD_SURFACE stores an already shaded color.
// The sign of the reflection exponent in diffuse.w carries the fog flag.
// Negative emission.w marks water's sun-only reflections.
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
    let position = surface_position(pixel.xy, depth);
    let material = TerrainSurface(base.rgb, unlit_color.rgb, oct_unpack(data.x), oct_unpack(data.y), oct_unpack(data.z), abs(base.w), unlit_color.w);
    let filtered = interpolate_shadows(pixel.xy, position, material.surface);
    var color = light_terrain(material, position, filtered);
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
    v.flags = (v.flags & (TEXTURED | TRANSLUCENT | DOUBLE_SIDED | FOG | EMITTER | WATER | WATERFALL | FOLIAGE)) | SCENERY;
    let material = prepare_terrain(v, prepare_material(v, front_facing, false, false));
    var result: PreparedSurface;
    result.diffuse = vec4(material.diffuse, select(material.power, -material.power, (v.flags & FOG) != 0u));
    result.geometry = vec4(oct_pack(material.normal), oct_pack(material.wet_normal), oct_pack(material.surface), bitcast<u32>(v.depth));
    result.emission = vec4(material.emission, material.specular);
    return result;
}

@fragment fn deferred_sky(input: Fragment) -> PreparedSurface {
    var result: PreparedSurface;
    result.diffuse = vec4(sky_color(input.normal).rgb, 0.0);
    result.geometry.w = FORWARD_SURFACE;
    return result;
}
