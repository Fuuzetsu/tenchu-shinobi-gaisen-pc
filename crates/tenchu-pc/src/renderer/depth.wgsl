fn point_depth(pixel: vec2<i32>, layer: i32, radius: f32) -> f32 {
    return textureLoad(shadow_maps, pixel, layer, 0);
}
fn shelter_depth(pixel: vec2<i32>, layer: i32) -> f32 {
    return textureLoad(weather_map, pixel, layer, 0);
}
