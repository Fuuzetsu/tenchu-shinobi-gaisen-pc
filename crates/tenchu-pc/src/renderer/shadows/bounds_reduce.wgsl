struct Region { area: vec4<u32>, face: u32, }
@group(0) @binding(0) var<uniform> regions: array<Region, LAYERS>;
@group(0) @binding(1) var source: texture_2d_array<f32>;
@group(0) @binding(2) var output_bounds: texture_storage_2d_array<rg32float, write>;
@compute @workgroup_size(8, 8)
fn main(@builtin(global_invocation_id) id: vec3<u32>) {
    let region = regions[id.z];
    let point = id.xy + region.area.xy;
    if any(point >= region.area.zw) { return; }
    let size = textureDimensions(source);
    var value = vec2(1.0, 0.0);
    for(var y = 0u; y < 2u; y++) {
        for(var x = 0u; x < 2u; x++) {
            let v = textureLoad(source, vec2<i32>(min(point * 2u + vec2(x,y), size - vec2(1u))), i32(region.face), 0).rg;
            value = vec2(min(value.x, v.x), max(value.y, v.y));
        }
    }
    textureStore(output_bounds, vec2<i32>(point), i32(region.face), vec4(value,0.0,0.0));
}
