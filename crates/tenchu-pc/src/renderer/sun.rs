use crate::settings::Directional;
use glam::{Mat4, Vec3};

pub fn direction(settings: &Directional) -> Vec3 {
    let azimuth = f32::from(settings.azimuth).to_radians();
    let elevation = f32::from(settings.elevation).to_radians();
    Vec3::new(
        azimuth.sin() * elevation.cos(),
        -elevation.sin(),
        azimuth.cos() * elevation.cos(),
    )
}

pub fn color(settings: &Directional) -> [f32; 3] {
    match settings.color {
        1 => [0.65, 0.78, 1.0],
        2 => [1.0, 0.85, 0.65],
        _ => [1.0; 3],
    }
}

pub fn matrix(settings: &Directional, camera: Mat4) -> Mat4 {
    let inverse = camera.inverse();
    let near = inverse.project_point3(Vec3::ZERO);
    let far = inverse.project_point3(Vec3::Z);
    let focus = near + (far - near).normalize() * 8000.0;
    let view = Mat4::look_to_rh(Vec3::ZERO, -direction(settings), Vec3::NEG_Y);
    let mut center = view.transform_point3(focus);
    let radius = f32::from(settings.radius) * 1000.0;
    let texel = 2.0 * radius / settings.resolution as f32;
    center.x = (center.x / texel).floor() * texel;
    center.y = (center.y / texel).floor() * texel;
    Mat4::orthographic_rh(
        center.x - radius,
        center.x + radius,
        center.y - radius,
        center.y + radius,
        -center.z - 100_000.0,
        -center.z + 100_000.0,
    ) * view
}

pub fn target(device: &wgpu::Device, resolution: u32) -> wgpu::TextureView {
    device
        .create_texture(&wgpu::TextureDescriptor {
            label: Some("Directional shadow"),
            size: wgpu::Extent3d {
                width: resolution,
                height: resolution,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        })
        .create_view(&Default::default())
}
