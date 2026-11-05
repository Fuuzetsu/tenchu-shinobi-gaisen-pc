struct Options {
    inverse: mat4x4<f32>,
    color: vec4<f32>,
    effects: vec4<f32>,
    finish: vec4<f32>,
    size: vec4<f32>,
}
@group(0) @binding(0) var scene: texture_2d<f32>;
@group(0) @binding(1) var bloom0: texture_2d<f32>;
@group(0) @binding(2) var bloom1: texture_2d<f32>;
@group(0) @binding(3) var bloom2: texture_2d<f32>;
@group(0) @binding(4) var bloom3: texture_2d<f32>;
@group(0) @binding(5) var linear_sampler: sampler;
@group(0) @binding(6) var depth_map: DEPTH_TYPE;
@group(0) @binding(7) var<uniform> options: Options;
@group(0) @binding(8) var depth_sampler: sampler;
@group(0) @binding(9) var ambient_occlusion: texture_2d<f32>;
@group(0) @binding(10) var heat_map: texture_2d<f32>;

struct Fragment { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32> }
struct Output { @location(0) color: vec4<f32>, @builtin(frag_depth) depth: f32 }
@vertex fn vertex(@builtin(vertex_index) index: u32) -> Fragment {
    let uv = vec2(f32((index << 1u) & 2u), f32(index & 2u));
    return Fragment(vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0), uv);
}


fn world(uv: vec2<f32>, depth: f32) -> vec3<f32> {
    let p = options.inverse * vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), depth, 1.0);
    return p.xyz / p.w;
}

fn surface(pixel: vec2<i32>) -> vec3<f32> {
    return world((vec2<f32>(pixel) + vec2(0.5)) / options.size.xy, depth_at(pixel));
}

fn occlusion(pixel: vec2<i32>, position: vec3<f32>) -> f32 {
    let right = surface(pixel + vec2(1, 0)) - position;
    let left = position - surface(pixel - vec2(1, 0));
    let down = surface(pixel + vec2(0, 1)) - position;
    let up = position - surface(pixel - vec2(0, 1));
    let dx = select(left, right, dot(right, right) < dot(left, left));
    let dy = select(up, down, dot(down, down) < dot(up, up));
    let cross_normal = cross(dx, dy);
    if dot(cross_normal, cross_normal) < 0.00001 { return 1.0; }
    var normal = normalize(cross_normal);
    let uv = (vec2<f32>(pixel) + vec2(0.5)) / options.size.xy;
    if dot(normal, world(uv, 0.0) - position) < 0.0 { normal = -normal; }
    let radius = options.effects.z;
    let screen_radius = clamp(radius / max(1.0, (length(dx) + length(dy)) * 0.5), 2.0, 64.0);
    var sum = 0.0;
    for (var i = 0; i < 8; i++) {
        let angle = f32(i) * 0.785398163;
        let direction = vec2(cos(angle), sin(angle));
        var horizon = 0.0;
        for (var j = 0; j < 2; j++) {
            let sample_pixel = pixel + vec2<i32>(round(direction * screen_radius * (0.35 + f32(j) * 0.5)));
            if any(sample_pixel < vec2(0)) || any(sample_pixel >= vec2<i32>(options.size.xy)) { continue; }
            let depth = depth_at(sample_pixel);
            if depth >= 0.999999 { continue; }
            let delta = world((vec2<f32>(sample_pixel) + vec2(0.5)) / options.size.xy, depth) - position;
            let distance = length(delta);
            if distance < 1.0 || distance > radius { continue; }
            horizon = max(horizon, max(0.0, dot(normal, delta / distance) - 0.08) * (1.0 - distance / radius));
        }
        sum += horizon;
    }
    return 1.0 - clamp(sum / 8.0 * options.effects.y * 2.0, 0.0, 0.8);
}

@fragment fn occlusion_pass(v: Fragment) -> @location(0) vec2<f32> {
    let pixel = vec2<i32>(v.uv * options.size.xy);
    let depth = depth_at(pixel);
    if depth >= 0.999999 { return vec2(1.0, 0.0); }
    let uv = (vec2<f32>(pixel) + vec2(0.5)) / options.size.xy;
    let position = world(uv, depth);
    let distance = length(position - world(uv, 0.0));
    return vec2(occlusion(pixel, position), distance / 1000.0);
}

fn reconstructed_occlusion(v: Fragment, depth: f32) -> f32 {
    let position = world(v.uv, depth);
    let distance = length(position - world(v.uv, 0.0));
    let size = vec2<i32>(textureDimensions(ambient_occlusion));
    let sample = v.uv * vec2<f32>(size) - vec2(0.5);
    let base = vec2<i32>(floor(sample));
    let blend = fract(sample);
    let tolerance = max(32.0, distance * 0.003);
    var value = 0.0;
    var weight = 0.0;
    for (var y = 0; y < 2; y++) {
        for (var x = 0; x < 2; x++) {
            let corner = vec2(x, y);
            let data = textureLoad(ambient_occlusion, clamp(base + corner, vec2(0), size - vec2(1)), 0).rg;
            let edge = max(0.0, 1.0 - abs(data.y * 1000.0 - distance) / tolerance);
            let bilinear = mix(vec2(1.0) - blend, blend, vec2<f32>(corner));
            let contribution = bilinear.x * bilinear.y * edge * edge;
            value += data.x * contribution;
            weight += contribution;
        }
    }
    if weight < 0.00001 { return occlusion(vec2<i32>(v.position.xy), position); }
    return value / weight;
}

fn filmic(color: vec3<f32>) -> vec3<f32> {
    // Narkowicz's CC0 fitted curve: https://knarkowicz.wordpress.com/2016/01/06/aces-filmic-tone-mapping-curve/
    return clamp((color * (2.51 * color + vec3(0.03))) / (color * (2.43 * color + vec3(0.59)) + vec3(0.14)), vec3(0.0), vec3(1.0));
}

@fragment fn fragment(v: Fragment) -> Output {
    let pixel = vec2<i32>(v.position.xy);
    let depth = depth_at(pixel);
    var uv = v.uv;
    if HEAT && options.finish.w > 0.0 {
        let heat = textureSampleLevel(heat_map, linear_sampler, uv, 0.0);
        if heat.w > 0.0001 {
            let plume_depth = heat.z / heat.w * 1000.0;
            let shift = heat.xy / max(1.0, heat.w);
            let warped = clamp(uv + shift, vec2(0.0), vec2(1.0));
            let surface = options.inverse * vec4(uv * vec2(2.0, -2.0) + vec2(-1.0, 1.0), depth, 1.0);
            let neighbor = options.inverse * vec4(warped * vec2(2.0, -2.0) + vec2(-1.0, 1.0), depth_at(vec2<i32>(warped * options.size.xy)), 1.0);
            if 1.0 / surface.w >= plume_depth && 1.0 / neighbor.w >= plume_depth { uv = warped; }
        }
    }
    var color = textureSampleLevel(scene, linear_sampler, uv, 0.0).rgb;
    let original_color = color;
    if depth < 0.999999 && options.effects.y > 0.0 {
        var ao: f32;
        if HALF_RES_AO { ao = reconstructed_occlusion(v, depth); }
        else { ao = occlusion(pixel, world(v.uv, depth)); }
        color *= mix(ao, 1.0, clamp(max(color.r, max(color.g, color.b)) - 1.0, 0.0, 1.0));
    }
    if options.finish.z > 0.0 {
        let step = 1.0 / options.size.xy;
        let neighbors = (textureSampleLevel(scene, linear_sampler, uv + vec2(step.x, 0.0), 0.0).rgb
            + textureSampleLevel(scene, linear_sampler, uv - vec2(step.x, 0.0), 0.0).rgb
            + textureSampleLevel(scene, linear_sampler, uv + vec2(0.0, step.y), 0.0).rgb
            + textureSampleLevel(scene, linear_sampler, uv - vec2(0.0, step.y), 0.0).rgb) * 0.25;
        color = max(vec3(0.0), color + (original_color - neighbors) * options.finish.z);
    }
    if options.effects.x > 0.0 {
        color += options.effects.x * (
            textureSampleLevel(bloom0, linear_sampler, uv, 0.0).rgb * 0.35
            + textureSampleLevel(bloom1, linear_sampler, uv, 0.0).rgb * 0.3
            + textureSampleLevel(bloom2, linear_sampler, uv, 0.0).rgb * 0.2
            + textureSampleLevel(bloom3, linear_sampler, uv, 0.0).rgb * 0.15);
    }
    color *= options.color.x * vec3(1.0 + options.color.w * 0.3, 1.0, 1.0 - options.color.w * 0.3);
    if options.effects.w != 0.0 { color = filmic(color); }
    color = pow(max(color, vec3(0.0)), vec3(1.0 / (2.2 * options.finish.x)));
    let gray = dot(color, vec3(0.2126, 0.7152, 0.0722));
    color = mix(vec3(gray), color, options.color.z);
    color = (color - vec3(0.5)) * options.color.y + vec3(0.5);
    color *= 1.0 - options.finish.y * smoothstep(0.15, 0.7, length(v.uv - vec2(0.5)));
    return Output(vec4(clamp(color, vec3(0.0), vec3(1.0)), 1.0), depth);
}
