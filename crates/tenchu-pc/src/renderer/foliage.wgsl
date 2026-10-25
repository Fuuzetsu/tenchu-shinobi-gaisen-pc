const FOLIAGE: u32 = 262144u;
const FLUTTER: u32 = 524288u;

fn foliage_position(position: vec3<f32>, flags: u32, time: f32, strength: f32) -> vec3<f32> {
    if strength == 0.0 || (flags & FOLIAGE) == 0u { return position; }
    let weight = f32(flags >> 24u) / 255.0;
    let phase = dot(position.xz, vec2(0.0003, 0.0004));
    let gust = vec2(sin(time * 0.9 + phase), cos(time * 0.67 + phase));
    let leaf = (flags & FLUTTER) != 0u;
    let branch_phase = dot(position, vec3(0.0013, 0.0018, 0.0009));
    let branch = select(vec2(0.0), vec2(sin(time * 1.8 + branch_phase), cos(time * 1.3 + branch_phase * 1.2)) * 32.0, leaf);
    let flutter = select(0.0, sin(time * 5.5 + dot(position, vec3(0.007, 0.008, 0.013))) * sin(time * 2.7 + phase), leaf);
    let offset = ((gust * 45.0 + branch) * weight * weight + vec2(flutter, flutter * 0.7) * 14.0 * weight) * strength;
    return position + vec3(offset.x, 0.0, offset.y);
}
