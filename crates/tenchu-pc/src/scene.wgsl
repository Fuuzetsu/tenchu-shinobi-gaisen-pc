const TEXTURED: u32 = 1u;
const SCREEN: u32 = 2u;
const TRANSLUCENT: u32 = 4u;
const SCREEN_WIDE: u32 = 8u;
const DOUBLE_SIDED: u32 = 16u;
const FOG: u32 = 32u;
const DEBUG: u32 = 64u;
const XRAY: u32 = 128u;

struct Camera { matrix: mat4x4<f32>, viewport: vec4<f32>, fog: vec4<f32>, options: vec4<f32> }
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

fn shade(v: Fragment, front_facing: bool) -> vec4<f32> {
    let surface_normal = normalize(cross(dpdx(v.world), dpdy(v.world)));
    var normal = surface_normal;
    if dot(v.normal, v.normal) > 0.0001 { normal = normalize(v.normal); }
    let du = dpdx(v.uv);
    let dv = dpdy(v.uv);
    if !front_facing && (v.flags & (SCREEN | DOUBLE_SIDED)) == 0u { discard; }
    let screen = (v.flags & (SCREEN | DEBUG)) != 0u;
    var color = v.color;
    var emission = vec3(0.0);
    if !screen && camera.options.z == 0.0 { color = vec3(1.0); }
    var semi = (v.flags & TRANSLUCENT) != 0u;
    if (v.flags & TEXTURED) != 0u {
        let nearest = vec2<i32>(floor(v.uv + vec2(0.001)));
        let pixel = texel(nearest, v.page, v.palette);
        let replaced = !screen && material_replaced(v.material);
        var base = rgb(pixel);
        if replaced {
            let decoded = material_texel(v.material, v.uv);
            if decoded.a == 0.0 { discard; }
            semi = semi && decoded.a < 0.75;
            base = decoded.rgb;
        } else {
            if pixel == 0u { discard; }
            semi = semi && (pixel & 32768u) != 0u;
        }
        if screen || camera.options.y != 0.0 {
            if !screen && (camera.options.w >= 2.0 || (replaced && camera.options.w != 0.0)) {
                color *= material_color(v.material, v.uv, du, dv, camera.viewport.y, camera.options.w, semi, (v.flags & TRANSLUCENT) != 0u, base);
            } else if !screen && camera.options.w != 0.0 {
                color *= filtered(v.uv, v.page, v.palette, pixel);
            } else {
                color *= base;
            }
        }
        if !screen && lighting.options.w != 0.0 && lighting.lamps.x != 0.0 {
            emission = material_emission(v.material, base);
        }
    }
    if !screen { color = color * illumination(v.world, normal, surface_normal) + emission; }
    if (v.flags & (FOG | SCREEN)) == FOG {
        let fog = clamp(camera.fog.x / max(v.depth, 1.0) + camera.fog.y, 0.0, 1.0);
        color *= 1.0 - fog;
    }
    if !screen { color *= camera.options.x; }
    return vec4(clamp(color, vec3(0.0), vec3(1.0)), select(0.0, 1.0, semi));
}

@fragment fn opaque(v: Fragment, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let color = shade(v, front_facing);
    if color.a != 0.0 { discard; }
    return vec4(color.rgb, 1.0);
}

@fragment fn translucent(v: Fragment, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    let color = shade(v, front_facing);
    if color.a == 0.0 { discard; }
    return vec4(color.rgb, 1.0);
}
