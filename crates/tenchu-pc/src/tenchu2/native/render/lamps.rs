use super::{Graphics, Meshes, Pack, Role, catalog};
use crate::{
    lighting::Source,
    visuals::lights::{self, Emitters},
};
use tenchu_assets::{Result, tenchu2::level::Level, texture::Vram};
use tenchu_core::{
    math::{Math, Matrix},
    tenchu2::environment::Environment,
};

pub(super) fn pack(
    vram: &Vram,
    sources: &[catalog::ModelSource<'_>],
    graphics: &Graphics,
) -> Result<Pack> {
    if graphics.asset_pack {
        Pack::for_models(vram, sources)
    } else {
        Ok(Pack::default())
    }
}

pub(super) fn prepare(
    vram: &Vram,
    sources: Vec<catalog::ModelSource<'_>>,
    graphics: &Graphics,
) -> Result<(Meshes, Emitters)> {
    let pack = pack(vram, &sources, graphics)?;
    let emitters = Emitters::new(
        vram,
        sources.iter().cloned(),
        &lights::rules(&pack, graphics.asset_pack),
    );
    let meshes = super::prepare_pack(vram, sources, graphics, &pack)?;
    Ok((meshes, emitters))
}

pub(super) fn sources(
    emitters: &Emitters,
    level: &Level,
    math: &Math,
    environment: Option<&Environment>,
) -> Vec<Source> {
    let mut out = Vec::new();
    let mut append = |part: &tenchu_assets::model::Part, matrix| {
        if let Some(source) = emitters.source(&part.geometry, || matrix) {
            out.push(source);
        }
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
    for (part, matrix) in
        super::static_models(level, math, environment.map(|e| e.static_phases.as_slice()))
    {
        append(part, matrix);
    }
    if let Some(environment) = environment {
        for object in &environment.objects {
            if object.kind >= 0
                && let Some(model) = object.model
            {
                append(
                    &level.objects[model],
                    Matrix {
                        translation: object.position,
                        rotation: math.rotate_yxz([0, object.yaw, 0]),
                    },
                );
            }
        }
    } else {
        for object in &level.scene.dynamic {
            let Some(model) = object.model else { continue };
            append(
                &level.objects[model],
                Matrix {
                    translation: object.position.into(),
                    rotation: math.rotate_yxz([0, object.facing, 0]),
                },
            );
        }
    }
    out
}

pub(super) fn defaults(level: &Level, math: &Math) -> Vec<Source> {
    let models = super::sources(
        &format!("map{}/props", level.id),
        &level.objects,
        Role::Prop,
    );
    let emitters = Emitters::new(&level.vram, models, &lights::defaults());
    sources(&emitters, level, math, None)
}
