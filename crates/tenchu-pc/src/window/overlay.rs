use crate::{
    debug::Debug,
    renderer::Renderer,
    settings::{Settings, menu::Menu, overlay::Edit},
};
use winit::{event::WindowEvent, window::Window};

pub(super) struct Overlay {
    renderer: egui_wgpu::Renderer,
    state: egui_winit::State,
    context: egui::Context,
    jobs: Vec<egui::ClippedPrimitive>,
    screen: egui_wgpu::ScreenDescriptor,
}

impl Overlay {
    pub fn new(window: &Window, renderer: &Renderer, format: wgpu::TextureFormat) -> Self {
        let context = egui::Context::default();
        context.set_visuals(egui::Visuals::dark());
        context.style_mut(|s| {
            s.visuals.override_text_color = Some(egui::Color32::from_rgb(222, 229, 238));
            s.spacing.slider_width = 100.0;
            s.spacing.interact_size.y = 22.0;
            s.visuals.window_fill = egui::Color32::from_rgb(23, 27, 33);
        });
        let state = egui_winit::State::new(
            context.clone(),
            egui::ViewportId::ROOT,
            window,
            Some(window.scale_factor() as f32),
            window.theme(),
            Some(renderer.device.limits().max_texture_dimension_2d as usize),
        );
        Self {
            renderer: egui_wgpu::Renderer::new(&renderer.device, format, None, 1, false),
            state,
            context,
            jobs: Vec::new(),
            screen: egui_wgpu::ScreenDescriptor {
                size_in_pixels: [1, 1],
                pixels_per_point: 1.0,
            },
        }
    }

    pub fn event(&mut self, window: &Window, event: &WindowEvent) {
        let _ = self.state.on_window_event(window, event);
    }

    pub fn editing(&self) -> bool {
        self.context.wants_keyboard_input()
    }

    pub fn update(
        &mut self,
        window: &Window,
        renderer: &Renderer,
        menu: &mut Menu,
        settings: &Settings,
        debug: &Debug,
        paused: bool,
        lights: usize,
    ) -> Vec<Edit> {
        let mut input = self.state.take_egui_input(window);
        // Navigation is shared with the controller menu; text edits retain native key/IME events.
        if !self.editing() && menu.edit.is_none() {
            input
                .events
                .retain(|event| !matches!(event, egui::Event::Key { .. } | egui::Event::Text(_)));
        }
        let mut edits = Vec::new();
        let output = self.context.run(input, |ctx| {
            edits = crate::settings::overlay::show(
                ctx,
                menu,
                settings,
                debug,
                paused,
                lights,
                &renderer.sample_counts,
            );
        });
        self.state
            .handle_platform_output(window, output.platform_output);
        self.jobs = self
            .context
            .tessellate(output.shapes, output.pixels_per_point);
        let size = window.inner_size();
        self.screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [size.width.max(1), size.height.max(1)],
            pixels_per_point: output.pixels_per_point,
        };
        for (id, image) in &output.textures_delta.set {
            self.renderer
                .update_texture(&renderer.device, &renderer.queue, *id, image);
        }
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        edits
    }

    pub fn draw(&mut self, renderer: &Renderer, target: &wgpu::TextureView) {
        if self.jobs.is_empty() {
            return;
        }
        let mut encoder = renderer
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Settings overlay"),
            });
        let callbacks = self.renderer.update_buffers(
            &renderer.device,
            &renderer.queue,
            &mut encoder,
            &self.jobs,
            &self.screen,
        );
        {
            let mut pass = encoder
                .begin_render_pass(&wgpu::RenderPassDescriptor {
                    label: Some("Settings overlay"),
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
            self.renderer.render(&mut pass, &self.jobs, &self.screen);
        }
        renderer
            .queue
            .submit(callbacks.into_iter().chain([encoder.finish()]));
    }
}

impl super::Application {
    pub(super) fn update_overlay(&mut self) {
        let was_open = self.menu.open;
        let Some(view) = &mut self.view else {
            return;
        };
        let edits = view.overlay.update(
            &view.window,
            &view.renderer,
            &mut self.menu,
            &self.settings,
            &self.debug,
            self.paused,
            view.lights.len(),
        );
        if edits.is_empty() && was_open == self.menu.open {
            return;
        }
        let previous = self.settings.clone();
        for edit in edits {
            self.menu.notice.clear();
            let result = match edit {
                Edit::Change(item, steps) => self.apply_item(item, steps),
                Edit::Preset(index) => {
                    self.settings.graphics.set_preset(index);
                    Ok(())
                }
                Edit::Reset(item) => self.reset_item(item),
                Edit::Coordinate(axis, value) => {
                    self.debug.target[axis] = value;
                    Ok(())
                }
            };
            if let Err(error) = result {
                self.menu.notice = error;
            }
        }
        self.apply_settings(&previous);
        if !self.menu.open {
            self.view.as_mut().unwrap().overlay.jobs.clear();
            self.controls.clear();
        }
        self.release_pointer();
        self.sync_pause();
        self.update_view();
    }
}
