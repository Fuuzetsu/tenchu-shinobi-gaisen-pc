const TEXTURED: u32 = 1u;
const SCREEN: u32 = 2u;
const TRANSLUCENT: u32 = 4u;

struct Camera { matrix: mat4x4<f32>, viewport: vec4<f32> }
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
}

@vertex fn vertex(v: Vertex) -> Fragment {
    var out: Fragment;
    if (v.flags & SCREEN) != 0u {
        out.position = vec4(v.position.x / (120.0 * camera.viewport.x), -v.position.y / 120.0, v.position.z, 1.0);
    } else {
        out.position = camera.matrix * vec4(v.position, 1.0);
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

fn shade(v: Fragment) -> vec4<f32> {
    var color = v.color;
    var semi = (v.flags & TRANSLUCENT) != 0u;
    if (v.flags & TEXTURED) != 0u {
        let uv = vec2<u32>(floor(v.uv + vec2(0.001))) & vec2(255u);
        let depth = (v.page >> 7u) & 3u;
        let x = (v.page & 15u) * 64u;
        let y = ((v.page >> 4u) & 1u) * 256u;
        var pixel: u32;
        if depth == 0u {
            let packed = word(x + uv.x / 4u, y + uv.y);
            let index = (packed >> ((uv.x & 3u) * 4u)) & 15u;
            pixel = word((v.palette & 63u) * 16u + index, v.palette >> 6u);
        } else if depth == 1u {
            let packed = word(x + uv.x / 2u, y + uv.y);
            let index = (packed >> ((uv.x & 1u) * 8u)) & 255u;
            pixel = word((v.palette & 63u) * 16u + index, v.palette >> 6u);
        } else {
            pixel = word(x + uv.x, y + uv.y);
        }
        if pixel == 0u { discard; }
        semi = semi && (pixel & 32768u) != 0u;
        color *= vec3<f32>(f32(pixel & 31u), f32((pixel >> 5u) & 31u), f32((pixel >> 10u) & 31u)) / 31.0;
    }
    return vec4(clamp(color, vec3(0.0), vec3(1.0)), select(0.0, 1.0, semi));
}

@fragment fn opaque(v: Fragment) -> @location(0) vec4<f32> {
    let color = shade(v);
    if color.a != 0.0 { discard; }
    return vec4(color.rgb, 1.0);
}

@fragment fn translucent(v: Fragment) -> @location(0) vec4<f32> {
    let color = shade(v);
    if color.a == 0.0 { discard; }
    return vec4(color.rgb, 1.0);
}
