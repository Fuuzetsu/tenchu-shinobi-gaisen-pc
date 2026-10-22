@group(2) @binding(2) var material_data: texture_2d<u32>;
fn material_count() -> u32 { return textureDimensions(material_data).y; }
fn material_source(index: u32) -> vec4<f32> { return bitcast<vec4<f32>>(textureLoad(material_data, vec2(0, i32(index)), 0)); }
fn material_info(index: u32) -> vec4<u32> { return textureLoad(material_data, vec2(1, i32(index)), 0); }
fn material_emissive(index: u32) -> vec4<f32> { return bitcast<vec4<f32>>(textureLoad(material_data, vec2(2, i32(index)), 0)); }
fn material_relief(index: u32) -> vec4<f32> { return bitcast<vec4<f32>>(textureLoad(material_data, vec2(3, i32(index)), 0)); }
fn material_rect(index: u32, level: u32) -> vec4<f32> { return bitcast<vec4<f32>>(textureLoad(material_data, vec2(i32(level + 4u), i32(index)), 0)); }
