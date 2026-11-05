struct Options {
    camera: mat4x4<f32>,
    right: vec4<f32>,
    params: vec4<f32>,
}
@group(0) @binding(0) var<uniform> options: Options;
@group(0) @binding(1) var depth_map: DEPTH_TYPE;
@group(0) @binding(2) var depth_sampler: sampler;
struct Fragment {
    @builtin(position) position: vec4<f32>,
    @location(0) uv: vec2<f32>,
    @location(1) clip: vec4<f32>,
    @location(2) @interpolate(flat) phase: f32,
}
@vertex fn vertex(@builtin(vertex_index) index: u32, @location(0) source: vec4<f32>, @location(1) plume: vec4<f32>) -> Fragment {
    let corners = array(vec2(-1.0, 0.0), vec2(1.0, 0.0), vec2(-1.0, 1.0), vec2(-1.0, 1.0), vec2(1.0, 0.0), vec2(1.0, 1.0));
    let uv = corners[index];
    let point = source.xyz + options.right.xyz * uv.x * source.w - vec3(0.0, uv.y * plume.x, 0.0);
    let clip = options.camera * vec4(point, 1.0);
    return Fragment(clip, uv, clip, plume.y);
}
@fragment fn fragment(v: Fragment) -> @location(0) vec4<f32> {
    let screen = v.clip.xy / v.clip.w * vec2(0.5, -0.5) + vec2(0.5);
    let depth = depth_at(vec2<i32>(screen * options.params.zw));
    if depth < v.clip.z / v.clip.w { discard; }
    let time = options.params.x;
    let rise = v.uv.y;
    let sway = sin(rise * 9.0 - time * 2.6 + v.phase) * 0.12 * rise;
    let edge = 1.0 - smoothstep(0.25, 1.0, abs(v.uv.x - sway));
    let density = edge * edge * smoothstep(0.0, 0.15, rise) * (1.0 - smoothstep(0.25, 1.0, rise));
    let wave = vec2(sin(rise * 24.0 - time * 5.0 + v.phase + v.uv.x * 7.0), cos(rise * 19.0 - time * 4.1 - v.phase + v.uv.x * 5.0));
    // Screen-space displacement is capped; even close flames remain a subtle shimmer.
    let offset = wave * vec2(2.5, 1.5) * options.params.y / vec2(1920.0, 1080.0);
    return vec4(offset * density, v.clip.w / 1000.0 * density, density);
}
