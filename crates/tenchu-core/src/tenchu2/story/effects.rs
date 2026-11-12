use crate::{
    Result,
    math::{Math, Position},
    random::Random,
    tenchu2::{
        effects::{self, Effect, blood::Spurt},
        terrain::Cursor,
    },
};
use serde::{Deserialize, Serialize};
use tenchu_assets::tenchu2::{
    collision::Collision,
    effects::Rules,
    story::{Command, EffectKind},
};

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Effects {
    pub tick: u64,
    pub particles: effects::Effects,
    pub frame: effects::Frame,
    pub cursor: Cursor,
    #[serde(default)]
    pub sprites: Vec<(i16, effects::sprites::Drawing)>,
    #[serde(default)]
    pub colors: std::collections::BTreeMap<i16, u8>,
}

impl Effects {
    pub fn ignite(&mut self, bones: &[crate::scene::ModelId]) -> Result<()> {
        for &bone in bones {
            let mut sprite = effects::sprites::Sprites::new(12, Position::ZERO, 1, 120);
            sprite.parent = Some(bone);
            sprite.size = [[1024; 2]; 2];
            sprite.depth_offset = -16;
            sprite.emission = Some(effects::Emission::BurningActor);
            sprite.frames = (12..=15)
                .map(|image| effects::sprites::Frame { image, duration: 0 })
                .collect();
            self.particles.insert(Effect::Sprites(sprite))?;
        }
        Ok(())
    }

    pub(super) fn animate_sprites(
        &mut self,
        resources: &tenchu_assets::tenchu2::story::Resources,
        actors: &[super::Motion],
        view: crate::tenchu2::visibility::View,
        moving: bool,
        draw: bool,
    ) {
        if draw {
            self.sprites.clear();
        }
        for (motion, animation) in actors.iter().zip(&resources.script.actors) {
            let Some(Some(sprite)) = resources.effects.get(&animation.model) else {
                continue;
            };
            let color = self.colors.entry(animation.model).or_insert(128);
            if moving {
                *color = ((i32::from(motion.rotation[0][1]) + 2048) >> 4) as u8;
            }
            if !draw {
                continue;
            }
            let phase = self.tick.wrapping_sub(1) & 3;
            *color = match animation.model {
                286 => 128,
                287 => [128, 112, 96, 112][phase as usize],
                _ => match phase {
                    0 => color.saturating_add(16),
                    2 => color.saturating_sub(16),
                    _ => *color,
                },
            };
            // 80046b94 temporarily extends the source sprite culling distance.
            let view = crate::tenchu2::visibility::View {
                distance: view.distance + 300 * 16,
                ..view
            };
            let Some(depth) = view.actor(&crate::math::Matrix {
                translation: motion.position,
                ..Default::default()
            }) else {
                continue;
            };
            let scale = ((i32::from(sprite.size) * 75) << 10) / (i32::from(depth) * 4);
            self.sprites.push((
                animation.model,
                effects::sprites::Drawing {
                    blood: None,
                    contact: None,
                    image: 15,
                    position: motion.position,
                    size: [sprite.size; 2],
                    scale: [scale as i16; 2],
                    color: [*color; 3],
                    rotation: 0,
                    depth: depth as i16,
                    blend: effects::surface::Blend::Add,
                },
            ));
        }
    }

    pub fn dispatch(
        &mut self,
        command: &Command,
        math: &Math,
        rules: &Rules,
        terrain: &Collision,
        random: &mut Random,
        camera_direction: Position,
    ) -> Result<()> {
        let Command::Effect {
            kind,
            position,
            heading,
            value,
        } = *command
        else {
            return Ok(());
        };
        use EffectKind::*;
        let position = Position::from(position);
        match kind {
            Sparks => self.particles.story_sparks(
                position,
                camera_direction.map(i32::wrapping_neg),
                math,
                rules,
            )?,
            WaterSpray => self.particles.water_spray(position, 1024, random)?,
            Dust => self.particles.trail(
                position,
                math.velocity(heading, -128, 0).into(),
                0,
                false,
                random,
            )?,
            Leaves => {
                let mut velocity = Position::from(math.velocity(heading, 4096, 0).map(i32::from));
                velocity.y = 1024;
                for _ in 0..2 {
                    self.particles
                        .insert(Effect::Leaves(effects::paper::Leaves::new(
                            position, velocity, random,
                        )))?;
                }
            }
            DispersingSmoke => {
                self.particles
                    .dispersing_smoke(position, Position::new(0, -32, 0), 2, random)?
            }
            Explosion => self.particles.explosion(position, 1, math, random)?,
            Smoke => self.particles.smoke(position, 3, false, math, random)?,
            Splash => {
                self.particles.ripple(position, 30)?;
                self.particles.splash(position, 1024, math, random)?;
            }
            Blood | SlowBlood => {
                self.particles.blood(
                    Spurt {
                        position,
                        yaw: heading,
                        speed: if kind == Blood { 100 } else { 36 },
                        gravity: if kind == Blood { 20 } else { 6 },
                    },
                    math,
                    rules,
                    random,
                )?;
                if let Some(Effect::Blood(blood)) = self.particles.entries.last_mut() {
                    // The source copies the fourth command word along with XYZ.
                    for drop in &mut blood.drops {
                        if drop.age < blood.lifetime {
                            drop.floor = value;
                        }
                    }
                }
            }
            Pool => {
                self.particles.death_pool(
                    position.wrapping_sub(Position::new(0, 1000, 0)),
                    terrain,
                    &mut self.cursor,
                )?;
                if let Some(Effect::Pool(pool)) = self.particles.entries.last_mut() {
                    pool.lifetime = 3000;
                    if value == 2 {
                        pool.age = 60;
                    }
                }
            }
            BloodBurst | RadialBlood | GreenBlood | PaleBlood => {
                let color = match kind {
                    GreenBlood => [0, 96, 0],
                    PaleBlood => [128, 112, 112],
                    _ => [128, 0, 0],
                };
                for index in 0..if kind == PaleBlood { 2 } else { 4 } {
                    let direction = if kind == RadialBlood {
                        // The fourth source burst has a zero vector, not negative Z.
                        [[256, 0, 0], [-256, 0, 0], [0, 0, 256], [0; 3]][index]
                    } else if kind == PaleBlood {
                        [[256, 0, 0], [-256, 0, 0]][index]
                    } else {
                        math.velocity(heading, 4096, 0).map(|v| i32::from(v) >> 2)
                    };
                    self.particles.directional_blood(
                        position,
                        direction.into(),
                        color,
                        6,
                        rules,
                        random,
                    )?;
                }
            }
        }
        Ok(())
    }
}
