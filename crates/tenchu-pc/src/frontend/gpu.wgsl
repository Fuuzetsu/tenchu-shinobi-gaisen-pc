struct Vertex {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) color: vec3<f32>,
    @location(2) @interpolate(flat) uv_bounds: vec4<f32>,
    @location(3) @interpolate(flat) room: f32,
};

@group(0) @binding(0) var artwork: texture_2d<f32>;
@group(0) @binding(1) var artwork_sampler: sampler;
override srgb_target: bool = false;

@vertex
fn vs_main(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) color: vec3<f32>, @location(3) uv_bounds: vec4<f32>, @location(4) room: f32) -> Vertex {
    return Vertex(vec4<f32>(position, 0.0, 1.0), uv, color, uv_bounds, room);
}

fn shade(v: Vertex, kind: u32) -> vec4<f32> {
    let half_texel = 0.5 / vec2<f32>(textureDimensions(artwork));
    let uv = clamp(v.uv, v.uv_bounds.xy + half_texel, v.uv_bounds.zw - half_texel);
    let texel = textureSample(artwork, artwork_sampler, uv);
    if texel.a < 0.01 || (kind == 1u && texel.a < 0.75) || (kind == 2u && texel.a >= 0.75) { discard; }
    var rgb = min(texel.rgb * v.color, vec3<f32>(1.0));
    if v.room != 0.0 { rgb = room_shade(uv, rgb); }
    if srgb_target {
        rgb = select(rgb / 12.92, pow((rgb + 0.055) / 1.055, vec3<f32>(2.4)), rgb > vec3<f32>(0.04045));
    }
    return vec4<f32>(rgb, 1.0);
}

@fragment fn fs_all(v: Vertex) -> @location(0) vec4<f32> { return shade(v, 0u); }
@fragment fn fs_opaque(v: Vertex) -> @location(0) vec4<f32> { return shade(v, 1u); }
@fragment fn fs_semi(v: Vertex) -> @location(0) vec4<f32> { return shade(v, 2u); }
