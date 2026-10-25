const TEXTURED: u32 = 1u;
const SCREEN: u32 = 2u;
const TRANSLUCENT: u32 = 4u;
const SCREEN_WIDE: u32 = 8u;
const DOUBLE_SIDED: u32 = 16u;
const FOG: u32 = 32u;
const DEBUG: u32 = 64u;
const XRAY: u32 = 128u;
const UI: u32 = 256u;
const EMITTER: u32 = 512u;
const SCENERY: u32 = 1024u;
const ALPHA: u32 = 2048u;
const STAIN: u32 = 4096u;
const SKY: u32 = 8192u;
const SOFT: u32 = 16384u;
const WATER: u32 = 65536u;
const WATERFALL: u32 = 131072u;
const FOOTPRINT: u32 = 1048576u;
const WET_CONTACT: u32 = 2097152u;

struct Camera { matrix: mat4x4<f32>, viewport: vec4<f32>, fog: vec4<f32>, options: vec4<f32>, fog_color: vec4<f32>, detail: vec4<f32>, eye: vec4<f32>, water: vec4<f32>, motion: vec4<f32> }
@group(0) @binding(0) var<uniform> camera: Camera;
@group(0) @binding(1) var vram: texture_2d<u32>;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) page: u32,
    @location(4) palette: u32,
    @location(5) flags: u32,
    @location(6) material: u32,
    @location(7) normal: vec3<f32>,
    @location(8) opacity: f32,
}
struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) page: u32,
    @location(3) @interpolate(flat) palette: u32,
    @location(4) @interpolate(flat) flags: u32,
    @location(5) depth: f32,
    @location(6) world: vec3<f32>,
    @location(7) @interpolate(flat) material: u32,
    @location(8) normal: vec3<f32>,
    @location(9) opacity: f32,
}

@vertex fn vertex(v: Vertex) -> Fragment {
    var out: Fragment;
    let position = foliage_position(v.position, v.flags, camera.eye.w, camera.motion.x);
    if (v.flags & SCREEN) != 0u {
        let half_width = select(120.0 * camera.viewport.x, 160.0, (v.flags & SCREEN_WIDE) != 0u);
        out.position = vec4(v.position.x / half_width, -v.position.y / 120.0, v.position.z, 1.0);
        out.depth = 0.0;
    } else {
        out.position = camera.matrix * vec4(position, 1.0);
        out.depth = out.position.w;
        if (v.flags & XRAY) != 0u { out.position.z = 0.0; }
    }
    out.world = position;
    out.color = v.color;
    out.uv = v.uv;
    out.page = v.page;
    out.palette = v.palette;
    out.flags = v.flags;
    out.material = v.material;
    out.normal = v.normal;
    out.opacity = v.opacity;
    return out;
}

fn shade(input: Fragment, front_facing: bool, translucent_pass: bool) -> vec4<f32> {
    var v = input;
    if (v.flags & (SCREEN | DEBUG)) == 0u {
        v.uv = material_coordinate(v.material, v.world, v.uv);
    }
    let world_dx = dpdx(v.world);
    let world_dy = dpdy(v.world);
    let surface_normal = normalize(cross(world_dy, world_dx));
    var normal = surface_normal;
    if dot(v.normal, v.normal) > 0.0001 {
        normal = normalize(v.normal);
        if dot(normal, surface_normal) < 0.0 { normal = -normal; }
    }
    let du = dpdx(v.uv);
    let dv = dpdy(v.uv);
    let detail = surface_detail(v, normal, world_dx, world_dy, du, dv);
    normal = water_normal(v, detail.normal, world_dx, world_dy);
    let coordinate = detail.coordinate;
    if (v.flags & SKY) != 0u { return sky_color(v.normal); }
    if !front_facing && (v.flags & (SCREEN | DOUBLE_SIDED)) == 0u { discard; }
    let screen = (v.flags & (SCREEN | DEBUG)) != 0u;
    let hdr = HDR_ENABLED && camera.viewport.z != 0.0 && (v.flags & (UI | DEBUG)) == 0u;
    let linear_lighting = hdr || (LIGHTING_ENABLED && !screen && lighting.options.w != 0.0);
    var color = v.color;
    var emission = vec4(0.0);
    if !screen && camera.options.z == 0.0 { color = vec3(1.0); }
    if (v.flags & (SCENERY | TEXTURED)) == (SCENERY | TEXTURED) {
        color = mix(color, vec3(1.0), camera.detail.z);
    }
    var semi = (v.flags & TRANSLUCENT) != 0u;
    if (v.flags & TEXTURED) != 0u {
        let nearest = vec2<i32>(floor(coordinate + vec2(0.001)));
        let pixel = texel(nearest, v.page, v.palette);
        let replaced = (v.flags & DEBUG) == 0u && select(material_replaced(v.material), material_authored(v.material), (v.flags & UI) != 0u);
        var base = rgb(pixel);
        if replaced {
            let decoded = material_texel(v.material, coordinate);
            if decoded.a == 0.0 { discard; }
            semi = semi && decoded.a < 0.75;
            base = decoded.rgb;
        } else {
            if pixel == 0u { discard; }
            semi = semi && (pixel & 32768u) != 0u;
        }
        if semi != translucent_pass { discard; }
        if screen || camera.options.y != 0.0 {
            if FILTER_MATERIALS && (!screen || replaced) && (camera.options.w >= 2.0 || ((replaced || camera.viewport.w != 0.0) && camera.options.w != 0.0)) {
                color *= material_color(v.material, coordinate, du, dv, camera.viewport.y, camera.options.w, camera.viewport.w, semi, (v.flags & TRANSLUCENT) != 0u, base);
            } else if FILTER_MATERIALS && !screen && camera.options.w != 0.0 {
                color *= filtered(coordinate, v.page, v.palette, pixel);
            } else {
                color *= base;
            }
        }
        if LIGHTING_ENABLED && !screen && (v.flags & EMITTER) != 0u && lighting.options.w != 0.0 && lighting.lamps.x != 0.0 {
            emission = material_emission(v.material, base);
            if linear_lighting { emission = vec4(emission.rgb * pow(max(base, vec3(0.0)), vec3(1.2)), emission.a); }
        }
    }
    if semi != translucent_pass { discard; }
    if (v.flags & (STAIN | SCENERY)) == STAIN {
        color = vec3(0.34, 0.012, 0.018) * (0.4 + 0.6 * dot(color, vec3(0.2126, 0.7152, 0.0722)));
    }
    if linear_lighting { color = pow(max(color, vec3(0.0)), vec3(2.2)); }
    if !screen {
        let light = select(illumination(v.world, normal, surface_normal), vec3(1.0), (v.flags & (ALPHA | EMITTER | TEXTURED)) == (ALPHA | EMITTER));
        color = mix(color * detail.cavity * light, emission.rgb, emission.a);
        color = water_color(v, color, normal, surface_normal, light, linear_lighting);
    }
    let surface_decal = (v.flags & (STAIN | FOOTPRINT | WET_CONTACT)) != 0u;
    if WEATHER_ENABLED && (v.flags & (SCENERY | TRANSLUCENT | WATER)) == SCENERY
        && ((v.flags & ALPHA) == 0u || surface_decal) {
        color = weather_surface(color, normal, surface_normal, v.world, max(length(world_dx), length(world_dy)), linear_lighting);
    }
    if (v.flags & (SCENERY | STAIN)) == (SCENERY | STAIN) {
        var absorption = vec3(0.52, 0.025, 0.045);
        if linear_lighting { absorption = pow(absorption, vec3(2.2)); }
        color *= absorption;
    }
    if (v.flags & FOOTPRINT) != 0u {
        color *= vec3(0.55, 0.62, 0.73);
    }
    if (v.flags & WET_CONTACT) != 0u { color = color * 0.62 + surface_reflection(v.world, normal, surface_normal, 0.15) * 0.45; }
    if (v.flags & (FOG | SCREEN)) == FOG {
        let fog = clamp(camera.fog.x / max(v.depth, 1.0) + camera.fog.y, 0.0, 1.0);
        let haze = select(0.0, 1.0 - exp(-max(v.depth, 0.0) * camera.fog_color.w / 50000.0), HDR_ENABLED);
        color = mix(color, camera.fog_color.rgb, 1.0 - (1.0 - fog) * (1.0 - haze));
    }
    if !screen {
        color = rain_atmosphere(color, v.world, linear_lighting);
        color *= camera.options.x;
    }
    if hdr { return vec4(max(color, vec3(0.0)), select(0.0, 1.0, semi)); }
    if linear_lighting {
        // Preserve hue and texture contrast instead of clipping bright lamp pools to white.
        let peak = max(color.r, max(color.g, color.b));
        if peak > 0.5 { color *= (0.5 + (peak - 0.5) / (1.0 + (peak - 0.5) * 2.0)) / peak; }
        color = pow(max(color, vec3(0.0)), vec3(1.0 / 2.2));
    }
    return vec4(clamp(color, vec3(0.0), vec3(1.0)), select(0.0, 1.0, semi));
}

@fragment fn opaque(v: Fragment, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    if (v.flags & ALPHA) != 0u { discard; }
    let color = shade(v, front_facing, false);
    return vec4(color.rgb, 1.0);
}

@fragment fn translucent(v: Fragment, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    if (v.flags & ALPHA) != 0u { discard; }
    let color = shade(v, front_facing, true);
    return vec4(color.rgb, 1.0);
}

@fragment fn alpha(v: Fragment, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let color = shade(v, front_facing, false);
    var opacity = v.opacity;
    if (v.flags & FOOTPRINT) != 0u {
        if !WEATHER_ENABLED { discard; }
        opacity *= weather_snow(normalize(v.normal), v.world);
    }
    if (v.flags & SOFT) != 0u { opacity *= 1.0 - smoothstep(0.1, 1.0, dot(v.uv, v.uv)); }
    return vec4(color.rgb, clamp(opacity, 0.0, 1.0));
}

fn sky_color(direction: vec3<f32>) -> vec4<f32> {
    let ray = normalize(direction);
    let height = clamp(-ray.y, 0.0, 1.0);
    let top = mix(vec3(0.00015, 0.0004, 0.0015), vec3(0.018, 0.08, 0.23), camera.detail.w);
    var color = mix(camera.fog_color.rgb, top, pow(height, 0.45));
    let alignment = max(0.0, dot(ray, lighting.sun_direction.xyz));
    let disc = smoothstep(cos(0.007), cos(0.0045), alignment);
    let halo = pow(alignment, 500.0) * 0.12;
    color += lighting.sun_color.rgb * (disc * 2.0 + halo) * min(1.0, lighting.sun_direction.w * 12.0);
    if WEATHER_ENABLED {
        color = mix(color, rain_color(), lighting.rain.y * (0.5 + 0.4 * (1.0 - height)));
    }
    color *= camera.options.x;
    if camera.viewport.z == 0.0 { color = pow(max(color, vec3(0.0)), vec3(1.0 / 2.2)); }
    return vec4(color, 1.0);
}
