// GLSL cannot load depth texels, so the same shadow pass also writes an R32F view.
@group(1) @binding(5) var point_depths: texture_2d_array<f32>;
@group(1) @binding(6) var shelter_depths: texture_2d_array<f32>;
fn point_depth(pixel: vec2<i32>, layer: i32, radius: f32) -> f32 {
    return textureLoad(point_depths, pixel, layer, 0).r;
}
fn shelter_depth(pixel: vec2<i32>, layer: i32) -> f32 {
    return textureLoad(shelter_depths, pixel, layer, 0).r;
}
