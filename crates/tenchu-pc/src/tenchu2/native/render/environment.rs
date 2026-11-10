use crate::{
    render_effects,
    render_mesh::{self, Vertex},
    renderer::{Mesh, Renderer},
    settings::{Graphics, WeatherMode},
    visuals::{
        footprints::Footprints, interpolation::Part, models::Meshes, surfaces::Surfaces,
        weather::Weather,
    },
};
use glam::{Mat4, Vec3};
use tenchu_assets::tenchu2::level::{Level, objects::StaticKind};
use tenchu_core::{
    math::{Math, Matrix},
    tenchu2::world::{Assets, World},
};

pub(super) struct Environment {
    pub caster: Mesh,
    pub stale: bool,
    pub surfaces: Surfaces,
    weather: Weather,
    footprints: Footprints,
    tick: Option<u64>,
    thunder: Option<tenchu_audio::ThunderCue>,
    spray: crate::visuals::water::Spray,
    pub contact: crate::visuals::contact::Contact,
}

pub(super) struct Frame<'a> {
    pub level: &'a Level,
    pub meshes: &'a Meshes,
    pub math: &'a Math,
    pub tick: u64,
    pub camera: Mat4,
    pub alpha: f32,
    pub contact: Option<&'a super::super::contact::History>,
    pub world: Option<(&'a World, &'a Assets)>,
}

impl Environment {
    pub fn new(level: &Level, meshes: &Meshes, math: &Math, renderer: &mut Renderer) -> Self {
        let mut vertices = Vec::new();
        let mut water = Vec::new();
        let mut append = |part: &tenchu_assets::model::Part, matrix| {
            meshes.visit(&part.geometry, matrix, false, 0, None, |triangle| {
                if triangle[0].flags & render_mesh::WATER != 0 {
                    water.extend(triangle.into_iter().copied());
                } else if triangle[0].flags & render_mesh::SKY == 0 {
                    vertices.extend(triangle.map(|v| Vertex {
                        flags: v.flags | render_mesh::SCENERY,
                        ..*v
                    }));
                }
            });
        };
        for part in &level.map {
            append(
                part,
                Matrix {
                    translation: part.position.map(|v| i32::from(v) * 10).into(),
                    ..Default::default()
                },
            );
        }
        for object in &level.scene.statics {
            if let StaticKind::Model(index) = object.kind {
                append(
                    &level.objects[index],
                    Matrix {
                        translation: object.position.into(),
                        rotation: math.rotate_yxz(object.rotation),
                    },
                );
            }
        }
        Self {
            caster: renderer.upload_shadow(&vertices),
            stale: false,
            surfaces: Surfaces::new(&vertices),
            weather: Default::default(),
            footprints: Default::default(),
            tick: None,
            thunder: None,
            spray: crate::visuals::water::Spray::new(&water),
            contact: Default::default(),
        }
    }

    pub fn rebuild(
        &mut self,
        level: &Level,
        meshes: &Meshes,
        math: &Math,
        renderer: &mut Renderer,
    ) {
        let mut replacement = Self::new(level, meshes, math, renderer);
        replacement.weather = std::mem::take(&mut self.weather);
        replacement.footprints = std::mem::take(&mut self.footprints);
        replacement.tick = self.tick;
        replacement.contact = std::mem::take(&mut self.contact);
        replacement.thunder = self.thunder.take();
        *self = replacement;
    }

    pub fn clear(&mut self) {
        self.weather = Default::default();
        self.footprints.clear();
        self.thunder = None;
    }

    pub fn clear_footprints(&mut self) {
        self.footprints.clear();
    }

    pub fn update(
        &mut self,
        frame: &Frame<'_>,
        graphics: &Graphics,
        renderer: &mut Renderer,
    ) -> render_effects::Geometry {
        let Frame {
            tick,
            camera,
            world,
            alpha,
            ..
        } = *frame;
        let inverse = camera.inverse();
        let center = inverse.project_point3(Vec3::ZERO);
        // Fast-forward may advance hundreds of source updates between presentations.
        let start = self
            .tick
            .map_or(tick, |old| (old + 1).max(tick.saturating_sub(29)));
        for step in start..=tick {
            self.weather
                .tick(step, center, &self.surfaces, &graphics.weather);
            if let Some(cue) = self.weather.thunder() {
                self.thunder = Some(cue);
            }
            self.footprints.tick_contacts(
                step,
                || world.map_or_else(Vec::new, |(world, assets)| contacts(world, assets)),
                &self.surfaces,
                &graphics.weather,
                self.weather.sample(&graphics.weather, 1.0).snow,
            );
        }
        let start = self
            .contact
            .last_tick()
            .map_or(tick.saturating_sub(29), |old| {
                (old + 1).max(tick.saturating_sub(29))
            });
        for step in start..=tick {
            if self
                .contact
                .begin_tick(step, &self.surfaces, graphics, center)
                && let Some(history) = frame.contact
            {
                use tenchu_core::tenchu2::effects::Contact;
                let point =
                    |p: tenchu_core::math::Position| Vec3::from(p.to_array().map(|v| v as f32));
                for &event in history.events(step) {
                    match event {
                        Contact::Spark { position, parried } if graphics.effects.sparks => {
                            self.contact.spark(
                                point(position),
                                Vec3::NEG_Y,
                                graphics.effects.spark_density,
                                if parried {
                                    [0.4, 0.65, 1.0]
                                } else {
                                    [1.0, 0.64, 0.2]
                                },
                            );
                        }
                        Contact::Splash { position, size } if graphics.effects.splashes => {
                            self.contact.splash(
                                point(position),
                                (f32::from(size) / 1024.0).clamp(0.35, 3.0),
                            );
                        }
                        Contact::Ripple { position, speed } if graphics.effects.splashes => {
                            self.contact
                                .ripple(point(position), (speed as f32 / 40.0).clamp(0.25, 2.0));
                        }
                        _ => {}
                    }
                }
            }
        }
        self.tick = Some(tick);
        if graphics.weather.mode == WeatherMode::Original {
            self.clear();
        }
        renderer.weather = self.weather.sample(&graphics.weather, 1.0);
        let mut geometry = self.weather.geometry(camera, 1.0, &graphics.weather);
        geometry.append(
            self.footprints
                .geometry(&graphics.weather, Some(camera), 0.0),
        );
        geometry.append(self.contact.geometry(camera, alpha, &graphics.effects));
        geometry.append(
            self.contact
                .impact_geometry(camera, alpha, &graphics.effects),
        );
        geometry.append(
            self.spray
                .geometry(tick as f32 / 30.0, camera, &graphics.water),
        );
        geometry
    }

    pub fn audio(
        &mut self,
        settings: &crate::settings::Weather,
    ) -> (tenchu_audio::Ambience, Option<tenchu_audio::ThunderCue>) {
        (
            self.weather.ambience(&self.surfaces, settings),
            self.thunder.take(),
        )
    }
}

fn contacts(world: &World, assets: &Assets) -> Vec<(Part, Vec3, Vec3)> {
    let mut scene = world.scene.clone();
    let mut contacts = Vec::new();
    for (agent, instance) in world.actors.iter().zip(&world.instances) {
        let actor = &agent.actor;
        if actor.flags & 8 != 0 || actor.health <= 0 || actor.kind & 0xf0 == 0x70 {
            continue;
        }
        let Some(definition) = assets.resources.registry.actor(actor.definition_kind()) else {
            continue;
        };
        let parts = &assets.models[&definition.model].parts;
        for index in [8, 14] {
            let Some((&joint, part)) = instance.rig.joints.get(index).zip(parts.get(index)) else {
                continue;
            };
            let Ok(matrix) = scene.world(joint) else {
                continue;
            };
            let Some(sole) = part
                .geometry
                .iter()
                .flat_map(|g| &g.vertices)
                .map(|v| v[1])
                .max()
            else {
                continue;
            };
            contacts.push((
                Part::Actor {
                    root: instance.rig.root,
                    model: joint,
                    kind: actor.kind,
                    inspection: false,
                },
                Vec3::from(matrix.transform([0, sole, 0]).to_array().map(|v| v as f32)),
                crate::visuals::models::rotation(matrix) * Vec3::Z,
            ));
        }
    }
    contacts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[ignore = "requires Japanese TENCHU2_DISC, TENCHU_MODEL_REVIEW and GPU; verifies shared snow/rain with native geometry"]
    fn native_weather_preserves_logic_and_uses_stage_surfaces() {
        let disc =
            crate::source::open_disc(std::env::var_os("TENCHU2_DISC").unwrap().as_ref()).unwrap();
        let output = std::path::PathBuf::from(std::env::var_os("TENCHU_MODEL_REVIEW").unwrap());
        std::fs::create_dir_all(&output).unwrap();
        let mut session = crate::tenchu2::native::Session::load(&disc, 9, 1).unwrap();
        for _ in 0..30 {
            session.step(0).unwrap();
        }
        let before = serde_json::to_vec(&session.world).unwrap();
        for mode in [WeatherMode::Original, WeatherMode::Snow, WeatherMode::Rain] {
            let mut graphics = Graphics {
                fog: false,
                asset_pack: false,
                ..Default::default()
            };
            graphics.weather.mode = mode;
            graphics.weather.amount = 100;
            graphics.weather.snow = 100;
            graphics.weather.pooling = 1000;
            let path = output.join(format!("native-weather-{mode:?}.png"));
            super::super::capture_frame(
                &session.assets.sprites.vram,
                &graphics,
                &path,
                |renderer| {
                    let mut drawing = super::super::Drawing::new(&session, &graphics, renderer)?;
                    drawing.update(&session, renderer, None)?;
                    if mode != WeatherMode::Original {
                        let e = drawing.scene.environment.as_mut().unwrap();
                        let eye = crate::debug::camera::Camera::from_view(
                            session.world.camera.view,
                            300.0,
                        )
                        .position;
                        e.weather.prewarm(
                            u64::from(session.world.tick),
                            eye,
                            &e.surfaces,
                            &graphics.weather,
                            20,
                        );
                        assert!(
                            e.surfaces
                                .cast(eye - Vec3::Y * 5000.0, eye + Vec3::Y * 20000.0)
                                .is_some()
                        );
                        let snow = e.weather.sample(&graphics.weather, 1.0).snow;
                        let feet = contacts(&session.world, &session.assets);
                        assert!(!feet.is_empty());
                        e.footprints.tick_contacts(
                            31,
                            || feet,
                            &e.surfaces,
                            &graphics.weather,
                            snow,
                        );
                        if mode == WeatherMode::Snow {
                            assert!(snow > 0.5);
                            assert!(
                                !e.footprints
                                    .geometry(&graphics.weather, None, 0.0)
                                    .vertices
                                    .is_empty(),
                                "native soles did not leave tracks"
                            );
                        }
                        drawing.update(&session, renderer, None)?;
                        let sample = renderer.weather;
                        drawing.update(&session, renderer, None)?;
                        assert_eq!(
                            renderer.weather.snow, sample.snow,
                            "pause must freeze accumulation"
                        );
                        assert_eq!(renderer.weather.wet, sample.wet);
                        let environment = drawing.scene.environment.as_ref().unwrap();
                        let footprints = postcard::to_allocvec(&environment.footprints).unwrap();
                        let weather = postcard::to_allocvec(&environment.weather).unwrap();
                        let mut rebuilt =
                            super::super::Drawing::new(&session, &graphics, renderer)?;
                        rebuilt.preserve_environment(&mut drawing);
                        rebuilt.update(&session, renderer, None)?;
                        let environment = rebuilt.scene.environment.as_ref().unwrap();
                        assert_eq!(
                            postcard::to_allocvec(&environment.footprints).unwrap(),
                            footprints,
                            "visual rebuild removed or advanced tracks"
                        );
                        assert_eq!(
                            postcard::to_allocvec(&environment.weather).unwrap(),
                            weather
                        );
                        drawing = rebuilt;
                    }
                    assert_eq!(serde_json::to_vec(&session.world).unwrap(), before);
                    Ok((drawing.scene, session.world.camera.view, Vec::new()))
                },
            )
            .unwrap();
        }
    }
}
