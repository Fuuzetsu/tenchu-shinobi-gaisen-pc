fn weapon_finish(v: Fragment, input: vec3<f32>, normal: vec3<f32>, surface: vec3<f32>, linear: bool) -> vec3<f32> {
    if (v.flags & (WEAPON_METAL | FOLIAGE | SCENERY)) != WEAPON_METAL || camera.motion.y <= 0.0 { return input; }
    let view = normalize(camera.eye.xyz - v.world);
    let ray = reflect(-view, normal);
    var sky = sky_color(ray).rgb / max(camera.options.x, 0.001);
    if camera.viewport.z == 0.0 { sky = pow(max(sky, vec3(0.0)), vec3(2.2)); }
    let fresnel = 0.35 + 0.65 * pow(1.0 - max(dot(normal, view), 0.0), 5.0);
    var reflected = sky * fresnel * 0.25 + surface_reflection(v.world, normal, surface, 0.45) * 0.75;
    if !linear { reflected = pow(max(reflected, vec3(0.0)), vec3(1.0 / 2.2)) * 0.5; }
    let strength = camera.motion.y * select(1.0, 0.18, (v.flags & WEAPON_COAT) != 0u);
    return input * (1.0 - min(strength, 1.0) * 0.15) + reflected * strength;
}

fn weapon_blood(v: Fragment) -> f32 {
    let uv = v.uv;
    let amount = v.opacity;
    if amount <= 0.0 { return 0.0; }
    let broad = sin(uv.y * 3.7 + sin(uv.x * 2.1)) * 0.22;
    let streak = sin(uv.x * 8.0 + sin(uv.y * 0.9)) * 0.14;
    let speckle = sin(uv.x * 19.0 + uv.y * 13.0) * sin(uv.y * 21.0) * 0.06;
    let base = smoothstep(0.48 + broad + streak + speckle - 0.08,
                         0.48 + broad + streak + speckle + 0.08, amount) * 0.94;
    let wet = bitcast<f32>(v.material);
    if wet <= 0.0 { return base; }
    let moving = uv - vec2(bitcast<f32>(v.page), bitcast<f32>(v.palette));
    let strands = smoothstep(0.4, 0.9, sin(moving.x * 8.0 + sin(moving.y * 1.7) * 0.4))
        * smoothstep(-0.5, 0.8, sin(moving.y * 4.1 + sin(moving.x) * 0.5));
    return mix(base, max(base * 0.78, strands * min(amount * 1.4, 0.97)), wet);
}
