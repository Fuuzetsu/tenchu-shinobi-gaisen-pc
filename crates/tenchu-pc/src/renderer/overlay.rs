impl super::Renderer {
    pub fn update_overlay_textures(
        &self,
        renderer: &mut egui_wgpu::Renderer,
        pending_free: &mut Vec<egui::TextureId>,
        delta: egui::TexturesDelta,
    ) {
        if !pending_free.is_empty() {
            // An update can replace a frame before it is drawn. Submit its uploads
            // before destroying textures; retain this frame's textures for captures too.
            self.queue.submit([]);
            for id in pending_free.drain(..) {
                renderer.free_texture(&id);
            }
        }
        for (id, image) in &delta.set {
            renderer.update_texture(&self.device, &self.queue, *id, image);
        }
        *pending_free = delta.free;
    }

    pub fn draw_overlay(
        &self,
        target: &wgpu::TextureView,
        renderer: &mut egui_wgpu::Renderer,
        jobs: &[egui::ClippedPrimitive],
        screen: &egui_wgpu::ScreenDescriptor,
    ) {
        if jobs.is_empty() {
            return;
        }
        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Interface overlay"),
            });
        let callbacks =
            renderer.update_buffers(&self.device, &self.queue, &mut encoder, jobs, screen);
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Interface overlay"),
                    color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                        view: target,
                        resolve_target: None,
                        ops: wgpu::Operations {
                            load: wgpu::LoadOp::Load,
                            store: wgpu::StoreOp::Store,
                        },
                    })],
                    depth_stencil_attachment: None,
                    timestamp_writes: None,
                    occlusion_query_set: None,
                })
                .forget_lifetime();
            renderer.render(&mut pass, jobs, screen);
        }
        self.queue
            .submit(callbacks.into_iter().chain([encoder.finish()]));
    }
}
