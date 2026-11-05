@group(3) @binding(0) var masks: texture_2d_array<f32>;
@group(3) @binding(1) var mask_sampler: sampler;
@vertex fn cached_blood_vertex(v: Vertex, @location(9) layer: u32) -> Fragment {
    var out = project_vertex(v); out.source = layer; return out;
}
fn cached_stain(input: Fragment, front_facing: bool, bounds: bool) -> vec4<f32> {
    let opacity = textureSampleLevel(masks, mask_sampler, input.color.xy, i32(input.source), 0.0).r;
    if opacity <= 0.0 { discard; }
    var v = input; v.color = vec3(0.0, 0.0, input.color.z); v.opacity = opacity;
    return ground_blood_color(v, front_facing, bounds);
}
@fragment fn cached_blood(input: Fragment, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> { return cached_stain(input,front_facing,false); }
@fragment fn cached_snow_blood(input: Fragment, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> { return cached_stain(input,front_facing,true); }
