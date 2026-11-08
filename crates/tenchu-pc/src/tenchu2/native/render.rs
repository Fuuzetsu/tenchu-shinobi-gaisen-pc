use crate::{
    render_effects::{self, Surface},
    render_mesh::{self, Vertex},
    renderer::Renderer,
    settings::Graphics,
    visuals::{catalog, models::Meshes, pack::Pack},
};
use std::collections::BTreeMap;
use tenchu_assets::{
    Result,
    model::Part,
    texture::{Image, Vram},
};
use tenchu_core::{
    math::{Matrix, Position},
    tenchu2::{
        effects::surface::Blend,
        world::{Assets, Frame, World},
    },
};

pub struct Drawing {
    models: Models,
    terrain: crate::renderer::Mesh,
    actors: crate::renderer::Mesh,
    effects: crate::renderer::EffectMesh,
}

impl Drawing {
    pub fn new(
        session: &super::Session,
        graphics: &Graphics,
        renderer: &mut Renderer,
    ) -> Result<Self> {
        let models = Models::new(&session.assets, &session.world, graphics, renderer)?;
        renderer.visual_time = session.world.tick as f32 / 30.0;
        let geometry = models.geometry(&session.world, &session.assets, &session.frame)?;
        Ok(Self {
            models,
            terrain: renderer.upload(&geometry.terrain),
            actors: renderer.upload(&geometry.actors),
            effects: renderer.upload_effects(geometry.effects),
        })
    }

    pub fn update(&mut self, session: &super::Session, renderer: &mut Renderer) -> Result<()> {
        let mut geometry = self
            .models
            .geometry(&session.world, &session.assets, &session.frame)?;
        renderer.update_mesh(&mut self.terrain, &mut geometry.terrain);
        renderer.update_mesh(&mut self.actors, &mut geometry.actors);
        renderer.update_effects(&mut self.effects, geometry.effects);
        renderer.visual_time = session.world.tick as f32 / 30.0;
        Ok(())
    }

    pub fn draw(
        &self,
        view: Matrix,
        target: &wgpu::TextureView,
        renderer: &mut Renderer,
    ) -> Result<()> {
        let [w, h] = renderer.size();
        let camera = glam::Mat4::perspective_rh(
            2.0 * (120.0f32 / 300.0).atan(),
            w as f32 / h as f32,
            50.0,
            250_000.0,
        ) * crate::renderer::view_matrix(view);
        renderer.draw_lit(
            target,
            camera,
            crate::renderer::SceneMeshes::new(&[&self.terrain], &[&self.actors]),
            Some(&self.effects),
            &[],
            &[],
        )
    }
}

pub fn capture(
    session: &super::Session,
    graphics: &Graphics,
    path: &std::path::Path,
) -> Result<()> {
    let instance = wgpu::Instance::new(&Default::default());
    let adapter = pollster::block_on(instance.request_adapter(&Default::default()))
        .ok_or("No graphics adapter")?;
    let format = wgpu::TextureFormat::Rgba8Unorm;
    let mut renderer = pollster::block_on(Renderer::new(
        &adapter,
        format,
        [1280, 720],
        &session.assets.sprites.vram,
    ))?;
    renderer.configure(graphics);
    let drawing = Drawing::new(session, graphics, &mut renderer)?;
    let texture = renderer.device.create_texture(&wgpu::TextureDescriptor {
        label: Some("Native JP capture"),
        size: wgpu::Extent3d {
            width: 1280,
            height: 720,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: wgpu::TextureDimension::D2,
        format,
        usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::COPY_SRC,
        view_formats: &[],
    });
    drawing.draw(
        session.world.camera.view,
        &texture.create_view(&Default::default()),
        &mut renderer,
    )?;
    eprintln!(
        "Native JP tick {}: position {:?}; {} actors; radar {:?}; pending {:?}",
        session.world.tick,
        session.world.actors[session.world.player].actor.position,
        session.world.actors.len(),
        session.world.ai.global.radar,
        session.pending
    );
    crate::capture::read_texture(&renderer, &texture, path)
}

pub struct Models {
    scenery: Meshes,
    bodies: BTreeMap<String, Meshes>,
    weapons: Meshes,
    items: Meshes,
    weapon_names: BTreeMap<usize, String>,
}

pub struct Geometry {
    pub terrain: Vec<Vertex>,
    pub actors: Vec<Vertex>,
    pub effects: render_effects::Geometry,
}

fn sources<'a>(name: &str, parts: &'a [Part]) -> Vec<catalog::ModelSource<'a>> {
    parts
        .iter()
        .enumerate()
        .map(|(index, part)| catalog::ModelSource {
            id: format!("tenchu2/{name}/{index}"),
            label: name.into(),
            geometry: &part.geometry,
        })
        .collect()
}

fn build(
    vram: &Vram,
    sources: Vec<catalog::ModelSource<'_>>,
    graphics: &Graphics,
) -> Result<Meshes> {
    let pack = if graphics.asset_pack {
        Pack::for_models(vram, &sources)?
    } else {
        Pack::default()
    };
    let mut meshes = Meshes::build(vram, &pack, graphics, sources, true)?;
    meshes.detach_textures();
    if graphics.asset_pack {
        meshes.replace_textures(&pack);
    }
    Ok(meshes)
}

impl Models {
    pub fn new(
        assets: &Assets,
        world: &World,
        graphics: &Graphics,
        renderer: &mut Renderer,
    ) -> Result<Self> {
        let mut scenery = sources(&format!("map{}/terrain", world.map), &assets.level.map);
        scenery.extend(sources(
            &format!("map{}/props", world.map),
            &assets.level.objects,
        ));
        for (id, part) in &assets.level.stumps {
            scenery.extend(sources(
                &format!("map{}/stump{id}", world.map),
                std::slice::from_ref(part),
            ));
        }
        let mut scenery = build(&assets.level.vram, scenery, graphics)?;
        renderer.bind_meshes(&mut scenery);
        let mut bodies = BTreeMap::new();
        for (name, model) in &assets.models {
            let mut meshes = build(
                &model.vram,
                sources(&format!("actors/{name}"), &model.parts),
                graphics,
            )?;
            meshes.skin(&model.parts);
            renderer.bind_meshes(&mut meshes);
            bodies.insert(name.clone(), meshes);
        }
        let weapon_sources = assets
            .weapons
            .iter()
            .flat_map(|(name, parts)| sources(&format!("weapons/{name}"), parts))
            .collect();
        let mut weapons = build(&assets.weapon_vram, weapon_sources, graphics)?;
        renderer.bind_meshes(&mut weapons);
        let item_sources = assets
            .items
            .models()
            .flat_map(|(id, parts)| sources(&format!("items/{id}"), parts))
            .collect();
        let mut items = build(&assets.items.vram, item_sources, graphics)?;
        renderer.bind_meshes(&mut items);
        let mut weapon_names = BTreeMap::new();
        for (agent, instance) in world.actors.iter().zip(&world.instances) {
            let definition = assets
                .resources
                .registry
                .actor(agent.actor.kind)
                .ok_or("Missing native actor")?;
            let mut roots: Vec<_> = instance.rig.weapons.iter().flatten().copied().collect();
            roots.sort_unstable();
            weapon_names.extend(
                roots.into_iter().zip(
                    assets
                        .resources
                        .weapons
                        .parts(definition.weapon)
                        .filter_map(|part| part.model.clone()),
                ),
            );
        }
        Ok(Self {
            scenery,
            bodies,
            weapons,
            items,
            weapon_names,
        })
    }

    pub fn geometry(&self, world: &World, assets: &Assets, frame: &Frame) -> Result<Geometry> {
        let mut output = Builder {
            view: world.camera.view,
            surfaces: Vec::new(),
            terrain: Vec::new(),
            actors: Vec::new(),
        };
        let math = &assets.resources.math;
        for part in &assets.level.map {
            output.model(
                &self.scenery,
                part,
                Matrix {
                    translation: part.position.map(|v| i32::from(v) * 10).into(),
                    ..Default::default()
                },
                true,
            );
        }
        for model in &frame.scenery.models {
            let part = if model.stump {
                assets.level.stumps.get(&model.model)
            } else {
                assets.level.objects.get(model.model)
            }
            .ok_or("Missing native scenery model")?;
            output.model(
                &self.scenery,
                part,
                Matrix {
                    translation: model.position,
                    rotation: math.rotate_yxz(model.rotation),
                },
                true,
            );
        }
        // Presentation must not refresh the simulation's cached coordinate matrices.
        let mut scene = world.scene.clone();
        for (agent, instance) in world.actors.iter().zip(&world.instances) {
            let actor = &agent.actor;
            if actor.flags & 8 != 0 {
                continue;
            }
            let definition = assets
                .resources
                .registry
                .actor(actor.kind)
                .ok_or("Missing native actor definition")?;
            let body = &assets.models[&definition.model];
            let meshes = &self.bodies[&definition.model];
            let matrices = instance
                .rig
                .joints
                .iter()
                .map(|&joint| scene.world(joint))
                .collect::<Result<Vec<_>>>()?;
            meshes.pose_skin(&matrices, None);
            for (part, &matrix) in body.parts.iter().zip(&matrices) {
                output.model(meshes, part, matrix, false);
            }
            for root in instance.rig.weapons.iter().take(3).flatten() {
                let name = self
                    .weapon_names
                    .get(root)
                    .ok_or("Missing native weapon instance")?;
                let parts = &assets.weapons[name];
                for (index, part) in parts.iter().enumerate() {
                    output.model(&self.weapons, part, scene.world(root + 1 + index)?, false);
                }
            }
        }
        if let Some(hook) = world.items.grapple() {
            if let Some(parts) = assets.items.model(0) {
                output.parts(&self.items, parts, hook.transform, math)?;
            }
            let hand = *world.instances[hook.owner]
                .rig
                .joints
                .get(11)
                .ok_or("Missing native grapple hand")?;
            let mut previous = scene.world(hand)?.translation;
            for &point in &hook.rope {
                output.line(previous, point);
                previous = point;
            }
        }
        if let Some(item) = world.items.held
            && let Some(parts) = assets.items.model(item)
        {
            let hand = *world.instances[world.player]
                .rig
                .joints
                .get(11)
                .ok_or("Missing native held-item hand")?;
            output.parts(&self.items, parts, scene.world(hand)?, math)?;
        }
        for entry in &world.items.entries {
            use tenchu_core::tenchu2::items::{State, arrow::Phase};
            let (model, transform, rotation) = match &entry.state {
                State::Projectile(p) => (p.model(), p.transform, p.rotation),
                State::Grenade(g) if g.visible => (6, g.transform, [0; 3]),
                State::Mine(m) => (7, m.transform, [0; 3]),
                State::Smoke(s) => (10, s.transform, [0; 3]),
                State::Arrow(arrow) => {
                    if arrow.flame {
                        let image =
                            assets.sprites.effects[12 + (world.tick.wrapping_sub(1) & 3) as usize];
                        let projected = output.project(arrow.flight.position);
                        if projected[2] > 0.0 {
                            let scale = 2400.0 / projected[2];
                            output.screen_sprite(
                                projected,
                                image.size.map(f32::from),
                                [scale; 2],
                                0,
                                [1.0; 3],
                                Some(image),
                                2,
                            );
                        }
                    }
                    if matches!(arrow.phase, Phase::Exploding(_)) && !arrow.flame {
                        continue;
                    }
                    let transform = if let Phase::Embedded {
                        parent: Some(parent),
                        ..
                    } = arrow.phase
                    {
                        scene.world(parent)?.compose(&arrow.transform)
                    } else {
                        arrow.transform
                    };
                    (arrow.model(), transform, arrow.rotation)
                }
                _ => continue,
            };
            if let Some(parts) = assets.items.model(model) {
                let transform = transform.compose(&Matrix {
                    rotation: math.rotate_yxz(rotation),
                    ..Default::default()
                });
                output.parts(&self.items, parts, transform, math)?;
            }
        }
        for sprite in &frame.scenery.sprites {
            let size = sprite.image.size.map(f32::from);
            output.billboard(
                sprite.position,
                size,
                sprite.scale.map(|v| v as f32 / 256.0),
                sprite.rotation,
                [f32::from(sprite.color) / 128.0; 3],
                Some(sprite.image),
                sprite.blend + 1,
            );
        }
        for sprite in &frame.effects.sprites {
            let image = assets.sprites.effects[usize::from(sprite.image)];
            let blend = blend(sprite.blend, sprite.image);
            let projected = output.project(sprite.position);
            if projected[2] <= 0.0 {
                continue;
            }
            let scale = sprite.scale.map(|v| f32::from(v) / 4096.0);
            output.screen_sprite(
                projected,
                image.size.map(f32::from),
                scale,
                sprite.rotation,
                sprite.color.map(|v| f32::from(v) / 128.0),
                Some(image),
                blend,
            );
        }
        for point in &frame.effects.points {
            output.screen_sprite(
                output.project(point.position),
                point.size.map(f32::from),
                [1.0; 2],
                0,
                point.color.map(|v| f32::from(v) / 255.0),
                None,
                if point.blend & 4 != 0 {
                    (point.blend & 3) + 1
                } else {
                    0
                },
            );
        }
        for quad in &frame.effects.quads {
            let image = assets.sprites.effects[usize::from(quad.image)];
            let mut transform = Matrix {
                translation: quad.position,
                ..Default::default()
            };
            if let Some(parent) = quad.parent {
                transform = scene.world(parent)?.compose(&transform);
            }
            let points = quad.corners.map(|v| {
                transform
                    .transform(v.to_array())
                    .map(|v| v as f32)
                    .to_array()
            });
            let blend = blend(quad.blend, quad.image);
            let mut vertices = render_effects::quad(
                points,
                [quad.color.map(|v| f32::from(v) / 128.0); 4],
                Some(image),
                blend,
            );
            for vertex in &mut vertices {
                vertex.flags &= !render_mesh::SCREEN;
                vertex.flags |= render_mesh::FOG | render_mesh::UNLIT;
            }
            output.surface(vertices, blend);
        }
        Ok(Geometry {
            terrain: output.terrain,
            actors: output.actors,
            effects: render_effects::surface_geometry(output.surfaces),
        })
    }
}

fn blend(blend: Blend, image: u8) -> u8 {
    match blend {
        // 8003a44c configures GsSPRITE attributes separately from the TIM page.
        Blend::Texture => match image {
            1 | 5 | 11 | 18 | 27 | 31 => 0,
            24 | 25 | 28 => 3,
            30 => 1,
            _ => 2,
        },
        Blend::Opaque => 0,
        Blend::Add => 2,
        Blend::SubtractMask => 3,
    }
}

struct Builder {
    view: Matrix,
    surfaces: Vec<Surface>,
    terrain: Vec<Vertex>,
    actors: Vec<Vertex>,
}

impl Builder {
    fn project(&self, point: Position) -> [f32; 3] {
        let point = point.map(|v| v as f32).to_array();
        let v: [f32; 3] = std::array::from_fn(|axis| {
            self.view.translation[axis] as f32
                + (0..3)
                    .map(|i| f32::from(self.view.rotation[axis][i]) * point[i] / 4096.0)
                    .sum::<f32>()
        });
        [v[0] * 300.0 / v[2], v[1] * 300.0 / v[2], v[2]]
    }

    fn surface(&mut self, vertices: Vec<Vertex>, blend: u8) {
        let depth = vertices
            .iter()
            .map(|v| {
                self.view.translation.z as f32
                    + (0..3)
                        .map(|i| f32::from(self.view.rotation[2][i]) * v.position[i] / 4096.0)
                        .sum::<f32>()
            })
            .sum::<f32>()
            / vertices.len() as f32;
        self.surfaces.push(Surface {
            depth,
            blend,
            vertices,
        });
    }

    fn model(&mut self, meshes: &Meshes, part: &Part, matrix: Matrix, terrain: bool) {
        meshes.visit(&part.geometry, matrix, true, 0, None, |triangle| {
            let mut triangle = triangle.map(|v| *v);
            if terrain {
                for v in &mut triangle {
                    v.flags |= render_mesh::SCENERY;
                }
            }
            if triangle[0].flags & render_mesh::TRANSLUCENT == 0 {
                if terrain {
                    self.terrain.extend(triangle);
                } else {
                    self.actors.extend(triangle);
                }
            } else {
                let blend = ((triangle[0].page >> 5) & 3) as u8 + 1;
                self.surface(triangle.to_vec(), blend);
            }
        });
    }

    fn parts(
        &mut self,
        meshes: &Meshes,
        parts: &[Part],
        matrix: Matrix,
        math: &tenchu_core::math::Math,
    ) -> Result<()> {
        let mut scene = tenchu_core::scene::Scene::default();
        let root = scene.add(None, matrix);
        let joints = tenchu_core::motion::Pose::new(parts)?.bind(&mut scene, root, math);
        for (part, joint) in parts.iter().zip(joints) {
            self.model(meshes, part, scene.world(joint)?, false);
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn billboard(
        &mut self,
        position: Position,
        size: [f32; 2],
        scale: [f32; 2],
        rotation: i32,
        color: [f32; 3],
        image: Option<Image>,
        blend: u8,
    ) {
        let projected = self.project(position);
        self.screen_sprite(
            projected,
            size,
            scale.map(|v| v * 300.0 / projected[2]),
            rotation,
            color,
            image,
            blend,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn screen_sprite(
        &mut self,
        center: [f32; 3],
        size: [f32; 2],
        scale: [f32; 2],
        rotation: i32,
        color: [f32; 3],
        image: Option<Image>,
        blend: u8,
    ) {
        if center[2] <= 0.0 {
            return;
        }
        let (sin, cos) = (rotation as f32 / 4096.0).to_radians().sin_cos();
        let points = std::array::from_fn(|i| {
            let x = ((i & 1) as f32 - 0.5) * size[0] * scale[0];
            let y = (((i >> 1) & 1) as f32 - 0.5) * size[1] * scale[1];
            [
                center[0] + x * cos - y * sin,
                center[1] + x * sin + y * cos,
                render_effects::depth(center[2]),
            ]
        });
        self.surfaces.push(Surface {
            depth: center[2],
            blend,
            vertices: render_effects::quad(points, [color; 4], image, blend),
        });
    }

    fn line(&mut self, a: Position, b: Position) {
        let a = self.project(a);
        let b = self.project(b);
        if a[2] <= 0.0 || b[2] <= 0.0 {
            return;
        }
        let delta = glam::Vec2::new(b[0] - a[0], b[1] - a[1]);
        let offset = glam::Vec2::new(-delta.y, delta.x).normalize_or_zero() * 0.6;
        let points = [
            [
                a[0] - offset.x,
                a[1] - offset.y,
                render_effects::depth(a[2]),
            ],
            [
                a[0] + offset.x,
                a[1] + offset.y,
                render_effects::depth(a[2]),
            ],
            [
                b[0] - offset.x,
                b[1] - offset.y,
                render_effects::depth(b[2]),
            ],
            [
                b[0] + offset.x,
                b[1] + offset.y,
                render_effects::depth(b[2]),
            ],
        ];
        self.surfaces.push(Surface {
            depth: (a[2] + b[2]) * 0.5,
            blend: 0,
            vertices: render_effects::quad(points, [[0.3, 0.26, 0.2]; 4], None, 0),
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires TENCHU2_DISC; exercises original joint counts across every native character and costume"]
    fn native_character_geometry_keeps_all_source_joints() {
        use tenchu_assets::tenchu2::characters;
        use tenchu_core::{math::Math, motion::Pose, scene::Scene};
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let math = Math {
            tables: tenchu_assets::tenchu2::math_tables(&disc).unwrap(),
        };
        let mut extended = 0;
        for entry in characters::load(&disc, None).unwrap() {
            let model = characters::rig(&disc, &entry.name).unwrap();
            let mut meshes = build(
                &model.vram,
                sources(&format!("actors/{}", entry.name), &model.parts),
                &Graphics::default(),
            )
            .unwrap();
            meshes.skin(&model.parts);
            extended += usize::from(model.parts.len() > 15);
            let mut scene = Scene::default();
            let root = scene.add(None, Matrix::default());
            let joints = Pose::new(&model.parts)
                .unwrap()
                .bind(&mut scene, root, &math);
            let mut matrices = joints
                .into_iter()
                .map(|joint| scene.world(joint).unwrap())
                .collect::<Vec<_>>();
            for pass in 0..2 {
                if pass == 1 {
                    for (i, matrix) in matrices.iter_mut().enumerate() {
                        matrix.rotation = math.rotate_yxz([0, i as i16 * 128, 0]);
                    }
                }
                meshes.pose_skin(&matrices, None);
                let mut count = 0;
                for (part, &matrix) in model.parts.iter().zip(&matrices) {
                    assert!(
                        meshes.visit(&part.geometry, matrix, true, 0, None, |triangle| {
                            for vertex in triangle {
                                assert!(
                                    vertex.position.iter().all(|v| v.is_finite()),
                                    "{}",
                                    entry.name
                                );
                                assert!(
                                    vertex.normal.iter().all(|v| v.is_finite()),
                                    "{}",
                                    entry.name
                                );
                                count += 1;
                            }
                        })
                    );
                }
                assert!(count > 0, "{}", entry.name);
            }
        }
        assert!(extended > 0, "did not exercise JP's extended rigs");
        eprintln!("{extended} native bodies with more than fifteen joints");
    }
}
