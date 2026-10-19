mod hinged;
#[cfg(test)]
mod tests;

use crate::{
    Result,
    actor::Context,
    effects::{Drawing, Graphic, View},
    math::Matrix,
    scene::{ModelId, Scene},
    visibility,
};
use tenchu_assets::{
    misc::{Kind, Spawn},
    stage::MissionAssets,
};

#[derive(Clone, Debug)]
pub struct Object {
    pub position: [i32; 3],
    pub kind: Kind,
    pub paused: bool,
    pub mode: u8,
    pub count: i32,
    pub model: Option<ModelId>,
    pub angle: i16,
    pub speed: i16,
}

#[derive(Clone, Debug)]
pub struct Misc {
    pub objects: Vec<Object>,
    pub models: Vec<(usize, Matrix)>,
    pub drawings: Vec<Drawing>,
    doors: Vec<[Option<ModelId>; 2]>,
    pitfalls: Vec<[Option<ModelId>; 2]>,
    sprites: Vec<ModelId>,
}

impl Misc {
    pub fn new(mission: &MissionAssets, scene: &mut Scene) -> Self {
        let mut templates = |rows: &[tenchu_assets::misc::Hinged]| {
            rows.iter()
                .map(|row| {
                    row.models
                        .map(|model| model.map(|_| scene.add(Some(0), Matrix::default())))
                })
                .collect()
        };
        Self {
            objects: Vec::new(),
            models: Vec::new(),
            drawings: Vec::new(),
            doors: templates(&mission.misc.doors),
            pitfalls: templates(&mission.misc.pitfalls),
            sprites: mission
                .misc
                .sprites
                .iter()
                .map(|_| scene.add(Some(0), Matrix::default()))
                .collect(),
        }
    }

    pub fn spawn(&mut self, spawn: Spawn, ctx: &mut Context) {
        if self.objects.len() == 200 {
            return;
        }
        let model = match spawn.kind {
            Kind::TextureScroll {
                rect,
                source,
                velocity,
            } => {
                ctx.effects.texture_scroll(rect, source, velocity);
                return;
            }
            Kind::Door { rotation, .. } | Kind::Pitfall { rotation, .. } => Some(ctx.scene.add(
                Some(0),
                Matrix {
                    translation: spawn.position,
                    rotation: ctx.math.rotate_yxz([0, rotation, 0]),
                },
            )),
            _ => None,
        };
        self.objects.push(Object {
            position: spawn.position,
            count: if matches!(spawn.kind, Kind::Fire) {
                10
            } else {
                0
            },
            kind: spawn.kind,
            paused: true,
            mode: 0,
            model,
            angle: 0,
            speed: 0,
        });
    }

    pub fn step(&mut self, clock: i32, ctx: &mut Context) -> Result<()> {
        self.models.clear();
        self.drawings.clear();
        if clock % 10 == 0 {
            for object in &mut self.objects {
                let paused = !(0..3).all(|axis| {
                    ctx.camera.reference[axis]
                        .wrapping_sub(object.position[axis])
                        .wrapping_abs()
                        < 15000
                });
                if paused != object.paused {
                    object.pause(paused, ctx)?;
                    object.paused = paused;
                }
            }
        }
        ctx.drawing.fog = true;
        let view = View {
            position: ctx.camera.position,
            matrix: ctx.camera.view,
        };
        for object in &mut self.objects {
            if object.paused {
                continue;
            }
            let position = object.position;
            match object.kind {
                Kind::Fire if object.mode == 0 => {
                    object.count = object.count.wrapping_sub(1);
                    if object.count < 1 {
                        ctx.effects.explosion(position, [0, -35, 0], ctx.random);
                        ctx.effects
                            .embers(position, [75, 180, 75], 10, ctx.random)?;
                        ctx.effects
                            .smoke(position, [0, -200, 0], 20, 6, ctx.random)?;
                        object.count = ctx.random.next15() % 150;
                        ctx.effects.sounds.push((position, 0x28));
                    }
                }
                Kind::Puff(radius) if object.mode == 0 => {
                    let mut p = position;
                    for (axis, radius) in [0, 2].into_iter().zip(radius) {
                        let span = radius.wrapping_mul(2);
                        p[axis] = p[axis]
                            .wrapping_add(if span > 0 {
                                ctx.random.next15() % span
                            } else {
                                0
                            })
                            .wrapping_sub(radius);
                    }
                    if clock & 1 == 0 {
                        p[1] = p[1].wrapping_add(2000);
                        ctx.effects.smoke(p, [0, -400, 0], 1, 1, ctx.random)?;
                        p[1] = p[1].wrapping_sub(2000);
                        ctx.effects.splash(p, [16384, 8192], 10);
                    }
                }
                Kind::Sound { index, delay } if object.mode == 0 && object.count <= clock => {
                    ctx.effects.sounds.push((position, i16::from(index) + 0x44));
                    let [low, high] = delay.map(i32::from);
                    let delay = low
                        + if high > low {
                            ctx.random.next15() % (high - low)
                        } else {
                            0
                        };
                    object.count = clock.wrapping_add(delay);
                }
                Kind::Snowfall if clock & 3 == 0 => {
                    let velocity = [
                        ctx.random.next15() % 20 - 10,
                        ctx.random.next15() % 50 + 50,
                        ctx.random.next15() % 20 - 10,
                    ]
                    .map(|v| v as i16);
                    let position = [
                        ctx.camera.reference[0].wrapping_add(ctx.random.next15() % 6000 - 3000),
                        ctx.camera.reference[1].wrapping_add(ctx.random.next15() % 3000 - 6000),
                        ctx.camera.reference[2].wrapping_add(ctx.random.next15() % 6000 - 3000),
                    ];
                    ctx.effects
                        .snow(position, velocity, 4096, &ctx.mission.terrain, ctx.cursor)?;
                }
                Kind::Bonfire(scale) if object.mode == 0 => {
                    let color = (ctx.random.next15() % 100 + 100) as u8;
                    if let Some(drawing) =
                        view.sprite(position, Graphic::Flash((clock % 4) as i16), scale, color)
                    {
                        self.drawings.push(drawing);
                    }
                    if clock & 15 == 0 {
                        ctx.effects.bleeds(
                            position,
                            [0, -60, 0],
                            100,
                            10,
                            30,
                            [100, 100, 60],
                            ctx.random,
                        );
                    }
                    if clock % 79 == 0 {
                        ctx.effects.sounds.push((position, 0x47));
                    }
                }
                Kind::Sprite(kind) => {
                    let color = (ctx.random.next15() % 60 + 98) as u8;
                    let model = self.sprites[kind];
                    ctx.scene.update(
                        model,
                        Matrix {
                            translation: position,
                            ..Matrix::default()
                        },
                    );
                    let matrix = view.matrix.compose(&ctx.scene.world(model)?);
                    if let Some(clip) = visibility::clip(
                        &matrix,
                        ctx.scene.models[model].attribute,
                        ctx.scene.models[model].clip,
                    ) {
                        let depth = clip.depth as i16 - 5;
                        if depth > 0 {
                            let (image, scale) = ctx.mission.misc.sprites[kind];
                            let scale = ((scale >> 2).wrapping_mul(300) / i32::from(depth)) as i16;
                            self.drawings.push(Drawing {
                                graphic: Graphic::Sprite { image, blend: 2 },
                                xy: visibility::project(&matrix, [0; 3]).0,
                                scale: [i32::from(scale); 2],
                                color: [color; 3],
                                rotation: 0,
                                depth,
                            });
                        }
                    }
                }
                Kind::Door { kind, .. } => {
                    object.hinged(ctx, self.doors[kind], &mut self.models)?
                }
                Kind::Pitfall { kind, .. } => {
                    object.hinged(ctx, self.pitfalls[kind], &mut self.models)?
                }
                _ => {}
            }
        }
        ctx.drawing.bytes += self.drawings.iter().map(Drawing::bytes).sum::<u32>();
        Ok(())
    }
}
