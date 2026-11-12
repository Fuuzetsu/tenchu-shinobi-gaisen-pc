use super::{Builder, Geometry, ModelStyle, Story, build, overlay, sources};
use crate::tenchu2::native::interpolation;
use crate::visuals::interpolation::Frame as PoseFrame;
use crate::{
    fonts, render_effects,
    renderer::Renderer,
    settings::Graphics,
    visuals::{
        catalog::{Region, Role},
        models::{Material, Meshes},
        pixels::Image,
    },
};
use std::{collections::BTreeMap, sync::Arc};
use tenchu_assets::Result;
use tenchu_core::math::Matrix;
use tenchu_core::tenchu2::{
    effects::shadow::{IMAGE, Shadow},
    terrain::{self, Cursor, Query},
    world::Foot,
};

pub(super) struct Models {
    pub(super) terrain: Meshes,
    lamps: crate::visuals::lights::Emitters,
    actors: BTreeMap<i16, Meshes>,
    shadow_widths: BTreeMap<i16, i16>,
    weapons: BTreeMap<String, Meshes>,
    textures: Vec<AnimatedTextures>,
    overrides: BTreeMap<i16, super::super::custom::Player>,
    equipment: BTreeMap<usize, Vec<Equipment>>,
    sprites: BTreeMap<i16, super::super::sprites::Sprites>,
}

struct Equipment {
    bone: usize,
    part: tenchu_assets::model::Part,
    meshes: Meshes,
    style: ModelStyle,
}

struct AnimatedTextures {
    owner: i16,
    revision: Option<u64>,
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
    pub(super) fn invalidate_textures(&mut self) {
        for textures in &mut self.textures {
            textures.revision = None;
        }
    }

    pub fn new(story: &Story, graphics: &Graphics, renderer: &mut Renderer) -> Result<Self> {
        renderer.update_vram(&story.sprites.vram);
        let sprites = story
            .resources
            .effects
            .iter()
            .filter_map(|(&id, sprite)| {
                let (vram, image) = sprite.as_ref()?.texture.as_ref()?;
                Some((
                    id,
                    super::super::sprites::Sprites::new(vram, std::iter::once(*image), renderer),
                ))
            })
            .collect();
        let level = &story.level;
        let resources = &story.resources;
        let mut terrain_sources = sources(
            &format!("map{}/terrain", level.id),
            &level.map,
            Role::Terrain,
        );
        terrain_sources.extend(super::super::background::sources(level));
        terrain_sources.extend(sources(
            &format!("map{}/props", level.id),
            &level.objects,
            Role::Prop,
        ));
        let (mut terrain, lamps) =
            super::super::lamps::prepare(&level.vram, terrain_sources, graphics)?;
        renderer.set_emitters(&lamps);
        terrain.detach_textures();
        renderer.bind_meshes(&mut terrain);
        let mut actors = BTreeMap::new();
        let mut shadow_widths = BTreeMap::new();
        let mut textures = Vec::new();
        for (&id, model) in &resources.models {
            if (0..106).contains(&id) && id != 0x4b {
                let bounds = tenchu_core::tenchu2::actor::model_bounds(&model.parts)?;
                let width = bounds[0].wrapping_add(bounds[2]);
                if width != 0 {
                    shadow_widths.insert(id, width);
                }
            }
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
                sources(
                    &format!("{directory}/{}", model.name),
                    &model.parts,
                    if directory == "actors" {
                        Role::Actor
                    } else {
                        Role::Other
                    },
                ),
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
                    revision: Some(vram.revision),
                    materials,
                });
            }
            actors.insert(id, mesh);
        }
        let mut weapons = BTreeMap::new();
        for (name, model) in &resources.weapons {
            let mut mesh = build(
                &model.vram,
                sources(&format!("weapons/{name}"), &model.parts, Role::Other),
                graphics,
            )?;
            renderer.bind_meshes(&mut mesh);
            weapons.insert(name.clone(), mesh);
        }
        let mut overrides = BTreeMap::new();
        for &id in &story.models.actors {
            let source = &resources.models[&id];
            let appearance = if let Some(custom) = &story.models.custom {
                crate::visuals::appearance::Appearance::custom(
                    &custom.id,
                    &custom.pack,
                    &source.vram,
                    graphics,
                )?
            } else if let Some(skin) = &story.models.skin {
                let parts = skin.parts.clone();
                let sources = parts
                    .iter()
                    .zip(&skin.sources)
                    .map(|(part, id)| crate::visuals::catalog::ModelSource {
                        id: id.clone(),
                        role: Role::Actor,
                        hero_part: crate::visuals::heroes::part(id),
                        label: String::new(),
                        geometry: &part.geometry,
                    })
                    .collect();
                let meshes = build(&skin.vram, sources, graphics)?;
                crate::visuals::appearance::Appearance::from_parts(parts, meshes)?
            } else {
                continue;
            };
            overrides.insert(
                id,
                super::super::custom::Player::from_appearance(appearance, &source.parts, renderer)?,
            );
        }
        let mut equipment = BTreeMap::new();
        for (&slot, attachments) in &story.models.equipment {
            let mut parts = Vec::new();
            for attachment in attachments {
                let part = tenchu_assets::model::Part {
                    parent: -1,
                    child: 0,
                    position: attachment.position,
                    geometry: attachment.model.geometry.clone(),
                };
                let mut meshes = build(
                    &attachment.model.vram,
                    vec![crate::visuals::catalog::ModelSource {
                        id: attachment.model.source_id.clone(),
                        role: Role::Other,
                        hero_part: None,
                        label: String::new(),
                        geometry: &part.geometry,
                    }],
                    graphics,
                )?;
                renderer.bind_meshes(&mut meshes);
                parts.push(Equipment {
                    bone: attachment.bone,
                    part,
                    meshes,
                    style: if attachment.model.source_id.starts_with("tenchu2/") {
                        ModelStyle::Actor
                    } else {
                        ModelStyle::Imported { concealed: false }
                    },
                });
            }
            equipment.insert(slot, parts);
        }
        Ok(Self {
            sprites,
            terrain,
            lamps,
            actors,
            shadow_widths,
            weapons,
            textures,
            overrides,
            equipment,
        })
    }

    pub fn geometry(
        &mut self,
        story: &Story,
        renderer: &mut Renderer,
        camera: Option<crate::debug::camera::Camera>,
        pose: Option<&PoseFrame>,
        motion: Option<&crate::visuals::motion::Frame>,
    ) -> Result<(Geometry, Vec<fonts::Text>)> {
        let resources = &story.resources;
        let level = &story.level;
        let math = &story.math;
        renderer.visual_time = f32::from(story.state.frame.max(0)) / 30.0;
        for textures in &mut self.textures {
            let vram = &resources.models[&textures.owner].vram;
            if textures.revision != Some(vram.revision) {
                textures.revision = Some(vram.revision);
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
        for (part, matrix) in super::super::static_models(level, math, None) {
            output.model(&self.terrain, part, matrix, ModelStyle::Terrain);
        }
        for object in &level.scene.dynamic {
            let Some(model) = object.model else { continue };
            output.model(
                &self.terrain,
                &level.objects[model],
                Matrix {
                    translation: object.position.into(),
                    rotation: math.rotate_yxz([0, object.facing, 0]),
                },
                ModelStyle::Prop,
            );
        }
        let mut shadow_cursor = story.state.effects.cursor.clone();
        for slot in 0..resources.script.actors.len() {
            let Some(actor) = self.actor(story, slot, pose)? else {
                continue;
            };
            actor.meshes.pose_skin(&actor.matrices, None);
            for (joint, (part, &matrix)) in actor.parts.iter().zip(&actor.matrices).enumerate() {
                let deformation = motion.and_then(|frame| {
                    actor.meshes.deformation(
                        frame,
                        actor.key(slot, joint),
                        part.geometry.as_ptr() as usize,
                        matrix,
                    )
                });
                output.deformed_model(
                    actor.meshes,
                    part,
                    matrix,
                    actor.style,
                    deformation.as_ref(),
                );
            }
            if renderer.graphics().blob_shadows
                && let Some(&width) = self.shadow_widths.get(&actor.id)
                && let Some(shadow) = actor.shadow(story, width, &mut shadow_cursor)?
                && (camera.is_some()
                    || !renderer.graphics().fog
                    || renderer.graphics().fog_distance > 100
                    || shadow.visible(story.state.view, level.draw_distance))
            {
                output.shadow(&shadow, story.sprites.effects[usize::from(IMAGE)]);
            }
            if let Some(equipment) = self.equipment.get(&slot) {
                for gear in equipment {
                    output.model(
                        &gear.meshes,
                        &gear.part,
                        actor.attachment(gear.bone).compose(&Matrix {
                            translation: gear.part.position.map(i32::from).into(),
                            ..Default::default()
                        }),
                        gear.style,
                    );
                }
            } else {
                for attachment in resources.attachments.iter().filter(|a| a.actor == slot) {
                    let weapon = &resources.weapons[&attachment.model];
                    output.parts(
                        &self.weapons[&attachment.model],
                        &weapon.parts,
                        actor.attachment(attachment.bone),
                        math,
                        false,
                    )?;
                }
            }
        }
        for (model, sprite) in &story.state.effects.sprites {
            let resource = resources.effects[model].as_ref().unwrap();
            let image = resource
                .texture
                .as_ref()
                .map_or(story.sprites.effects[15], |(_, image)| *image);
            let start = output.surfaces.len();
            output.particle_sprite(sprite, image, story.state.view, camera.is_some());
            if let Some(binding) = self.sprites.get_mut(model) {
                binding.retain(&mut output.surfaces[start..], renderer)?;
            }
        }
        output.particles(
            super::super::particles::Particles {
                scene: &tenchu_core::scene::Scene::default(),
                view: story.state.view,
                sprites: &story.sprites,
                frame: &story.state.effects.frame,
            },
            renderer.graphics(),
            1.0,
            camera.is_some(),
        )?;
        let texts = overlay::append(
            &mut output,
            resources,
            &story.state.overlay,
            renderer,
            story.event,
        );
        Ok((
            Geometry {
                lamps: super::super::lamps::sources(&self.lamps, level, math, None),
                backgrounds: super::super::background::geometry(
                    &level.background,
                    &level.shading,
                    &self.terrain,
                    view,
                    math,
                    story.state.frame.max(0) as u32,
                ),
                terrain: output.terrain,
                props: output.props,
                actors: output.actors,
                effects: render_effects::surface_geometry(output.surfaces),
            },
            texts,
        ))
    }

    fn actor<'a>(
        &'a self,
        story: &'a Story,
        slot: usize,
        pose: Option<&PoseFrame>,
    ) -> Result<Option<Actor<'a>>> {
        let animation = &story.resources.script.actors[slot];
        let Some(model) = story.resources.models.get(&animation.model) else {
            return Ok(None);
        };
        let Some(matrices) = story.matrices(slot)? else {
            return Ok(None);
        };
        let matrices = matrices
            .into_iter()
            .enumerate()
            .map(|(joint, matrix)| {
                PoseFrame::transform(
                    pose,
                    interpolation::part(slot, joint, animation.model),
                    matrix,
                )
            })
            .collect::<Vec<_>>();
        if let Some(player) = self.overrides.get(&animation.model) {
            let mut target = player.pose(
                &matrices,
                Matrix {
                    translation: model.parts[0].position.map(i32::from).into(),
                    rotation: matrices[0].rotation,
                },
            );
            if let (Some(source), Some(replacement)) = (
                pose_bottom(&self.actors[&animation.model], &model.parts, &matrices),
                pose_bottom(&player.appearance.meshes, &player.appearance.parts, &target),
            ) {
                let offset = (source - replacement).round() as i32;
                for matrix in &mut target {
                    matrix.translation.y = matrix.translation.y.wrapping_add(offset);
                }
            }
            Ok(Some(Actor {
                id: animation.model,
                parts: &player.appearance.parts,
                meshes: &player.appearance.meshes,
                matrices: target,
                original: Some(matrices),
                style: ModelStyle::Imported { concealed: false },
            }))
        } else {
            Ok(Some(Actor {
                id: animation.model,
                parts: &model.parts,
                meshes: &self.actors[&animation.model],
                matrices,
                original: None,
                style: ModelStyle::Actor,
            }))
        }
    }

    pub fn motion_inputs(
        &self,
        story: &Story,
        options: &crate::settings::Motion,
    ) -> Result<Vec<crate::visuals::motion::Input>> {
        let mut inputs = Vec::new();
        if !options.actors() {
            return Ok(inputs);
        }
        for slot in 0..story.resources.script.actors.len() {
            let Some(actor) = self.actor(story, slot, None)? else {
                continue;
            };
            if !actor.meshes.has_motion() {
                continue;
            }
            for (joint, (part, &matrix)) in actor.parts.iter().zip(&actor.matrices).enumerate() {
                let geometry = part.geometry.as_ptr() as usize;
                if let Some(rules) = actor.meshes.motion_rules(geometry) {
                    crate::visuals::motion::Input::append(
                        &mut inputs,
                        actor.key(slot, joint),
                        geometry,
                        matrix,
                        rules,
                        options,
                    );
                }
            }
        }
        Ok(inputs)
    }
}

fn pose_bottom(
    meshes: &Meshes,
    parts: &[tenchu_assets::model::Part],
    matrices: &[Matrix],
) -> Option<f32> {
    meshes.pose_skin(matrices, None);
    let mut bottom = f32::NEG_INFINITY;
    for (part, &matrix) in parts.iter().zip(matrices) {
        meshes.visit(&part.geometry, matrix, false, 0, None, |triangle| {
            for vertex in triangle {
                bottom = bottom.max(vertex.position[1]);
            }
        });
    }
    bottom.is_finite().then_some(bottom)
}

struct Actor<'a> {
    id: i16,
    parts: &'a [tenchu_assets::model::Part],
    meshes: &'a Meshes,
    matrices: Vec<Matrix>,
    original: Option<Vec<Matrix>>,
    style: ModelStyle,
}

impl Actor<'_> {
    fn shadow(&self, story: &Story, width: i16, cursor: &mut Cursor) -> Result<Option<Shadow>> {
        let joints = [
            8,
            if matches!(self.id, 0x20 | 0x21 | 0x48 | 0x49) {
                11
            } else {
                14
            },
        ];
        let mut feet = [Foot::default(); 2];
        for (foot, joint) in feet.iter_mut().zip(joints) {
            if self.original.as_ref().unwrap_or(&self.matrices).len() <= joint {
                return Err("Missing Tenchu 2 cutscene shadow joint".into());
            }
            foot.position = self.attachment(joint).translation;
            let mut probe = foot.position;
            probe.y = probe.y.wrapping_sub(500);
            foot.floor = terrain::level(&story.level.collision, cursor, probe, Query::Surface)?;
        }
        Ok(if feet.iter().all(|foot| foot.floor.is_some()) {
            Shadow::new(&feet, width, &story.math)
        } else {
            None
        })
    }

    fn key(&self, slot: usize, joint: usize) -> crate::visuals::interpolation::Part {
        let joint = if self.original.is_some() {
            tenchu_assets::tenchu2::BONES[joint]
        } else {
            joint
        };
        interpolation::part(slot, joint, self.id)
    }

    fn attachment(&self, bone: usize) -> Matrix {
        if let Some(original) = &self.original {
            super::super::custom::Player::attach(original[bone], bone, original, &self.matrices)
        } else {
            self.matrices[bone]
        }
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

#[cfg(test)]
mod tests;
