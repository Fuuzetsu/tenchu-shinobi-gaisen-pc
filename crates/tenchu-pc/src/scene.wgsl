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
}
struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) color: vec3<f32>,
    @location(1) uv: vec2<f32>,
    @location(2) @interpolate(flat) page: u32,
    @location(3) @interpolate(flat) palette: u32,
    @location(4) @interpolate(flat) flags: u32,
    @location(5) depth: f32,
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
    out.color = v.color;
    out.uv = v.uv;
    out.page = v.page;
    out.palette = v.palette;
    out.flags = v.flags;
    return out;
}

fn word(x: u32, y: u32) -> u32 {
    return textureLoad(vram, vec2<i32>(i32(x & 1023u), i32(y & 511u)), 0).r;
}

fn texel(coordinate: vec2<i32>, page: u32, palette: u32) -> u32 {
    let uv = vec2<u32>(coordinate & vec2(255));
        let depth = (page >> 7u) & 3u;
        let x = (page & 15u) * 64u;
        let y = ((page >> 4u) & 1u) * 256u;
        var pixel: u32;
        if depth == 0u {
            let packed = word(x + uv.x / 4u, y + uv.y);
            let index = (packed >> ((uv.x & 3u) * 4u)) & 15u;
            pixel = word((palette & 63u) * 16u + index, palette >> 6u);
        } else if depth == 1u {
            let packed = word(x + uv.x / 2u, y + uv.y);
            let index = (packed >> ((uv.x & 1u) * 8u)) & 255u;
            pixel = word((palette & 63u) * 16u + index, palette >> 6u);
        } else {
            pixel = word(x + uv.x, y + uv.y);
        }
    return pixel;
}

fn rgb(pixel: u32) -> vec3<f32> {
    return vec3<f32>(f32(pixel & 31u), f32((pixel >> 5u) & 31u), f32((pixel >> 10u) & 31u)) / 31.0;
}

fn filtered(uv: vec2<f32>, page: u32, palette: u32, nearest: u32) -> vec3<f32> {
    let base = vec2<i32>(floor(uv));
    let f = fract(uv);
    var color = vec3(0.0);
    var weight = 0.0;
    for (var y = 0; y < 2; y++) {
        for (var x = 0; x < 2; x++) {
            let pixel = texel(base + vec2(x, y), page, palette);
            let w = select(1.0 - f.x, f.x, x == 1) * select(1.0 - f.y, f.y, y == 1);
            if pixel != 0u && (pixel & 32768u) == (nearest & 32768u) {
                color += rgb(pixel) * w;
                weight += w;
            }
        }
    }
    return select(rgb(nearest), color / max(weight, 0.0001), weight > 0.0001);
}

fn shade(v: Fragment, front_facing: bool) -> vec4<f32> {
    if !front_facing && (v.flags & (SCREEN | DOUBLE_SIDED)) == 0u { discard; }
    let screen = (v.flags & (SCREEN | DEBUG)) != 0u;
    var color = v.color;
    if !screen && camera.options.z == 0.0 { color = vec3(1.0); }
    var semi = (v.flags & TRANSLUCENT) != 0u;
    if (v.flags & TEXTURED) != 0u {
        let nearest = vec2<i32>(floor(v.uv + vec2(0.001)));
        let pixel = texel(nearest, v.page, v.palette);
        if pixel == 0u { discard; }
        semi = semi && (pixel & 32768u) != 0u;
        if screen || camera.options.y != 0.0 {
            if !screen && camera.options.w != 0.0 {
                color *= filtered(v.uv, v.page, v.palette, pixel);
            } else {
                color *= rgb(pixel);
            }
        }
    }
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
