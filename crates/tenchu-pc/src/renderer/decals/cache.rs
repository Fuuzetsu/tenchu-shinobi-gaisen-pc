use super::{Decal, Renderer, bounds};
#[cfg(test)]
use super::{Frustum, Mat4};
use crate::{
    render_mesh::{self, Vertex},
    renderer::geometry,
    visuals::surfaces::Source,
};
use glam::{Mat2, Vec2, Vec3, Vec3Swizzles};
use std::{collections::HashMap, ops::Range, sync::Arc};
use wgpu::util::DeviceExt;

const SIZE: u32 = 256;
const CAPACITY: usize = 256;
const FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::R16Float;

struct Stamp {
    page: Arc<Vec<Vertex>>,
    range: Range<usize>,
}
impl PartialEq for Stamp {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.page, &other.page) && self.range == other.range
    }
}

struct Tile {
    source: Arc<Source>,
    stamps: Vec<Stamp>,
    surface: [Vertex; 3],
    coverage: Option<(wgpu::Buffer, u32)>,
    #[cfg(test)]
    bounds: [Vec3; 2],
}

pub(in crate::renderer) struct Cache {
    slots: Vec<Option<Tile>>,
    layers: Vec<wgpu::TextureView>,
    bindings: wgpu::BindGroup,
    coverage: wgpu::RenderPipeline,
    shaded: [wgpu::RenderPipeline; 2],
    pages: Vec<Arc<Vec<Vertex>>>,
    surfaces: Option<(geometry::Buffers, wgpu::Buffer, u32)>,
    pub fallback: Vec<Vec<Range<u32>>>,
}

impl Cache {
    pub fn new(renderer: &Renderer) -> Self {
        let device = &renderer.device;
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Retained blood coverage"),
            size: wgpu::Extent3d {
                width: SIZE,
                height: SIZE,
                depth_or_array_layers: CAPACITY as u32,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: FORMAT,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });
        let layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Cached blood coverage"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2Array,
                        multisampled: false,
                    },
                    count: None,
                },
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });
        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Blood coverage interpolation"),
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });
        let bindings = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Cached blood coverage"),
            layout: &layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: wgpu::BindingResource::TextureView(
                        &texture.create_view(&Default::default()),
                    ),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });
        let full = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Cached blood shading"),
            bind_group_layouts: &[
                &renderer.pipelines.opaque.get_bind_group_layout(0),
                &renderer.shadows.layout,
                &renderer.materials.layout,
                &layout,
            ],
            push_constant_ranges: &[],
        });
        let shader = super::super::pipelines::Options::new(&renderer.graphics, renderer.samples)
            .blood_cache_shader(device, renderer.gl);
        let shaded = ["cached_blood", "cached_snow_blood"].map(|entry| {
            device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label: Some(entry),
                layout: Some(&full),
                vertex: wgpu::VertexState {
                    module: &shader,
                    entry_point: Some("cached_blood_vertex"),
                    compilation_options: Default::default(),
                    buffers: &[
                        geometry::LAYOUTS[0].clone(),
                        geometry::LAYOUTS[1].clone(),
                        wgpu::VertexBufferLayout {
                            array_stride: 4,
                            step_mode: wgpu::VertexStepMode::Vertex,
                            attributes: &wgpu::vertex_attr_array![9 => Uint32],
                        },
                    ],
                },
                primitive: wgpu::PrimitiveState {
                    front_face: wgpu::FrontFace::Cw,
                    cull_mode: None,
                    ..Default::default()
                },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: wgpu::TextureFormat::Depth32Float,
                    depth_write_enabled: false,
                    depth_compare: wgpu::CompareFunction::LessEqual,
                    stencil: Default::default(),
                    bias: wgpu::DepthBiasState {
                        constant: -2,
                        ..Default::default()
                    },
                }),
                multisample: Default::default(),
                fragment: Some(wgpu::FragmentState {
                    module: &shader,
                    entry_point: Some(entry),
                    compilation_options: Default::default(),
                    targets: &[Some(wgpu::ColorTargetState {
                        format: renderer.world_format(),
                        blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                        write_mask: wgpu::ColorWrites::ALL,
                    })],
                }),
                multiview: None,
                cache: None,
            })
        });
        let mask = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Blood mask generation"),
            source: wgpu::ShaderSource::Wgsl(
                concat!(
                    include_str!("../blood_mask.wgsl"),
                    "\n",
                    include_str!("cache_coverage.wgsl")
                )
                .into(),
            ),
        });
        let blend = wgpu::BlendComponent {
            src_factor: wgpu::BlendFactor::One,
            dst_factor: wgpu::BlendFactor::OneMinusSrcAlpha,
            operation: wgpu::BlendOperation::Add,
        };
        let coverage = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Cache blood masks"), layout: None,
            vertex: wgpu::VertexState { module: &mask, entry_point: Some("vertex"), compilation_options: Default::default(), buffers: &[wgpu::VertexBufferLayout { array_stride: 20, step_mode: wgpu::VertexStepMode::Vertex, attributes: &wgpu::vertex_attr_array![0 => Float32x2, 1 => Float32x2, 2 => Float32] }] },
            primitive: Default::default(), depth_stencil: None, multisample: Default::default(),
            fragment: Some(wgpu::FragmentState { module: &mask, entry_point: Some("fragment"), compilation_options: Default::default(), targets: &[Some(wgpu::ColorTargetState { format: FORMAT, blend: Some(wgpu::BlendState { color: blend, alpha: blend }), write_mask: wgpu::ColorWrites::ALL })] }),
            multiview: None, cache: None,
        });
        let layers = (0..CAPACITY)
            .map(|i| {
                texture.create_view(&wgpu::TextureViewDescriptor {
                    dimension: Some(wgpu::TextureViewDimension::D2),
                    base_array_layer: i as u32,
                    array_layer_count: Some(1),
                    ..Default::default()
                })
            })
            .collect();
        Self {
            slots: (0..CAPACITY).map(|_| None).collect(),
            layers,
            bindings,
            coverage,
            shaded,
            pages: Vec::new(),
            surfaces: None,
            fallback: Vec::new(),
        }
    }

    pub fn prepare(&mut self, device: &wgpu::Device, decals: &[Decal]) {
        if self.pages.len() == decals.len()
            && self
                .pages
                .iter()
                .zip(decals)
                .all(|(a, b)| Arc::ptr_eq(a, &b._source))
        {
            return;
        }
        self.pages = decals.iter().map(|d| d._source.clone()).collect();
        let mut groups: HashMap<Arc<Source>, Vec<Stamp>> = HashMap::new();
        let mut order = Vec::new();
        for decal in decals.iter().filter(|d| d.ground_blood) {
            for source in &decal.sources {
                let entry = groups.entry(source.source.clone()).or_insert_with(|| {
                    order.push(source.source.clone());
                    Vec::new()
                });
                entry.push(Stamp {
                    page: decal._source.clone(),
                    range: source.vertices.clone(),
                });
            }
        }
        groups.retain(|source, stamps| {
            let tri = source.vertices();
            let bound = bounds(&tri);
            stamps.len() >= 4
                && (bound[1] - bound[0]).max_element() <= 4000.0
                && stamps.iter().all(|s| {
                    s.page.get(s.range.clone()).is_some_and(|v| {
                        v.len() >= 3 && v.iter().all(|p| (p.opacity - v[0].opacity).abs() < 0.00001)
                    })
                })
        });
        if groups.len() > CAPACITY {
            groups.clear();
        }
        let keys: Vec<_> = groups.keys().cloned().collect();
        let mut ambiguous = std::collections::HashSet::new();
        for (i, a) in keys.iter().enumerate() {
            let ta = a.vertices();
            for b in &keys[i + 1..] {
                let tb = b.vertices();
                if (ta[0].position[1] - tb[0].position[1]).abs() < 1.0 && overlaps(&ta, &tb) {
                    ambiguous.insert(a.clone());
                    ambiguous.insert(b.clone());
                }
            }
        }
        groups.retain(|key, _| !ambiguous.contains(key));
        if decals
            .iter()
            .any(|d| d.ground_blood && d.sources.is_empty())
        {
            groups.clear();
        }
        let mut geometry_changed = false;
        // Keep existing slots; regenerating one receiver must not move the others.
        for slot in &mut self.slots {
            if slot
                .as_ref()
                .is_some_and(|tile| !groups.contains_key(&tile.source))
            {
                *slot = None;
                geometry_changed = true;
            }
        }
        let mut known: HashMap<_, _> = self
            .slots
            .iter()
            .enumerate()
            .filter_map(|(i, t)| t.as_ref().map(|t| (t.source.clone(), i)))
            .collect();
        for source in order {
            let Some(stamps) = groups.remove(&source) else {
                continue;
            };
            let index = known
                .get(&source)
                .copied()
                .or_else(|| self.slots.iter().position(Option::is_none));
            let Some(index) = index else {
                continue;
            };
            if self.slots[index]
                .as_ref()
                .is_some_and(|t| t.stamps == stamps)
            {
                continue;
            }
            geometry_changed |= self.slots[index]
                .as_ref()
                .is_none_or(|old| old.source != source);
            let Some(tile) = tile(device, source.clone(), stamps) else {
                geometry_changed |= self.slots[index].is_some();
                self.slots[index] = None;
                continue;
            };
            self.slots[index] = Some(tile);
            known.insert(source, index);
        }
        if geometry_changed {
            let mut vertices = Vec::new();
            let mut layers = Vec::new();
            for (index, tile) in self.slots.iter().enumerate() {
                if let Some(tile) = tile {
                    vertices.extend(tile.surface);
                    layers.extend([index as u32; 3]);
                }
            }
            self.surfaces = (!vertices.is_empty()).then(|| {
                (
                    geometry::Buffers::new(device, &vertices),
                    device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("Cached blood receiver layers"),
                        contents: bytemuck::cast_slice(&layers),
                        usage: wgpu::BufferUsages::VERTEX,
                    }),
                    vertices.len() as u32,
                )
            });
        }
        let cached: std::collections::HashSet<_> =
            self.slots.iter().flatten().map(|t| &t.source).collect();
        self.fallback = decals
            .iter()
            .map(|decal| {
                let mut kept = Vec::new();
                let mut cursor = 0;
                if decal.ground_blood {
                    for source in &decal.sources {
                        if cached.contains(&source.source) {
                            if cursor < source.vertices.start {
                                kept.push(cursor as u32..source.vertices.start as u32);
                            }
                            cursor = source.vertices.end;
                        }
                    }
                }
                if cursor < decal.mesh.count as usize {
                    kept.push(cursor as u32..decal.mesh.count);
                }
                kept
            })
            .collect();
    }

    #[cfg(test)]
    pub fn useful(&self, matrix: Mat4) -> bool {
        let frustum = Frustum::new(matrix);
        self.slots
            .iter()
            .flatten()
            .any(|t| frustum.contains(t.bounds, 2.0))
    }

    pub fn dirty(&self) -> bool {
        self.slots.iter().flatten().any(|t| t.coverage.is_some())
    }

    pub fn update(&mut self, encoder: &mut wgpu::CommandEncoder) {
        for (index, tile) in self.slots.iter_mut().enumerate() {
            let Some((vertices, count)) = tile.as_mut().and_then(|t| t.coverage.take()) else {
                continue;
            };
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Update retained blood mask"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &self.layers[index],
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.coverage);
            pass.set_vertex_buffer(0, vertices.slice(..));
            pass.draw(0..count, 0..1);
        }
    }

    pub fn draw(&self, pass: &mut wgpu::RenderPass<'_>, snow: bool) {
        let Some((surfaces, layers, count)) = &self.surfaces else {
            return;
        };
        pass.set_pipeline(&self.shaded[usize::from(snow)]);
        pass.set_bind_group(3, &self.bindings, &[]);
        surfaces.bind(pass);
        pass.set_vertex_buffer(2, layers.slice(..));
        pass.draw(0..*count, 0..1);
    }
}

fn tile(device: &wgpu::Device, source: Arc<Source>, stamps: Vec<Stamp>) -> Option<Tile> {
    let mut surface = source.vertices();
    let bound = bounds(&surface);
    let extent = (bound[1] - bound[0]).max_element().max(1.0) * SIZE as f32 / (SIZE - 4) as f32;
    let origin = bound[0].xz() - Vec2::splat(extent * 2.0 / SIZE as f32);
    let mut masks: Vec<[f32; 5]> = Vec::new();
    for stamp in &stamps {
        let vertices = &stamp.page[stamp.range.clone()];
        let triangle = vertices.chunks_exact(3).find(|t| {
            let [a, b, c] = [t[0], t[1], t[2]].map(|v| Vec3::from(v.position).xz());
            Mat2::from_cols(b - a, c - a).determinant().abs() > 0.001
        })?;
        let [a, b, c] = [triangle[0], triangle[1], triangle[2]];
        let p = Vec3::from(a.position).xz();
        let inverse = Mat2::from_cols(
            Vec3::from(b.position).xz() - p,
            Vec3::from(c.position).xz() - p,
        )
        .inverse();
        let uv = Vec2::new(a.color[0], a.color[1]);
        let transform = Mat2::from_cols(
            Vec2::new(b.color[0], b.color[1]) - uv,
            Vec2::new(c.color[0], c.color[1]) - uv,
        ) * inverse;
        let [min, max] = bounds(vertices).map(|v| v.xz());
        let pad = Vec2::splat(extent * 2.0 / SIZE as f32);
        let [min, max] = [min - pad, max + pad];
        let corners = [min, Vec2::new(max.x, min.y), max, Vec2::new(min.x, max.y)];
        for i in [0, 1, 2, 0, 2, 3] {
            let xy = (corners[i] - origin) / extent;
            let coordinate = uv + transform * (corners[i] - p);
            masks.push([xy.x, xy.y, coordinate.x, coordinate.y, a.opacity]);
        }
    }
    for v in &mut surface {
        let uv = (Vec3::from(v.position).xz() - origin) / extent;
        let luminance = Vec3::from(v.color).dot(Vec3::new(0.2126, 0.7152, 0.0722));
        v.color = [uv.x, uv.y, luminance];
        v.opacity = 1.0;
        v.position[1] -= 0.5;
        v.flags = (v.flags & !render_mesh::EMITTER)
            | render_mesh::SCENERY
            | render_mesh::STAIN
            | render_mesh::SOFT
            | render_mesh::ALPHA;
    }
    let buffer = |label, bytes: &[u8]| {
        device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some(label),
            contents: bytes,
            usage: wgpu::BufferUsages::VERTEX,
        })
    };
    Some(Tile {
        source,
        stamps,
        surface,
        coverage: Some((
            buffer("Blood mask stamps", bytemuck::cast_slice(&masks)),
            masks.len() as u32,
        )),
        #[cfg(test)]
        bounds: bound,
    })
}

fn overlaps(a: &[Vertex; 3], b: &[Vertex; 3]) -> bool {
    let a = a.map(|v| Vec3::from(v.position).xz());
    let b = b.map(|v| Vec3::from(v.position).xz());
    for tri in [&a, &b] {
        for i in 0..3 {
            let edge = tri[(i + 1) % 3] - tri[i];
            let axis = Vec2::new(-edge.y, edge.x).normalize_or_zero();
            let interval = |t: &[Vec2; 3]| {
                t.iter()
                    .map(|v| v.dot(axis))
                    .fold((f32::INFINITY, f32::NEG_INFINITY), |(min, max), p| {
                        (min.min(p), max.max(p))
                    })
            };
            let (amin, amax) = interval(&a);
            let (bmin, bmax) = interval(&b);
            if amax <= bmin + 0.001 || bmax <= amin + 0.001 {
                return false;
            }
        }
    }
    true
}
