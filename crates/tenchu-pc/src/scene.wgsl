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

struct Camera { matrix: mat4x4<f32>, viewport: vec4<f32>, fog: vec4<f32>, options: vec4<f32>, fog_color: vec4<f32>, detail: vec4<f32>, eye: vec4<f32> }
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
}

@vertex fn vertex(v: Vertex) -> Fragment {
    var out: Fragment;
    if (v.flags & SCREEN) != 0u {
        let half_width = select(120.0 * camera.viewport.x, 160.0, (v.flags & SCREEN_WIDE) != 0u);
        out.position = vec4(v.position.x / half_width, -v.position.y / 120.0, v.position.z, 1.0);
        out.depth = 0.0;
    } else {
        out.position = camera.matrix * vec4(v.position, 1.0);
        out.depth = out.position.w;
        if (v.flags & XRAY) != 0u { out.position.z = 0.0; }
    }
    out.world = v.position;
    out.color = v.color;
    out.uv = v.uv;
    out.page = v.page;
    out.palette = v.palette;
    out.flags = v.flags;
    out.material = v.material;
    out.normal = v.normal;
    return out;
}

fn shade(v: Fragment, front_facing: bool, translucent_pass: bool) -> vec4<f32> {
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
    normal = detail.normal;
    let coordinate = detail.coordinate;
    if !front_facing && (v.flags & (SCREEN | DOUBLE_SIDED)) == 0u { discard; }
    let screen = (v.flags & (SCREEN | DEBUG)) != 0u;
    let hdr = camera.viewport.z != 0.0 && (v.flags & (UI | DEBUG)) == 0u;
    let linear_lighting = hdr || (!screen && lighting.options.w != 0.0);
    var color = v.color;
    var emission = vec4(0.0);
    if !screen && camera.options.z == 0.0 { color = vec3(1.0); }
    var semi = (v.flags & TRANSLUCENT) != 0u;
    if (v.flags & TEXTURED) != 0u {
        let nearest = vec2<i32>(floor(coordinate + vec2(0.001)));
        let pixel = texel(nearest, v.page, v.palette);
        let replaced = !screen && material_replaced(v.material);
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
            if !screen && (camera.options.w >= 2.0 || ((replaced || camera.viewport.w != 0.0) && camera.options.w != 0.0)) {
                color *= material_color(v.material, coordinate, du, dv, camera.viewport.y, camera.options.w, camera.viewport.w, semi, (v.flags & TRANSLUCENT) != 0u, base);
            } else if !screen && camera.options.w != 0.0 {
                color *= filtered(coordinate, v.page, v.palette, pixel);
            } else {
                color *= base;
            }
        }
        if !screen && (v.flags & EMITTER) != 0u && lighting.options.w != 0.0 && lighting.lamps.x != 0.0 {
            emission = material_emission(v.material, base);
            if linear_lighting { emission = vec4(emission.rgb * pow(max(base, vec3(0.0)), vec3(1.2)), emission.a); }
        }
    }
    if semi != translucent_pass { discard; }
    if linear_lighting { color = pow(max(color, vec3(0.0)), vec3(2.2)); }
    if !screen { color = mix(color * detail.cavity * illumination(v.world, normal, surface_normal), emission.rgb, emission.a); }
    if (v.flags & (FOG | SCREEN)) == FOG {
        let fog = clamp(camera.fog.x / max(v.depth, 1.0) + camera.fog.y, 0.0, 1.0);
        let haze = 1.0 - exp(-max(v.depth, 0.0) * camera.fog_color.w / 50000.0);
        color = mix(color, camera.fog_color.rgb, 1.0 - (1.0 - fog) * (1.0 - haze));
    }
    if !screen { color *= camera.options.x; }
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
    let color = shade(v, front_facing, false);
    return vec4(color.rgb, 1.0);
}

@fragment fn translucent(v: Fragment, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let color = shade(v, front_facing, true);
    return vec4(color.rgb, 1.0);
}
