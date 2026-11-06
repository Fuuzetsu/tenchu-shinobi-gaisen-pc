struct Volume {
    rays: mat4x4<f32>,
    matrices: array<mat4x4<f32>, 2>,
    spans: array<vec4<f32>, 2>,
    eye: vec4<f32>,
}
@group(0) @binding(0) var<uniform> volume: Volume;
@group(0) @binding(1) var weather_map: texture_depth_2d_array;
@group(0) @binding(2) var shadow_sampler: sampler_comparison;
@group(0) @binding(3) var haze: texture_storage_3d<rgba16float, write>;

@compute @workgroup_size(8, 8, 1)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let size = textureDimensions(haze);
    if any(id >= size) { return; }
    let uv = (vec2<f32>(id.xy) + vec2(0.5)) / vec2<f32>(size.xy);
    let ndc = uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    let depth = rain_view_depth(f32(id.z) / f32(size.z - 1u));
    let ray = volume.rays[0].xyz + volume.rays[1].xyz * ndc.x + volume.rays[2].xyz * ndc.y;
    let position = volume.rays[3].xyz + ray * depth;
    let delta = position - volume.eye.xyz;
    let distance = length(delta);
    let radius = 80.0 + min(distance / 30.0, 420.0);
    let start0 = (volume.matrices[0] * vec4(volume.eye.xyz, 1.0)).xyz;
    let start1 = (volume.matrices[1] * vec4(volume.eye.xyz, 1.0)).xyz;
    let delta0 = (volume.matrices[0] * vec4(delta, 0.0)).xyz;
    let delta1 = (volume.matrices[1] * vec4(delta, 0.0)).xyz;
    let exposed = rain_view_exposure(start0, start1, delta0, delta1, radius, volume.spans[0].xy, volume.spans[1].xy);
    let amount = min(0.94, 1.0 - exp(-distance * volume.eye.w * exposed / 18000.0));
    textureStore(haze, vec3<i32>(id), vec4(amount));
}
