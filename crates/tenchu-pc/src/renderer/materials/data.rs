use super::{MAX_MIPS, Material};

pub(super) enum Data {
    Buffer(wgpu::Buffer),
    Texture(wgpu::Texture, wgpu::TextureView),
}

pub(super) fn uses_texture(device: &wgpu::Device) -> bool {
    device.limits().max_storage_buffers_per_shader_stage == 0
}

pub(super) fn binding_type(device: &wgpu::Device) -> wgpu::BindingType {
    if uses_texture(device) {
        wgpu::BindingType::Texture {
            sample_type: wgpu::TextureSampleType::Uint,
            view_dimension: wgpu::TextureViewDimension::D2,
            multisampled: false,
        }
    } else {
        wgpu::BindingType::Buffer {
            ty: wgpu::BufferBindingType::Storage { read_only: true },
            has_dynamic_offset: false,
            min_binding_size: None,
        }
    }
}

impl Data {
    pub(super) fn limit(device: &wgpu::Device) -> usize {
        let limits = device.limits();
        if uses_texture(device) {
            limits.max_texture_dimension_2d as usize
        } else {
            limits.max_storage_buffer_binding_size as usize / std::mem::size_of::<Material>()
        }
    }

    pub(super) fn new(device: &wgpu::Device, queue: &wgpu::Queue, materials: &[Material]) -> Self {
        let capacity = materials
            .len()
            .next_power_of_two()
            .max(64)
            .min(Self::limit(device));
        if !uses_texture(device) {
            let data = Self::Buffer(device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Material source regions and mip rectangles"),
                size: (capacity * std::mem::size_of::<Material>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }));
            data.write(queue, 0, materials);
            return data;
        }
        // WebGL2 has no storage buffers. Each row preserves one record's exact bits.
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Material metadata"),
            size: wgpu::Extent3d {
                width: (MAX_MIPS + 4) as u32,
                height: capacity as u32,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Rgba32Uint,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let view = texture.create_view(&Default::default());
        let data = Self::Texture(texture, view);
        data.write(queue, 0, materials);
        data
    }

    pub(super) fn capacity(&self) -> usize {
        match self {
            Self::Buffer(buffer) => buffer.size() as usize / std::mem::size_of::<Material>(),
            Self::Texture(texture, _) => texture.height() as usize,
        }
    }

    pub(super) fn binding(&self) -> wgpu::BindingResource<'_> {
        match self {
            Self::Buffer(buffer) => buffer.as_entire_binding(),
            Self::Texture(_, view) => wgpu::BindingResource::TextureView(view),
        }
    }

    pub(super) fn write(&self, queue: &wgpu::Queue, index: usize, materials: &[Material]) {
        let stride = std::mem::size_of::<Material>();
        let bytes = bytemuck::cast_slice(materials);
        match self {
            Self::Buffer(buffer) => queue.write_buffer(buffer, (index * stride) as u64, bytes),
            Self::Texture(texture, _) => queue.write_texture(
                wgpu::TexelCopyTextureInfo {
                    origin: wgpu::Origin3d {
                        x: 0,
                        y: index as u32,
                        z: 0,
                    },
                    ..texture.as_image_copy()
                },
                bytes,
                wgpu::TexelCopyBufferLayout {
                    offset: 0,
                    bytes_per_row: Some(stride as u32),
                    rows_per_image: None,
                },
                wgpu::Extent3d {
                    height: materials.len() as u32,
                    ..texture.size()
                },
            ),
        }
    }
}

pub(in crate::renderer) fn shader(device: &wgpu::Device) -> String {
    let access = if uses_texture(device) {
        include_str!("texture.wgsl")
    } else {
        include_str!("buffer.wgsl")
    };
    format!("{access}\n{}", include_str!("../materials.wgsl"))
}
