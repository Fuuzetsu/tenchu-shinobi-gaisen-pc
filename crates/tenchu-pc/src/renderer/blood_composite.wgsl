@group(3) @binding(0) var blood_opacity: texture_2d<f32>;
@group(3) @binding(1) var blood_identity: texture_2d<u32>;
@group(3) @binding(2) var blood_surface: texture_2d<f32>;

@vertex fn blood_vertex(v: Vertex, @location(9) source: u32,
    @builtin(vertex_index) vertex_id: u32, @builtin(instance_index) first: u32) -> Fragment {
    var out = project_vertex(v);
    out.identity = first + vertex_id / 3u;
    out.source = source;
    return out;
}

struct BloodCoverage {
    @location(0) opacity: vec4<f32>,
    @location(1) identity: u32,
    @location(2) surface: vec4<f32>,
}

@fragment fn blood_coverage(input: Fragment, @builtin(front_facing) front_facing: bool) -> BloodCoverage {
    var v = input;
    v.flags = (v.flags & (TEXTURED | TRANSLUCENT | DOUBLE_SIDED | FOG | FOLIAGE)) | SCENERY | SOFT | ALPHA;
    let opacity = clamp(v.opacity * blood_mask(v.color.xy), 0.0, 1.0);
    if opacity <= 0.0 { discard; }
    _ = shade(v, front_facing, false, true, true, true, false);
    // Half floats represent these digits exactly. Min of digits and complements
    // proves that every contributing stain has the same nonzero source ID.
    let code = vec2(f32(v.source & 2047u), f32(v.source >> 11u));
    let tag = select(vec4(0.0), vec4(code, vec2(2047.0) - code), v.source != 0u);
    return BloodCoverage(vec4(opacity, 0.0, 0.0, opacity), v.identity, tag);
}

fn composed_blood(input: Fragment, front_facing: bool, bounds: bool) -> vec4<f32> {
    let pixel = vec2<i32>(input.position.xy);
    let tag = textureLoad(blood_surface, pixel, 0);
    let merge = all(tag.xy + tag.zw == vec2(2047.0));
    if merge && textureLoad(blood_identity, pixel, 0).r != input.identity { discard; }
    let color = ground_blood_color(input, front_facing, bounds);
    let opacity = textureLoad(blood_opacity, pixel, 0).r;
    return vec4(color.rgb, select(color.a, opacity, merge));
}

@fragment fn blood_composite(input: Fragment, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    return composed_blood(input, front_facing, false);
}

@fragment fn snow_blood_composite(input: Fragment, @builtin(front_facing) front_facing: bool) -> @location(0) vec4<f32> {
    return composed_blood(input, front_facing, true);
}

