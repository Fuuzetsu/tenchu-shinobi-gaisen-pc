use super::{Renderer, decals, pipelines, post, wet_lighting};
use crate::settings::{BloodStyle, Graphics, WeatherMode};

#[derive(Clone, PartialEq)]
struct Key {
    options: pipelines::Options,
    size: [u32; 2],
    samples: u32,
    post: bool,
    occlusion: bool,
    half_ao: bool,
    heat: bool,
    composite: bool,
    cache: bool,
    wet: bool,
    screen_shadows: bool,
}

impl Key {
    fn new(renderer: &Renderer, graphics: &Graphics, size: [u32; 2]) -> Self {
        let samples = if renderer.sample_counts.contains(&graphics.samples) {
            graphics.samples
        } else {
            1
        };
        Self {
            options: pipelines::Options::new(graphics, samples),
            size,
            samples,
            post: graphics.post.enabled,
            occlusion: graphics.post.occlusion > 0,
            half_ao: graphics.rendering.half_resolution_ao,
            heat: post::heat_enabled(graphics),
            composite: renderer.composite_supported
                && samples == 1
                && graphics.post.enabled
                && (graphics.lighting.enabled
                    || graphics.sky.enabled
                    || graphics.weather.mode != WeatherMode::Original)
                && [graphics.blood.ground, graphics.blood.death].contains(&BloodStyle::Modern),
            cache: graphics.rendering.cached_blood
                && renderer.composite_supported
                && graphics.post.enabled
                && !renderer.gl,
            wet: wet_lighting::configured(graphics, samples, renderer.gl),
            screen_shadows: graphics.rendering.quarter_resolution_shadows,
        }
    }
}

pub(super) struct Context {
    pub device: wgpu::Device,
    pub layout: wgpu::PipelineLayout,
    pub scene_layout: wgpu::BindGroupLayout,
    pub shadow_layout: wgpu::BindGroupLayout,
    pub material_layout: wgpu::BindGroupLayout,
    pub shader: wgpu::ShaderModule,
    pub graphics: Graphics,
    pub samples: u32,
    pub gl: bool,
    format: wgpu::TextureFormat,
    depth: wgpu::TextureView,
    key: Key,
    previous: Key,
}

impl Context {
    fn new(renderer: &Renderer, graphics: Graphics, size: [u32; 2]) -> Self {
        let key = Key::new(renderer, &graphics, size);
        Self {
            device: renderer.device.clone(),
            layout: renderer.layout.clone(),
            scene_layout: renderer.pipelines.opaque.get_bind_group_layout(0),
            shadow_layout: renderer.shadows.layout.clone(),
            material_layout: renderer.materials.layout.clone(),
            shader: renderer.shader.clone(),
            samples: key.samples,
            gl: renderer.gl,
            format: renderer.format,
            depth: renderer.depth.clone(),
            previous: Key::new(renderer, &renderer.graphics, renderer.size),
            key,
            graphics,
        }
    }

    pub fn world_format(&self) -> wgpu::TextureFormat {
        if self.key.post {
            post::FORMAT
        } else {
            self.format
        }
    }

    fn build(mut self) -> Prepared {
        let old = &self.previous;
        let key = &self.key;
        let shader =
            (old.options != key.options).then(|| key.options.shader(&self.device, self.gl));
        if let Some(shader) = &shader {
            self.shader = shader.clone();
        }
        let pipeline_changed =
            shader.is_some() || old.samples != key.samples || old.post != key.post;
        let pipelines = pipeline_changed.then(|| {
            pipelines::create(
                &self.device,
                self.world_format(),
                &self.layout,
                &self.shader,
                self.samples,
            )
        });
        let attachments_changed =
            old.size != key.size || old.samples != key.samples || old.post != key.post;
        let attachments = attachments_changed.then(|| {
            let depth = Renderer::attachment(
                &self.device,
                key.size,
                wgpu::TextureFormat::Depth32Float,
                self.samples,
            );
            let color = (self.samples > 1).then(|| {
                Renderer::attachment(&self.device, key.size, self.world_format(), self.samples)
            });
            (depth, color)
        });
        let post_changed = attachments_changed
            || old.occlusion != key.occlusion
            || old.half_ao != key.half_ao
            || old.heat != key.heat;
        let post = post_changed.then(|| {
            key.post.then(|| {
                post::Post::new(
                    &self.device,
                    self.format,
                    key.size,
                    self.samples,
                    attachments.as_ref().map_or(&self.depth, |(depth, _)| depth),
                    &self.graphics,
                    self.gl,
                )
            })
        });
        let composite = (pipeline_changed || old.composite != key.composite).then(|| {
            key.composite
                .then(|| decals::composite::Composite::new(&self))
        });
        let cache = (pipeline_changed || old.cache != key.cache)
            .then(|| key.cache.then(|| decals::cache::Cache::new(&self)));
        let wet =
            (pipeline_changed || old.wet != key.wet || old.screen_shadows != key.screen_shadows)
                .then(|| key.wet.then(|| wet_lighting::WetLighting::new(&self)));
        Prepared {
            shader,
            pipelines,
            attachments,
            post,
            composite,
            cache,
            wet,
        }
    }
}

struct Prepared {
    shader: Option<wgpu::ShaderModule>,
    pipelines: Option<pipelines::Pipelines>,
    attachments: Option<(wgpu::TextureView, Option<wgpu::TextureView>)>,
    post: Option<Option<post::Post>>,
    composite: Option<Option<decals::composite::Composite>>,
    cache: Option<Option<decals::cache::Cache>>,
    wet: Option<Option<wet_lighting::WetLighting>>,
}

impl Prepared {
    fn apply(self, renderer: &mut Renderer, graphics: Graphics, size: [u32; 2]) {
        if let Some(shader) = self.shader {
            renderer.shader = shader;
        }
        if let Some(pipelines) = self.pipelines {
            renderer.pipelines = pipelines;
        }
        if let Some((depth, color)) = self.attachments {
            renderer.depth = depth;
            renderer.multisample = color;
            if let Some(composite) = &mut renderer.composite {
                composite.targets = None;
            }
        }
        if let Some(post) = self.post {
            renderer.post = post;
        }
        if let Some(composite) = self.composite {
            renderer.composite = composite;
        }
        if let Some(cache) = self.cache {
            renderer.blood_cache = cache;
        }
        if let Some(wet) = self.wet {
            renderer.wet_lighting = wet;
        }
        renderer.samples = Key::new(renderer, &graphics, size).samples;
        renderer.size = size;
        renderer.materials.configure(&graphics);
        renderer.graphics = graphics;
    }
}

impl Renderer {
    pub(super) fn configure_at(&mut self, graphics: Graphics, size: [u32; 2]) {
        Context::new(self, graphics.clone(), size)
            .build()
            .apply(self, graphics, size);
    }
}

#[cfg(all(test, not(target_arch = "wasm32")))]
mod tests {
    use super::*;
    use crate::renderer::{
        game_camera,
        tests::{fixture, read_pixels, rectangle},
    };
    use std::{
        sync::mpsc,
        time::{Duration, Instant},
    };

    fn finish(renderer: &mut Renderer) {
        let deadline = Instant::now() + Duration::from_secs(120);
        while renderer.preparing() {
            renderer.poll_preparation().unwrap();
            assert!(Instant::now() < deadline, "GPU preparation did not finish");
            std::thread::sleep(Duration::from_millis(5));
        }
    }

    #[test]
    #[ignore = "requires a GPU; asynchronous configuration must coalesce changes and preserve rendering"]
    fn preparation_does_not_wait_and_discards_stale_work() {
        let (mut renderer, target) = fixture(&Default::default());
        let original = renderer.graphics.clone();
        let size = renderer.size;
        let mut rain = original.clone();
        rain.weather.mode = WeatherMode::Rain;
        rain.post.enabled = true;
        rain.post.occlusion = 50;
        rain.lighting.enabled = true;
        rain.rendering.quarter_resolution_shadows = true;
        let context = Context::new(&renderer, rain.clone(), size);
        let building = context.key.clone();
        let (release, gate) = mpsc::channel();
        renderer.preparation = Some(Preparation {
            graphics: rain.clone(),
            size,
            building,
            worker: std::thread::spawn(move || {
                gate.recv().unwrap();
                context.build()
            }),
        });
        // The worker cannot finish until released; these calls must not wait for it.
        renderer.prepare_resize([31, 27]).unwrap();
        renderer.prepare(&original, size).unwrap();
        renderer.poll_preparation().unwrap();
        assert!(renderer.preparing());
        assert_eq!(renderer.graphics, original);
        assert_eq!(renderer.size, size);
        release.send(()).unwrap();
        finish(&mut renderer);
        assert_eq!(renderer.graphics, original);
        assert!(renderer.post.is_none());
        assert_eq!(renderer.size, size);

        let mesh = renderer.upload(&rectangle(1000, 128, None, false, false, false));
        let view = target.create_view(&Default::default());
        let camera = game_camera(Default::default(), 1.0);
        renderer.configure(&rain);
        renderer.draw(&view, camera, &[&mesh], None).unwrap();
        let reference = read_pixels(&renderer, &target);
        renderer.configure(&original);
        renderer.prepare(&rain, size).unwrap();
        finish(&mut renderer);
        renderer.draw(&view, camera, &[&mesh], None).unwrap();
        assert_eq!(read_pixels(&renderer, &target), reference);
        renderer.prepare_resize([31, 27]).unwrap();
        finish(&mut renderer);
        assert_eq!(renderer.size, [31, 27]);
        renderer.prepare(&original, size).unwrap();
        finish(&mut renderer);
        renderer.draw(&view, camera, &[&mesh], None).unwrap();
        renderer.prepare(&original, size).unwrap();
        assert!(
            !renderer.preparing(),
            "unchanged size/settings must not rebuild"
        );
    }
}

#[cfg(any(target_os = "android", all(test, not(target_arch = "wasm32"))))]
pub(super) struct Preparation {
    graphics: Graphics,
    size: [u32; 2],
    building: Key,
    worker: std::thread::JoinHandle<Prepared>,
}

#[cfg(any(target_os = "android", all(test, not(target_arch = "wasm32"))))]
impl Renderer {
    pub fn preparing(&self) -> bool {
        self.preparation.is_some()
    }

    pub fn prepare_resize(&mut self, size: [u32; 2]) -> tenchu_assets::Result<()> {
        let graphics = self
            .preparation
            .as_ref()
            .map_or(&self.graphics, |p| &p.graphics)
            .clone();
        self.prepare(&graphics, size)
    }

    /// Coalesces changes while the driver compiles. The caller must keep servicing
    /// platform events without issuing GPU work: some drivers lock during compilation.
    pub fn prepare(&mut self, graphics: &Graphics, size: [u32; 2]) -> tenchu_assets::Result<()> {
        if let Some(pending) = &mut self.preparation {
            pending.graphics = graphics.clone();
            pending.size = size;
            return Ok(());
        }
        let context = Context::new(self, graphics.clone(), size);
        if context.key == context.previous {
            self.materials.configure(graphics);
            self.graphics = graphics.clone();
            return Ok(());
        }
        let building = context.key.clone();
        let worker = std::thread::Builder::new()
            .name("tenchu-graphics".into())
            .spawn(move || context.build())
            .map_err(|e| e.to_string())?;
        self.preparation = Some(Preparation {
            graphics: graphics.clone(),
            size,
            building,
            worker,
        });
        Ok(())
    }

    pub fn poll_preparation(&mut self) -> tenchu_assets::Result<()> {
        if !self
            .preparation
            .as_ref()
            .is_some_and(|p| p.worker.is_finished())
        {
            return Ok(());
        }
        let pending = self.preparation.take().unwrap();
        let prepared = pending
            .worker
            .join()
            .map_err(|_| "Graphics preparation failed")?;
        if pending.building == Key::new(self, &pending.graphics, pending.size) {
            prepared.apply(self, pending.graphics, pending.size);
            Ok(())
        } else {
            drop(prepared);
            self.prepare(&pending.graphics, pending.size)
        }
    }
}
