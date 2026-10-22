struct Material {
    source: vec4<f32>,
    info: vec4<u32>,
    emission: vec4<f32>,
    relief: vec4<f32>,
    mips: array<vec4<f32>, MAX_MIPS>,
}
@group(2) @binding(2) var<storage, read> materials: array<Material>;
fn material_count() -> u32 { return arrayLength(&materials); }
fn material_source(index: u32) -> vec4<f32> { return materials[index].source; }
fn material_info(index: u32) -> vec4<u32> { return materials[index].info; }
fn material_emissive(index: u32) -> vec4<f32> { return materials[index].emission; }
fn material_relief(index: u32) -> vec4<f32> { return materials[index].relief; }
fn material_rect(index: u32, level: u32) -> vec4<f32> { return materials[index].mips[level]; }
