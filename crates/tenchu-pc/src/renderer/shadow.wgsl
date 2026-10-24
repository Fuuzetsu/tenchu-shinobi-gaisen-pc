struct Face { matrix: mat4x4<f32>, options: vec4<f32> }
const SCREEN: u32 = 2u;
const DEBUG: u32 = 64u;
const ALPHA: u32 = 2048u;
const WATER: u32 = 65536u;
@group(0) @binding(0) var<uniform> face: Face;
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
    @location(5) world: vec3<f32>,
}

@vertex fn vertex(v: Vertex) -> Fragment {
    var out: Fragment;
    let position = foliage_position(v.position, v.flags, face.options.y, face.options.z);
    out.position = face.matrix * vec4(position, 1.0);
    out.uv = v.uv;
    out.page = v.page;
    out.palette = v.palette;
    out.flags = v.flags;
    out.material = v.material;
    out.world = position;
    return out;
}

@fragment fn fragment(v: Fragment) {
    if face.options.w != 0.0 && (v.flags & 1024u) == 0u { discard; }
    if (v.flags & (SCREEN | DEBUG | ALPHA | WATER)) != 0u { discard; }
    if (v.flags & 1u) != 0u {
        var color = vec3(0.0);
        if material_replaced(v.material) {
            let decoded = material_texel(v.material, material_coordinate(v.material, v.world, v.uv));
            let alpha = decoded.a;
            color = decoded.rgb;
            if alpha == 0.0 || ((v.flags & 4u) != 0u && alpha < 0.75) { discard; }
        } else {
            let pixel = texel(vec2<i32>(floor(v.uv + vec2(0.001))), v.page, v.palette);
            color = rgb(pixel);
            if pixel == 0u || ((v.flags & 4u) != 0u && (pixel & 32768u) != 0u) { discard; }
        }
        if face.options.x != 0.0 && (v.flags & 512u) != 0u && material_transmits(v.material, color) { discard; }
    } else if (v.flags & 4u) != 0u {
        discard;
    }
}
