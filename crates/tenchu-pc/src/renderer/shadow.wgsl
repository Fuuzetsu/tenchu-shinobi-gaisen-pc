@group(0) @binding(0) var<uniform> matrix: mat4x4<f32>;
@group(0) @binding(1) var vram: texture_2d<u32>;

struct Vertex {
    @location(0) position: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) uv: vec2<f32>,
    @location(3) page: u32,
    @location(4) palette: u32,
    @location(5) flags: u32,
    @location(6) material: u32,
}
struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) @interpolate(flat) page: u32,
    @location(2) @interpolate(flat) palette: u32,
    @location(3) @interpolate(flat) flags: u32,
    @location(4) @interpolate(flat) material: u32,
}

@vertex fn vertex(v: Vertex) -> Fragment {
    var out: Fragment;
    out.position = matrix * vec4(v.position, 1.0);
    out.uv = v.uv;
    out.page = v.page;
    out.palette = v.palette;
    out.flags = v.flags;
    out.material = v.material;
    return out;
}

@fragment fn fragment(v: Fragment) {
    // Screen/debug geometry and genuinely translucent texels do not cast shadows.
    if (v.flags & 66u) != 0u { discard; }
    if (v.flags & 1u) != 0u {
        if material_replaced(v.material) {
            let alpha = material_texel(v.material, v.uv).a;
            if alpha == 0.0 || ((v.flags & 4u) != 0u && alpha < 0.75) { discard; }
        } else {
            let pixel = texel(vec2<i32>(floor(v.uv + vec2(0.001))), v.page, v.palette);
            if pixel == 0u || ((v.flags & 4u) != 0u && (pixel & 32768u) != 0u) { discard; }
        }
    } else if (v.flags & 4u) != 0u {
        discard;
    }
}
