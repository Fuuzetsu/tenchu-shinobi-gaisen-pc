fn blood_mask(p: vec2<f32>) -> f32 {
    let ripple = sin(p.x * 11.0 + sin(p.y * 9.0)) * sin(p.y * 13.0 - p.x * 3.0);
    let radius = length(p) + ripple * 0.045;
    return 1.0 - smoothstep(0.52, 0.82, radius);
}

