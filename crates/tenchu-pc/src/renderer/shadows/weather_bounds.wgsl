SOURCE
struct Transform {
    height: vec4<f32>,
    layer: u32,
}
@group(0) @binding(1) var<uniform> transform: Transform;

fn height_bounds(point: vec2<i32>, depth: f32, size: vec2<i32>) -> vec2<f32> {
    let ndc = (vec2<f32>(point) + vec2(0.5)) / vec2<f32>(size) * vec2(2.0, -2.0) + vec2(-1.0, 1.0);
    return vec2(dot(transform.height, vec4(ndc, depth, 1.0)));
}

@vertex fn vertex(@builtin(vertex_index) index: u32) -> @builtin(position) vec4<f32> {
    let p = array(vec2(-1.0, -1.0), vec2(3.0, -1.0), vec2(-1.0, 3.0));
    return vec4(p[index], 0.0, 1.0);
}

@fragment fn fragment(@builtin(position) position: vec4<f32>) -> @location(0) vec2<f32> {
    let start = vec2<i32>(position.xy) * STRIDE;
    let size = vec2<i32>(textureDimensions(source));
    var bounds = vec2(1e30, -1e30);
    for (var y = 0; y < STRIDE; y++) {
        for (var x = 0; x < STRIDE; x++) {
            let point = min(start + vec2(x, y), size - vec2(1));
            let value = SAMPLE;
            bounds = vec2(min(bounds.x, value.x), max(bounds.y, value.y));
        }
    }
    return bounds;
}
