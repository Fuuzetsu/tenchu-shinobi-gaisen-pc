@group(1) @binding(9) var coarse_shadows: texture_2d_array<f32>;
fn coarse_visibility(uv: vec2<f32>, layer: u32, reference: f32, gradient: vec2<f32>, radius: f32, size: vec2<f32>) -> f32 {
    return coarse_box_visibility(uv, layer, reference, gradient, uv - vec2(radius), uv + vec2(radius), size);
}
fn coarse_box_visibility(uv: vec2<f32>, layer: u32, reference: f32, gradient: vec2<f32>, low_bound: vec2<f32>, high_bound: vec2<f32>, size: vec2<f32>) -> f32 {
    if lighting.lamps.w == 0.0 { return -1.0; }
    let edge = vec2(shadow_scale(layer));
    let low_uv = clamp(low_bound, vec2(0.5) / size, edge - vec2(0.5) / size);
    let high_uv = clamp(high_bound, vec2(0.5) / size, edge - vec2(0.5) / size);
    // Include both texels of every bilinear comparison and the receiver depth gradient.
    let low = vec2<u32>(max(floor(low_uv * size - vec2(0.5)),vec2(0.0))) / SHADOW_BOUND_TILE;
    let high = vec2<u32>(min(floor(high_uv * size - vec2(0.5)) + vec2(1.0), size - vec2(1.0))) / SHADOW_BOUND_TILE;
    let extent = high - low;
    let mip = 32u - countLeadingZeros(max(extent.x,extent.y));
    let a = vec2<i32>(low >> vec2(mip));
    let b = vec2<i32>(high >> vec2(mip));
    let aa = textureLoad(coarse_shadows,a,i32(layer),i32(mip)).rg;
    let ab = textureLoad(coarse_shadows,vec2(b.x,a.y),i32(layer),i32(mip)).rg;
    let ba = textureLoad(coarse_shadows,vec2(a.x,b.y),i32(layer),i32(mip)).rg;
    let bb = textureLoad(coarse_shadows,b,i32(layer),i32(mip)).rg;
    let bounds = vec2(min(min(aa.x,ab.x),min(ba.x,bb.x)),max(max(aa.y,ab.y),max(ba.y,bb.y)));
    let near = min(gradient * (low_uv - uv), gradient * (high_uv - uv));
    let far = max(gradient * (low_uv - uv), gradient * (high_uv - uv));
    if reference + far.x + far.y < bounds.x - 0.000002 { return 1.0; }
    if reference + near.x + near.y > bounds.y + 0.000002 { return 0.0; }
    return -1.0;
}
