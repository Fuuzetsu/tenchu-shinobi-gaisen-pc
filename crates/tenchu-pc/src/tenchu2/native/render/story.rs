use super::{Builder, Geometry, ModelStyle, SceneDrawing, build, capture_frame, sources};
use crate::{render_effects, settings::Graphics};
use std::{collections::BTreeMap, path::Path};
use tenchu_assets::{
    ReadAt, Result,
    disc::Disc,
    tenchu2::{
        level::{Level, objects::StaticKind},
        math_tables, motions,
        story::Resources,
    },
};
use tenchu_core::{
    math::{Math, Matrix},
    motion::Pose,
    random::Random,
    tenchu2::story::{Faces, Motion, Presentation, PresentationFrame, TextureChange},
};

mod overlay;

pub(in crate::tenchu2::native) fn capture(
    disc: &Disc<impl ReadAt>,
    sequence: usize,
    event: u16,
    tick: u32,
    seed: u32,
    graphics: &Graphics,
    path: &Path,
) -> Result<()> {
    let mut resources = Resources::load(disc, event)?;
    let level = Level::japanese_scene(disc, resources.area, sequence, false)?;
    let math = Math {
        tables: math_tables(disc)?,
    };
    let basis = motions::spline_basis(disc)?;
    let mut camera = Motion::new(&resources.script.camera);
    let mut actors: Vec<_> = resources.script.actors.iter().map(Motion::new).collect();
    let mut faces = Faces::new(&resources);
    let mut random = Random::new(seed);
    let mut presentation = Presentation::default();
    let mut overlay = PresentationFrame::default();
    let mut view = Matrix::default();
    let frame = tick.min(resources.script.duration as u32 - 1) as i16;
    for frame in 0..=frame {
        camera.update(&resources.script.camera, frame, &basis)?;
        view = camera.camera_view(i32::from(presentation.camera_roll), &math)?;
        overlay = presentation.update(&resources.script, frame, true);
        for change in faces.update(&resources.script.expressions, frame, true, &mut random) {
            if let TextureChange::Patch { model, index } = change {
                resources.faces[&model][usize::from(index)]
                    .apply(&mut resources.models.get_mut(&model).unwrap().vram);
            }
        }
        for (motion, animation) in actors.iter_mut().zip(&resources.script.actors) {
            motion.update(animation, frame, &basis)?;
        }
    }
    for actor in &resources.script.actors {
        if actor.model > 290 {
            let vram = resources.models[&actor.texture_model].vram.clone();
            resources.models.get_mut(&actor.model).unwrap().vram = vram;
        }
    }
    capture_frame(&level.vram, graphics, path, |renderer| {
        renderer.visual_time = f32::from(frame) / 30.0;
        let mut output = Builder {
            view,
            surfaces: Vec::new(),
            terrain: Vec::new(),
            actors: Vec::new(),
        };
        let mut terrain_sources = sources(&format!("map{}/terrain", level.id), &level.map);
        terrain_sources.extend(sources(&format!("map{}/props", level.id), &level.objects));
        let mut terrain = build(&level.vram, terrain_sources, graphics)?;
        renderer.bind_meshes(&mut terrain);
        for part in &level.map {
            output.model(
                &terrain,
                part,
                Matrix {
                    translation: part.position.map(|v| i32::from(v) * 10).into(),
                    ..Default::default()
                },
                ModelStyle::Terrain,
            );
        }
        for object in &level.scene.statics {
            if let StaticKind::Model(index) = object.kind {
                output.model(
                    &terrain,
                    &level.objects[index],
                    Matrix {
                        translation: object.position.into(),
                        rotation: math.rotate_yxz(object.rotation),
                    },
                    ModelStyle::Terrain,
                );
            }
        }
        for object in &level.scene.dynamic {
            output.model(
                &terrain,
                &level.objects[object.model],
                Matrix {
                    translation: object.position.into(),
                    rotation: math.rotate_yxz([0, object.facing, 0]),
                },
                ModelStyle::Terrain,
            );
        }
        let mut models = BTreeMap::new();
        for (&id, model) in &resources.models {
            let directory = match id {
                0..=105 | 290..=293 => "actors",
                106..=231 => "weapons",
                _ => "event",
            };
            let mut mesh = build(
                &model.vram,
                sources(&format!("{directory}/{}", model.name), &model.parts),
                graphics,
            )?;
            mesh.skin(&model.parts);
            renderer.bind_meshes(&mut mesh);
            models.insert(id, mesh);
        }
        let mut weapons = BTreeMap::new();
        for (name, model) in &resources.weapons {
            let mut mesh = build(
                &model.vram,
                sources(&format!("weapons/{name}"), &model.parts),
                graphics,
            )?;
            renderer.bind_meshes(&mut mesh);
            weapons.insert(name, mesh);
        }
        for (slot, (motion, animation)) in actors.iter().zip(&resources.script.actors).enumerate() {
            let Some(model) = resources.models.get(&animation.model) else {
                continue;
            };
            let mut pose = Pose::new(&model.parts)?;
            pose.translation[0] = motion.position;
            for (rotation, &value) in pose.rotation.iter_mut().zip(&motion.rotation) {
                *rotation = value;
            }
            let matrices = pose.matrices(&math, Matrix::default());
            let mesh = &models[&animation.model];
            mesh.pose_skin(&matrices, None);
            for (part, &matrix) in model.parts.iter().zip(&matrices) {
                output.model(mesh, part, matrix, ModelStyle::Actor);
            }
            for attachment in resources.attachments.iter().filter(|a| a.actor == slot) {
                let weapon = &resources.weapons[&attachment.model];
                output.parts(
                    &weapons[&attachment.model],
                    &weapon.parts,
                    matrices[attachment.bone],
                    &math,
                )?;
            }
        }
        let texts = overlay::append(&mut output, &resources, &overlay, renderer, event);
        let geometry = Geometry {
            terrain: output.terrain,
            actors: output.actors,
            effects: render_effects::surface_geometry(output.surfaces),
        };
        Ok((SceneDrawing::new(geometry, renderer), view, texts))
    })?;
    eprintln!(
        "Native JP event {event}, sequence {sequence}, frame {frame}: {} actor tracks. Scene/overlay inspection only; timed 3D effects, distortion and sound are not presented by this capture.",
        actors.len()
    );
    Ok(())
}
