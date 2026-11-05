struct Mask { @builtin(position) position: vec4<f32>, @location(0) uv: vec2<f32>, @location(1) opacity: f32 }
@vertex fn vertex(@location(0) position: vec2<f32>, @location(1) uv: vec2<f32>, @location(2) opacity: f32) -> Mask {
    return Mask(vec4(position * vec2(2.0, -2.0) + vec2(-1.0, 1.0), 0.0, 1.0), uv, opacity);
}
@fragment fn fragment(input: Mask) -> @location(0) vec4<f32> {
    let opacity = clamp(blood_mask(input.uv) * input.opacity, 0.0, 1.0);
    return vec4(opacity, 0.0, 0.0, opacity);
}
