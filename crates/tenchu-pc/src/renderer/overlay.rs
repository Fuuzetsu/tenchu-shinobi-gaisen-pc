impl super::Renderer {
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
