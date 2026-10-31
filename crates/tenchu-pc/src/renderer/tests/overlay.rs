use super::*;

#[test]
#[ignore = "requires a graphics adapter; interface textures must survive captures and uploads from skipped frames"]
fn interface_textures_survive_until_the_next_frame() {
    let (renderer, target) = fixture(&Vram::default());
    let mut overlay = egui_wgpu::Renderer::new(&renderer.device, target.format(), None, 1, false);
    let mut pending_free = Vec::new();
    let delta = |id| egui::TexturesDelta {
        set: vec![(
            id,
            egui::epaint::ImageDelta::full(
                egui::ColorImage::new([2, 2], egui::Color32::RED),
                egui::TextureOptions::NEAREST,
            ),
        )],
        free: vec![id],
    };
    let screen = egui_wgpu::ScreenDescriptor {
        size_in_pixels: renderer.size(),
        pixels_per_point: 1.0,
    };
    let rect = egui::Rect::from_min_size(
        egui::Pos2::ZERO,
        egui::vec2(target.width() as f32, target.height() as f32),
    );
    let id = egui::TextureId::Managed(1);
    renderer.update_overlay_textures(&mut overlay, &mut pending_free, delta(id));
    let mut mesh = egui::Mesh::with_texture(id);
    mesh.add_rect_with_uv(
        rect,
        egui::Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
    let jobs = [egui::ClippedPrimitive {
        clip_rect: rect,
        primitive: egui::epaint::Primitive::Mesh(mesh),
    }];
    let view = target.create_view(&Default::default());
    for _ in 0..2 {
        renderer.draw_overlay(&view, &mut overlay, &jobs, &screen);
        assert_eq!(read_pixel(&renderer, &target), [255, 0, 0, 255]);
    }

    // Discard several UI updates without drawing or otherwise submitting their uploads.
    for index in 2..5 {
        renderer.update_overlay_textures(
            &mut overlay,
            &mut pending_free,
            delta(egui::TextureId::Managed(index)),
        );
        assert!(
            overlay
                .texture(&egui::TextureId::Managed(index - 1))
                .is_none()
        );
    }
    renderer.update_overlay_textures(&mut overlay, &mut pending_free, Default::default());
    renderer.draw_overlay(&view, &mut overlay, &[], &screen);
    renderer.queue.submit([]);
    renderer.device.poll(wgpu::Maintain::Wait);
    assert!(pending_free.is_empty());
    assert!(overlay.texture(&egui::TextureId::Managed(4)).is_none());
}
