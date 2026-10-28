use crate::{
    renderer::Renderer,
    settings::{menu::Menu, overlay::Edit},
};
use winit::{event::WindowEvent, window::Window};

pub(super) struct Overlay {
    pub opaque: bool,
    pub dirty: bool,
    renderer: egui_wgpu::Renderer,
    state: egui_winit::State,
    context: egui::Context,
    jobs: Vec<egui::ClippedPrimitive>,
    shapes: Vec<egui::epaint::ClippedShape>,
    screen: egui_wgpu::ScreenDescriptor,
}

pub(super) struct Extras<'a> {
    pub labels: &'a [crate::debug::Label],
    pub text: &'a [crate::fonts::Text],
    pub performance: &'a mut super::performance::Performance,
    pub benchmarks: Option<&'a mut super::benchmark::Benchmarks>,
    pub presets: Option<&'a mut crate::settings::library::Manager>,
    pub controls: Option<&'a mut crate::controls::Controls>,
    pub demo: Option<&'a mut crate::demo::Player>,
    pub video_time: Option<f64>,
    pub frontend: Option<(
        &'a mut crate::frontend::Frontend,
        &'a crate::campaign::Campaign,
    )>,
}

pub(super) struct Updates {
    edits: Vec<Edit>,
    demo: Option<crate::demo::overlay::Action>,
    frontend: Option<crate::campaign::Action>,
    cancel_benchmark: bool,
}

impl Overlay {
    pub fn new(window: &Window, renderer: &Renderer, format: wgpu::TextureFormat) -> Self {
        let context = egui::Context::default();
        crate::fonts::install(&context);
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
        let mut overlay_renderer =
            egui_wgpu::Renderer::new(&renderer.device, format, None, 1, false);
        crate::frontend::gpu::register(&mut overlay_renderer, format);
        Self {
            opaque: false,
            dirty: true,
            renderer: overlay_renderer,
            state,
            context,
            jobs: Vec::new(),
            shapes: Vec::new(),
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
        state: crate::settings::overlay::State<'_>,
        mut extras: Extras<'_>,
    ) -> Updates {
        self.opaque = extras.frontend.is_some();
        let mut input = self.state.take_egui_input(window);
        if let Some(time) = extras.video_time {
            self.context.set_zoom_factor(3.0);
            self.context.style_mut(|s| s.animation_time = 0.0);
            input.screen_rect = Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(1280.0, 720.0),
            ));
            input
                .viewports
                .entry(egui::ViewportId::ROOT)
                .or_default()
                .native_pixels_per_point = Some(1.0);
            input.time = Some(time);
            input.predicted_dt = 1.0 / 60.0;
            input.events.clear();
            input.focused = true;
        } else {
            // A native resize can be visible before its Resized event configures the surface.
            let [width, height] = renderer.size();
            let scale = window.scale_factor() as f32 * self.context.zoom_factor();
            input.screen_rect = Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(width as f32, height as f32) / scale,
            ));
        }
        // Navigation is shared with the controller menu; text edits retain native key/IME events.
        if !self.editing()
            && menu.edit.is_none()
            && !extras.controls.as_ref().is_some_and(|c| c.editor.open)
        {
            input
                .events
                .retain(|event| !matches!(event, egui::Event::Key { .. } | egui::Event::Text(_)));
        }
        let mut edits = Vec::new();
        let mut demo_action = None;
        let mut frontend_action = None;
        let mut cancel_benchmark = false;
        let output = self.context.run(input, |ctx| {
            if let Some((frontend, campaign)) = &mut extras.frontend {
                frontend_action = frontend.show(
                    ctx,
                    campaign,
                    !menu.open && extras.demo.is_none(),
                    extras.video_time.is_some(),
                    state.settings,
                );
            }
            if !self.opaque {
                crate::fonts::paint(ctx, ctx.screen_rect(), extras.text);
                paint_labels(ctx, extras.labels);
            }
            let mut show_performance = state.settings.show_performance;
            let profiling = show_performance;
            if show_performance && !state.debug.photo_mode {
                extras.performance.show(
                    ctx,
                    &mut show_performance,
                    menu.open,
                    renderer.timings.is_some(),
                    extras.video_time.is_some(),
                );
            }
            if let Some(demo) = &mut extras.demo {
                demo_action = crate::demo::overlay::show(
                    ctx,
                    demo,
                    state.settings,
                    state.debug,
                    state.paused,
                );
            }
            edits = crate::settings::overlay::show(
                ctx,
                menu,
                state,
                extras.presets.as_ref().map(|p| &p.library),
            );
            if menu.open
                && let Some(presets) = &mut extras.presets
                && let Some(choice) = presets.show(ctx, state.settings)
            {
                edits.push(Edit::Preset(choice));
            }
            if let Some(benchmarks) = &mut extras.benchmarks {
                cancel_benchmark = benchmarks.show(ctx);
            }
            if menu.open
                && let Some(controls) = &mut extras.controls
            {
                controls.show_bindings(ctx);
            }
            if show_performance != profiling {
                edits.push(Edit::Change(crate::settings::menu::Item::Performance, 1));
            }
        });
        self.state
            .handle_platform_output(window, output.platform_output);
        let changed = self.shapes != output.shapes
            || self.screen.size_in_pixels != renderer.size()
            || self.screen.pixels_per_point != output.pixels_per_point
            || !output.textures_delta.is_empty();
        self.dirty |= changed;
        if changed {
            self.shapes = output.shapes;
            self.jobs = self
                .context
                .tessellate(self.shapes.clone(), output.pixels_per_point);
        }
        self.screen = egui_wgpu::ScreenDescriptor {
            size_in_pixels: renderer.size(),
            pixels_per_point: output.pixels_per_point,
        };
        for (id, image) in &output.textures_delta.set {
            self.renderer
                .update_texture(&renderer.device, &renderer.queue, *id, image);
        }
        for id in &output.textures_delta.free {
            self.renderer.free_texture(id);
        }
        Updates {
            edits,
            demo: demo_action,
            frontend: frontend_action,
            cancel_benchmark,
        }
    }

    pub fn draw(&mut self, renderer: &Renderer, target: &wgpu::TextureView) {
        renderer.draw_overlay(target, &mut self.renderer, &self.jobs, &self.screen);
    }
}

impl super::Application {
    pub(super) fn update_overlay(&mut self) {
        let was_open = self.menu.open;
        let library_was_open = self.demo.as_ref().is_some_and(|d| d.library);
        let frontend_visible = self.frontend_visible();
        #[cfg(not(target_arch = "wasm32"))]
        let video_time = self
            .video
            .as_ref()
            .map(|v| v.frames as f64 / crate::video::FPS as f64);
        #[cfg(target_arch = "wasm32")]
        let video_time = None;
        let Some(view) = &mut self.view else {
            return;
        };
        self.menu.player_models.clone_from(&view.player_models);
        self.menu.artwork_status = view.artwork_status();
        let updates = view.overlay.update(
            &view.window,
            &view.renderer,
            &mut self.menu,
            crate::settings::overlay::State {
                settings: &self.settings,
                debug: &self.debug,
                paused: self.paused,
                lights: view.lights.len(),
                samples: &view.renderer.sample_counts,
            },
            Extras {
                labels: &view.labels,
                text: &view.text,
                performance: &mut self.performance,
                benchmarks: Some(&mut self.benchmarks),
                presets: Some(&mut self.presets),
                controls: Some(&mut self.controls),
                demo: self.demo.as_mut(),
                video_time,
                frontend: if frontend_visible {
                    self.frontend.as_mut().map(|f| (f, &self.campaign))
                } else {
                    None
                },
            },
        );
        if updates.cancel_benchmark {
            if let Err(error) = self.finish_benchmark(Some("Cancelled by user".into())) {
                self.pause_error(error);
            }
            return;
        }
        if let Some(action) = updates.demo {
            self.demo_action(action);
        }
        if let Some(action) = updates.frontend {
            if matches!(self.campaign.screen, crate::campaign::Screen::Results(_)) {
                self.action(action);
            } else {
                self.frontend_action(action);
            }
        }
        if library_was_open != self.demo.as_ref().is_some_and(|d| d.library) {
            self.sync_pause();
        }
        let edits = updates.edits;
        if edits.is_empty() && was_open == self.menu.open {
            return;
        }
        let previous = self.settings.clone();
        for edit in edits {
            self.menu.notice.clear();
            let result = match edit {
                Edit::Change(item, steps) => self.apply_item(item, steps),
                Edit::Preset(choice) => self.presets.library.apply(choice, &mut self.settings),
                Edit::GameplayPreset(index) => self.set_gameplay_preset(index),
                Edit::Benchmark => {
                    self.benchmarks.open = true;
                    Ok(())
                }
                Edit::PresetLibrary => {
                    self.presets.open = true;
                    Ok(())
                }
                Edit::Controls => {
                    self.controls.editor.open = true;
                    self.controls.cancel_capture();
                    self.controls.clear();
                    Ok(())
                }
                Edit::Reset(item) => self.reset_item(item),
                Edit::PlayerModel(id) => {
                    self.settings.player_model = id;
                    Ok(())
                }
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
            self.controls.close_editor();
        }
        self.release_pointer();
        self.sync_pause();
        self.update_view();
    }
}

fn paint_labels(ctx: &egui::Context, labels: &[crate::debug::Label]) {
    use crate::debug::{Anchor, Label};
    let screen = ctx.screen_rect();
    let painter = ctx.layer_painter(egui::LayerId::background());
    for Label(anchor, text, color) in labels {
        let origin = match *anchor {
            Anchor::Projected([x, y]) => {
                screen.min + egui::vec2(x * screen.width(), y * screen.height())
            }
            Anchor::TopLeft([x, y]) => screen.min + egui::vec2(x, y),
            Anchor::BottomLeft([x, y]) => screen.left_bottom() + egui::vec2(x, -y),
        };
        let color = color.map(|v| (v * 255.0).clamp(0.0, 255.0) as u8);
        let color = egui::Color32::from_rgb(color[0], color[1], color[2]);
        let text = painter.layout_no_wrap(text.clone(), egui::FontId::monospace(15.0), color);
        let origin = egui::pos2(
            origin.x.min(screen.right() - text.size().x - 5.0).max(5.0),
            origin.y,
        );
        painter.rect_filled(
            egui::Rect::from_min_size(origin, text.size()).expand(3.0),
            3.0,
            egui::Color32::from_black_alpha(175),
        );
        painter.galley(origin, text, color);
    }
}
