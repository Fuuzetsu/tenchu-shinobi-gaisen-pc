use super::catalog;
use serde::Serialize;
use std::{
    collections::{BTreeMap, BTreeSet},
    fmt::Write,
    fs,
    io::BufWriter,
    path::Path,
};
use tenchu_assets::{Result, stage::MissionAssets};

pub mod interface;
#[cfg(test)]
mod trial;
mod usage;

#[derive(Serialize)]
struct Model {
    id: String,
    label: String,
    file: String,
    vertices: usize,
    faces: usize,
}

#[derive(Serialize)]
struct Manifest {
    version: u32,
    stage: String,
    units: &'static str,
    alpha: &'static str,
    textures: Vec<String>,
    texture_details: BTreeMap<String, usage::Detail>,
    mission_usage: serde_json::Value,
    models: Vec<Model>,
}

pub fn export(mission: &MissionAssets, directory: &Path) -> Result<()> {
    let manifest_path = directory.join("catalog.json");
    if directory.exists()
        && fs::read_dir(directory)
            .map_err(|e| e.to_string())?
            .next()
            .is_some()
    {
        return Err(format!(
            "{} is not empty; export to a new directory to preserve edits",
            directory.display()
        ));
    }
    fs::create_dir_all(directory.join("textures")).map_err(|e| e.to_string())?;
    let mut regions = catalog::regions(mission);
    for &image in &mission.images {
        let region = catalog::Region::sprite(&mission.vram, image);
        regions.entry(region).or_insert_with(|| {
            catalog::image_id(
                [region.rect[2], region.rect[3]],
                &region.original_pixels(&mission.vram),
            )
        });
    }
    let mut textures = BTreeSet::new();
    for (&region, id) in &regions {
        if !textures.insert(id.clone()) {
            continue;
        }
        let file = fs::File::create(directory.join("textures").join(format!("{id}.png")))
            .map_err(|e| e.to_string())?;
        let mut encoder = png::Encoder::new(
            BufWriter::new(file),
            u32::from(region.rect[2]),
            u32::from(region.rect[3]),
        );
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder
            .write_header()
            .map_err(|e| e.to_string())?
            .write_image_data(&region.original_pixels(&mission.vram))
            .map_err(|e| e.to_string())?;
    }
    let mut definitions = BTreeMap::new();
    let mut models = Vec::new();
    for source in catalog::models(mission) {
        let file = format!("models/{}.obj", source.id);
        let path = directory.join(&file);
        fs::create_dir_all(path.parent().unwrap()).map_err(|e| e.to_string())?;
        let depth = Path::new(&file).components().count() - 1;
        let mut obj = format!(
            "# {}\nmtllib {}materials.mtl\n",
            source.label,
            "../".repeat(depth)
        );
        let mut vertex = 1;
        let mut faces = 0;
        for geometry in source.geometry {
            for face in crate::render_mesh::draw_tmd_faces(geometry) {
                let [a, b, c] = std::array::from_fn(|i| {
                    glam::Vec3::from(
                        geometry.vertices[face.corners[i].vertex as usize].map(f32::from),
                    )
                });
                if (b - a).cross(c - a).length_squared() == 0.0 {
                    continue;
                }
                let region = catalog::face_region(&mission.vram, face);
                let definition = super::pack::Material {
                    texture: region.map(|r| regions[&r].clone()),
                    blend: if face.translucent {
                        face.texture.map_or(1, |t| ((t.page >> 5) & 3) as u8 + 1)
                    } else {
                        0
                    },
                    double_sided: face.double_sided,
                };
                let material = format!(
                    "{}_b{}_d{}",
                    definition
                        .texture
                        .as_ref()
                        .map_or_else(|| "flat".into(), |id| format!("tex_{id}")),
                    definition.blend,
                    u8::from(definition.double_sided)
                );
                definitions.insert(material.clone(), definition);
                writeln!(obj, "usemtl {material}").unwrap();
                for corner in &face.corners {
                    let [x, y, z] = geometry.vertices[corner.vertex as usize];
                    let [r, g, b] = corner
                        .color
                        .map(|c| f32::from(c) / if region.is_some() { 128.0 } else { 255.0 });
                    writeln!(obj, "v {x} {y} {z} {r:.6} {g:.6} {b:.6}").unwrap();
                    let [u, v] = region.map_or([0.0, 0.0], |r| {
                        [
                            (f32::from(corner.uv[0]) - f32::from(r.rect[0]) + 0.5)
                                / f32::from(r.rect[2]),
                            1.0 - (f32::from(corner.uv[1]) - f32::from(r.rect[1]) + 0.5)
                                / f32::from(r.rect[3]),
                        ]
                    });
                    writeln!(obj, "vt {u:.8} {v:.8}").unwrap();
                }
                let order: &[usize] = if face.corners.len() == 4 {
                    &[0, 1, 2, 1, 3, 2]
                } else {
                    &[0, 1, 2]
                };
                for triangle in order.chunks_exact(3) {
                    writeln!(
                        obj,
                        "f {0}/{0} {1}/{1} {2}/{2}",
                        vertex + triangle[0],
                        vertex + triangle[1],
                        vertex + triangle[2]
                    )
                    .unwrap();
                    faces += 1;
                }
                vertex += face.corners.len();
            }
        }
        fs::write(path, obj).map_err(|e| e.to_string())?;
        models.push(Model {
            id: source.id,
            label: source.label,
            file,
            vertices: vertex - 1,
            faces,
        });
    }
    let mut materials = String::new();
    for (name, definition) in &definitions {
        writeln!(materials, "newmtl {name}\nKd 1 1 1").unwrap();
        if let Some(id) = &definition.texture {
            writeln!(materials, "map_Kd textures/{id}.png").unwrap();
        }
        materials.push('\n');
    }
    fs::write(directory.join("materials.mtl"), materials).map_err(|e| e.to_string())?;
    let manifest = Manifest {
        version: 1,
        stage: mission.stage.directory.clone(),
        units: "Original game units; negative Y is up. Actor parts are local to the existing animation rig.",
        alpha: "PNG alpha 0 = hole, 128 = PS1 semi-transparency flag, 255 = opaque. Blend mode is still selected by the face.",
        textures: textures.into_iter().collect(),
        texture_details: usage::collect(mission, &regions)?,
        mission_usage: usage::mission(mission),
        models,
    };
    fs::write(
        &manifest_path,
        serde_json::to_vec_pretty(&manifest).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    let pack = super::pack::Manifest {
        version: 1,
        recipes: BTreeMap::new(),
        surfaces: BTreeMap::new(),
        water: super::water::rules(&Default::default(), false),
        vegetation: super::vegetation::rules(&Default::default(), false),
        motion: super::motion::rules(&Default::default(), false),
        textures: manifest
            .textures
            .iter()
            .map(|id| (id.clone(), format!("textures/{id}.png")))
            .collect(),
        models: BTreeMap::new(),
        model_patches: BTreeMap::new(),
        materials: definitions,
        lights: Some(super::lights::defaults()),
        player_models: super::appearance::exported(mission),
    };
    fs::write(
        directory.join("pack.json"),
        serde_json::to_vec_pretty(&pack).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    println!(
        "Exported {} textures and {} models to {}",
        manifest.textures.len(),
        manifest.models.len(),
        directory.display()
    );
    Ok(())
}
