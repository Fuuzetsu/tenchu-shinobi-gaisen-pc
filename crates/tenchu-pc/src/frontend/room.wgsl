struct RoomLighting {
    direction: vec4<f32>,
    color: vec4<f32>,
    horizon: vec4<f32>,
    zenith: vec4<f32>,
    options: vec4<f32>,
    precipitation: vec4<f32>,
}
@group(0) @binding(2) var<uniform> room: RoomLighting;

fn room_window(uv: vec2<f32>) -> vec2<f32> {
    let x = (uv.x - 0.106) / 0.213;
    return vec2(x, (uv.y - 0.157 - x * 0.028) / (0.39 - x * 0.075));
}

fn room_aperture(p: vec2<f32>, softness: f32) -> f32 {
    let edge = min(min(p.x, 1.0 - p.x), min(p.y, 1.0 - p.y));
    let opening = smoothstep(-softness, softness, edge);
    if room.color.w == 0.0 { return opening; }
    let bars = min(fract(p.x * 8.0), 1.0 - fract(p.x * 8.0));
    let crossbar = min(abs(p.y - 0.33), abs(p.y - 0.74));
    return opening * smoothstep(0.06, 0.06 + softness * 4.0, bars)
        * smoothstep(0.012, 0.012 + softness, crossbar);
}

fn room_sky(p: vec2<f32>) -> vec3<f32> {
    let ray = normalize(vec3((p.x - 0.5) * 0.75, -1.0 + p.y * 1.2, 1.0));
    let height = clamp(-ray.y, 0.0, 1.0);
    var color = mix(room.horizon.rgb, room.zenith.rgb, pow(height, 0.45));
    let alignment = max(0.0, dot(ray, room.direction.xyz));
    let disc = smoothstep(cos(0.007), cos(0.0045), alignment);
    color += room.color.rgb * (disc * 2.0 + pow(alignment, 500.0) * 0.12)
        * min(1.0, room.direction.w * 12.0) * room.zenith.w;
    let overcast = mix(vec3(0.025, 0.04, 0.065), vec3(0.22, 0.28, 0.34), room.options.x);
    color = mix(color, overcast, room.horizon.w * (0.5 + 0.4 * (1.0 - height)));
    return pow(max(color, vec3(0.0)), vec3(1.0 / 2.2));
}

fn room_hash(cell: vec2<f32>) -> vec3<f32> {
    var h = fract(cell.xyx * vec3(0.1031, 0.1030, 0.0973));
    h += dot(h, h.yxz + vec3(33.33));
    return fract((h.xxy + h.yzz) * h.zyx);
}

fn room_precipitation(p: vec2<f32>, pixel: vec2<f32>) -> f32 {
    let rain = room.precipitation.x > 0.0;
    let amount = max(room.precipitation.x, room.precipitation.y);
    if amount == 0.0 { return 0.0; }
    let slope = room.precipitation.w;
    let speed = select(0.11, 1.8, rain);
    let time = room.precipitation.z;
    var coverage = 0.0;
    for (var layer = 0; layer < 3; layer++) {
        let scale = 9.0 + f32(layer) * 6.0;
        let fall = speed * (1.0 + f32(layer) * 0.2);
        var point = p - vec2(slope, 1.0) * (time * fall);
        if rain { point.x -= slope * point.y; }
        let grid = point * scale + vec2(f32(layer) * 17.7);
        let cell = floor(grid);
        let random = room_hash(cell);
        let center = vec2(0.3) + random.xy * 0.4;
        let local = fract(grid) - center;
        var distance = length(local) - mix(0.045, 0.1, random.y);
        if rain {
            distance = max(abs(local.x) - 0.014, abs(local.y) - 0.23);
        }
        let aa = max(0.005, max(pixel.x, pixel.y) * scale * (1.0 + abs(slope)));
        coverage += (1.0 - smoothstep(-aa, aa, distance))
            * step(random.z, amount * 0.7) * (0.65 - f32(layer) * 0.15);
    }
    return min(coverage, 1.0);
}

fn room_shade(uv: vec2<f32>, painted: vec3<f32>, pixel: vec2<f32>) -> vec3<f32> {
    let luminance = dot(painted, vec3(0.2126, 0.7152, 0.0722));
    let saturation = max(painted.r, max(painted.g, painted.b)) - min(painted.r, min(painted.g, painted.b));
    // Lettering is baked into all four retail paintings; icons/counts are separate draws.
    let label_area = (uv.x > 0.34 && uv.x < 0.65 && uv.y < 0.38)
        || (uv.x < 0.49 && uv.y > 0.85);
    if label_area && luminance > 0.5 && saturation < 0.13 { return painted; }

    let window = room_window(uv);
    let inside = min(min(window.x, 1.0 - window.x), min(window.y, 1.0 - window.y));
    let pane = smoothstep(-0.035, 0.01, inside) * smoothstep(0.14, 0.6, luminance);
    var outside = room_sky(window) * (0.55 + 0.45 * luminance);
    if pane > 0.0 && all(window > vec2(0.0)) && all(window < vec2(1.0)) {
        let bars = min(fract(window.x * 8.0), 1.0 - fract(window.x * 8.0));
        let crossbar = min(abs(window.y - 0.33), abs(window.y - 0.74));
        let opening = smoothstep(0.07, 0.1, bars) * smoothstep(0.014, 0.022, crossbar);
        let flakes = room_precipitation(window, pixel) * opening;
        let color = mix(vec3(0.45, 0.55, 0.7), vec3(0.85, 0.9, 1.0), room.options.x);
        outside = mix(outside, color, flakes);
    }

    // A shallow projection onto the painted back wall and chest top, not a 3D room.
    let chest = smoothstep(0.58, 0.65, uv.y) * (1.0 - smoothstep(0.38, 0.45, uv.x));
    let distance = mix(0.35, 0.22, chest);
    let direction = vec2(0.7 + room.direction.x * 0.6, -room.direction.y * 0.9 + 0.15);
    let projected = room_window(uv - direction * distance);
    let soft = 0.045 + room.options.w * 0.008 + distance * 0.08;
    let light = room_aperture(projected, soft) * max(0.0, room.direction.z) * room.direction.w;
    let receiver = smoothstep(0.28, 0.34, uv.y) * (1.0 - smoothstep(0.8, 0.88, uv.y));
    let lantern = exp(-dot((uv - vec2(0.665, 0.26)) * vec2(3.2, 2.4), (uv - vec2(0.665, 0.26)) * vec2(3.2, 2.4)));
    let ambient = 0.45 + room.options.y * 0.55;
    let gain = vec3(ambient) + room.color.rgb * light * receiver * 1.8
        + vec3(0.4, 0.22, 0.08) * lantern * room.options.z;
    let spill_offset = (uv - vec2(0.33, 0.4)) / vec2(0.12, 0.25);
    let painted_spill = exp(-dot(spill_offset, spill_offset)) * (1.0 - room.options.x) * 0.6;
    let result = painted * gain * (1.0 - painted_spill);
    return clamp(mix(result, outside, pane), vec3(0.0), vec3(1.0));
}
