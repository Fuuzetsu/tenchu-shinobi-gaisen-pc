use crate::render_mesh::Vertex;
use bytemuck::{Pod, Zeroable};
use wgpu::util::DeviceExt;

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
struct Pose {
    position: [f32; 3],
    normal: [f32; 3],
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
}

#[derive(Default)]
pub(super) struct Upload {
    pose: Vec<Pose>,
    attributes: Vec<Attributes>,
}

impl Upload {
    fn split(&mut self, vertices: &[Vertex]) {
        self.pose.clear();
        self.attributes.clear();
        self.pose.reserve(vertices.len());
        self.attributes.reserve(vertices.len());
        for v in vertices {
            self.pose.push(Pose {
                position: v.position,
                normal: v.normal,
            });
            self.attributes.push(Attributes {
                color: v.color,
                uv: v.uv,
                page: v.page,
                palette: v.palette,
                flags: v.flags,
                material: v.material,
                opacity: v.opacity,
            });
        }
    }

    pub fn create(device: &wgpu::Device, vertices: &[Vertex]) -> Buffers {
        let mut upload = Self::default();
        upload.split(vertices);
        let create = |label, contents| {
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some(label),
                contents,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            })
        };
        Buffers {
            pose: create("Mesh poses", bytemuck::cast_slice(&upload.pose)),
            attributes: create("Mesh attributes", bytemuck::cast_slice(&upload.attributes)),
            cached: Vec::new(),
        }
    }

    pub fn update(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        buffers: &mut Buffers,
        vertices: &[Vertex],
    ) {
        self.split(vertices);
        update_buffer(
            device,
            queue,
            &mut buffers.pose,
            bytemuck::cast_slice(&self.pose),
            wgpu::BufferUsages::VERTEX,
        );
        let attributes = bytemuck::cast_slice(&self.attributes);
        if bytemuck::cast_slice::<_, u8>(&buffers.cached) != attributes {
            update_buffer(
                device,
                queue,
                &mut buffers.attributes,
                attributes,
                wgpu::BufferUsages::VERTEX,
            );
            buffers.cached.clone_from(&self.attributes);
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
    if buffer.size() < data.len() as u64 {
        *buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Dynamic geometry"),
            size: (data.len() as u64).next_power_of_two(),
            usage: usage | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });
    }
    queue.write_buffer(buffer, 0, data);
}
