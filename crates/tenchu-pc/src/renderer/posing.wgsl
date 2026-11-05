struct Vertex {
    px: f32, py: f32, pz: f32,
    nx: f32, ny: f32, nz: f32,
}
struct Rule {
    axis_start: vec4<f32>,
    delta_end: vec4<f32>,
}
struct Source {
    pose: Vertex,
    mask: u32,
    padding: u32,
}
struct Transform {
    r0: vec4<f32>, r1: vec4<f32>, r2: vec4<f32>,
    n0: vec4<f32>, n1: vec4<f32>, n2: vec4<f32>,
    rules: array<Rule, 4>,
    control: vec4<u32>,
}

@group(0) @binding(0) var<storage, read> source: array<Source>;
@group(0) @binding(1) var<storage, read> transforms: array<Transform>;
@group(0) @binding(2) var<storage, read> jobs: array<vec4<u32>>;
@group(0) @binding(3) var<storage, read_write> output_vertices: array<Vertex>;

fn normalized(value: vec3<f32>) -> vec3<f32> {
    let scale = inverseSqrt(dot(value, value));
    if scale > 0.0 && scale <= 3.402823466e+38 {
        return value * scale;
    }
    return vec3(0.0);
}

@compute @workgroup_size(128)
fn main(@builtin(workgroup_id) group: vec3<u32>, @builtin(local_invocation_index) lane: u32) {
    let job = jobs[group.x];
    if lane >= job.z { return; }
    let rest = source[job.x + lane];
    let v = rest.pose;
    let t = transforms[job.w];
    let translation = vec3(t.r0.w, t.r1.w, t.r2.w);
    var position = t.r0.xyz * v.px + t.r1.xyz * v.py + t.r2.xyz * v.pz + translation;
    var normal = normalized(t.n0.xyz * v.nx + t.n1.xyz * v.ny + t.n2.xyz * v.nz);
    if t.control.y > 0u {
        let normal_matrix = mat3x3(t.n0.xyz, t.n1.xyz, t.n2.xyz);
        let local = transpose(normal_matrix) * (position - translation);
        for (var i = 0u; i < t.control.y; i++) {
            let rule = t.rules[i];
            let delta = rule.delta_end.xyz;
            if all(delta == vec3(0.0)) || (rest.mask & (1u << i)) == 0u { continue; }
            let projection = dot(local, rule.axis_start.xyz);
            let absolute = (t.control.x & (1u << i)) != 0u;
            let distance = select(projection, abs(projection), absolute);
            let range = rule.delta_end.w - rule.axis_start.w;
            let weight = clamp((distance - rule.axis_start.w) / range, 0.0, 1.0);
            // Rust signum distinguishes negative zero.
            let direction = select(1.0, select(1.0, -1.0, (bitcast<u32>(projection) & 0x80000000u) != 0u), absolute);
            let gradient = normal_matrix * (rule.axis_start.xyz * (6.0 * weight * (1.0 - weight) / range) * direction);
            position += delta * (weight * weight * (3.0 - 2.0 * weight));
            normal -= gradient * (dot(normal, delta) / max(1.0 + dot(gradient, delta), 0.25));
        }
        normal = normalized(normal);
    }
    output_vertices[job.y + lane] = Vertex(position.x, position.y, position.z, normal.x, normal.y, normal.z);
}
