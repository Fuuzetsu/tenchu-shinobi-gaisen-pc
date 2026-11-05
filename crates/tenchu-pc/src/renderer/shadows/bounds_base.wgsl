struct Region { area: vec4<u32>, face: u32, }
@group(0) @binding(0) var<uniform> regions: array<Region, LAYERS>;
@group(0) @binding(1) var source: texture_depth_2d_array;
@group(0) @binding(2) var output_bounds: texture_storage_2d_array<rg32float, write>;
var<workgroup> values: array<vec2<f32>, 64>;
@compute @workgroup_size(8, 8)
fn main(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_id) lane: vec3<u32>, @builtin(local_invocation_index) index: u32) {
    let region = regions[group.z];
    let point = group.xy + region.area.xy;
    if any(point >= region.area.zw) { return; }
    let size = textureDimensions(source);
    let start = point * BASE_TILE + lane.xy * (BASE_TILE / 8u);
    var value = vec2(1.0, 0.0);
    for(var y = 0u; y < BASE_TILE / 8u; y++) {
        for(var x = 0u; x < BASE_TILE / 8u; x++) {
            let depth = textureLoad(source, vec2<i32>(min(start + vec2(x,y), size - vec2(1u))), i32(region.face), 0);
            value = vec2(min(value.x, depth), max(value.y, depth));
        }
    }
    values[index] = value;
    workgroupBarrier();
    for(var step = 32u; step > 0u; step /= 2u) {
        if index < step {
            let a = values[index];
            let b = values[index + step];
            values[index] = vec2(min(a.x,b.x),max(a.y,b.y));
        }
        workgroupBarrier();
    }
    if index == 0u { textureStore(output_bounds, vec2<i32>(point), i32(region.face), vec4(values[0],0.0,0.0)); }
}
