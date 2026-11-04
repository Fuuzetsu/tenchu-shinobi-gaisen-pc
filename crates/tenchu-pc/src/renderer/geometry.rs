use crate::render_mesh::Vertex;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Pose {
    position: [f32; 3],
    normal: [f32; 3],
}

impl From<&Vertex> for Pose {
    fn from(vertex: &Vertex) -> Self {
        Self {
            position: vertex.position,
            normal: vertex.normal,
        }
    }
}

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Attributes {
    color: [f32; 3],
    uv: [f32; 2],
    page: u32,
    palette: u32,
    flags: u32,
    material: u32,
    opacity: f32,
}

impl From<&Vertex> for Attributes {
    fn from(vertex: &Vertex) -> Self {
        Self {
            color: vertex.color,
            uv: vertex.uv,
            page: vertex.page,
            palette: vertex.palette,
            flags: vertex.flags,
            material: vertex.material,
            opacity: vertex.opacity,
        }
    }
}

impl Attributes {
    fn matches(&self, vertex: &Vertex) -> bool {
        self.color.map(f32::to_bits) == vertex.color.map(f32::to_bits)
            && self.uv.map(f32::to_bits) == vertex.uv.map(f32::to_bits)
            && self.page == vertex.page
            && self.palette == vertex.palette
            && self.flags == vertex.flags
            && self.material == vertex.material
            && self.opacity.to_bits() == vertex.opacity.to_bits()
    }
}

pub(super) const LAYOUTS: [wgpu::VertexBufferLayout<'static>; 2] = [
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Pose>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![0 => Float32x3, 7 => Float32x3],
    },
    wgpu::VertexBufferLayout {
        array_stride: std::mem::size_of::<Attributes>() as u64,
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &wgpu::vertex_attr_array![1 => Float32x3, 2 => Float32x2, 3 => Uint32, 4 => Uint32, 5 => Uint32, 6 => Uint32, 8 => Float32],
    },
];

pub(super) struct Buffers {
    pub pose: wgpu::Buffer,
    attributes: wgpu::Buffer,
    cached: Vec<Attributes>,
}

impl Buffers {
    pub fn bind(&self, pass: &mut wgpu::RenderPass<'_>) {
        pass.set_vertex_buffer(0, self.pose.slice(..));
        pass.set_vertex_buffer(1, self.attributes.slice(..));
    }

    pub fn new(device: &wgpu::Device, vertices: &[Vertex]) -> Self {
        let poses: Vec<_> = vertices.iter().map(Pose::from).collect();
        let attributes: Vec<_> = vertices.iter().map(Attributes::from).collect();
        let create = |label, contents| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            })
        };
        Self {
            pose: create("Mesh poses", bytemuck::cast_slice(&poses)),
            attributes: create("Mesh attributes", bytemuck::cast_slice(&attributes)),
            cached: Vec::new(),
        }
    }

    pub fn update(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, vertices: &[Vertex]) {
        let changed = self.cached.len() != vertices.len()
            || vertices
                .iter()
                .zip(&self.cached)
                .any(|(vertex, cached)| !cached.matches(vertex));
        let size = (vertices.len() * std::mem::size_of::<Pose>()) as u64;
        if let Some(size) = wgpu::BufferSize::new(size) {
            resize_buffer(
                device,
                &mut self.pose,
                size.get(),
                wgpu::BufferUsages::VERTEX,
            );
            let mut staging = queue
                .write_buffer_with(&self.pose, 0, size)
                .expect("Could not allocate mesh pose staging buffer");
            for (vertex, bytes) in vertices.iter().zip(
                staging
                    .as_mut()
                    .chunks_exact_mut(std::mem::size_of::<Pose>()),
            ) {
                bytes.copy_from_slice(bytemuck::bytes_of(&Pose::from(vertex)));
            }
        }
        if changed {
            self.cached.clear();
            self.cached.extend(vertices.iter().map(Attributes::from));
            update_buffer(
                device,
                queue,
                &mut self.attributes,
                bytemuck::cast_slice(&self.cached),
                wgpu::BufferUsages::VERTEX,
            );
        }
    }
}

pub(super) fn update_buffer(
    device: &wgpu::Device,
    queue: &wgpu::Queue,
    buffer: &mut wgpu::Buffer,
    data: &[u8],
    usage: wgpu::BufferUsages,
) {
    if data.is_empty() {
        return;
    }
    resize_buffer(device, buffer, data.len() as u64, usage);
    queue.write_buffer(buffer, 0, data);
}

fn resize_buffer(
    device: &wgpu::Device,
    buffer: &mut wgpu::Buffer,
    size: u64,
    usage: wgpu::BufferUsages,
) {
    if buffer.size() < size {
        *buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic geometry"),
            size: size.next_power_of_two(),
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }
}
