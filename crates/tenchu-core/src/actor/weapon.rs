use super::{Actor, Context};
use crate::{Result, ids::ATTR_WEAPON_DRAWN, math::Matrix, scene::ModelId};
use tenchu_assets::weapon::kind::*;

#[derive(Clone, Debug)]
pub struct Weapon {
    pub kind: i16,
    pub model: ModelId,
    pub rendered: Matrix,
}

impl Actor {
    fn weapon(
        &mut self,
        ctx: &mut Context,
        part: usize,
        kind: i16,
        hand: Option<usize>,
        slot: Option<usize>,
    ) -> Result<()> {
        if let Some(hand) = hand {
            self.weapon_hitbox[hand] = ctx
                .mission
                .weapons
                .hitboxes
                .iter()
                .position(|h| h.kind == kind);
        }
        if let Some(slot) = slot
            && ctx.mission.weapons.models.contains_key(&kind)
        {
            let parent = *self
                .parts
                .get(part)
                .ok_or("Missing weapon attachment part")?;
            self.weapons[slot] = Some(Weapon {
                kind,
                model: ctx.scene.add(Some(parent), Matrix::default()),
                rendered: Matrix::default(),
            });
        }
        Ok(())
    }

    pub(super) fn setup_weapons(&mut self, ctx: &mut Context) -> Result<()> {
        let kind = ctx.mission.characters[self.definition].weapon;
        match kind {
            CLAW | FIST | JAW => {
                if kind != JAW {
                    self.weapon(ctx, 0, kind, Some(1), None)?;
                }
                self.weapon(ctx, 0, kind, Some(0), None)?;
            }
            JYUTE | EN | KODATI => {
                if kind != KODATI {
                    self.weapon(ctx, 0, kind + 1, None, Some(0))?;
                }
                self.weapon(ctx, 13, kind, Some(0), Some(2))?;
                self.weapon(ctx, 14, kind, Some(1), Some(3))?;
            }
            JYURUR | CROWR => {
                self.weapon(ctx, 13, kind, Some(0), Some(0))?;
                self.weapon(ctx, 14, kind + 1, Some(1), Some(1))?;
            }
            SABRE | ANDON | IKARI | BOU | YARI | KABUTUTI | SASUMATA | HALBERT | KON | NAGI
            | ENGETU | SEVEN => {
                if kind == SABRE {
                    self.weapon(ctx, 14, CLAW, Some(1), None)?;
                }
                self.weapon(ctx, 13, kind, Some(0), Some(0))?;
            }
            KEITOU => {
                self.weapon(ctx, 13, kind, Some(0), Some(2))?;
                self.weapon(ctx, 1, KEITOUB, None, Some(0))?;
            }
            KATANAL => {
                self.weapon(ctx, 13, kind, Some(0), Some(2))?;
                self.weapon(ctx, 14, SAYAL, None, Some(1))?;
                self.weapon(ctx, 14, TUKAL, None, Some(0))?;
            }
            KATANA_0 | HOUTOU | KATANA_1 | KOZUKA | NINJA => {
                if matches!(kind, KATANA_0 | HOUTOU | KATANA_1) {
                    self.weapon(ctx, 0, kind + 1, None, Some(1))?;
                    self.weapon(ctx, 0, kind + 2, None, Some(0))?;
                    for weapon in self.weapons[..2].iter().flatten() {
                        ctx.scene.set_translation(
                            weapon.model,
                            [
                                i32::from(ctx.mission.characters[self.definition].width / 3),
                                0,
                                0,
                            ],
                        );
                    }
                }
                self.weapon(ctx, 13, kind, Some(0), Some(2))?;
            }
            KATANA_2 => {
                self.weapon(ctx, 13, kind, Some(0), Some(2))?;
                self.weapon(ctx, 14, kind, Some(1), Some(3))?;
                self.weapon(ctx, 0, TUKAANI, None, Some(0))?;
            }
            TEPPO | GUN => self.weapon(ctx, 13, kind, None, Some(0))?,
            YUMI | KATAYUMI => {
                self.weapon(ctx, 13, kind, (kind == KATAYUMI).then_some(0), Some(0))?;
                self.weapon(ctx, 1, kind + 2, None, Some(1))?;
                self.weapon(ctx, 14, kind + 1, None, Some(2))?;
            }
            _ => {}
        }
        Ok(())
    }

    pub(crate) fn equip(&mut self, ctx: &mut Context, drawn: bool) {
        if (self.attribute & ATTR_WEAPON_DRAWN != 0) == drawn {
            return;
        }
        self.attribute ^= ATTR_WEAPON_DRAWN;
        match ctx.mission.characters[self.definition].weapon {
            KODATI | JYUTE | EN | KATANA_2 => {
                self.weapons.swap(0, 2);
                self.weapons.swap(1, 3);
            }
            KOZUKA | NINJA | KEITOU | KATANA_0 | HOUTOU | KATANA_1 => self.weapons.swap(0, 2),
            _ => {}
        }
    }
}
