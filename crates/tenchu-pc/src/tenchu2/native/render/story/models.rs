use super::{Builder, Geometry, ModelStyle, Story, build, overlay, sources};
use crate::{
    fonts, render_effects,
    renderer::Renderer,
    settings::Graphics,
    visuals::{
        catalog::Region,
        models::{Material, Meshes},
        pixels::Image,
    },
};
use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::{Result, tenchu2::level::objects::StaticKind};
use tenchu_core::{math::Matrix, motion::Pose};

pub(super) struct Models {
    pub(super) terrain: Meshes,
    actors: BTreeMap<i16, Meshes>,
    weapons: BTreeMap<String, Meshes>,
    textures: Vec<AnimatedTextures>,
}

struct AnimatedTextures {
    owner: i16,
    revision: u64,
    materials: Vec<AnimatedMaterial>,
}

struct AnimatedMaterial {
    region: Region,
    material: Material,
    base: Arc<Image>,
    original: Vec<u8>,
    previous: Vec<u8>,
}

impl Models {
    pub fn new(story: &Story, graphics: &Graphics, renderer: &mut Renderer) -> Result<Self> {
        let level = &story.level;
        let resources = &story.resources;
        let mut terrain_sources = sources(&format!("map{}/terrain", level.id), &level.map);
        terrain_sources.extend(super::super::background::sources(level));
        terrain_sources.extend(sources(&format!("map{}/props", level.id), &level.objects));
        let mut terrain = build(&level.vram, terrain_sources, graphics)?;
        renderer.bind_meshes(&mut terrain);
        let mut actors = BTreeMap::new();
        let mut textures = Vec::new();
        for (&id, model) in &resources.models {
            let directory = match id {
                0..=105 | 290..=293 => "actors",
                106..=231 => "weapons",
                _ => "event",
            };
            let owner = resources
                .script
                .actors
                .iter()
                .find(|a| a.model == id && id > 290)
                .map_or(id, |a| a.texture_model);
            let vram = &resources.models[&owner].vram;
            let mut mesh = super::super::prepare(
                vram,
                sources(&format!("{directory}/{}", model.name), &model.parts),
                graphics,
            )?;
            mesh.skin(&model.parts);
            let regions: Vec<_> = mesh.materials().iter().map(|m| m.region).collect();
            mesh.detach_textures();
            let mut materials = Vec::new();
            let mut regions = regions.into_iter().enumerate();
            mesh.bind(|material| {
                let (slot, region) = regions.next().unwrap();
                let Some(region) = region.filter(|region| {
                    resources
                        .faces
                        .get(&owner)
                        .is_some_and(|patches| patches.iter().any(|p| overlaps(*region, p.rect())))
                }) else {
                    return renderer.bind_material(material);
                };
                let original = region.pixels(vram);
                let mut material = material.clone();
                material.id = format!("tenchu2/event/{}/actor/{id}/{slot}", story.event);
                let handle = renderer.bind_material(&material);
                materials.push(AnimatedMaterial {
                    region,
                    base: material.image.clone(),
                    material,
                    previous: original.clone(),
                    original,
                });
                handle
            });
            if !materials.is_empty() {
                textures.push(AnimatedTextures {
                    owner,
                    revision: vram.revision,
                    materials,
                });
            }
            actors.insert(id, mesh);
        }
        let mut weapons = BTreeMap::new();
        for (name, model) in &resources.weapons {
            let mut mesh = build(
                &model.vram,
                sources(&format!("weapons/{name}"), &model.parts),
                graphics,
            )?;
            renderer.bind_meshes(&mut mesh);
            weapons.insert(name.clone(), mesh);
        }
        Ok(Self {
            terrain,
            actors,
            weapons,
            textures,
        })
    }

    pub fn geometry(
        &mut self,
        story: &Story,
        renderer: &mut Renderer,
        camera: Option<crate::debug::camera::Camera>,
    ) -> Result<(Geometry, Vec<fonts::Text>)> {
        let resources = &story.resources;
        let level = &story.level;
        let math = &story.math;
        renderer.visual_time = f32::from(story.state.frame.max(0)) / 30.0;
        for textures in &mut self.textures {
            let vram = &resources.models[&textures.owner].vram;
            if textures.revision != vram.revision {
                textures.revision = vram.revision;
                for material in &mut textures.materials {
                    material.update(material.region.pixels(vram), renderer);
                }
            }
        }
        let view = camera.map_or(story.state.view, |c| c.source_view());
        let mut output = Builder::new(view, level, renderer.graphics());
        if let Some(camera) = camera {
            output.distance = 120.0 / (camera.fov.to_radians() * 0.5).tan();
        }
        for part in &level.map {
            output.model(
                &self.terrain,
                part,
                Matrix {
                    translation: part.position.map(|v| i32::from(v) * 10).into(),
                    ..Default::default()
                },
                ModelStyle::Terrain,
            );
        }
        for object in &level.scene.statics {
            if let StaticKind::Model(index) = object.kind {
                output.model(
                    &self.terrain,
                    &level.objects[index],
                    Matrix {
                        translation: object.position.into(),
                        rotation: math.rotate_yxz(object.rotation),
                    },
                    ModelStyle::Terrain,
                );
            }
        }
        for object in &level.scene.dynamic {
            output.model(
                &self.terrain,
                &level.objects[object.model],
                Matrix {
                    translation: object.position.into(),
                    rotation: math.rotate_yxz([0, object.facing, 0]),
                },
                ModelStyle::Terrain,
            );
        }
        for (slot, (motion, animation)) in story
            .state
            .actors
            .iter()
            .zip(&resources.script.actors)
            .enumerate()
        {
            let Some(model) = resources.models.get(&animation.model) else {
                continue;
            };
            let mut pose = Pose::new(&model.parts)?;
            pose.translation[0] = motion.position;
            for (rotation, &value) in pose.rotation.iter_mut().zip(&motion.rotation) {
                *rotation = value;
            }
            let matrices = pose.matrices(math, Matrix::default());
            let mesh = &self.actors[&animation.model];
            mesh.pose_skin(&matrices, None);
            for (part, &matrix) in model.parts.iter().zip(&matrices) {
                output.model(mesh, part, matrix, ModelStyle::Actor);
            }
            for attachment in resources.attachments.iter().filter(|a| a.actor == slot) {
                let weapon = &resources.weapons[&attachment.model];
                output.parts(
                    &self.weapons[&attachment.model],
                    &weapon.parts,
                    matrices[attachment.bone],
                    math,
                )?;
            }
        }
        let texts = overlay::append(
            &mut output,
            resources,
            &story.state.overlay,
            renderer,
            story.event,
        );
        Ok((
            Geometry {
                backgrounds: super::super::background::geometry(
                    &level.background,
                    &level.shading,
                    &self.terrain,
                    view,
                    math,
                    story.state.frame.max(0) as u32,
                ),
                terrain: output.terrain,
                actors: output.actors,
                effects: render_effects::surface_geometry(output.surfaces),
            },
            texts,
        ))
    }
}

fn overlaps(region: Region, patch: [u16; 4]) -> bool {
    let [x, y, w, h] = tenchu_assets::texture::Image {
        page: region.page,
        palette: region.palette,
        uv: [region.rect[0] as u8, region.rect[1] as u8],
        size: [region.rect[2], region.rect[3]],
    }
    .word_rect()
    .map(u32::from);
    let [px, py, pw, ph] = patch.map(u32::from);
    x < px + pw && px < x + w && y < py + ph && py < y + h
}

impl AnimatedMaterial {
    fn update(&mut self, pixels: Vec<u8>, renderer: &mut Renderer) {
        if self.previous == pixels {
            return;
        }
        let mut image = (*self.base).clone();
        let [_, _, width, height] = self.region.rect.map(usize::from);
        let [w, h] = image.size.map(|v| v as usize);
        for y in 0..h {
            for x in 0..w {
                let source = (y * height / h * width + x * width / w) * 4;
                if pixels[source..source + 4] != self.original[source..source + 4] {
                    let at = (y * w + x) * 4;
                    image.rgba[at..at + 4].copy_from_slice(&pixels[source..source + 4]);
                }
            }
        }
        self.previous = pixels;
        self.material.image = Arc::new(image);
        renderer.bind_material(&self.material);
    }
}
